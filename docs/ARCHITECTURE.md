# Architecture

Project White is a secure peer-to-peer folder transfer tool designed for developers.

## Layers

```
┌─────────────┐
│    CLI      │  Argument parsing, display, exit codes
├─────────────┤
│ Application │  Orchestration, logging, error mapping
├─────────────┤
│  Session    │  Session lifecycle, state machine, server communication
├─────────────┤
│  Transfer   │  File chunking, sequencing, ACK/retry, reassembly
├─────────────┤
│  Security   │  X25519, AEAD, HKDF, BLAKE3, key management
├─────────────┤
│ Networking  │  TCP sockets, NAT traversal, connection lifecycle
├─────────────┤
│     OS      │
└─────────────┘
```

## Constraints

- Single-crate Rust project.
- No databases, queues, caches, or external services beyond the signaling server.
- No configuration file — CLI flags and environment variables only.
- Ephemeral session keys — never written to disk.

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | Memory safety, single binary, audited crypto |
| Transport | TCP | Reliable, ordered, OS-managed retransmission |
| Key exchange | X25519 | Constant-time, small keys, widely deployed |
| Encryption | ChaCha20-Poly1305 | Fast in software, AEAD built-in |
| Key derivation | HKDF-SHA256 | Domain separation via info string |
| Hashing | BLAKE3 | Fast, incremental, keyed mode available |
| Session code | Base58, 8 chars | Typeable, ~47 bits entropy |

## Files

```
src/
├── main.rs              Entry point
├── cli/mod.rs           CLI argument parsing
├── protocol/mod.rs      Packet types, TLV codec
└── crypto/mod.rs        All cryptographic operations
```
