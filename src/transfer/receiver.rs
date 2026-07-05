use std::path::Path;

use tokio::io::AsyncWriteExt;
use thiserror::Error;

use crate::crypto;
use crate::protocol::{ChunkPayload, CompletePayload, PacketType, ProtocolError};
use crate::transfer::handshake::{self, recv_packet, send_packet};
use crate::transfer::manifest::Manifest;
use crate::transfer::session_manager::SessionManager;

pub async fn receive_folder(
    server: &str,
    code: &str,
    _chunk_size: usize,
    timeout_secs: u64,
    output: &Path,
) -> Result<(), ReceiveError> {
    let (_secret, public) = crypto::generate_keypair();
    use base64::Engine;
    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(public.as_bytes());

    let local_addr = "0.0.0.0:0".to_string();

    let sm = SessionManager::new(server);
    let sender = sm.join_session(code, &public_key_b64, &local_addr).await?;

    let sender_fingerprint = compute_fingerprint(&sender.public_key);

    println!("Sender fingerprint: {}", sender_fingerprint);
    println!("Verify this matches the sender's display.");
    println!("Waiting for sender to approve...");

    sm.wait_for_approval(code, timeout_secs).await?;
    println!("Approved! Connecting to sender...");

    let addr = sender.addr.clone();
    let connect_fut = tokio::net::TcpStream::connect(&addr);
    let mut stream = tokio::time::timeout(
        tokio::time::Duration::from_secs(15),
        connect_fut,
    )
    .await
    .map_err(|_| ReceiveError::Timeout)?
    .map_err(ReceiveError::Io)?;

    println!("Connected. Performing key exchange...");
    let hs = handshake::receiver_handshake(&mut stream, code).await?;
    let verify_hex = hex::encode_upper(hs.verification_hash);
    println!(
        "Session fingerprint: {} {}  {} {}",
        &verify_hex[0..2],
        &verify_hex[2..4],
        &verify_hex[4..6],
        &verify_hex[6..8]
    );

    println!("Receiving manifest...");
    let manifest_pkt = recv_packet(&mut stream, PacketType::Manifest).await?;
    let aad_manifest = make_aad(PacketType::Manifest, code);
    let nonce_manifest = crypto::make_nonce(&hs.nonce_base, 1);
    let manifest_json = crypto::decrypt(
        &hs.session_keys,
        &nonce_manifest,
        &manifest_pkt.payload,
        &aad_manifest,
    )?;

    let manifest: Manifest = serde_json::from_slice(&manifest_json)
        .map_err(ReceiveError::Serialize)?;

    send_packet(&mut stream, PacketType::Ack, &[]).await?;

    let total_files = manifest.files.len();
    let total_bytes: u64 = manifest.files.iter().map(|f| f.size).sum();
    println!(
        "Receiving {} files ({} bytes total)",
        total_files, total_bytes
    );

    std::fs::create_dir_all(output)
        .map_err(ReceiveError::Io)?;

    let mut seq: u64 = 2;
    for file in &manifest.files {
        let file_path = output.join(&file.path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(ReceiveError::Io)?;
        }

        let mut output_file = tokio::fs::File::create(&file_path)
            .await
            .map_err(ReceiveError::Io)?;
        let mut hasher = blake3::Hasher::new();
        let mut remaining = file.size;

        eprint!("  {}... ", file.path);

        while remaining > 0 {
            let chunk_pkt = recv_packet(&mut stream, PacketType::Chunk).await?;
            let aad_chunk = make_aad(PacketType::Chunk, code);
            let nonce_chunk = crypto::make_nonce(&hs.nonce_base, seq);
            let decrypted = crypto::decrypt(
                &hs.session_keys,
                &nonce_chunk,
                &chunk_pkt.payload,
                &aad_chunk,
            )?;

            let chunk = ChunkPayload::decode(&decrypted)?;

            let chunk_data = &chunk.data;
            let write_size = chunk_data.len().min(remaining as usize);
            output_file
                .write_all(&chunk_data[..write_size])
                .await
                .map_err(ReceiveError::Io)?;
            hasher.update(&chunk_data[..write_size]);
            remaining -= write_size as u64;

            send_packet(&mut stream, PacketType::Ack, &[]).await?;
            seq += 1;
        }

        output_file.flush().await.map_err(ReceiveError::Io)?;

        let file_hash = *hasher.finalize().as_bytes();
        if file_hash != file.hash {
            return Err(ReceiveError::Integrity(file.path.clone()));
        }

        eprintln!("done");
    }

    let complete_pkt = recv_packet(&mut stream, PacketType::Complete).await?;
    let aad_complete = make_aad(PacketType::Complete, code);
    let nonce_complete = crypto::make_nonce(&hs.nonce_base, seq);
    let complete_decrypted = crypto::decrypt(
        &hs.session_keys,
        &nonce_complete,
        &complete_pkt.payload,
        &aad_complete,
    )?;
    let complete = CompletePayload::decode(&complete_decrypted)?;

    if complete.root_hash != manifest.root_hash {
        return Err(ReceiveError::RootHashMismatch);
    }

    send_packet(&mut stream, PacketType::Ack, &[]).await?;

    println!("Verification OK. Root hash matches.");
    println!("Transfer complete! Files saved to: {}",
        output.canonicalize().unwrap_or_else(|_| output.to_path_buf()).display()
    );

    Ok(())
}

fn make_aad(ptype: PacketType, session_code: &str) -> Vec<u8> {
    let mut aad = vec![ptype as u8];
    aad.extend_from_slice(session_code.as_bytes());
    aad
}

fn compute_fingerprint(public_key_b64: &str) -> String {
    let hash = blake3::hash(public_key_b64.as_bytes());
    hex::encode_upper(&hash.as_bytes()[..4])
}

#[derive(Error, Debug)]
pub enum ReceiveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("session error: {0}")]
    Session(#[from] crate::transfer::session_manager::SessionError),

    #[error("handshake error: {0}")]
    Handshake(#[from] crate::transfer::handshake::HandshakeError),

    #[error("crypto error: {0}")]
    Crypto(#[from] crate::crypto::CryptoError),

    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("bad public key from sender")]
    BadPublicKey,

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("connection timeout")]
    Timeout,

    #[error("integrity check failed for: {0}")]
    Integrity(String),

    #[error("root hash mismatch")]
    RootHashMismatch,
}

impl From<ProtocolError> for ReceiveError {
    fn from(e: ProtocolError) -> Self {
        ReceiveError::Protocol(e.to_string())
    }
}
