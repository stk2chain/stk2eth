# GitHub Secrets Setup Guide

## Overview

This guide provides step-by-step instructions for configuring GitHub Secrets required for automatic deployment to DigitalOcean.

## Prerequisites

- GitHub repository admin access
- DigitalOcean account
- SpacetimeDB account
- Terminal/SSH access

## Step 1: DigitalOcean Access Token

1. **Generate API Token:**
   - Go to [DigitalOcean API Tokens](https://cloud.digitalocean.com/account/api/tokens)
   - Click **Generate New Token**
   - Name: `stk2eth-github-actions`
   - Scopes: Select **Read** and **Write**
   - Click **Generate Token**
   - **Copy the token immediately** (you won't see it again)

2. **Add to GitHub:**
   - Go to your repo: **Settings → Secrets and variables → Actions**
   - Click **New repository secret**
   - Name: `DO_ACCESS_TOKEN`
   - Value: Paste the token
   - Click **Add secret**

## Step 2: DigitalOcean Container Registry

### Create Registry

```bash
# Install doctl
brew install doctl  # macOS
# OR
wget https://github.com/digitalocean/doctl/releases/download/v1.98.1/doctl-1.98.1-linux-amd64.tar.gz
tar xf doctl-1.98.1-linux-amd64.tar.gz
sudo mv doctl /usr/local/bin

# Authenticate
doctl auth init

# Create registry
doctl registry create stk2eth

# Login to registry
doctl registry login
```

### Get Registry Token

```bash
# Get the registry token
doctl registry kubernetes-manifest | grep -A 1 'dockerconfigjson:' | tail -n 1 | base64 -d | jq -r '.auths["registry.digitalocean.com"].auth' | base64 -d | cut -d: -f2
```

**Add to GitHub:**
- Name: `DO_REGISTRY_TOKEN`
- Value: The token from above command

## Step 3: DigitalOcean Droplet Setup

### Create Droplet

1. **Create via Dashboard:**
   - Go to [DigitalOcean Droplets](https://cloud.digitalocean.com/droplets)
   - Click **Create Droplet**
   - Choose:
     - Image: **Ubuntu 22.04 LTS**
     - Size: **Basic** → **$12/month** (2GB RAM, 1 vCPU)
     - Region: Choose nearest to your users
     - Authentication: **SSH Keys** (create if needed)
   - Click **Create Droplet**
   - **Note the IP address**

2. **Or create via CLI:**
   ```bash
   doctl compute droplet create stk2eth-production \
     --region nyc3 \
     --size s-1vcpu-2gb \
     --image ubuntu-22-04-x64 \
     --ssh-keys $(doctl compute ssh-key list --format ID --no-header)

   # Get IP address
   doctl compute droplet list --format Name,PublicIPv4
   ```

**Add to GitHub:**
- Name: `DO_DROPLET_IP`
- Value: Your droplet IP address (e.g., `142.93.123.45`)

### Setup Droplet

```bash
# SSH into droplet
ssh root@YOUR_DROPLET_IP

# Run setup script
curl -sSL https://raw.githubusercontent.com/stk2chain/stk2eth/develop/scripts/setup-droplet.sh | bash

# Or manually:
git clone https://github.com/stk2chain/stk2eth.git
cd stk2eth
chmod +x scripts/setup-droplet.sh
./scripts/setup-droplet.sh
```

## Step 4: SSH Key for Deployment

### Generate SSH Key

```bash
# On your local machine
ssh-keygen -t ed25519 -C "github-actions-stk2eth" -f ~/.ssh/stk2eth_deploy_key

# This creates:
# - Private key: ~/.ssh/stk2eth_deploy_key
# - Public key: ~/.ssh/stk2eth_deploy_key.pub
```

### Add Public Key to Droplet

```bash
# Copy public key to droplet
ssh-copy-id -i ~/.ssh/stk2eth_deploy_key.pub root@YOUR_DROPLET_IP

# Or manually:
cat ~/.ssh/stk2eth_deploy_key.pub | ssh root@YOUR_DROPLET_IP "mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys"

# Test the key
ssh -i ~/.ssh/stk2eth_deploy_key root@YOUR_DROPLET_IP "echo 'SSH key works!'"
```

### Add Private Key to GitHub

```bash
# Display private key
cat ~/.ssh/stk2eth_deploy_key
```

**Add to GitHub:**
- Name: `DO_SSH_PRIVATE_KEY`
- Value: Entire content of private key (including BEGIN and END lines)

```
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
...
(paste entire key)
...
-----END OPENSSH PRIVATE KEY-----
```

## Step 5: SpacetimeDB Configuration

### Get SpacetimeDB Credentials

1. **Login to SpacetimeDB:**
   - Go to [SpacetimeDB Console](https://console.spacetimedb.com)
   - Login or create account

2. **Deploy Database (if not done):**
   ```bash
   # Install SpacetimeDB CLI
   curl --proto '=https' --tlsv1.2 -sSf https://install.spacetimedb.com | sh

   # Login
   spacetime login

   # Publish module
   cd ussdgeth
   spacetime publish -s maincloud -- ussdgeth

   # Note the database ID and details
   ```

3. **Get Database Info:**
   ```bash
   # List databases
   spacetime list -s maincloud

   # Get database details
   spacetime describe <database-name> -s maincloud
   ```

### Add SpacetimeDB Secrets to GitHub

| Secret Name | Value | Example |
|-------------|-------|---------|
| `SPACETIME_SERVER` | Server domain | `maincloud.spacetimedb.com` |
| `SPACETIME_DB_NAME` | Database name | `ussdgeth` |
| `SPACETIME_DB_ID` | Database ID (hex) | `c20038a8288ea16e2e033895c451272e1d83feb26e4bce6a75c24908c29408b4` |
| `SPACETIME_URL` | Full server URL | `https://maincloud.spacetimedb.com` |
| `SPACETIME_API_URL` | API gateway URL | `https://maincloud.spacetimedb.com/v1/database/gateway` |
| `SPACETIME_AUTH_TOKEN` | Your auth token | Get from `~/.spacetime/config.toml` or console |

**Get Auth Token:**
```bash
# From config file
cat ~/.spacetime/config.toml | grep token

# Or get new token from console
spacetime identity show
```

## Step 6: Optional - Slack Notifications

### Setup Slack Webhook

1. **Create Slack App:**
   - Go to [Slack API Apps](https://api.slack.com/apps)
   - Click **Create New App** → **From scratch**
   - Name: `STK2ETH Deployments`
   - Select workspace

2. **Enable Incoming Webhooks:**
   - Click **Incoming Webhooks**
   - Toggle **Activate Incoming Webhooks** to **On**
   - Click **Add New Webhook to Workspace**
   - Select channel (e.g., `#deployments`)
   - Click **Allow**
   - **Copy webhook URL**

3. **Add to GitHub:**
   - Name: `SLACK_WEBHOOK_URL`
   - Value: Your webhook URL (e.g., `https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXX`)

## Step 7: Verify All Secrets

### Required Secrets Checklist

Go to **Settings → Secrets and variables → Actions** and verify:

- [ ] `DO_ACCESS_TOKEN` - DigitalOcean API token
- [ ] `DO_REGISTRY_TOKEN` - Container registry token
- [ ] `DO_DROPLET_IP` - Droplet IP address
- [ ] `DO_SSH_PRIVATE_KEY` - SSH private key for deployment
- [ ] `SPACETIME_SERVER` - SpacetimeDB server
- [ ] `SPACETIME_DB_NAME` - Database name
- [ ] `SPACETIME_DB_ID` - Database ID
- [ ] `SPACETIME_URL` - SpacetimeDB URL
- [ ] `SPACETIME_API_URL` - API gateway URL
- [ ] `SPACETIME_AUTH_TOKEN` - Authentication token

### Optional Secrets

- [ ] `SLACK_WEBHOOK_URL` - Slack notifications (optional)

## Step 8: Test Deployment

### Manual Test

1. **Trigger workflow manually:**
   - Go to **Actions** → **Deploy to DigitalOcean**
   - Click **Run workflow**
   - Select `staging` environment
   - Click **Run workflow**

2. **Watch deployment:**
   - Click on the running workflow
   - Monitor each step
   - Check for any errors

3. **Verify on droplet:**
   ```bash
   ssh root@YOUR_DROPLET_IP
   docker ps -f name=stk2eth
   curl http://localhost:8080/health
   ```

### Automatic Test

1. **Make a change and push to develop:**
   ```bash
   git checkout develop
   echo "# Test deployment" >> README.md
   git add README.md
   git commit -m "test(ci): trigger deployment test"
   git push origin develop
   ```

2. **Watch GitHub Actions:**
   - Go to **Actions** tab
   - Watch deployment workflow
   - Verify successful deployment

## Troubleshooting

### Secret Not Working

1. **Check secret exists:**
   - Go to **Settings → Secrets and variables → Actions**
   - Verify secret is listed

2. **Check secret value:**
   - Secrets are masked in logs
   - Re-add if unsure about value

3. **Check workflow file:**
   - Verify secret name matches exactly (case-sensitive)
   - Check `${{ secrets.SECRET_NAME }}` syntax

### SSH Key Issues

```bash
# Test SSH key locally
ssh -i ~/.ssh/stk2eth_deploy_key root@YOUR_DROPLET_IP

# Check authorized_keys on droplet
ssh root@YOUR_DROPLET_IP "cat ~/.ssh/authorized_keys"

# Check permissions
ssh root@YOUR_DROPLET_IP "chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys"

# Verify key format
cat ~/.ssh/stk2eth_deploy_key | head -n 1
# Should be: -----BEGIN OPENSSH PRIVATE KEY-----
```

### Registry Issues

```bash
# Re-login to registry
doctl registry login

# Test pulling image
docker pull registry.digitalocean.com/stk2eth/stk2eth:latest

# Check registry credentials
cat ~/.docker/config.json
```

### SpacetimeDB Connection Issues

```bash
# Test connection
curl -X GET https://maincloud.spacetimedb.com/database/list \
  -H "Authorization: Bearer YOUR_TOKEN"

# Verify database exists
spacetime list -s maincloud

# Check auth token
spacetime identity show
```

## Security Best Practices

1. **Rotate Secrets Regularly:**
   - Regenerate tokens every 90 days
   - Update in GitHub immediately

2. **Use Least Privilege:**
   - API tokens: minimum required permissions
   - SSH keys: dedicated for deployment only

3. **Monitor Secret Usage:**
   - Review GitHub Actions logs
   - Check for unauthorized access

4. **Backup Secrets Securely:**
   - Store in password manager
   - Document secret locations

5. **Never Commit Secrets:**
   - Use `.env.example` for templates
   - Add `.env` to `.gitignore`

## Quick Setup Script

```bash
#!/bin/bash
# Quick setup script for all secrets

echo "STK2ETH GitHub Secrets Setup"
echo "============================"
echo ""

read -p "DigitalOcean Access Token: " DO_ACCESS_TOKEN
read -p "DO Registry Token: " DO_REGISTRY_TOKEN
read -p "DO Droplet IP: " DO_DROPLET_IP
read -p "Path to SSH private key: " SSH_KEY_PATH

read -p "SpacetimeDB Server: " SPACETIME_SERVER
read -p "SpacetimeDB Name: " SPACETIME_DB_NAME
read -p "SpacetimeDB ID: " SPACETIME_DB_ID
read -p "SpacetimeDB URL: " SPACETIME_URL
read -p "SpacetimeDB API URL: " SPACETIME_API_URL
read -p "SpacetimeDB Auth Token: " SPACETIME_AUTH_TOKEN

read -p "GitHub Repo (owner/repo): " REPO

# Set secrets using gh CLI
gh secret set DO_ACCESS_TOKEN -b "$DO_ACCESS_TOKEN" -R "$REPO"
gh secret set DO_REGISTRY_TOKEN -b "$DO_REGISTRY_TOKEN" -R "$REPO"
gh secret set DO_DROPLET_IP -b "$DO_DROPLET_IP" -R "$REPO"
gh secret set DO_SSH_PRIVATE_KEY < "$SSH_KEY_PATH" -R "$REPO"

gh secret set SPACETIME_SERVER -b "$SPACETIME_SERVER" -R "$REPO"
gh secret set SPACETIME_DB_NAME -b "$SPACETIME_DB_NAME" -R "$REPO"
gh secret set SPACETIME_DB_ID -b "$SPACETIME_DB_ID" -R "$REPO"
gh secret set SPACETIME_URL -b "$SPACETIME_URL" -R "$REPO"
gh secret set SPACETIME_API_URL -b "$SPACETIME_API_URL" -R "$REPO"
gh secret set SPACETIME_AUTH_TOKEN -b "$SPACETIME_AUTH_TOKEN" -R "$REPO"

echo "✅ All secrets configured!"
```

Save as `setup-secrets.sh`, make executable, and run:
```bash
chmod +x setup-secrets.sh
./setup-secrets.sh
```

## Support

For issues:
- **GitHub Secrets:** GitHub Support or repository admin
- **DigitalOcean:** DO Support or check [DO Community](https://www.digitalocean.com/community)
- **SpacetimeDB:** [SpacetimeDB Discord](https://discord.gg/spacetimedb)
