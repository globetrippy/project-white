#!/usr/bin/env bash
# Oracle Cloud Free Tier — pw-server deployment script
# Run this on your Oracle Cloud VM (Ubuntu 24.04 ARM or AMD).

set -euo pipefail

echo "=== Project White — Signaling Server Setup ==="

# ─── 1. Install dependencies ───────────────────────────────
echo "[1/5] Installing dependencies..."
sudo apt-get update -qq
sudo apt-get install -y -qq curl openssl

# ─── 2. Download pw-server binary ──────────────────────────
echo "[2/5] Downloading pw-server..."
# Replace with your actual GitHub release URL after pushing.
LATEST_URL="https://github.com/YOUR_USER/project-white/releases/latest/download/pw-server-linux-arm64"
curl -sL "$LATEST_URL" -o /tmp/pw-server
chmod +x /tmp/pw-server
sudo mv /tmp/pw-server /usr/local/bin/pw-server
pw-server --version

# ─── 3. Create systemd service ─────────────────────────────
echo "[3/5] Creating systemd service..."
sudo tee /etc/systemd/system/pw-server.service > /dev/null <<'SERVICE'
[Unit]
Description=Project White Signaling Server
After=network.target

[Service]
Type=simple
User=nobody
Group=nogroup
ExecStart=/usr/local/bin/pw-server
Restart=on-failure
RestartSec=10
Environment=PW_SERVER_ADDR=0.0.0.0:443
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
SERVICE

sudo systemctl daemon-reload
sudo systemctl enable pw-server

# ─── 4. Configure firewall (Oracle Cloud default) ──────────
echo "[4/5] Configuring firewall..."
sudo ufw allow 443/tcp 2>/dev/null || true
sudo ufw --force enable 2>/dev/null || true

# ─── 5. Set up TLS with Let's Encrypt ──────────────────────
echo "[5/5] Setting up TLS..."
# Note: Replace YOUR_DOMAIN with your actual domain.
# If you don't have a domain yet, skip this step.
# The server will still work over HTTP for LAN transfers.

# sudo apt-get install -y -qq nginx certbot python3-certbot-nginx
# sudo certbot --nginx -d your-domain.com --non-interactive --agree-tos -m you@email.com
# Then configure nginx as a reverse proxy on port 443 → localhost:8080

echo ""
echo "=== Setup complete ==="
echo ""
echo "Next steps:"
echo "  1. Configure DNS: point your domain → this server's public IP"
echo "  2. Set up TLS with: sudo certbot --nginx -d your-domain.com"
echo "  3. Start the server: sudo systemctl start pw-server"
echo "  4. Check status: sudo systemctl status pw-server"
echo ""
echo "Public IP: $(curl -s ifconfig.me 2>/dev/null || echo 'unknown')"
