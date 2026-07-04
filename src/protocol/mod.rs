//! Wire protocol for Project White.
//!
//! All messages use a TLV (Type-Length-Value) framing:
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │ Type     (1 byte)   │ Length  (4 bytes, BE) │
//! ├─────────────────────────────────────────────┤
//! │ Payload  (Length bytes)                      │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! Every encrypted payload uses the session ID as
//! AEAD associated data to prevent cross-session replay.
//! See `crypto::aead::encrypt`.

use thiserror::Error;

// ─── Packet Type Definitions ─────────────────────────────────

/// Packet type identifiers.
///
/// Assigned per the approved protocol specification.
/// Types 0x01–0x0A are reserved for Version 1.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketType {
    HandshakeInit = 0x01,
    HandshakeAck = 0x02,
    HandshakeDone = 0x03,
    Manifest = 0x04,
    Chunk = 0x05,
    Ack = 0x06,
    Complete = 0x07,
    Error = 0x08,
    Ping = 0x09,
    Pong = 0x0A,
}

impl TryFrom<u8> for PacketType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> std::result::Result<Self, ProtocolError> {
        match value {
            0x01 => Ok(Self::HandshakeInit),
            0x02 => Ok(Self::HandshakeAck),
            0x03 => Ok(Self::HandshakeDone),
            0x04 => Ok(Self::Manifest),
            0x05 => Ok(Self::Chunk),
            0x06 => Ok(Self::Ack),
            0x07 => Ok(Self::Complete),
            0x08 => Ok(Self::Error),
            0x09 => Ok(Self::Ping),
            0x0A => Ok(Self::Pong),
            _ => Err(ProtocolError::UnknownPacketType(value)),
        }
    }
}

// ─── TLV Codec ──────────────────────────────────────────────

/// A framed protocol packet: type tag + length-prefixed payload.
#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    pub packet_type: PacketType,
    pub payload: Vec<u8>,
}

impl Packet {
    /// Create a new packet.
    pub fn new(packet_type: PacketType, payload: Vec<u8>) -> Self {
        Self {
            packet_type,
            payload,
        }
    }

    /// Encode this packet into its TLV wire representation.
    ///
    /// Returns `5 + payload.len()` bytes:
    ///   [type: 1] [length: 4 BE] [payload: N]
    pub fn encode(&self) -> Vec<u8> {
        let len = self.payload.len();
        let mut buf = Vec::with_capacity(5 + len);
        buf.push(self.packet_type as u8);
        buf.extend_from_slice(&(len as u32).to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode a single packet from the front of a byte buffer.
    ///
    /// Returns the decoded packet and the number of bytes consumed.
    /// Returns `ProtocolError::IncompletePacket` if fewer than 5
    /// header bytes or the full payload are available.
    pub fn decode(data: &[u8]) -> Result<(Self, usize), ProtocolError> {
        if data.len() < 5 {
            return Err(ProtocolError::IncompletePacket);
        }
        let packet_type = PacketType::try_from(data[0])?;
        let payload_len =
            u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
        let total = 5 + payload_len;
        if data.len() < total {
            return Err(ProtocolError::IncompletePacket);
        }
        let payload = data[5..total].to_vec();
        Ok((Self::new(packet_type, payload), total))
    }

    /// Decode all complete packets from a byte buffer, returning
    /// the packets and any remaining bytes.
    pub fn decode_all(data: &[u8]) -> (Vec<Self>, &[u8]) {
        let mut packets = Vec::new();
        let mut offset = 0;
        while offset < data.len() {
            if data.len() - offset < 5 {
                break;
            }
            let payload_len = match u32::from_be_bytes(
                [data[offset + 1], data[offset + 2], data[offset + 3], data[offset + 4]],
            ) {
                n if (5 + n as usize) > data.len() - offset => break,
                n => n as usize,
            };
            let total = 5 + payload_len;
            match Self::decode(&data[offset..offset + total]) {
                Ok((pkt, _)) => packets.push(pkt),
                Err(_) => break,
            }
            offset += total;
        }
        (packets, &data[offset..])
    }
}

// ─── Specific Payload Helpers ───────────────────────────────

/// Payload carried by `HandshakeInit` (0x01).
pub struct HandshakeInitPayload {
    /// 8 random bytes used as the nonce base for this session.
    pub nonce_base: [u8; 8],
    /// Sender's X25519 public key (32 bytes).
    pub public_key: [u8; 32],
}

impl HandshakeInitPayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(40);
        buf.extend_from_slice(&self.nonce_base);
        buf.extend_from_slice(&self.public_key);
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() != 40 {
            return Err(ProtocolError::InvalidPayloadLength {
                expected: 40,
                actual: data.len(),
            });
        }
        let mut nonce_base = [0u8; 8];
        let mut public_key = [0u8; 32];
        nonce_base.copy_from_slice(&data[0..8]);
        public_key.copy_from_slice(&data[8..40]);
        Ok(Self {
            nonce_base,
            public_key,
        })
    }
}

/// Payload carried by `HandshakeAck` (0x02).
pub struct HandshakeAckPayload {
    /// Receiver's X25519 public key (32 bytes).
    pub public_key: [u8; 32],
}

impl HandshakeAckPayload {
    pub fn encode(&self) -> Vec<u8> {
        self.public_key.to_vec()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() != 32 {
            return Err(ProtocolError::InvalidPayloadLength {
                expected: 32,
                actual: data.len(),
            });
        }
        let mut public_key = [0u8; 32];
        public_key.copy_from_slice(data);
        Ok(Self { public_key })
    }
}

/// Payload carried by `HandshakeDone` (0x03).
pub struct HandshakeDonePayload {
    /// First 8 bytes of BLAKE3(shared_secret || "pw-v1-verify")
    pub verification_hash: [u8; 8],
}

impl HandshakeDonePayload {
    pub fn encode(&self) -> Vec<u8> {
        self.verification_hash.to_vec()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() != 8 {
            return Err(ProtocolError::InvalidPayloadLength {
                expected: 8,
                actual: data.len(),
            });
        }
        let mut verification_hash = [0u8; 8];
        verification_hash.copy_from_slice(data);
        Ok(Self { verification_hash })
    }
}

/// Payload carried by `Ack` (0x06).
pub struct AckPayload {
    /// Sequence number of the chunk being acknowledged.
    pub sequence: u64,
}

impl AckPayload {
    pub fn encode(&self) -> Vec<u8> {
        self.sequence.to_be_bytes().to_vec()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() != 8 {
            return Err(ProtocolError::InvalidPayloadLength {
                expected: 8,
                actual: data.len(),
            });
        }
        let mut buf = [0u8; 8];
        buf.copy_from_slice(data);
        Ok(Self {
            sequence: u64::from_be_bytes(buf),
        })
    }
}

/// Payload carried by `Error` (0x08).
pub struct ErrorPayload {
    pub error_code: u8,
    pub message: String,
}

impl ErrorPayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = vec![self.error_code];
        buf.extend_from_slice(self.message.as_bytes());
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.is_empty() {
            return Err(ProtocolError::InvalidPayloadLength {
                expected: 1,
                actual: 0,
            });
        }
        let error_code = data[0];
        let message = String::from_utf8_lossy(&data[1..]).to_string();
        Ok(Self {
            error_code,
            message,
        })
    }
}

/// Chunk packet payload: sequence number + encrypted chunk data.
pub struct ChunkPayload {
    pub sequence: u64,
    pub data: Vec<u8>,
}

impl ChunkPayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + self.data.len());
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 8 {
            return Err(ProtocolError::InvalidPayloadLength {
                expected: 8,
                actual: data.len(),
            });
        }
        let mut seq_buf = [0u8; 8];
        seq_buf.copy_from_slice(&data[0..8]);
        Ok(Self {
            sequence: u64::from_be_bytes(seq_buf),
            data: data[8..].to_vec(),
        })
    }
}

/// Payload carried by `Complete` (0x07).
pub struct CompletePayload {
    /// 32-byte BLAKE3 root hash (encrypted as part of the AEAD payload).
    pub root_hash: [u8; 32],
}

impl CompletePayload {
    pub fn encode(&self) -> Vec<u8> {
        self.root_hash.to_vec()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() != 32 {
            return Err(ProtocolError::InvalidPayloadLength {
                expected: 32,
                actual: data.len(),
            });
        }
        let mut root_hash = [0u8; 32];
        root_hash.copy_from_slice(data);
        Ok(Self { root_hash })
    }
}

// ─── Error Codes ────────────────────────────────────────────

pub mod error_codes {
    pub const SESSION_NOT_FOUND: u8 = 0x01;
    pub const SESSION_FULL: u8 = 0x02;
    pub const HANDSHAKE_FAILED: u8 = 0x03;
    pub const TRANSFER_INTERRUPTED: u8 = 0x04;
    pub const INTEGRITY_FAILURE: u8 = 0x05;
    pub const TIMEOUT: u8 = 0x06;
    pub const INTERNAL_ERROR: u8 = 0x07;
}

// ─── Errors ─────────────────────────────────────────────────

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ProtocolError {
    #[error("unknown packet type: 0x{0:02x}")]
    UnknownPacketType(u8),

    #[error("incomplete packet (need more bytes)")]
    IncompletePacket,

    #[error("invalid payload length: expected {expected}, got {actual}")]
    InvalidPayloadLength { expected: usize, actual: usize },

    #[error("protocol violation: {0}")]
    Violation(&'static str),
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_type_try_from() {
        for code in 0x01..=0x0A {
            assert!(PacketType::try_from(code).is_ok());
        }
        assert!(PacketType::try_from(0x00).is_err());
        assert!(PacketType::try_from(0x0B).is_err());
        assert!(PacketType::try_from(0xFF).is_err());
    }

    #[test]
    fn test_packet_encode_decode_roundtrip() {
        let pkt = Packet::new(PacketType::HandshakeInit, vec![0; 40]);
        let encoded = pkt.encode();
        let (decoded, consumed) = Packet::decode(&encoded).unwrap();
        assert_eq!(decoded, pkt);
        assert_eq!(consumed, 45);
    }

    #[test]
    fn test_packet_encode_decode_empty_payload() {
        let pkt = Packet::new(PacketType::Ping, vec![]);
        let encoded = pkt.encode();
        let (decoded, consumed) = Packet::decode(&encoded).unwrap();
        assert_eq!(decoded, pkt);
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_packet_decode_incomplete_header() {
        let result = Packet::decode(&[0x01, 0x00]);
        assert!(matches!(result, Err(ProtocolError::IncompletePacket)));
    }

    #[test]
    fn test_packet_decode_incomplete_payload() {
        let mut buf = vec![0x05];
        buf.extend_from_slice(&(100u32).to_be_bytes());
        buf.extend_from_slice(&vec![0; 50]);
        let result = Packet::decode(&buf);
        assert!(matches!(result, Err(ProtocolError::IncompletePacket)));
    }

    #[test]
    fn test_packet_decode_all() {
        let pkt1 = Packet::new(PacketType::Ping, vec![]);
        let pkt2 = Packet::new(PacketType::Pong, vec![]);
        let mut buf = pkt1.encode();
        buf.extend_from_slice(&pkt2.encode());
        buf.push(0xFF);

        let (packets, remaining) = Packet::decode_all(&buf);
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0], pkt1);
        assert_eq!(packets[1], pkt2);
        assert_eq!(remaining, &[0xFF]);
    }

    #[test]
    fn test_handshake_init_payload_roundtrip() {
        let payload = HandshakeInitPayload {
            nonce_base: [1, 2, 3, 4, 5, 6, 7, 8],
            public_key: [0xAB; 32],
        };
        let encoded = payload.encode();
        let decoded = HandshakeInitPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.nonce_base, payload.nonce_base);
        assert_eq!(decoded.public_key, payload.public_key);
    }

    #[test]
    fn test_handshake_ack_payload_roundtrip() {
        let payload = HandshakeAckPayload {
            public_key: [0xCD; 32],
        };
        let encoded = payload.encode();
        let decoded = HandshakeAckPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.public_key, payload.public_key);
    }

    #[test]
    fn test_ack_payload_roundtrip() {
        let payload = AckPayload { sequence: 42 };
        let encoded = payload.encode();
        let decoded = AckPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 42);
    }

    #[test]
    fn test_chunk_payload_roundtrip() {
        let payload = ChunkPayload {
            sequence: 7,
            data: vec![0xAA, 0xBB, 0xCC],
        };
        let encoded = payload.encode();
        let decoded = ChunkPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 7);
        assert_eq!(decoded.data, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_error_payload_roundtrip() {
        let payload = ErrorPayload {
            error_code: 0x04,
            message: "connection lost".into(),
        };
        let encoded = payload.encode();
        let decoded = ErrorPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.error_code, 0x04);
        assert_eq!(decoded.message, "connection lost");
    }

    #[test]
    fn test_complete_payload_roundtrip() {
        let payload = CompletePayload {
            root_hash: [0x42; 32],
        };
        let encoded = payload.encode();
        let decoded = CompletePayload::decode(&encoded).unwrap();
        assert_eq!(decoded.root_hash, [0x42; 32]);
    }

    #[test]
    fn test_handshake_init_invalid_length() {
        let result = HandshakeInitPayload::decode(&[0; 10]);
        assert!(matches!(result, Err(ProtocolError::InvalidPayloadLength { .. })));
    }
}
