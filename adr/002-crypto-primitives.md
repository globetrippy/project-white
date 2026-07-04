# ADR 002: Cryptographic Primitives

**Status:** Accepted  
**Date:** 2026-07-04  
**RFC Reference:** Section 7.1, Architecture RFC  

## Context

Need standard, audited cryptographic primitives for key exchange, encryption, hashing, and key derivation.

## Decision

| Operation | Primitive | Justification |
|-----------|-----------|---------------|
| Key exchange | X25519 (Curve25519 ECDH) | Wide deployment, constant-time, small keys |
| Symmetric encryption | ChaCha20-Poly1305 (IETF variant) | Fast in software, AEAD built-in, no HW dependency |
| Key derivation | HKDF-SHA256 (RFC 5869) | TLS 1.3 standard, domain separation via `info` |
| Hashing | BLAKE3 | Extremely fast, incremental, keyed mode |
| Memory erasure | `zeroize` crate | Prevents compiler elision of zeroing |

## Fixes from Architecture Review

1. **Nonce construction:** `BLAKE3(base || seq)[..12]` — prevents nonce reuse without per-chunk state.
2. **HKDF salt:** `BLAKE3(session_code)` — binds keys to a specific session.
3. **AEAD AAD:** `[packet_type_byte] + session_id.as_bytes()` — prevents cross-session replay.

## Rejected Alternatives

- **AES-256-GCM:** HW-dependent (AES-NI), slower in software than ChaCha20.
- **SHA-256:** BLAKE3 is faster for all message sizes.
- **Custom nonce scheme:** Risk of nonce reuse. Deterministic derivation from nonce_base + seq is simpler.
