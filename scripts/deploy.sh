#!/bin/bash
# STK2ETH Deployment Script for DigitalOcean
# This script handles the deployment of STK2ETH services on a DigitalOcean droplet

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
DEPLOY_DIR="/opt/stk2eth"
LOG_DIR="$DEPLOY_DIR/logs"
BACKUP_DIR="$DEPLOY_DIR/backups"
MAX_BACKUPS=5

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi

    if [ ! -f "$DEPLOY_DIR/.env" ]; then
        log_error ".env file not found at $DEPLOY_DIR/.env"
        exit 1
    fi

    log_info "Prerequisites check passed"
}

# Create backup
create_backup() {
    log_info "Creating backup..."

    mkdir -p "$BACKUP_DIR"

    BACKUP_NAME="backup_$(date +%Y%m%d_%H%M%S)"
    BACKUP_PATH="$BACKUP_DIR/$BACKUP_NAME"

    mkdir -p "$BACKUP_PATH"

    # Backup current .env
    if [ -f "$DEPLOY_DIR/.env" ]; then
        cp "$DEPLOY_DIR/.env" "$BACKUP_PATH/.env"
    fi

    # Backup docker-compose.yml
    if [ -f "$DEPLOY_DIR/docker-compose.yml" ]; then
        cp "$DEPLOY_DIR/docker-compose.yml" "$BACKUP_PATH/docker-compose.yml"
    fi

    # Export container logs
    if docker ps -q -f name=stk2eth &> /dev/null; then
        docker-compose logs --no-color > "$BACKUP_PATH/logs.txt" 2>&1 || true
    fi

    log_info "Backup created at $BACKUP_PATH"

    # Cleanup old backups (keep last MAX_BACKUPS)
    BACKUP_COUNT=$(ls -1 "$BACKUP_DIR" | wc -l)
    if [ "$BACKUP_COUNT" -gt "$MAX_BACKUPS" ]; then
        log_info "Cleaning up old backups (keeping last $MAX_BACKUPS)..."
        ls -1t "$BACKUP_DIR" | tail -n +$((MAX_BACKUPS + 1)) | xargs -I {} rm -rf "$BACKUP_DIR/{}"
    fi
}

# Pull latest images
pull_images() {
    log_info "Pulling latest Docker images..."

    source "$DEPLOY_DIR/.env"

    docker pull ${DOCKER_REGISTRY:-registry.digitalocean.com}/${DOCKER_USERNAME:-stk2eth}/stk2eth-ussdclient:${IMAGE_TAG:-latest}
    docker pull ${DOCKER_REGISTRY:-registry.digitalocean.com}/${DOCKER_USERNAME:-stk2eth}/stk2eth-ethclient:${IMAGE_TAG:-latest}

    log_info "Images pulled successfully"
}

# Stop existing containers
stop_containers() {
    log_info "Stopping existing containers..."

    cd "$DEPLOY_DIR"

    if docker ps -q -f name=stk2eth &> /dev/null; then
        docker-compose down --timeout 30
        log_info "Containers stopped successfully"
    else
        log_warn "No running containers found"
    fi
}

# Start new containers
start_containers() {
    log_info "Starting new containers..."

    cd "$DEPLOY_DIR"
    docker-compose up -d

    log_info "Waiting for containers to be healthy..."
    sleep 10

    # Check if containers are running
    if docker ps -q -f name=stk2eth &> /dev/null; then
        log_info "Containers started successfully"
        docker-compose ps
    else
        log_error "Failed to start containers"
        return 1
    fi
}

# Verify deployment
verify_deployment() {
    log_info "Verifying deployment..."

    # Check USSD client health
    USSD_HEALTH=$(docker inspect --format='{{.State.Health.Status}}' stk2eth-ussdclient 2>/dev/null || echo "unknown")

    if [ "$USSD_HEALTH" = "healthy" ] || [ "$USSD_HEALTH" = "unknown" ]; then
        log_info "USSD client status: $USSD_HEALTH"
    else
        log_warn "USSD client status: $USSD_HEALTH"
    fi

    # Check if services are responding
    if curl -f -s -o /dev/null http://localhost:8080/health; then
        log_info "Health endpoint is responding"
    else
        log_warn "Health endpoint is not responding"
    fi

    # Display container status
    docker ps -f name=stk2eth --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
}

# Cleanup old images
cleanup_images() {
    log_info "Cleaning up old Docker images..."

    # Remove dangling images
    docker image prune -f

    # Remove images older than 7 days (keep recent versions)
    docker image prune -af --filter "until=168h" || true

    log_info "Cleanup completed"
}

# Rollback function
rollback() {
    log_error "Deployment failed, initiating rollback..."

    # Find latest backup
    LATEST_BACKUP=$(ls -1t "$BACKUP_DIR" | head -n 1)

    if [ -n "$LATEST_BACKUP" ]; then
        log_info "Rolling back to backup: $LATEST_BACKUP"

        # Restore .env
        if [ -f "$BACKUP_DIR/$LATEST_BACKUP/.env" ]; then
            cp "$BACKUP_DIR/$LATEST_BACKUP/.env" "$DEPLOY_DIR/.env"
        fi

        # Restart with previous configuration
        cd "$DEPLOY_DIR"
        docker-compose down
        docker-compose up -d

        log_info "Rollback completed"
    else
        log_error "No backup found for rollback"
        exit 1
    fi
}

# Main deployment flow
main() {
    log_info "Starting STK2ETH deployment..."
    log_info "Deployment directory: $DEPLOY_DIR"
    log_info "Timestamp: $(date)"

    # Change to deployment directory
    cd "$DEPLOY_DIR"

    # Check prerequisites
    check_prerequisites

    # Create backup before deployment
    create_backup

    # Pull latest images
    if ! pull_images; then
        log_error "Failed to pull images"
        rollback
        exit 1
    fi

    # Stop existing containers
    stop_containers

    # Start new containers
    if ! start_containers; then
        log_error "Failed to start containers"
        rollback
        exit 1
    fi

    # Verify deployment
    verify_deployment

    # Cleanup old images
    cleanup_images

    # Save deployment info
    echo "$(date): Deployment completed successfully" >> "$DEPLOY_DIR/deployment.log"

    log_info "✅ Deployment completed successfully!"
}

# Trap errors and rollback
trap 'rollback' ERR

# Run main function
main "$@"
