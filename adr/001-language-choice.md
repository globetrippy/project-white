# ADR 001: Language — Rust

**Status:** Accepted  
**Date:** 2026-07-04  
**RFC Reference:** Section 3, Architecture RFC  

## Context

Need a language that produces a single static binary, is memory-safe, has good cryptography libraries, and is cross-platform.

## Decision

Use Rust with the following crates:
- `clap` for CLI argument parsing
- `tokio` for async runtime
- `x25519-dalek` for key exchange
- `chacha20poly1305` for AEAD encryption
- `hkdf` + `sha2` for key derivation
- `blake3` for hashing
- `zeroize` for memory erasure
- `libc` for memory locking

## Consequences

- Slower compile times and steeper learning curve.
- Stronger security guarantees via memory safety and type system.
- Single static binary deployment.

## Rejected Alternatives

- **Go:** Faster compile, simpler concurrency, but GC pause (irrelevant), weaker memory safety guarantees.
- **Python:** No single binary, poor cross-platform packaging, slow.
- **C:** Maximum control, but memory safety risks, no stdlib crypto.
