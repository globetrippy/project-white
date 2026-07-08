# Architecture

Project White is a secure peer-to-peer folder transfer tool designed for developers.

## Layers

```text
╭────────────────────── Architecture Layers ────────────────────────╮
│                                                                   │
│  CLI                          Argument parsing, display           │
│  Application                  Orchestration, logging              │
│  Session                      Lifecycle, state machine            │
│  Transfer                     Chunking, ACK/retry, reassembly     │
│  Security                     X25519, AEAD, HKDF, BLAKE3          │
│  Networking                   TCP sockets, NAT traversal          │
│                                                                   │
╰───────────────────────────────────────────────────────────────────╯
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
├── main.rs                  Entry point
├── lib.rs                   Crate root, module declarations
├── update.rs                Self-update (pw update)
├── cli/
│   └── mod.rs               CLI argument parsing
├── crypto/
│   └── mod.rs               Key exchange, AEAD, HKDF, BLAKE3
├── protocol/
│   └── mod.rs               Packet types, TLV codec, payload helpers
├── server/
│   ├── mod.rs               Axum router
│   ├── codec.rs             Server TLV codec
│   ├── handlers.rs          HTTP handlers (create/join/poll/approve/delete)
│   └── session.rs           Session store with expiry GC
├── transfer/
│   ├── mod.rs               Module re-exports
│   ├── handshake.rs         TCP handshake (sender + receiver)
│   ├── manifest.rs          File manifest builder
│   ├── receiver.rs          Receive folder logic
│   ├── sender.rs            Send folder logic
│   └── session_manager.rs   Signaling server API client
├── ui/
│   └── mod.rs               Terminal UI helpers (console + indicatif)
└── bin/
    └── server.rs            Server entry point
```
