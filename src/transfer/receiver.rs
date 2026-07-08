use std::path::Path;

use tokio::io::AsyncWriteExt;
use tokio::time::{timeout, Duration};
use thiserror::Error;

/// Timeout for individual packet recv/send during data transfer (seconds).
const PACKET_TIMEOUT_SECS: u64 = 30;

use crate::crypto;
use crate::protocol::{ChunkPayload, CompletePayload, PacketType, ProtocolError};
use crate::transfer::handshake::{self, recv_packet, send_packet};
use crate::transfer::manifest::Manifest;
use crate::transfer::session_manager::SessionManager;
use crate::ui;

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
    ui::success(&format!("Joined session {}", code));

    let sender_fingerprint = compute_fingerprint(&sender.public_key);
    ui::fingerprint("sender fingerprint:", &sender_fingerprint);
    ui::info("Verify this matches the sender's display.");

    let spinner = ui::new_spinner("Waiting for sender to approve...");
    sm.wait_for_approval(code, timeout_secs).await?;
    spinner.finish_and_clear();

    ui::success("Approved");

    let spinner = ui::new_spinner("Connecting to sender...");
    let addr = sender.addr.clone();
    let connect_fut = tokio::net::TcpStream::connect(&addr);
    let mut stream = tokio::time::timeout(
        tokio::time::Duration::from_secs(15),
        connect_fut,
    )
    .await
    .map_err(|_| ReceiveError::Timeout)?
    .map_err(ReceiveError::Io)?;
    spinner.finish_and_clear();

    let spinner = ui::new_spinner("Performing key exchange...");
    let hs = handshake::receiver_handshake(&mut stream, code).await?;
    spinner.finish_and_clear();

    ui::success("Key exchange complete");
    let verify_hex = hex::encode_upper(hs.verification_hash);
    ui::fingerprint(
        "session fingerprint:",
        &format!(
            "{} {}  {} {}",
            &verify_hex[0..2],
            &verify_hex[2..4],
            &verify_hex[4..6],
            &verify_hex[6..8]
        ),
    );

    ui::info("Receiving manifest...");
    let manifest_pkt = recv_packet(&mut stream, PacketType::Manifest).await?;
    let aad_manifest = make_aad(PacketType::Manifest, code);
    let nonce_manifest = crypto::make_nonce(&hs.nonce_base, 1);
    let manifest_json = crypto::decrypt(
        &hs.session_keys,
        &nonce_manifest,
        &manifest_pkt.payload,
        &aad_manifest,
    )?;

    let manifest: Manifest =
        serde_json::from_slice(&manifest_json).map_err(ReceiveError::Serialize)?;

    send_packet(&mut stream, PacketType::Ack, &[]).await?;

    let total_files = manifest.files.len();
    let total_bytes: u64 = manifest.files.iter().map(|f| f.size).sum();

    ui::success(&format!(
        "Receiving {} file{} ({})",
        total_files,
        if total_files == 1 { "" } else { "s" },
        format_bytes(total_bytes)
    ));

    std::fs::create_dir_all(output).map_err(ReceiveError::Io)?;

    // Validate manifest paths: reject any with parent-dir (..) or root-dir (/)
    // components that could escape the output directory.
    for file in &manifest.files {
        for component in Path::new(&file.path).components() {
            match component {
                std::path::Component::ParentDir | std::path::Component::RootDir => {
                    return Err(ReceiveError::Protocol(format!(
                        "path traversal detected in manifest: {}",
                        file.path
                    )));
                }
                _ => {}
            }
        }
    }

    let pb = ui::new_progress_bar(total_bytes);
    let mut seq: u64 = 2;

    for file in &manifest.files {
        let file_path = output.join(&file.path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(ReceiveError::Io)?;
        }

        let mut output_file =
            tokio::fs::File::create(&file_path)
                .await
                .map_err(ReceiveError::Io)?;
        let mut hasher = blake3::Hasher::new();
        let mut remaining = file.size;

        pb.set_message(file.path.clone());

        while remaining > 0 {
            let chunk_pkt = timeout(
                Duration::from_secs(PACKET_TIMEOUT_SECS),
                recv_packet(&mut stream, PacketType::Chunk),
            )
            .await
            .map_err(|_| ReceiveError::Protocol("chunk recv timeout".into()))?
            .map_err(ReceiveError::Handshake)?;
            let aad_chunk = make_aad(PacketType::Chunk, code);
            let nonce_chunk = crypto::make_nonce(&hs.nonce_base, seq);
            let decrypted = crypto::decrypt(
                &hs.session_keys,
                &nonce_chunk,
                &chunk_pkt.payload,
                &aad_chunk,
            )?;

            let chunk = ChunkPayload::decode(&decrypted)?;

            // Validate chunk sequence matches expected counter.
            // (Nonce derivation also depends on seq, providing cryptographic
            //  protection; this is defense-in-depth against logic errors.)
            if chunk.sequence != seq {
                return Err(ReceiveError::Protocol(format!(
                    "chunk sequence mismatch: expected {}, got {}",
                    seq, chunk.sequence
                )));
            }

            let chunk_data = &chunk.data;
            let write_size = chunk_data.len().min(remaining as usize);
            output_file
                .write_all(&chunk_data[..write_size])
                .await
                .map_err(ReceiveError::Io)?;
            hasher.update(&chunk_data[..write_size]);
            remaining -= write_size as u64;

            timeout(
                Duration::from_secs(PACKET_TIMEOUT_SECS),
                send_packet(&mut stream, PacketType::Ack, &[]),
            )
            .await
            .map_err(|_| ReceiveError::Protocol("ack send timeout".into()))?
            .map_err(ReceiveError::Handshake)?;
            seq += 1;

            pb.inc(write_size as u64);
        }

        output_file
            .flush()
            .await
            .map_err(ReceiveError::Io)?;

        let file_hash = *hasher.finalize().as_bytes();
        if file_hash != file.hash {
            return Err(ReceiveError::Integrity(file.path.clone()));
        }
    }

    pb.finish_and_clear();

    let complete_pkt = timeout(
        Duration::from_secs(PACKET_TIMEOUT_SECS),
        recv_packet(&mut stream, PacketType::Complete),
    )
    .await
    .map_err(|_| ReceiveError::Protocol("complete recv timeout".into()))?
    .map_err(ReceiveError::Handshake)?;
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

    timeout(
        Duration::from_secs(PACKET_TIMEOUT_SECS),
        send_packet(&mut stream, PacketType::Ack, &[]),
    )
    .await
    .map_err(|_| ReceiveError::Protocol("complete ack send timeout".into()))?
    .map_err(ReceiveError::Handshake)?;

    let output_path = output
        .canonicalize()
        .unwrap_or_else(|_| output.to_path_buf());
    ui::success("Transfer complete");
    ui::detail("saved to:", &output_path.display().to_string());

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

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
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
