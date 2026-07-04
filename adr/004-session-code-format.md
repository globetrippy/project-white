# ADR 004: Session Code — Base58, 8 Characters

**Status:** Accepted  
**Date:** 2026-07-04  
**RFC Reference:** Section 7 (ADR 003 in original), Security Fix #1  

## Context

Need a human-friendly session code that is typeable, distinctive, and has enough entropy to prevent brute force during the 10-minute session window.

## Decision

Use 8-character base58 (Bitcoin-style, excludes `0`, `O`, `I`, `l`).

## Rationale

- Entropy: `log2(58^8) ≈ 47 bits`.
- 47 bits × rate limit of 10 join attempts/min = 8.5 × 10^12 minutes to brute force — sufficient for a short-lived session.
- Excludes ambiguous characters (`0/O`, `1/l/I`) for manual transcription.
- More compact than base64 (no `+`, `/`, `=`).

## Rejected Alternatives

- **UUID:** Too long for human transcription (36 characters).
- **Numeric (8 digits):** Too little entropy (~27 bits).
- **Base64:** Includes `+`, `/`, `=` which are not typeable.
- **6 characters:** Only ~35 bits of entropy — too low even with rate limiting.
