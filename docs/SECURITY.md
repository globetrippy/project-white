# Security Model

## Assumptions

- Network is malicious (WiFi, ISP, carrier).
- Signaling server is malicious (or compromised).
- Only sender and receiver machines are trusted.
- Private keys never leave the local device.

## Threat Model

| Threat | Mitigation |
|--------|------------|
| Eavesdropping on network | All data is AEAD-encrypted (ChaCha20-Poly1305). No plaintext on wire. |
| Man-in-the-middle on signaling | Session code + interactive key verification. Both sides compare fingerprint out of band. |
| Compromised signaling server | Server only sees public keys and IP addresses. Cannot decrypt data. Session code is bound into HKDF salt. |
| Replay attack | Nonces derived from per-session random base + sequence number. AEAD authentication prevents replay. |
| Malicious file injection | Each chunk is authenticated via AEAD. Receiver verifies hashes against encrypted manifest. |
| Session code brute force | 47 bits entropy + server rate limit (10 join attempts/minute). |
| Key leakage via swap | `mlockall()` prevents paging of process memory. |
| Key leakage after use | `zeroize` on drop for all secret key material. |
| Cross-session replay | AEAD AAD includes packet type byte + session UUID. Encrypted payloads cannot be replayed in other sessions. |

## Key Lifecycle

1. **Generation:** Ephemeral X25519 keypair created per session via `OsRng`.
2. **Exchange:** Public keys sent during handshake over direct TCP connection.
3. **Derivation:** ECDH → HKDF (salt = BLAKE3(session_code), info = "pw-v1-session").
4. **Usage:** Session keys held in memory only, protected by `mlockall()`.
5. **Destruction:** `zeroize` on drop + Rust drop semantics.

## What V1 Does NOT Protect Against

- **Traffic analysis:** An observer can see that a transfer occurred, its size, and the endpoints.
- **Metadata leakage:** File names are encrypted, but file counts and sizes may be inferred from chunk counts. (Mitigation: padding in future version.)
- **Denial of service:** A peer can drop the connection mid-transfer.
- **Compromised local machine:** If the attacker has access to the sender/receiver machine, all protections are void.
