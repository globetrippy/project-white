//! Cryptographic operations for Project White.
//!
//! # Security guarantees
//!
//! - All session keys are ephemeral (per-session X25519 keypairs).
//! - Keys are zeroed on drop via `Zeroize`.
//! - Process memory is locked to prevent swap disclosure (best-effort).
//! - All symmetric encryption uses ChaCha20-Poly1305 (AEAD).
//! - Nonces are derived deterministically from a random base + sequence
//!   number, preventing reuse within a session.
//! - AEAD associated data binds ciphertext to a specific session and
//!   packet type, preventing cross-session replay.
//! - Key derivation uses HKDF-SHA256 with the session code as salt
//!   (domain separation per session).
//!
//! # Threat model
//!
//! See `docs/SECURITY.md` for the full threat model.
//! Only industry-standard primitives are used.
//! No cryptographic algorithms are invented here.

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};
use hkdf::Hkdf;
use rand_core::OsRng;
use sha2::Sha256;
use thiserror::Error;
use x25519_dalek::{EphemeralSecret, PublicKey};
use zeroize::Zeroize;

// ─── Re-exports ─────────────────────────────────────────────

pub use x25519_dalek::SharedSecret;

// ─── Session Keys ──────────────────────────────────────────

/// Ephemeral session key material.
///
/// Derived once per session from the ECDH shared secret.
/// Automatically zeroed on drop.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SessionKeys {
    /// 32-byte key for ChaCha20-Poly1305 encryption.
    pub encryption_key: [u8; 32],
    /// 32-byte key for session authentication / future HMAC use.
    pub auth_key: [u8; 32],
}

impl SessionKeys {
    fn new(encryption_key: [u8; 32], auth_key: [u8; 32]) -> Self {
        Self {
            encryption_key,
            auth_key,
        }
    }
}

// ─── Key Generation ────────────────────────────────────────

/// Generate an ephemeral X25519 keypair for a single session.
///
/// The secret key is held in memory and zeroed on drop.
/// The public key is sent to the peer during the handshake.
pub fn generate_keypair() -> (EphemeralSecret, PublicKey) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    (secret, public)
}

// ─── Key Exchange ──────────────────────────────────────────

/// Perform X25519 ECDH key exchange.
///
/// Both sides derive the same `SharedSecret`.
/// The shared secret is used as input to HKDF session key derivation.
pub fn key_exchange(secret: EphemeralSecret, public: &PublicKey) -> SharedSecret {
    secret.diffie_hellman(public)
}

// ─── Session Key Derivation ────────────────────────────────

/// Derive session encryption and auth keys from the ECDH shared secret.
///
/// Uses HKDF-SHA256 with:
/// - `salt` = BLAKE3(session_code) — binds keys to a specific session,
///   preventing cross-session key reuse and providing domain separation.
/// - `ikm` = ECDH shared secret (32 bytes)
/// - `info` = `b"pw-v1-session"` (domain separation string)
///
/// Output: 64 bytes (32 encryption key + 32 auth key).
pub fn derive_session_keys(
    shared_secret: &SharedSecret,
    session_code: &str,
) -> SessionKeys {
    let salt = blake3::hash(session_code.as_bytes());
    let hk = Hkdf::<Sha256>::new(Some(salt.as_bytes()), shared_secret.as_bytes());
    let mut okm = [0u8; 64];
    hk.expand(b"pw-v1-session", &mut okm)
        .expect("64 bytes is a valid HKDF output length");

    let mut encryption_key = [0u8; 32];
    let mut auth_key = [0u8; 32];
    encryption_key.copy_from_slice(&okm[..32]);
    auth_key.copy_from_slice(&okm[32..]);

    okm.zeroize();

    SessionKeys::new(encryption_key, auth_key)
}

// ─── Nonce Generation ──────────────────────────────────────

/// Derive a 12-byte AEAD nonce from the session nonce base and
/// a sequence number.
///
/// Construction: `BLAKE3(nonce_base || u64::to_be_bytes(seq))[..12]`
///
/// This ensures:
/// - Non-unique nonce base across sessions is not a problem (each session
///   has a different key).
/// - Within a session, each unique sequence number produces a unique nonce.
/// - 12 bytes fits the ChaCha20-Poly1305 nonce size exactly.
pub fn make_nonce(nonce_base: &[u8; 8], sequence: u64) -> [u8; 12] {
    let mut input = [0u8; 16];
    input[..8].copy_from_slice(nonce_base);
    input[8..].copy_from_slice(&sequence.to_be_bytes());
    let hash = blake3::hash(&input);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&hash.as_bytes()[..12]);
    nonce
}

// ─── AEAD Encryption / Decryption ─────────────────────────

/// Encrypt plaintext with ChaCha20-Poly1305.
///
/// # Arguments
///
/// * `keys` - Session keys (uses `encryption_key`)
/// * `nonce` - 12-byte nonce (use `make_nonce`)
/// * `plaintext` - Data to encrypt
/// * `aad` - Additional authenticated data (bound to ciphertext)
///
/// Returns ciphertext with the 16-byte authentication tag appended.
///
/// Per the architecture, AAD should be:
/// `[packet_type_byte] + session_id.as_bytes()`.
pub fn encrypt(
    keys: &SessionKeys,
    nonce: &[u8; 12],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let key = Key::from_slice(&keys.encryption_key);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(nonce, Payload { msg: plaintext, aad })
        .map_err(|_| CryptoError::EncryptionFailed)
}

/// Decrypt ciphertext with ChaCha20-Poly1305.
///
/// Returns the original plaintext on success.
/// Returns `CryptoError::DecryptionFailed` if authentication fails
/// (tampered data, wrong key, wrong nonce, or wrong AAD).
pub fn decrypt(
    keys: &SessionKeys,
    nonce: &[u8; 12],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let key = Key::from_slice(&keys.encryption_key);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, Payload { msg: ciphertext, aad })
        .map_err(|_| CryptoError::DecryptionFailed)
}

// ─── Hashing ───────────────────────────────────────────────

/// Compute a BLAKE3 hash of arbitrary data.
///
/// Used for:
/// - File integrity verification (manifest contains per-file hashes)
/// - Root hash computation (BLAKE3 of all file hashes concatenated)
/// - HKDF salt (BLAKE3 of session code)
/// - Nonce derivation (BLAKE3 of nonce_base || seq)
pub fn hash_data(data: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(data);
    *hash.as_bytes()
}

/// Compute the session verification hash.
///
/// Both sides compute this independently and display it.
/// Users compare the short string out of band.
///
/// Construction: `BLAKE3(shared_secret || "pw-v1-verify")[..8]`
pub fn session_verification_hash(shared_secret: &SharedSecret) -> [u8; 8] {
    let mut input = Vec::with_capacity(32 + 13);
    input.extend_from_slice(shared_secret.as_bytes());
    input.extend_from_slice(b"pw-v1-verify");
    let hash = blake3::hash(&input);
    let mut result = [0u8; 8];
    result.copy_from_slice(&hash.as_bytes()[..8]);
    result
}

/// Format a verification hash as a human-readable fingerprint.
///
/// Example output: `"8D F6 7H 2A  9C 4E 1B 5F"`
pub fn format_fingerprint(hash: &[u8; 8]) -> String {
    let hex_str = hex::encode_upper(hash);
    let mut formatted = String::with_capacity(23);
    let chars: Vec<char> = hex_str.chars().collect();
    for (i, chunk) in chars.chunks(2).enumerate() {
        if i > 0 && i % 4 == 0 {
            formatted.push(' ');
            formatted.push(' ');
        } else if i > 0 {
            formatted.push(' ');
        }
        formatted.extend(chunk);
    }
    formatted
}

// ─── Memory Locking ────────────────────────────────────────

/// Attempt to lock the process memory to prevent key material from
/// being paged to disk.
///
/// This is a best-effort hardening measure. If it fails (e.g.,
/// insufficient privileges or unsupported platform), a warning is
/// returned but the process continues.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn lock_memory() -> Result<(), String> {
    let ret = unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) };
    if ret == 0 {
        Ok(())
    } else {
        Err(format!(
            "mlockall failed: {}",
            std::io::Error::last_os_error()
        ))
    }
}

/// Memory locking is not supported on this platform.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn lock_memory() -> Result<(), String> {
    Err("memory locking not supported on this platform".into())
}

// ─── Errors ─────────────────────────────────────────────────

#[derive(Error, Debug, Clone, PartialEq)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptionFailed,

    #[error("decryption failed (tampered data or wrong key)")]
    DecryptionFailed,
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let (secret, public) = generate_keypair();
        let public_bytes: [u8; 32] = public.to_bytes();
        assert_eq!(public_bytes.len(), 32);
        // secret is consumed below, so we just verify public is valid
        let _ = secret;
    }

    #[test]
    fn test_key_exchange_produces_matching_shared_secrets() {
        let (alice_secret, alice_public) = generate_keypair();
        let (bob_secret, bob_public) = generate_keypair();

        let alice_shared = key_exchange(alice_secret, &bob_public);
        let bob_shared = key_exchange(bob_secret, &alice_public);

        assert_eq!(
            alice_shared.as_bytes(),
            bob_shared.as_bytes(),
            "both sides must derive the same shared secret"
        );
    }

    #[test]
    fn test_key_exchange_different_keys_produce_different_secrets() {
        let (alice_secret, alice_public) = generate_keypair();
        let (bob_secret, bob_public) = generate_keypair();
        let (eve_secret, eve_public) = generate_keypair();

        let alice_bob = key_exchange(alice_secret, &bob_public);
        let alice_eve = key_exchange(eve_secret, &alice_public);

        assert_ne!(
            alice_bob.as_bytes(),
            alice_eve.as_bytes(),
            "different peers must produce different shared secrets"
        );
        // bob_secret and eve_secret consumed above
        let _ = (bob_secret, bob_public, eve_public, alice_public);
    }

    #[test]
    fn test_session_key_derivation_deterministic() {
        let (alice_secret, bob_public) = {
            let (s, _) = generate_keypair();
            let (_, p) = generate_keypair();
            (s, p)
        };
        let shared = key_exchange(alice_secret, &bob_public);
        let session_code = "test-code-123";

        let keys1 = derive_session_keys(&shared, session_code);
        let keys2 = derive_session_keys(&shared, session_code);

        assert_eq!(keys1.encryption_key, keys2.encryption_key);
        assert_eq!(keys1.auth_key, keys2.auth_key);
    }

    #[test]
    fn test_session_key_derivation_different_codes_produce_different_keys() {
        let (alice_secret, bob_public) = {
            let (s, _) = generate_keypair();
            let (_, p) = generate_keypair();
            (s, p)
        };
        let shared = key_exchange(alice_secret, &bob_public);

        let keys_a = derive_session_keys(&shared, "code-A");
        let keys_b = derive_session_keys(&shared, "code-B");

        assert_ne!(keys_a.encryption_key, keys_b.encryption_key);
        assert_ne!(keys_a.auth_key, keys_b.auth_key);
    }

    #[test]
    fn test_nonce_unique_per_sequence() {
        let nonce_base = [0xAB; 8];
        let nonce_1 = make_nonce(&nonce_base, 1);
        let nonce_2 = make_nonce(&nonce_base, 2);
        assert_ne!(nonce_1, nonce_2);
    }

    #[test]
    fn test_nonce_length() {
        let nonce = make_nonce(&[0; 8], 0);
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (alice_secret, bob_public) = {
            let (s, _) = generate_keypair();
            let (_, p) = generate_keypair();
            (s, p)
        };
        let shared = key_exchange(alice_secret, &bob_public);
        let keys = derive_session_keys(&shared, "roundtrip-test");
        let nonce = make_nonce(&[0x01; 8], 1);

        let plaintext = b"Hello, Project White!";
        let aad = b"\x05session-1234";

        let ciphertext = encrypt(&keys, &nonce, plaintext, aad).unwrap();
        assert_ne!(ciphertext, plaintext, "ciphertext must differ from plaintext");

        let decrypted = decrypt(&keys, &nonce, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let (alice_secret, bob_public) = {
            let (s, _) = generate_keypair();
            let (_, p) = generate_keypair();
            (s, p)
        };
        let shared = key_exchange(alice_secret, &bob_public);

        let (other_secret, other_public) = generate_keypair();
        let other_shared = key_exchange(other_secret, &other_public);

        let keys_good = derive_session_keys(&shared, "test");
        let keys_bad = derive_session_keys(&other_shared, "test");
        let nonce = make_nonce(&[0x02; 8], 0);

        let ciphertext = encrypt(&keys_good, &nonce, b"secret data", b"aad").unwrap();
        let result = decrypt(&keys_bad, &nonce, &ciphertext, b"aad");
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn test_decrypt_wrong_aad_fails() {
        let (alice_secret, bob_public) = {
            let (s, _) = generate_keypair();
            let (_, p) = generate_keypair();
            (s, p)
        };
        let shared = key_exchange(alice_secret, &bob_public);
        let keys = derive_session_keys(&shared, "aad-test");
        let nonce = make_nonce(&[0x03; 8], 0);

        let ciphertext = encrypt(&keys, &nonce, b"secret", b"session-A").unwrap();
        let result = decrypt(&keys, &nonce, &ciphertext, b"session-B");
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn test_decrypt_tampered_ciphertext_fails() {
        let (alice_secret, bob_public) = {
            let (s, _) = generate_keypair();
            let (_, p) = generate_keypair();
            (s, p)
        };
        let shared = key_exchange(alice_secret, &bob_public);
        let keys = derive_session_keys(&shared, "tamper-test");
        let nonce = make_nonce(&[0x04; 8], 0);

        let mut ciphertext = encrypt(&keys, &nonce, b"sensitive", b"aad").unwrap();
        // Flip a bit in the ciphertext
        ciphertext[0] ^= 0xFF;
        let result = decrypt(&keys, &nonce, &ciphertext, b"aad");
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn test_hash_data() {
        let hash = hash_data(b"hello");
        assert_eq!(hash.len(), 32);
        // Deterministic
        assert_eq!(hash_data(b"hello"), hash_data(b"hello"));
        assert_ne!(hash_data(b"hello"), hash_data(b"world"));
    }

    #[test]
    fn test_session_verification_hash_deterministic() {
        let (alice_secret, bob_public) = {
            let (s, _) = generate_keypair();
            let (_, p) = generate_keypair();
            (s, p)
        };
        let shared = key_exchange(alice_secret, &bob_public);
        let (_, other_public) = generate_keypair();

        let hash_a = session_verification_hash(&shared);
        let hash_b = session_verification_hash(&shared);
        assert_eq!(hash_a, hash_b);

        // Different shared secret produces different hash
        let other_shared_a = {
            let (s, _) = generate_keypair();
            s.diffie_hellman(&other_public)
        };
        let hash_c = session_verification_hash(&other_shared_a);
        assert_ne!(hash_a, hash_c);
    }

    #[test]
    fn test_format_fingerprint() {
        let hash = [0x8D, 0xF6, 0x7B, 0x2A, 0x9C, 0x4E, 0x1B, 0x5F];
        let formatted = format_fingerprint(&hash);
        assert_eq!(formatted, "8D F6 7B 2A  9C 4E 1B 5F");
    }

    #[test]
    fn test_memory_locking_best_effort() {
        // Should not panic or crash
        let _ = lock_memory();
    }
}
