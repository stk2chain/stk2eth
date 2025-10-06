#!/bin/bash
# STK2ETH Health Check Script
# Verifies that all services are running correctly

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

DEPLOY_DIR="/opt/stk2eth"
USSD_PORT=${USSD_PORT:-8080}
MAX_RETRIES=5
RETRY_DELAY=5

log_info() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Check if containers are running
check_containers() {
    echo "=== Container Status ==="

    RUNNING_CONTAINERS=$(docker ps -f name=stk2eth --format "{{.Names}}" | wc -l)

    if [ "$RUNNING_CONTAINERS" -eq 0 ]; then
        log_error "No STK2ETH containers are running"
        return 1
    fi

    log_info "$RUNNING_CONTAINERS container(s) running"
    docker ps -f name=stk2eth --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

    return 0
}

# Check container health status
check_container_health() {
    echo ""
    echo "=== Container Health ==="

    for container in $(docker ps -f name=stk2eth --format "{{.Names}}"); do
        HEALTH=$(docker inspect --format='{{.State.Health.Status}}' "$container" 2>/dev/null || echo "no healthcheck")

        if [ "$HEALTH" = "healthy" ]; then
            log_info "$container: $HEALTH"
        elif [ "$HEALTH" = "no healthcheck" ]; then
            log_warn "$container: $HEALTH"
        else
            log_error "$container: $HEALTH"
            return 1
        fi
    done

    return 0
}

# Check USSD endpoint
check_ussd_endpoint() {
    echo ""
    echo "=== USSD Endpoint Check ==="

    for i in $(seq 1 $MAX_RETRIES); do
        HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST http://localhost:$USSD_PORT/ussd \
            -H "Content-Type: application/x-www-form-urlencoded" \
            -d "sessionId=healthcheck&serviceCode=*4337#&phoneNumber=+123456789&networkCode=99999&text=" \
            2>/dev/null || echo "000")

        if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
            log_info "USSD endpoint responding (HTTP $HTTP_CODE)"
            return 0
        else
            log_warn "Attempt $i/$MAX_RETRIES: USSD endpoint returned HTTP $HTTP_CODE"
            if [ $i -lt $MAX_RETRIES ]; then
                sleep $RETRY_DELAY
            fi
        fi
    done

    log_error "USSD endpoint is not responding after $MAX_RETRIES attempts"
    return 1
}

# Check health endpoint
check_health_endpoint() {
    echo ""
    echo "=== Health Endpoint Check ==="

    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
        http://localhost:$USSD_PORT/health 2>/dev/null || echo "000")

    if [ "$HTTP_CODE" = "200" ]; then
        log_info "Health endpoint responding (HTTP $HTTP_CODE)"
        return 0
    else
        log_warn "Health endpoint returned HTTP $HTTP_CODE"
        return 1
    fi
}

# Check container logs for errors
check_logs() {
    echo ""
    echo "=== Recent Error Logs ==="

    cd "$DEPLOY_DIR"

    ERROR_COUNT=$(docker-compose logs --tail=100 2>&1 | grep -iE "error|fatal|panic" | wc -l || echo "0")

    if [ "$ERROR_COUNT" -gt 0 ]; then
        log_warn "Found $ERROR_COUNT error(s) in recent logs"
        docker-compose logs --tail=20 2>&1 | grep -iE "error|fatal|panic" || true
        return 1
    else
        log_info "No errors found in recent logs"
        return 0
    fi
}

# Check disk space
check_disk_space() {
    echo ""
    echo "=== Disk Space Check ==="

    DISK_USAGE=$(df -h /opt | awk 'NR==2 {print $5}' | sed 's/%//')

    if [ "$DISK_USAGE" -gt 90 ]; then
        log_error "Disk usage critical: ${DISK_USAGE}%"
        return 1
    elif [ "$DISK_USAGE" -gt 80 ]; then
        log_warn "Disk usage high: ${DISK_USAGE}%"
    else
        log_info "Disk usage: ${DISK_USAGE}%"
    fi

    return 0
}

# Check memory usage
check_memory() {
    echo ""
    echo "=== Memory Usage ==="

    MEMORY_USAGE=$(free | grep Mem | awk '{printf("%.0f", $3/$2 * 100)}')

    if [ "$MEMORY_USAGE" -gt 90 ]; then
        log_error "Memory usage critical: ${MEMORY_USAGE}%"
        return 1
    elif [ "$MEMORY_USAGE" -gt 80 ]; then
        log_warn "Memory usage high: ${MEMORY_USAGE}%"
    else
        log_info "Memory usage: ${MEMORY_USAGE}%"
    fi

    return 0
}

# Check Docker daemon
check_docker_daemon() {
    echo "=== Docker Daemon Check ==="

    if docker info > /dev/null 2>&1; then
        log_info "Docker daemon is running"
        return 0
    else
        log_error "Docker daemon is not running"
        return 1
    fi
}

# Display summary
display_summary() {
    echo ""
    echo "=== Health Check Summary ==="
    echo "Timestamp: $(date)"
    echo "Checks Passed: $CHECKS_PASSED"
    echo "Checks Failed: $CHECKS_FAILED"
    echo "Checks Warning: $CHECKS_WARNING"

    if [ "$CHECKS_FAILED" -gt 0 ]; then
        log_error "Health check FAILED"
        return 1
    elif [ "$CHECKS_WARNING" -gt 0 ]; then
        log_warn "Health check passed with warnings"
        return 0
    else
        log_info "All health checks PASSED"
        return 0
    fi
}

# Main health check
main() {
    echo "========================================="
    echo "  STK2ETH Health Check"
    echo "========================================="
    echo ""

    CHECKS_PASSED=0
    CHECKS_FAILED=0
    CHECKS_WARNING=0

    # Run all checks
    check_docker_daemon && ((CHECKS_PASSED++)) || ((CHECKS_FAILED++))
    check_containers && ((CHECKS_PASSED++)) || ((CHECKS_FAILED++))
    check_container_health && ((CHECKS_PASSED++)) || ((CHECKS_WARNING++))
    check_ussd_endpoint && ((CHECKS_PASSED++)) || ((CHECKS_FAILED++))
    check_health_endpoint && ((CHECKS_PASSED++)) || ((CHECKS_WARNING++))
    check_logs && ((CHECKS_PASSED++)) || ((CHECKS_WARNING++))
    check_disk_space && ((CHECKS_PASSED++)) || ((CHECKS_WARNING++))
    check_memory && ((CHECKS_PASSED++)) || ((CHECKS_WARNING++))

    # Display summary
    display_summary
}

# Run main function
main "$@"
