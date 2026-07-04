# ADR 003: Transport — TCP

**Status:** Accepted  
**Date:** 2026-07-04  
**RFC Reference:** Section 4.6, Architecture RFC  

## Context

Need a reliable, ordered transport for file transfer.

## Decision

Use TCP for Version 1.

## Rationale

- Simplest reliable transport — every language has first-class support.
- TCP stack handles retransmission, congestion control, and ordering (would need reimplementation over UDP).
- NAT hole-punching for TCP is well-understood (port prediction, simultaneous open).
- For V1, if TCP hole-punching fails, that is a documented limitation.

## Rejected Alternatives

- **QUIC:** Excellent but requires a library (`quinn`), adds complexity around connection migration and 0-RTT. Candidate for V2.
- **WebRTC:** Designed for browser-to-browser. Complex protocol stack (SDP, ICE, DTLS, SRTP/SCTP). No benefit for CLI-to-CLI.
- **Raw UDP + custom reliability:** Would require reimplementing TCP features (retransmission, ordering, congestion control) with no benefit for V1.
