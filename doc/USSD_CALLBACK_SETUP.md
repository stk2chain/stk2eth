# USSD Callback URL Setup Guide

This guide provides detailed instructions for setting up callback URLs for USSD development and testing. We cover two primary approaches: **Ngrok** for quick local development and a **Development Server** for stable team environments.

## Table of Contents
- [Option 1: Ngrok Setup (Local Development)](#option-1-ngrok-setup-local-development)
- [Option 2: Development Server Setup](#option-2-development-server-setup)
- [Testing Your Setup](#testing-your-setup)
- [Troubleshooting](#troubleshooting)

---

## Option 1: Ngrok Setup (Local Development)

### Overview
Ngrok creates a secure tunnel from the internet to your local development machine, perfect for testing USSD integrations without deploying code.

### Prerequisites
- Running `ussdclient` locally
- Ngrok account (free tier is sufficient)
- Internet connection

### Step-by-Step Setup

#### 1. Install Ngrok

**macOS:**
```bash
brew install ngrok
```

**Linux:**
```bash
curl -s https://ngrok-agent.s3.amazonaws.com/ngrok.asc | sudo tee /etc/apt/trusted.gpg.d/ngrok.asc >/dev/null
echo "deb https://ngrok-agent.s3.amazonaws.com buster main" | sudo tee /etc/apt/sources.list.d/ngrok.list
sudo apt update && sudo apt install ngrok
```

**Windows:**
Download from [https://ngrok.com/download](https://ngrok.com/download)

#### 2. Create Ngrok Account and Authenticate
```bash
# Sign up at https://dashboard.ngrok.com/signup
# Get your authtoken from https://dashboard.ngrok.com/get-started/your-authtoken

# Configure ngrok with your authtoken
ngrok config add-authtoken YOUR_AUTH_TOKEN_HERE
```

#### 3. Start the USSD Client Service
```bash
# Navigate to ussdclient directory
cd /path/to/stk2eth/ussdclient

# Start the service (default port 8080)
cargo run

# You should see:
# Server running on http://0.0.0.0:8080
```

#### 4. Start Ngrok Tunnel
In a new terminal:
```bash
# Create tunnel to your local service
ngrok http 8080

# You'll see output like:
# Session Status                online
# Account                       your-email@example.com
# Version                       3.5.0
# Region                        United States (us)
# Latency                       32ms
# Web Interface                 http://127.0.0.1:4040
# Forwarding                    https://abc123xyz.ngrok-free.app -> http://localhost:8080
```

#### 5. Configure USSD Provider

Copy your ngrok URL and configure it in your USSD provider (e.g., Africa's Talking):

**Africa's Talking Sandbox Configuration:**
```
Service Code: *384#
Callback URL: https://abc123xyz.ngrok-free.app/ussd
```

**Example callback URL format:**
```
https://[your-subdomain].ngrok-free.app/ussd
```

### Ngrok Configuration File (Optional)

Create `~/.ngrok2/ngrok.yml` for persistent configuration:
```yaml
version: "2"
authtoken: YOUR_AUTH_TOKEN_HERE
tunnels:
  ussd:
    proto: http
    addr: 8080
    inspect: true
    bind_tls: true
    host_header: rewrite
    headers:
      X-Forwarded-Proto: https
```

Run with configuration:
```bash
ngrok start ussd
```

### Advanced Ngrok Features

#### Custom Subdomain (Paid Feature)
```bash
ngrok http 8080 --subdomain=stk2eth-dev
# Results in: https://stk2eth-dev.ngrok-free.app
```

#### Request Inspection
Access the inspection interface at [http://localhost:4040](http://localhost:4040) to:
- View all incoming requests
- Inspect request/response bodies
- Replay requests for debugging

#### Basic Authentication
```bash
ngrok http 8080 --auth="username:password"
```

---

## Option 2: Development Server Setup

### Overview
A dedicated development server provides a stable URL for team development and integration testing.

### Prerequisites
- Linux VPS (Ubuntu 20.04+ recommended)
- Domain name (optional but recommended)
- Basic Linux administration knowledge

### Server Providers
- **DigitalOcean**: $6/month droplet
- **AWS EC2**: t3.micro (free tier eligible)
- **Linode**: $5/month Nanode
- **Hetzner**: €4.51/month CX11

### Step-by-Step Setup

#### 1. Server Initial Setup

**Connect to your server:**
```bash
ssh root@your-server-ip
```

**Create a dedicated user:**
```bash
# Create user for the application
adduser stk2eth
usermod -aG sudo stk2eth

# Switch to new user
su - stk2eth
```

**Update system packages:**
```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y curl git build-essential pkg-config libssl-dev
```

#### 2. Install Dependencies

**Install Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup target add wasm32-unknown-unknown
```

**Install SpacetimeDB:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://install.spacetimedb.com | sh
```

**Install Node.js (for additional tools):**
```bash
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs
```

#### 3. Clone and Build Project

```bash
# Clone the repository
cd /opt
sudo mkdir stk2eth
sudo chown stk2eth:stk2eth stk2eth
cd stk2eth
git clone https://github.com/yourusername/stk2eth.git .

# Build the USSD client
cd ussdclient
cargo build --release

# The binary will be at:
# /opt/stk2eth/ussdclient/target/release/ussdclient
```

#### 4. Configure Environment

Create environment file:
```bash
nano /opt/stk2eth/ussdclient/.env
```

Add configuration:
```env
# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# SpacetimeDB Configuration
SPACETIMEDB_URL=http://localhost:3000
SPACETIMEDB_MODULE=ussdgeth

# USSD Provider Configuration (Africa's Talking example)
AT_USERNAME=sandbox
AT_API_KEY=your_api_key_here
AT_SHORTCODE=*384#

# Logging
RUST_LOG=info
```

#### 5. Setup Systemd Service

Create service file:
```bash
sudo nano /etc/systemd/system/ussdclient.service
```

Add service configuration:
```ini
[Unit]
Description=STK2ETH USSD Client Service
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=stk2eth
WorkingDirectory=/opt/stk2eth/ussdclient
Environment="RUST_LOG=info"
EnvironmentFile=/opt/stk2eth/ussdclient/.env
ExecStart=/opt/stk2eth/ussdclient/target/release/ussdclient
Restart=always
RestartSec=10
StandardOutput=append:/var/log/ussdclient/access.log
StandardError=append:/var/log/ussdclient/error.log

[Install]
WantedBy=multi-user.target
```

Create log directory and start service:
```bash
# Create log directory
sudo mkdir -p /var/log/ussdclient
sudo chown stk2eth:stk2eth /var/log/ussdclient

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable ussdclient
sudo systemctl start ussdclient

# Check status
sudo systemctl status ussdclient
```

#### 6. Install and Configure Nginx

```bash
# Install Nginx
sudo apt install -y nginx

# Remove default site
sudo rm /etc/nginx/sites-enabled/default

# Create site configuration
sudo nano /etc/nginx/sites-available/ussd
```

Add Nginx configuration:
```nginx
upstream ussd_backend {
    server 127.0.0.1:8080;
    keepalive 64;
}

server {
    listen 80;
    server_name ussd.yourdomain.com;  # Replace with your domain

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name ussd.yourdomain.com;  # Replace with your domain

    # SSL certificates (will be configured by Certbot)
    # ssl_certificate /etc/letsencrypt/live/ussd.yourdomain.com/fullchain.pem;
    # ssl_certificate_key /etc/letsencrypt/live/ussd.yourdomain.com/privkey.pem;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Logging
    access_log /var/log/nginx/ussd_access.log;
    error_log /var/log/nginx/ussd_error.log;

    # USSD endpoint
    location /ussd {
        proxy_pass http://ussd_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;

        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # Health check endpoint
    location /health {
        proxy_pass http://ussd_backend;
        access_log off;
    }
}
```

Enable site:
```bash
sudo ln -s /etc/nginx/sites-available/ussd /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

#### 7. Setup SSL Certificate

```bash
# Install Certbot
sudo apt install -y certbot python3-certbot-nginx

# Obtain SSL certificate
sudo certbot --nginx -d ussd.yourdomain.com \
  --non-interactive \
  --agree-tos \
  --email your-email@example.com

# Auto-renewal is configured automatically
# Test renewal:
sudo certbot renew --dry-run
```

#### 8. Configure Firewall

```bash
# Install and configure UFW
sudo apt install -y ufw

# Configure firewall rules
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Enable firewall
sudo ufw enable
```

#### 9. Setup Monitoring (Optional)

**Install monitoring tools:**
```bash
# Install htop for process monitoring
sudo apt install -y htop

# Install netdata for comprehensive monitoring
bash <(curl -Ss https://my-netdata.io/kickstart.sh)
```

**Setup log rotation:**
```bash
sudo nano /etc/logrotate.d/ussdclient
```

Add:
```
/var/log/ussdclient/*.log {
    daily
    missingok
    rotate 14
    compress
    delaycompress
    notifempty
    create 640 stk2eth stk2eth
    sharedscripts
    postrotate
        systemctl reload ussdclient 2>/dev/null || true
    endscript
}
```

### Server Maintenance Commands

```bash
# View logs
sudo journalctl -u ussdclient -f

# Restart service
sudo systemctl restart ussdclient

# Update application
cd /opt/stk2eth
git pull
cd ussdclient
cargo build --release
sudo systemctl restart ussdclient

# Check service status
sudo systemctl status ussdclient

# View Nginx logs
sudo tail -f /var/log/nginx/ussd_access.log
sudo tail -f /var/log/nginx/ussd_error.log
```

---

## Testing Your Setup

### 1. Test Health Endpoint

**For Ngrok:**
```bash
curl https://abc123xyz.ngrok-free.app/health
```

**For Development Server:**
```bash
curl https://ussd.yourdomain.com/health
```

Expected response:
```json
{
  "status": "healthy",
  "spacetimedb": "connected"
}
```

### 2. Test USSD Endpoint

```bash
curl -X POST https://your-url/ussd \
  -H "Content-Type: application/json" \
  -d '{
    "sessionId": "test-session-123",
    "phoneNumber": "+254712345678",
    "networkCode": "63902",
    "serviceCode": "*384#",
    "text": ""
  }'
```

Expected response:
```json
{
  "text": "CON Welcome to STK2ETH\n1. Send ETH\n2. Check Balance\n3. Settings",
  "sessionContinue": true
}
```

### 3. Africa's Talking Simulator

For Africa's Talking users, test using their simulator:
1. Log into your Africa's Talking account
2. Go to Sandbox
3. Launch USSD simulator
4. Enter your service code (*384#)
5. Test the flow

---

## Troubleshooting

### Common Issues and Solutions

#### Ngrok Issues

**Issue: "Tunnel not found"**
```bash
# Solution: Restart ngrok
killall ngrok
ngrok http 8080
```

**Issue: "Invalid host header"**
```bash
# Solution: Add host header rewrite
ngrok http 8080 --host-header=rewrite
```

**Issue: "Rate limited"**
```bash
# Solution: Sign up for free account or upgrade plan
# Free account has 40 connections/minute limit
```

#### Development Server Issues

**Issue: "502 Bad Gateway"**
```bash
# Check if ussdclient is running
sudo systemctl status ussdclient

# Check logs
sudo journalctl -u ussdclient -n 50

# Restart service
sudo systemctl restart ussdclient
```

**Issue: "Connection refused"**
```bash
# Check if port is listening
sudo netstat -tlnp | grep 8080

# Check firewall
sudo ufw status

# Check Nginx configuration
sudo nginx -t
```

**Issue: "SSL certificate issues"**
```bash
# Renew certificate
sudo certbot renew --force-renewal

# Check certificate status
sudo certbot certificates
```

### Debug Commands

```bash
# Check service logs
tail -f /var/log/ussdclient/error.log

# Monitor incoming requests (Nginx)
tail -f /var/log/nginx/ussd_access.log

# Check system resources
htop

# Test internal connectivity
curl http://localhost:8080/health

# Check DNS resolution
nslookup ussd.yourdomain.com
```

---

## Security Best Practices

### For Both Setups

1. **API Key Management**
   - Never commit API keys to git
   - Use environment variables
   - Rotate keys regularly

2. **Request Validation**
   - Validate all incoming USSD requests
   - Implement rate limiting
   - Log suspicious activities

3. **HTTPS Only**
   - Always use HTTPS for callbacks
   - Verify SSL certificates
   - Use strong TLS versions (1.2+)

### For Development Server

1. **Server Hardening**
   ```bash
   # Disable root SSH login
   sudo nano /etc/ssh/sshd_config
   # Set: PermitRootLogin no

   # Use SSH keys instead of passwords
   ssh-copy-id stk2eth@your-server-ip
   ```

2. **Regular Updates**
   ```bash
   # Setup automatic security updates
   sudo apt install unattended-upgrades
   sudo dpkg-reconfigure --priority=low unattended-upgrades
   ```

3. **Monitoring**
   - Setup alerts for service downtime
   - Monitor resource usage
   - Track failed authentication attempts

---

## Additional Resources

### Documentation
- [Ngrok Documentation](https://ngrok.com/docs)
- [Africa's Talking USSD API](https://developers.africastalking.com/docs/ussd)
- [Nginx Documentation](https://nginx.org/en/docs/)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)

### Support Channels
- GitHub Issues: [Project Issues](https://github.com/yourusername/stk2eth/issues)
- Ngrok Support: [support@ngrok.com](mailto:support@ngrok.com)
- Team Chat: Your internal communication channel

---

## Quick Reference Card

### Ngrok Commands
```bash
ngrok http 8080                    # Basic tunnel
ngrok http 8080 --region=eu        # EU region
ngrok http 8080 --inspect=false    # Disable inspection
ngrok status                        # Check tunnel status
```

### Server Management
```bash
sudo systemctl start ussdclient    # Start service
sudo systemctl stop ussdclient     # Stop service
sudo systemctl restart ussdclient  # Restart service
sudo systemctl status ussdclient   # Check status
sudo journalctl -u ussdclient -f   # View logs
```

### Testing Commands
```bash
curl http://localhost:8080/health  # Local health check
curl https://your-url/health       # Remote health check
```

---

*Document Version: 1.0*
*Last Updated: October 2025*
*Maintained by: STK2ETH Development Team*