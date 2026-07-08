#!/bin/sh
# Project-White — single-command installer
# Usage: curl -sSfL https://raw.githubusercontent.com/globetrippy/project-white/main/install.sh | sh
set -eu

SERVER="${PW_SERVER:-https://pw-server-gna4.onrender.com}"
BINARY_NAME="pw"

# ─── Platform Detection ──────────────────────────────────────────
UNAME_S=$(uname -s)
UNAME_M=$(uname -m)

case "$UNAME_S" in
  Linux)  OS="linux" ;;
  Darwin) OS="darwin" ;;
  *)
    echo "error: unsupported OS ($UNAME_S). Project-White only supports Linux and macOS."
    echo "       You can still build from source: cargo install project-white"
    exit 1
    ;;
esac

case "$UNAME_M" in
  x86_64)        ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *)
    echo "error: unsupported architecture ($UNAME_M)."
    exit 1
    ;;
esac

BINARY="pw-$OS-$ARCH"
DOWNLOAD_URL="$SERVER/download/$BINARY"

# ─── Download ─────────────────────────────────────────────────────
echo "  · Downloading Project-White for $OS/$ARCH..."
echo "  · Server: $SERVER"

TMPFILE=$(mktemp /tmp/pw.XXXXXXXXXX)
trap 'rm -f "$TMPFILE"' EXIT

HTTP_CODE=$(curl -sSfL -w '%{http_code}' -o "$TMPFILE" "$DOWNLOAD_URL" 2>/dev/null || echo "000")

if [ "$HTTP_CODE" != "200" ]; then
  echo "  · Pre-built binary not available, building from source..."
  echo ""

  if ! command -v cargo >/dev/null 2>&1; then
    echo "  ✗ Rust toolchain not found. Install it first:"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
  fi

  TMPDIR=$(mktemp -d /tmp/pw-build.XXXXXXXXXX)
  trap 'rm -rf "$TMPDIR"' EXIT

  echo "  · Cloning repo..."
  git clone --depth 1 https://github.com/globetrippy/project-white.git "$TMPDIR" 2>/dev/null || {
    echo "  ✗ Failed to clone. Try manually:"
    echo "    git clone https://github.com/globetrippy/project-white.git && cd project-white && cargo build --release --bin pw"
    exit 1
  }

  cd "$TMPDIR"
  echo "  · Building pw (release)..."
  cargo build --release --bin pw 2>&1
  cp target/release/pw "$TMPFILE"
  echo ""
fi

chmod +x "$TMPFILE"

# ─── Install ─────────────────────────────────────────────────────
if [ -w /usr/local/bin ]; then
  mv "$TMPFILE" /usr/local/bin/pw
  echo "  ✓ Installed to /usr/local/bin/pw"
elif [ -w "$HOME/.local/bin" ] || mkdir -p "$HOME/.local/bin" 2>/dev/null; then
  mv "$TMPFILE" "$HOME/.local/bin/pw"
  echo "  ✓ Installed to $HOME/.local/bin/pw"
  case ":$PATH:" in
    *:"$HOME/.local/bin":*) ;;
    *) echo "  · Add to PATH: export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
  esac
else
  mv "$TMPFILE" "$HOME/.pw"
  echo "  ✓ Installed to $HOME/.pw"
  echo "  · Add to PATH: export PATH=\"\$HOME/.pw:\$PATH\""
fi

echo ""
echo "  ✓ Project-White ready. Run: pw --help"
