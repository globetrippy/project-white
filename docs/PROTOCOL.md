# Protocol Specification

## Wire Format

All packets use TLV (Type-Length-Value) framing:

```
┌─────────────────────────────────────────────┐
│ Type     (1 byte)   │ Length  (4 bytes, BE) │
├─────────────────────────────────────────────┤
│ Payload  (Length bytes)                      │
└─────────────────────────────────────────────┘
```

## Packet Types

| Code | Name | Payload |
|------|------|---------|
| 0x01 | HANDSHAKE_INIT | Nonce base (8 bytes) + Sender public key (32 bytes) |
| 0x02 | HANDSHAKE_ACK | Receiver public key (32 bytes) |
| 0x03 | HANDSHAKE_DONE | Verification hash (8 bytes) |
| 0x04 | MANIFEST | Encrypted manifest blob |
| 0x05 | CHUNK | Sequence number (8 bytes BE) + encrypted chunk data |
| 0x06 | ACK | Sequence number (8 bytes BE) |
| 0x07 | COMPLETE | Root hash (32 bytes) |
| 0x08 | ERROR | Error code (1 byte) + UTF-8 message |
| 0x09 | PING | Empty |
| 0x0A | PONG | Empty |

## Session Flow

```
Sender                    Server                    Receiver
  │                         │                         │
  │── POST /session ───────>│                         │
  │<── { code } ────────────│                         │
  │                         │    ── POST /join ──────>│
  │<── { peer_info } ──────│                         │
  │════ TCP connect ══════════════════════════════════>│
  │══ HANDSHAKE_INIT ════════════════════════════════>│
  │<══ HANDSHAKE_ACK ═════════════════════════════════│
  │══ HANDSHAKE_DONE ════════════════════════════════>│
  │══ MANIFEST ═══════════════════════════════════════>│
  │══ CHUNK[0..N] ═══════════════════════════════════>│
  │<══ ACK[0..N] ════════════════════════════════════│
  │══ COMPLETE ══════════════════════════════════════>│
  │══ DISCONNECT ════════════════════════════════════>│
```

## Error Codes

| Code | Name | Meaning |
|------|------|---------|
| 0x01 | SESSION_NOT_FOUND | Invalid or expired session code |
| 0x02 | SESSION_FULL | Session already has two peers |
| 0x03 | HANDSHAKE_FAILED | Key mismatch or protocol violation |
| 0x04 | TRANSFER_INTERRUPTED | Connection lost during transfer |
| 0x05 | INTEGRITY_FAILURE | Hash mismatch on received data |
| 0x06 | TIMEOUT | Peer did not respond |
| 0x07 | INTERNAL_ERROR | Unexpected error |
