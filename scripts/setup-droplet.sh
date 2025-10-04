#!/bin/bash
# DigitalOcean Droplet Initial Setup Script
# Run this script once on a fresh Ubuntu 22.04 droplet

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Update system
update_system() {
    log_info "Updating system packages..."
    apt-get update
    apt-get upgrade -y
    apt-get install -y \
        curl \
        wget \
        git \
        vim \
        htop \
        ufw \
        fail2ban \
        ca-certificates \
        gnupg \
        lsb-release
}

# Install Docker
install_docker() {
    log_info "Installing Docker..."

    # Add Docker's official GPG key
    mkdir -p /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg

    # Set up the repository
    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
      $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

    # Install Docker Engine
    apt-get update
    apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

    # Start and enable Docker
    systemctl start docker
    systemctl enable docker

    # Verify installation
    docker --version
    docker compose version

    log_info "Docker installed successfully"
}

# Configure firewall
configure_firewall() {
    log_info "Configuring firewall..."

    # Allow SSH
    ufw allow 22/tcp

    # Allow HTTP/HTTPS
    ufw allow 80/tcp
    ufw allow 443/tcp

    # Allow USSD port
    ufw allow 8080/tcp

    # Enable firewall
    ufw --force enable

    log_info "Firewall configured"
}

# Configure fail2ban
configure_fail2ban() {
    log_info "Configuring fail2ban..."

    systemctl start fail2ban
    systemctl enable fail2ban

    log_info "fail2ban configured"
}

# Setup deployment directory
setup_deployment_directory() {
    log_info "Setting up deployment directory..."

    mkdir -p /opt/stk2eth
    mkdir -p /opt/stk2eth/logs
    mkdir -p /opt/stk2eth/backups

    chmod 755 /opt/stk2eth

    log_info "Deployment directory created at /opt/stk2eth"
}

# Install doctl (DigitalOcean CLI)
install_doctl() {
    log_info "Installing doctl..."

    cd /tmp
    wget https://github.com/digitalocean/doctl/releases/download/v1.98.1/doctl-1.98.1-linux-amd64.tar.gz
    tar xf doctl-1.98.1-linux-amd64.tar.gz
    mv doctl /usr/local/bin
    rm doctl-1.98.1-linux-amd64.tar.gz

    doctl version

    log_info "doctl installed successfully"
}

# Configure Docker logging
configure_docker_logging() {
    log_info "Configuring Docker logging..."

    cat > /etc/docker/daemon.json <<EOF
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  }
}
EOF

    systemctl restart docker

    log_info "Docker logging configured"
}

# Setup log rotation
setup_log_rotation() {
    log_info "Setting up log rotation..."

    cat > /etc/logrotate.d/stk2eth <<EOF
/opt/stk2eth/logs/*.log {
    daily
    missingok
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 root root
    sharedscripts
}
EOF

    log_info "Log rotation configured"
}

# Create monitoring script
create_monitoring_script() {
    log_info "Creating monitoring script..."

    cat > /opt/stk2eth/monitor.sh <<'EOF'
#!/bin/bash
# Simple monitoring script

# Check if containers are running
if ! docker ps | grep -q stk2eth; then
    echo "$(date): STK2ETH containers not running, attempting restart..." >> /opt/stk2eth/logs/monitor.log
    cd /opt/stk2eth && docker-compose up -d
fi

# Check disk space
DISK_USAGE=$(df -h /opt | awk 'NR==2 {print $5}' | sed 's/%//')
if [ "$DISK_USAGE" -gt 90 ]; then
    echo "$(date): CRITICAL - Disk usage at ${DISK_USAGE}%" >> /opt/stk2eth/logs/monitor.log
    # Cleanup old Docker images
    docker image prune -af --filter "until=72h"
fi
EOF

    chmod +x /opt/stk2eth/monitor.sh

    # Add to crontab (run every 5 minutes)
    (crontab -l 2>/dev/null; echo "*/5 * * * * /opt/stk2eth/monitor.sh") | crontab -

    log_info "Monitoring script created and scheduled"
}

# Setup SSH security
configure_ssh() {
    log_info "Configuring SSH security..."

    # Backup original config
    cp /etc/ssh/sshd_config /etc/ssh/sshd_config.backup

    # Configure SSH settings
    sed -i 's/#PermitRootLogin yes/PermitRootLogin prohibit-password/' /etc/ssh/sshd_config
    sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config
    sed -i 's/#PubkeyAuthentication yes/PubkeyAuthentication yes/' /etc/ssh/sshd_config

    # Restart SSH
    systemctl restart sshd

    log_info "SSH security configured"
}

# Setup swap (for low memory droplets)
setup_swap() {
    log_info "Setting up swap space..."

    # Check if swap already exists
    if [ -f /swapfile ]; then
        log_warn "Swap file already exists, skipping..."
        return
    fi

    # Create 2GB swap
    fallocate -l 2G /swapfile
    chmod 600 /swapfile
    mkswap /swapfile
    swapon /swapfile

    # Make swap permanent
    echo '/swapfile none swap sw 0 0' | tee -a /etc/fstab

    # Configure swappiness
    sysctl vm.swappiness=10
    echo 'vm.swappiness=10' | tee -a /etc/sysctl.conf

    log_info "Swap space configured"
}

# Install monitoring tools
install_monitoring_tools() {
    log_info "Installing monitoring tools..."

    apt-get install -y \
        nethogs \
        iotop \
        iftop \
        sysstat

    log_info "Monitoring tools installed"
}

# Create deployment user (optional)
create_deploy_user() {
    log_info "Creating deployment user..."

    if id "deploy" &>/dev/null; then
        log_warn "User 'deploy' already exists, skipping..."
        return
    fi

    useradd -m -s /bin/bash deploy
    usermod -aG docker deploy

    # Setup SSH for deploy user
    mkdir -p /home/deploy/.ssh
    chmod 700 /home/deploy/.ssh
    chown -R deploy:deploy /home/deploy/.ssh

    log_info "Deployment user 'deploy' created"
    log_warn "Remember to add SSH public key to /home/deploy/.ssh/authorized_keys"
}

# Main setup
main() {
    log_info "Starting DigitalOcean droplet setup for STK2ETH..."
    log_info "This will install Docker, configure firewall, and setup deployment environment"

    update_system
    install_docker
    configure_firewall
    configure_fail2ban
    setup_deployment_directory
    install_doctl
    configure_docker_logging
    setup_log_rotation
    create_monitoring_script
    configure_ssh
    setup_swap
    install_monitoring_tools
    create_deploy_user

    log_info "✅ Droplet setup completed successfully!"
    echo ""
    log_info "Next steps:"
    echo "1. Add your SSH public key to /root/.ssh/authorized_keys or /home/deploy/.ssh/authorized_keys"
    echo "2. Configure DigitalOcean Container Registry: doctl registry login"
    echo "3. Set up GitHub secrets for CI/CD deployment"
    echo "4. Push your first deployment from GitHub Actions"
    echo ""
    log_info "Deployment directory: /opt/stk2eth"
    log_info "Firewall status:"
    ufw status
}

# Run main function
main "$@"
