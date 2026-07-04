# Deployment Guide

## 1. GitHub — Private Repository

1. Go to https://github.com and create a free account.
2. Click **+** → **New repository**.
3. Name: `project-white` (or `pw`).
4. Set to **Private**.
5. Do NOT initialize with README (we already have one).
6. Click **Create repository**.
7. Share the repo URL with me (e.g., `https://github.com/youruser/project-white.git`).

I will push all the code.

---

## 2. Oracle Cloud — Free Tier VM

1. Go to https://cloud.oracle.com and sign up for **Free Tier**.
   - Requires a credit card for identity verification (no charges).
2. After approval, go to **Compute** → **Instances** → **Create instance**.
3. Configure:
   - **Name:** `pw-server`
   - **Image:** Ubuntu 24.04 (ARM or AMD)
   - **Shape:** VM.Standard.A1.Flex (ARM, 4 OCPUs, 24 GB RAM — always free)
     - Or VM.Standard.E2.1.Micro (AMD, 1 OCPU, 1 GB RAM — always free)
   - **SSH key:** Download or generate new key pair
   - **Boot volume:** 100 GB (always free)
4. Click **Create**.
5. After creation, note the **Public IP address**.
6. Share the public IP and SSH private key with me.

**Security List:** By default, port 8080 is open. We'll use port 443 with TLS.

---

## 3. Server Deployment

After you share the VM's public IP, I will:
1. Build the release binary
2. Copy it to the VM via SCP
3. Set up systemd service
4. Configure TLS with Let's Encrypt
5. Test the server

The server will run as a systemd service and auto-start on reboot.

---

## 4. Client Configuration

After deployment, configure the CLI to use your server:

```bash
export PW_SERVER=https://your-server-ip:443
pw send ./my-project
```

Or set it permanently in your shell profile.
