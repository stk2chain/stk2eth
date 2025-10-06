# Docker Deployment Guide - STK2ETH

## Overview

This guide covers Docker deployment for the STK2ETH project. The project includes multi-stage Dockerfiles for production-ready containerized deployments.

## Architecture

The Docker setup provides three deployment options:

1. **Separate Services** - Individual containers for USSD client and ETH client
2. **All-in-One** - Single container running both services via supervisor
3. **Docker Compose** - Orchestrated multi-container deployment

## Quick Start

### Prerequisites

- Docker 20.10+ installed
- Docker Compose 2.0+ (optional, for orchestration)
- `.env` file configured (copy from `.env.example`)

### Build Images

```bash
# Build all images
make docker-build

# Or build individual images
make docker-build-ussd    # USSD client only
make docker-build-eth     # ETH client only
make docker-build-all     # All-in-one image
```

### Run Containers

#### Option 1: Using Make Commands

```bash
# Run all-in-one container (recommended for simple deployments)
make docker-run

# Or run separate services
make docker-run-ussd
make docker-run-eth

# View logs
make docker-logs

# Stop all containers
make docker-stop
```

#### Option 2: Using Docker Compose

```bash
# Start all services
make docker-compose-up

# View status
docker-compose ps

# View logs
docker-compose logs -f

# Stop all services
make docker-compose-down
```

#### Option 3: Direct Docker Commands

```bash
# Run USSD client
docker run -d \
  --name stk2eth-ussdclient \
  --env-file .env \
  -p 8080:8080 \
  stk2chain/stk2eth-ussdclient:latest

# Run ETH client
docker run -d \
  --name stk2eth-ethclient \
  --env-file .env \
  stk2chain/stk2eth-ethclient:latest

# Run all-in-one
docker run -d \
  --name stk2eth-all \
  --env-file .env \
  -p 8080:8080 \
  stk2chain/stk2eth:latest
```

## Configuration

### Environment Variables

Required environment variables (set in `.env`):

```bash
# SpacetimeDB Configuration
SPACETIME_SERVER=maincloud.spacetimedb.com
SPACETIME_DB_NAME=ussdgeth
SPACETIME_DB_ID=your-database-id
SPACETIME_AUTH_TOKEN=your-auth-token

# Service Ports
USSD_PORT=8080

# Logging
RUST_LOG=info
```

### Docker Registry Configuration

For pushing to a custom registry:

```bash
# Set in .env or export
export DOCKER_REGISTRY=ghcr.io
export DOCKER_USERNAME=stk2chain
export IMAGE_TAG=v1.0.0

# Build with custom tag
make docker-build

# Push to registry
make docker-push
```

## Multi-Stage Build Details

### Stage 1: Builder

- Base: `rust:1.83-slim-bookworm`
- Compiles Rust workspace
- Strips binaries for size optimization
- Output: Optimized release binaries

### Stage 2: USSD Client Runtime

- Base: `debian:bookworm-slim`
- Non-root user: `ussd`
- Exposed port: `8080`
- Health check: `curl http://localhost:8080/health`
- Final image size: ~80MB

### Stage 3: ETH Client Runtime

- Base: `debian:bookworm-slim`
- Non-root user: `eth`
- No exposed ports (internal service)
- Final image size: ~70MB

### Stage 4: All-in-One Runtime

- Base: `debian:bookworm-slim`
- Process manager: `supervisor`
- Non-root user: `stk2eth`
- Runs both services in single container
- Final image size: ~120MB

## Production Deployment

### Using Docker Compose (Recommended)

```bash
# Production deployment
docker-compose -f docker-compose.yml up -d

# Scale services if needed
docker-compose up -d --scale ussdclient=3
```

### Kubernetes Deployment

Example deployment manifests:

```yaml
# ussdclient-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: stk2eth-ussdclient
spec:
  replicas: 3
  selector:
    matchLabels:
      app: stk2eth-ussdclient
  template:
    metadata:
      labels:
        app: stk2eth-ussdclient
    spec:
      containers:
      - name: ussdclient
        image: ghcr.io/stk2chain/stk2eth-ussdclient:latest
        ports:
        - containerPort: 8080
        envFrom:
        - configMapRef:
            name: stk2eth-config
        - secretRef:
            name: stk2eth-secrets
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: stk2eth-ussdclient
spec:
  selector:
    app: stk2eth-ussdclient
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

### Health Checks

All images include health checks:

```bash
# USSD Client health check
curl http://localhost:8080/health

# Container health status
docker ps --filter "name=stk2eth" --format "table {{.Names}}\t{{.Status}}"
```

## Troubleshooting

### View Container Logs

```bash
# All containers
make docker-logs

# Specific container
docker logs -f stk2eth-ussdclient
docker logs -f stk2eth-ethclient

# With docker-compose
docker-compose logs -f ussdclient
```

### Debug Container

```bash
# Execute shell in running container
docker exec -it stk2eth-ussdclient /bin/bash

# Check environment variables
docker exec stk2eth-ussdclient env | grep SPACETIME

# Test connectivity to SpacetimeDB
docker exec stk2eth-ussdclient curl -v https://maincloud.spacetimedb.com
```

### Common Issues

#### Container exits immediately

```bash
# Check logs for errors
docker logs stk2eth-ussdclient

# Verify environment variables
docker exec stk2eth-ussdclient env

# Run interactively for debugging
docker run -it --rm --env-file .env stk2chain/stk2eth-ussdclient:latest /bin/bash
```

#### Health check failing

```bash
# Check if service is listening
docker exec stk2eth-ussdclient netstat -tlnp

# Test health endpoint manually
docker exec stk2eth-ussdclient curl -v http://localhost:8080/health
```

#### SpacetimeDB connection issues

```bash
# Verify network connectivity
docker exec stk2eth-ussdclient ping -c 3 maincloud.spacetimedb.com

# Check DNS resolution
docker exec stk2eth-ussdclient nslookup maincloud.spacetimedb.com

# Test HTTPS connection
docker exec stk2eth-ussdclient curl -v https://maincloud.spacetimedb.com
```

## Image Optimization

### Size Optimization

Current image sizes:
- USSD Client: ~80MB
- ETH Client: ~70MB
- All-in-One: ~120MB

Further optimization tips:

```dockerfile
# Use musl for static linking (smaller size)
FROM rust:1.83-alpine AS builder
RUN apk add --no-cache musl-dev

# Multi-architecture builds
docker buildx build --platform linux/amd64,linux/arm64 -t myimage:latest .
```

### Build Cache Optimization

```bash
# Use BuildKit for better caching
export DOCKER_BUILDKIT=1

# Build with cache from registry
docker build --cache-from ghcr.io/stk2chain/stk2eth:latest .
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Build and Push Docker Images
  run: |
    echo ${{ secrets.GITHUB_TOKEN }} | docker login ghcr.io -u ${{ github.actor }} --password-stdin
    export IMAGE_TAG=${{ github.sha }}
    make docker-build
    make docker-push
```

### GitLab CI

```yaml
docker-build:
  stage: build
  script:
    - docker login -u $CI_REGISTRY_USER -p $CI_REGISTRY_PASSWORD $CI_REGISTRY
    - export IMAGE_TAG=$CI_COMMIT_SHA
    - make docker-build
    - make docker-push
```

## Monitoring

### Prometheus Metrics (Future)

```bash
# Expose metrics endpoint
curl http://localhost:8080/metrics
```

### Log Aggregation

```yaml
# docker-compose.yml logging configuration
logging:
  driver: "fluentd"
  options:
    fluentd-address: "localhost:24224"
    tag: "stk2eth.{{.Name}}"
```

## Security Best Practices

1. **Non-root users** - All containers run as non-root users
2. **Secret management** - Use Docker secrets or external secret managers
3. **Image scanning** - Scan images for vulnerabilities:

```bash
# Using Trivy
trivy image stk2chain/stk2eth-ussdclient:latest

# Using Docker Scout
docker scout cves stk2chain/stk2eth-ussdclient:latest
```

4. **Network isolation** - Use custom networks for service isolation
5. **Read-only filesystem** - Mount volumes as read-only where possible

## Makefile Commands Reference

```bash
make docker-build          # Build all images
make docker-build-ussd     # Build USSD client image
make docker-build-eth      # Build ETH client image
make docker-build-all      # Build all-in-one image
make docker-run            # Run all-in-one container
make docker-run-ussd       # Run USSD client container
make docker-run-eth        # Run ETH client container
make docker-stop           # Stop all containers
make docker-clean          # Remove all containers and images
make docker-logs           # View container logs
make docker-push           # Push images to registry
make docker-compose-up     # Start with docker-compose
make docker-compose-down   # Stop docker-compose services
```

## Support

For issues and questions:
- GitHub Issues: https://github.com/stk2chain/stk2eth/issues
- Documentation: See README.md and CLAUDE.md
