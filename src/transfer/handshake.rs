use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use x25519_dalek::PublicKey;
use thiserror::Error;

/// Maximum allowed packet payload size (50 MB).
/// Prevents memory-exhaustion attacks from crafted oversized packets.
const MAX_PACKET_SIZE: usize = 50_000_000;

use crate::crypto;
use crate::protocol::{
    HandshakeAckPayload, HandshakeDonePayload, HandshakeInitPayload, Packet, PacketType,
};

pub struct HandshakeResult {
    pub session_keys: crypto::SessionKeys,
    pub verification_hash: [u8; 8],
    pub peer_public_key: [u8; 32],
    pub nonce_base: [u8; 8],
}

pub async fn sender_handshake(
    stream: &mut TcpStream,
    session_code: &str,
) -> Result<HandshakeResult, HandshakeError> {
    let (secret, public) = crypto::generate_keypair();
    let nonce_base: [u8; 8] = rand::random();

    let init = HandshakeInitPayload {
        nonce_base,
        public_key: public.to_bytes(),
    };
    send_packet(stream, PacketType::HandshakeInit, &init.encode()).await?;

    let ack_pkt = recv_packet(stream, PacketType::HandshakeAck).await?;
    let ack = HandshakeAckPayload::decode(&ack_pkt.payload)?;

    let peer_public = PublicKey::from(ack.public_key);
    let shared_secret = crypto::key_exchange(secret, &peer_public);
    let session_keys = crypto::derive_session_keys(&shared_secret, session_code);
    let verification_hash = crypto::session_verification_hash(&shared_secret);

    let done = HandshakeDonePayload { verification_hash };
    send_packet(stream, PacketType::HandshakeDone, &done.encode()).await?;

    Ok(HandshakeResult {
        session_keys,
        verification_hash,
        peer_public_key: ack.public_key,
        nonce_base,
    })
}

pub async fn receiver_handshake(
    stream: &mut TcpStream,
    session_code: &str,
) -> Result<HandshakeResult, HandshakeError> {
    let (secret, public) = crypto::generate_keypair();

    let init_pkt = recv_packet(stream, PacketType::HandshakeInit).await?;
    let init = HandshakeInitPayload::decode(&init_pkt.payload)?;

    let ack = HandshakeAckPayload {
        public_key: public.to_bytes(),
    };
    send_packet(stream, PacketType::HandshakeAck, &ack.encode()).await?;

    let peer_public = PublicKey::from(init.public_key);
    let shared_secret = crypto::key_exchange(secret, &peer_public);
    let session_keys = crypto::derive_session_keys(&shared_secret, session_code);
    let verification_hash = crypto::session_verification_hash(&shared_secret);

    let done_pkt = recv_packet(stream, PacketType::HandshakeDone).await?;
    let done = HandshakeDonePayload::decode(&done_pkt.payload)?;

    if done.verification_hash != verification_hash {
        return Err(HandshakeError::VerificationMismatch);
    }

    Ok(HandshakeResult {
        session_keys,
        verification_hash,
        peer_public_key: init.public_key,
        nonce_base: init.nonce_base,
    })
}

pub async fn send_packet(
    stream: &mut TcpStream,
    ptype: PacketType,
    payload: &[u8],
) -> Result<(), HandshakeError> {
    let pkt = Packet::new(ptype, payload.to_vec());
    let encoded = pkt.encode();
    stream
        .write_all(&encoded)
        .await
        .map_err(HandshakeError::Io)
}

pub async fn recv_packet(
    stream: &mut TcpStream,
    expected: PacketType,
) -> Result<Packet, HandshakeError> {
    let mut header = [0u8; 5];
    stream
        .read_exact(&mut header)
        .await
        .map_err(HandshakeError::Io)?;

    let ptype =
        PacketType::try_from(header[0])?;
    if ptype != expected {
        return Err(HandshakeError::UnexpectedPacket {
            expected,
            got: ptype,
        });
    }
    let payload_len =
        u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

    if payload_len > MAX_PACKET_SIZE {
        return Err(HandshakeError::Protocol(format!(
            "packet too large: {} bytes (max {})",
            payload_len, MAX_PACKET_SIZE
        )));
    }

    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
            stream
                .read_exact(&mut payload)
                .await
                .map_err(HandshakeError::Io)?;
    }

    Ok(Packet::new(ptype, payload))
}

#[derive(Error, Debug)]
pub enum HandshakeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("unexpected packet: expected {expected:?}, got {got:?}")]
    UnexpectedPacket {
        expected: PacketType,
        got: PacketType,
    },

    #[error("crypto error: {0}")]
    Crypto(#[from] crate::crypto::CryptoError),

    #[error("verification hash mismatch")]
    VerificationMismatch,

    #[error("handshake timeout")]
    Timeout,
}

impl From<crate::protocol::ProtocolError> for HandshakeError {
    fn from(e: crate::protocol::ProtocolError) -> Self {
        HandshakeError::Protocol(e.to_string())
    }
}
