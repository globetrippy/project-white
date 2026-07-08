#!/bin/sh
# Project-White — single-command installer
# Usage: curl -sSfL https://pw-server-gna4.onrender.com/install.sh | sh
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
  echo ""
  echo "  ✗ Binary not available for $OS/$ARCH on this server yet."
  echo "    Available platforms: linux-x86_64 (testing phase)"
  echo ""
  echo "    To build from source:"
  echo "      git clone https://github.com/globetrippy/project-white.git"
  echo "      cd project-white && cargo build --release --bin pw"
  echo "      sudo cp target/release/pw /usr/local/bin/"
  exit 1
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
