# Deployment Guide

## Choose a Provider

- **Koyeb** (recommended) — No credit card required, forever free
- **Oracle Cloud** — Alternative, requires credit card

---

## Option A: Koyeb (No Credit Card Required)

1. Go to https://app.koyeb.com and sign up (no credit card needed).
2. Click **Create App**.
3. Connect your GitHub account and select the `project-white` private repo.
4. Builder: **Dockerfile** (auto-detected).
5. Port: **8080**.
6. Region: Choose the closest to you.
7. Health check path: `/health`.
8. Click **Create**.

Koyeb will build and deploy automatically. Your server URL will be `https://pw-project-white-name.koyeb.app`.

Set the client environment variable:
```bash
export PW_SERVER=https://pw-project-white-name.koyeb.app
pw send ./my-project
```

> **Note:** Koyeb free tier auto-sleeps on zero traffic. The first request after idle may take <500ms to wake up.

---

## Option B: Oracle Cloud — Free Tier VM

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

**Security List:** Open port 8080 and 443.

Run the setup script on the VM:
```bash
# From your machine, copy the script:
scp deploy/oracle-cloud-setup.sh ubuntu@<IP>:~

# SSH into the VM:
ssh ubuntu@<IP>
chmod +x oracle-cloud-setup.sh
./oracle-cloud-setup.sh
```

The server will run on port 443 with TLS. Configure the client:
```bash
export PW_SERVER=https://<IP>
pw send ./my-project
```

---

## Client Configuration

```bash
export PW_SERVER=https://your-server-url:443
pw send ./my-project
```

Or set it permanently in your shell profile.
