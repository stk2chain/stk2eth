# Automatic Deployment Guide - STK2ETH to DigitalOcean

## Overview

This guide covers the automatic deployment pipeline that builds Docker images and deploys to DigitalOcean when code is merged to the `develop` branch.

## Architecture

```
GitHub (develop branch)
    ↓ (on push)
GitHub Actions
    ↓
Build Docker Images
    ↓
Push to DO Container Registry
    ↓
SSH to DigitalOcean Droplet
    ↓
Pull & Deploy Containers
    ↓
Health Check & Verify
```

## Prerequisites

### 1. DigitalOcean Droplet Setup

Create an Ubuntu 22.04 droplet and run the setup script:

```bash
# SSH into your droplet
ssh root@your-droplet-ip

# Download and run setup script
curl -sSL https://raw.githubusercontent.com/stk2chain/stk2eth/develop/scripts/setup-droplet.sh | bash

# Or manually:
git clone https://github.com/stk2chain/stk2eth.git
cd stk2eth
chmod +x scripts/setup-droplet.sh
./scripts/setup-droplet.sh
```

The setup script will:
- Install Docker and Docker Compose
- Configure firewall (UFW)
- Setup fail2ban for security
- Create deployment directory `/opt/stk2eth`
- Configure log rotation
- Setup monitoring cron jobs
- Configure SSH security
- Setup swap space

### 2. DigitalOcean Container Registry

1. Create a Container Registry in DigitalOcean:
   ```bash
   doctl registry create stk2eth
   ```

2. Login to the registry from your droplet:
   ```bash
   doctl registry login
   ```

3. Get the registry token for GitHub Actions:
   ```bash
   doctl registry kubernetes-manifest | grep secret
   ```

### 3. SSH Key Setup

1. Generate SSH key pair (on your local machine):
   ```bash
   ssh-keygen -t ed25519 -C "github-actions-deploy" -f ~/.ssh/do_deploy_key
   ```

2. Add public key to droplet:
   ```bash
   ssh-copy-id -i ~/.ssh/do_deploy_key.pub root@your-droplet-ip
   ```

3. Save private key for GitHub Secrets (see below)

## GitHub Secrets Configuration

Add the following secrets to your GitHub repository:

Navigate to: **Settings → Secrets and variables → Actions → New repository secret**

### Required Secrets

| Secret Name | Description | How to Get |
|-------------|-------------|------------|
| `DO_ACCESS_TOKEN` | DigitalOcean API token | [DO API Tokens](https://cloud.digitalocean.com/account/api/tokens) |
| `DO_REGISTRY_TOKEN` | Container Registry token | `doctl registry kubernetes-manifest` |
| `DO_DROPLET_IP` | Your droplet IP address | From DO dashboard |
| `DO_SSH_PRIVATE_KEY` | SSH private key for deployment | Content of `~/.ssh/do_deploy_key` |
| `SPACETIME_SERVER` | SpacetimeDB server URL | e.g., `maincloud.spacetimedb.com` |
| `SPACETIME_DB_NAME` | Database name | e.g., `ussdgeth` |
| `SPACETIME_DB_ID` | Database ID | From SpacetimeDB console |
| `SPACETIME_URL` | SpacetimeDB full URL | e.g., `https://maincloud.spacetimedb.com` |
| `SPACETIME_API_URL` | SpacetimeDB API URL | Gateway endpoint |
| `SPACETIME_AUTH_TOKEN` | SpacetimeDB auth token | From SpacetimeDB console |

### Optional Secrets

| Secret Name | Description |
|-------------|-------------|
| `SLACK_WEBHOOK_URL` | Slack webhook for notifications |

## Deployment Workflow

### Automatic Deployment (Main Flow)

When code is merged to `develop` branch:

1. **Build Stage** (5-10 minutes):
   - GitHub Actions triggers
   - Builds 3 Docker images (ussdclient, ethclient, all-in-one)
   - Pushes to DigitalOcean Container Registry
   - Tags with commit SHA and `latest`

2. **Deploy Stage** (2-5 minutes):
   - SSH to droplet
   - Creates backup of current deployment
   - Pulls latest images from registry
   - Stops existing containers gracefully (30s timeout)
   - Starts new containers
   - Waits for services to be healthy

3. **Verify Stage** (1-2 minutes):
   - Runs health checks
   - Tests USSD endpoint
   - Tests health endpoint
   - Verifies container status

4. **Notify Stage**:
   - Sends deployment summary
   - Updates GitHub deployment status
   - Sends Slack notification (if configured)

### Manual Deployment

Trigger deployment manually via GitHub Actions:

1. Go to **Actions** → **Deploy to DigitalOcean**
2. Click **Run workflow**
3. Select environment: `staging` or `production`
4. Click **Run workflow**

### Rollback on Failure

If deployment fails, automatic rollback occurs:

1. Restores previous `.env` from backup
2. Restarts containers with previous configuration
3. Logs rollback event
4. Notifies team of failure

## Deployment Commands

### On DigitalOcean Droplet

```bash
# View running containers
docker ps -f name=stk2eth

# View logs
cd /opt/stk2eth
docker-compose logs -f

# Manual deployment
cd /opt/stk2eth
./deploy.sh

# Health check
cd /opt/stk2eth
./healthcheck.sh

# Restart services
cd /opt/stk2eth
docker-compose restart

# Stop services
cd /opt/stk2eth
docker-compose down

# View deployment history
cat /opt/stk2eth/deployment.log

# View backups
ls -lah /opt/stk2eth/backups/
```

### From GitHub Actions

```bash
# Trigger deployment
gh workflow run deploy.yml

# View deployment status
gh run list --workflow=deploy.yml

# View logs
gh run view <run-id> --log
```

## Monitoring & Logs

### Container Logs

```bash
# All containers
docker-compose logs -f

# Specific service
docker-compose logs -f ussdclient
docker-compose logs -f ethclient

# Last 100 lines
docker-compose logs --tail=100

# Follow logs with timestamps
docker-compose logs -f -t
```

### System Logs

```bash
# Application logs
tail -f /opt/stk2eth/logs/*.log

# Deployment logs
tail -f /opt/stk2eth/deployment.log

# Monitoring logs
tail -f /opt/stk2eth/logs/monitor.log

# System journal
journalctl -u docker -f
```

### Health Monitoring

The system includes automatic monitoring:

1. **Cron Job** (every 5 minutes):
   - Checks if containers are running
   - Monitors disk space (alerts at 90%)
   - Auto-restarts failed containers
   - Cleans up old images when disk is full

2. **Docker Health Checks**:
   - USSD client: HTTP health endpoint
   - Automatic restart on failure

3. **Manual Health Check**:
   ```bash
   cd /opt/stk2eth
   ./healthcheck.sh
   ```

## Troubleshooting

### Deployment Failed

1. Check GitHub Actions logs:
   ```bash
   gh run view <run-id> --log
   ```

2. Check droplet logs:
   ```bash
   ssh root@your-droplet-ip
   cd /opt/stk2eth
   docker-compose logs --tail=100
   ```

3. Run health check:
   ```bash
   cd /opt/stk2eth
   ./healthcheck.sh
   ```

### Containers Not Starting

1. Check Docker daemon:
   ```bash
   systemctl status docker
   ```

2. Check environment variables:
   ```bash
   cat /opt/stk2eth/.env
   ```

3. Check disk space:
   ```bash
   df -h
   docker system df
   ```

4. Try manual start:
   ```bash
   cd /opt/stk2eth
   docker-compose up
   ```

### Cannot Connect to Registry

1. Verify registry login:
   ```bash
   doctl registry login
   ```

2. Check registry credentials:
   ```bash
   cat ~/.docker/config.json
   ```

3. Re-authenticate:
   ```bash
   doctl auth init
   doctl registry login
   ```

### SSH Connection Failed

1. Verify SSH key:
   ```bash
   ssh -i ~/.ssh/do_deploy_key root@your-droplet-ip
   ```

2. Check firewall:
   ```bash
   ufw status
   ```

3. Verify droplet IP in secrets

### High Memory/Disk Usage

1. Cleanup old images:
   ```bash
   docker image prune -af
   ```

2. Cleanup old containers:
   ```bash
   docker container prune -f
   ```

3. Check disk usage:
   ```bash
   docker system df
   du -sh /opt/stk2eth/*
   ```

4. View resource usage:
   ```bash
   docker stats
   htop
   ```

## Manual Rollback

If you need to manually rollback:

```bash
# SSH to droplet
ssh root@your-droplet-ip

# Navigate to deployment directory
cd /opt/stk2eth

# List available backups
ls -lah backups/

# Restore from specific backup
cp backups/backup_YYYYMMDD_HHMMSS/.env .env

# Restart with previous version
docker-compose down
docker-compose up -d

# Or use the deploy script's rollback
# (automatically uses latest backup)
```

## Security Best Practices

1. **SSH Keys**: Use SSH keys, not passwords
2. **Firewall**: Only expose necessary ports (22, 80, 443, 8080)
3. **fail2ban**: Automatically installed to prevent brute force
4. **Secrets**: Never commit secrets to git
5. **Registry**: Use private container registry
6. **Updates**: Regular system updates via `apt-get update && apt-get upgrade`
7. **Monitoring**: Review logs regularly for suspicious activity

## Scaling & Performance

### Horizontal Scaling

For high traffic, use multiple droplets with load balancer:

1. Create multiple droplets using `setup-droplet.sh`
2. Add load balancer in DigitalOcean
3. Point load balancer to droplets on port 8080
4. Update GitHub secrets with load balancer IP

### Vertical Scaling

Upgrade droplet size:

1. Snapshot current droplet
2. Resize droplet in DO dashboard
3. No changes needed to deployment pipeline

### Database Scaling

SpacetimeDB scaling is handled separately:
- See SpacetimeDB documentation for scaling
- Update connection strings in GitHub secrets

## Cost Optimization

1. **Image Cleanup**: Automatic cleanup of old images (72h retention)
2. **Log Rotation**: Logs rotated daily, kept for 14 days
3. **Droplet Size**: Start with $12/month droplet, scale as needed
4. **Registry**: Free 500MB, then $20/month for 1GB
5. **Monitoring**: Use built-in monitoring, avoid premium tools initially

## Support & Resources

- **Deployment Issues**: Check GitHub Actions logs first
- **Container Issues**: Check Docker logs on droplet
- **Network Issues**: Verify firewall and DO networking settings
- **Documentation**: See DOCKER.md for Docker-specific info

## Quick Reference

```bash
# Deploy from GitHub
git push origin develop  # Automatic deployment

# Check deployment status
gh run list --workflow=deploy.yml

# SSH to droplet
ssh root@YOUR_DROPLET_IP

# View services
docker ps -f name=stk2eth

# View logs
docker-compose logs -f

# Health check
cd /opt/stk2eth && ./healthcheck.sh

# Manual deploy
cd /opt/stk2eth && ./deploy.sh

# Restart services
docker-compose restart

# Stop services
docker-compose down

# Cleanup
docker system prune -af
```

## Next Steps

After successful deployment:

1. ✅ Verify services are running: `docker ps`
2. ✅ Test USSD endpoint: `curl -X POST http://YOUR_IP:8080/ussd`
3. ✅ Check health: `curl http://YOUR_IP:8080/health`
4. ✅ Review logs: `docker-compose logs`
5. ✅ Setup monitoring alerts (optional)
6. ✅ Configure domain name (optional)
7. ✅ Setup SSL/TLS with Let's Encrypt (optional)
