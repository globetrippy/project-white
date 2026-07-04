# Deployment Guide

## Recommended: Render + cron-job.org (Free, No Credit Card)

- **Render** — hosts the signaling server
- **cron-job.org** — keeps it awake by pinging `/health` every 10 minutes

---

## Step 1: Deploy on Render

1. Go to https://dashboard.render.com/register and click **Sign up with GitHub**.
   - No credit card required.
2. Authorize Render to access your GitHub account.
3. On the dashboard, click **New** → **Web Service**.
4. Connect your GitHub account and select the `project-white` repository.
5. Fill in:
   - **Name:** `pw-server` (or anything)
   - **Environment:** `Docker`
   - **Region:** Choose the closest to you
   - **Branch:** `main`
   - **Health Check Path:** `/health`
6. Scroll down. Under **Advanced**, add environment variable:
   - `PW_SERVER_ADDR` = `0.0.0.0:8080`
7. Leave everything else as default. Click **Create Web Service**.

Render will build the Docker image and deploy. Wait a few minutes. Once done, you'll get a URL like `https://pw-server.onrender.com`.

## Step 2: Keep It Awake

1. Go to https://cron-job.org and sign up (free, no credit card).
2. Click **Create Cron Job**.
3. Fill in:
   - **Title:** `pw-server wakeup`
   - **URL:** `https://pw-server.onrender.com/health`
   - **Schedule:** `Every 10 minutes`
4. Click **Create**.

Now the server stays warm — no cold start delay.

## Step 3: Configure Your CLI

```bash
export PW_SERVER=https://pw-server.onrender.com
pw send ./my-project
```

---

## Alternative: Oracle Cloud (Requires Credit Card)

...

(rest of Oracle Cloud instructions kept as fallback)
