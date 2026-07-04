# Project White

Secure peer-to-peer folder transfer for developers.

```
pw send ./Project
→ Share code: aB3xK9mZ

pw receive aB3xK9mZ
→ Folder received
```

No accounts. No cloud. No configuration. End-to-end encrypted.

## Install

```bash
# Homebrew (future)
brew install project-white

# From source
cargo install project-white
```

## Usage

```bash
# Send a folder
pw send ./my-project

# Receive a folder
pw receive aB3xK9mZ
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Protocol](docs/PROTOCOL.md)
- [Security Model](docs/SECURITY.md)
- [Architecture Decision Records](adr/)

## Status

Version 0.1.0 — Foundation complete. Signaling server and transfer engine in development.

## License

MIT OR Apache-2.0
