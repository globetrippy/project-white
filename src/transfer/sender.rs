use std::io::Read;
use std::path::Path;

use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};
use thiserror::Error;

/// Timeout for individual packet sends/ack receives during data transfer (seconds).
/// Prevents connection hangs when the peer disappears mid-transfer.
const PACKET_TIMEOUT_SECS: u64 = 30;

use crate::crypto;
use crate::protocol::{ChunkPayload, CompletePayload, PacketType, ProtocolError};
use crate::transfer::handshake::{self, send_packet};
use crate::transfer::manifest;
use crate::transfer::session_manager::SessionManager;
use crate::ui;

pub async fn send_folder(
    server: &str,
    path: &Path,
    chunk_size: usize,
    timeout_secs: u64,
    public_ip: Option<String>,
    yes: bool,
) -> Result<(), SendError> {
    let (_secret, public) = crypto::generate_keypair();
    use base64::Engine;
    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(public.as_bytes());

    let listener = TcpListener::bind("0.0.0.0:0")
        .await
        .map_err(SendError::Io)?;
    let local_addr = listener.local_addr().map_err(SendError::Io)?;

    let addr_str = match public_ip {
        Some(ref ip) => format!("{}:{}", ip, local_addr.port()),
        None => {
            let local_ip = get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string());
            format!("{}:{}", local_ip, local_addr.port())
        }
    };

    let sm = SessionManager::new(server);
    let session = sm.create_session(&public_key_b64, &addr_str).await?;
    ui::success(&format!("Session created: {}", session.code));

    let spinner = ui::new_spinner("Waiting for receiver to connect...");
    let poll = sm.wait_for_receiver(&session.code, timeout_secs).await?;
    let _receiver = poll.receiver.ok_or(SendError::NoReceiver)?;
    spinner.finish_and_clear();

    ui::success("Receiver connected");
    ui::fingerprint(
        "receiver fingerprint:",
        &poll.receiver_fingerprint
            .unwrap_or_else(|| "unknown".to_string()),
    );

    if !yes {
        println!("    Accept connection? [Y/n]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();
        if input == "n" || input == "no" {
            sm.delete_session(&session.code).await.ok();
            return Err(SendError::Rejected);
        }
    }

    sm.approve_session(&session.code, &session.session_id)
        .await?;
    ui::success("Approved");

    let spinner = ui::new_spinner("Waiting for receiver connection...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let (mut stream, _) = listener
        .accept()
        .await
        .map_err(SendError::Io)?;
    spinner.finish_and_clear();

    let spinner = ui::new_spinner("Performing key exchange...");
    let hs = handshake::sender_handshake(&mut stream, &session.code).await?;
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

    ui::info("Scanning files...");
    let manifest = manifest::build_manifest(path)?;
    let total_files = manifest.files.len();
    let total_bytes: u64 = manifest.files.iter().map(|f| f.size).sum();

    if total_files == 0 {
        return Err(SendError::NoFiles);
    }

    ui::success(&format!(
        "Found {} file{} ({})",
        total_files,
        if total_files == 1 { "" } else { "s" },
        format_bytes(total_bytes)
    ));

    let manifest_json =
        serde_json::to_vec(&manifest).map_err(SendError::Serialize)?;
    let aad_manifest = make_aad(PacketType::Manifest, &session.code);
    let nonce_manifest = crypto::make_nonce(&hs.nonce_base, 1);
    let encrypted_manifest = crypto::encrypt(
        &hs.session_keys,
        &nonce_manifest,
        &manifest_json,
        &aad_manifest,
    )?;
    timeout(
        Duration::from_secs(PACKET_TIMEOUT_SECS),
        send_packet(&mut stream, PacketType::Manifest, &encrypted_manifest),
    )
    .await
    .map_err(|_| SendError::Protocol("manifest send timeout".into()))?
    .map_err(SendError::Handshake)?;
    let _ack = timeout(
        Duration::from_secs(PACKET_TIMEOUT_SECS),
        handshake::recv_packet(&mut stream, PacketType::Ack),
    )
    .await
    .map_err(|_| SendError::Protocol("manifest ack timeout".into()))?
    .map_err(SendError::Handshake)?;

    let pb = ui::new_progress_bar(total_bytes);
    let mut seq: u64 = 2;

    for file in &manifest.files {
        let file_path = path.join(&file.path);
        let mut disk_file = std::fs::File::open(&file_path).map_err(SendError::Io)?;
        let mut file_buf = vec![0u8; chunk_size];

        pb.set_message(file.path.clone());

        loop {
            let n = disk_file.read(&mut file_buf).map_err(SendError::Io)?;
            if n == 0 {
                break;
            }

            let chunk_data = ChunkPayload {
                sequence: seq,
                data: file_buf[..n].to_vec(),
            };
            let aad_chunk = make_aad(PacketType::Chunk, &session.code);
            let nonce_chunk = crypto::make_nonce(&hs.nonce_base, seq);
            let encrypted_chunk = crypto::encrypt(
                &hs.session_keys,
                &nonce_chunk,
                &chunk_data.encode(),
                &aad_chunk,
            )?;
            timeout(
                Duration::from_secs(PACKET_TIMEOUT_SECS),
                send_packet(&mut stream, PacketType::Chunk, &encrypted_chunk),
            )
            .await
            .map_err(|_| SendError::Protocol("chunk send timeout".into()))?
            .map_err(SendError::Handshake)?;

            let _ack = timeout(
                Duration::from_secs(PACKET_TIMEOUT_SECS),
                handshake::recv_packet(&mut stream, PacketType::Ack),
            )
            .await
            .map_err(|_| SendError::Protocol("ack recv timeout".into()))?
            .map_err(SendError::Handshake)?;
            seq += 1;

            pb.inc(n as u64);
        }
    }

    pb.finish_and_clear();

    let aad_complete = make_aad(PacketType::Complete, &session.code);
    let nonce_complete = crypto::make_nonce(&hs.nonce_base, seq);
    let complete_payload = CompletePayload {
        root_hash: manifest.root_hash,
    };
    let encrypted_complete = crypto::encrypt(
        &hs.session_keys,
        &nonce_complete,
        &complete_payload.encode(),
        &aad_complete,
    )?;
    timeout(
        Duration::from_secs(PACKET_TIMEOUT_SECS),
        send_packet(&mut stream, PacketType::Complete, &encrypted_complete),
    )
    .await
    .map_err(|_| SendError::Protocol("complete send timeout".into()))?
    .map_err(SendError::Handshake)?;
    let _ack = timeout(
        Duration::from_secs(PACKET_TIMEOUT_SECS),
        handshake::recv_packet(&mut stream, PacketType::Ack),
    )
    .await
    .map_err(|_| SendError::Protocol("complete ack timeout".into()))?
    .map_err(SendError::Handshake)?;

    sm.delete_session(&session.code).await.ok();

    let root_hex = hex::encode_upper(&manifest.root_hash[..8]);
    ui::success("Transfer complete");
    ui::detail("root hash:", &format!("{}...", root_hex));

    Ok(())
}

fn make_aad(ptype: PacketType, session_code: &str) -> Vec<u8> {
    let mut aad = vec![ptype as u8];
    aad.extend_from_slice(session_code.as_bytes());
    aad
}

fn get_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local = socket.local_addr().ok()?;
    Some(local.ip().to_string())
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
pub enum SendError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("session error: {0}")]
    Session(#[from] crate::transfer::session_manager::SessionError),

    #[error("handshake error: {0}")]
    Handshake(#[from] crate::transfer::handshake::HandshakeError),

    #[error("manifest error: {0}")]
    Manifest(#[from] crate::transfer::manifest::ManifestError),

    #[error("crypto error: {0}")]
    Crypto(#[from] crate::crypto::CryptoError),

    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("no receiver joined")]
    NoReceiver,

    #[error("no files found to send")]
    NoFiles,

    #[error("transfer rejected by user")]
    Rejected,

    #[error("protocol error: {0}")]
    Protocol(String),
}

impl From<ProtocolError> for SendError {
    fn from(e: ProtocolError) -> Self {
        SendError::Protocol(e.to_string())
    }
}
