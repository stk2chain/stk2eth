.PHONY: all build check test clean run-ussd run-eth dev logs install setup help \
		lint fmt fmt-check quality \
		docker-build docker-build-ussd docker-build-eth docker-build-all \
		docker-run docker-run-ussd docker-run-eth docker-stop docker-clean \
		docker-logs docker-compose-up docker-compose-down docker-push

# Load variables from .env
ifneq (,$(wildcard .env))
    include .env
    export $(shell sed 's/=.*//' .env)
endif

# Docker configuration
DOCKER_REGISTRY ?= ghcr.io
DOCKER_USERNAME ?= stk2chain
IMAGE_NAME ?= stk2eth
IMAGE_TAG ?= latest
USSD_IMAGE := $(DOCKER_REGISTRY)/$(DOCKER_USERNAME)/$(IMAGE_NAME)-ussdclient:$(IMAGE_TAG)
ETH_IMAGE := $(DOCKER_REGISTRY)/$(DOCKER_USERNAME)/$(IMAGE_NAME)-ethclient:$(IMAGE_TAG)
ALL_IMAGE := $(DOCKER_REGISTRY)/$(DOCKER_USERNAME)/$(IMAGE_NAME):$(IMAGE_TAG)

# Default target
all: check build test

# Install dependencies
install:
	@echo "Installing Rust if not present..."
	@which rustc || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	@echo "Installing SpacetimeDB CLI..."
	@which spacetime || cargo install spacetimedb-cli
	@echo "Dependencies installed"

# Setup project
setup: install
	@echo "Setting up SpacetimeDB..."
	@spacetime version || echo "⚠ SpacetimeDB CLI not found - install manually"
	@echo "Setup complete"

# Check components
check:
	@echo "Checking workspace compilation..."
	@cargo check --workspace
	@echo "Checking individual components..."
	@cd ussdgeth && cargo check
	@cd ussdclient && cargo check
	@cd ethclient && cargo check
	@echo "✓ All components compile successfully"

# Linting with clippy
lint:
	@echo "Running clippy linter..."
	@cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "✓ Linting passed"

# Check code formatting
fmt-check:
	@echo "Checking code formatting..."
	@cargo fmt --all -- --check
	@echo "✓ Formatting check passed"

# Format code
fmt:
	@echo "Formatting code..."
	@cargo fmt --all
	@echo "✓ Code formatted"

# Run all quality checks (lint + format)
quality: fmt-check lint
	@echo "✓ All quality checks passed"

# Build everything
build:
	@echo "Building workspace..."
	@cargo build --workspace
	@cargo build --workspace --release
	@echo "✓ All components built successfully"

# Build contracts
build-contracts:
	@echo "Building smart contracts..."
	@cd contracts && forge build
	@echo "✓ Contracts built successfully"

# Run tests
test:
	@echo "Running tests..."
	@cargo test --workspace
	@cd contracts && forge test
	@echo "✓ All tests passed"

# Deploy module to SpacetimeDB (remote)
deploy-db:
	@echo "Deploying $(SPACETIME_DB_NAME) to $(SPACETIME_SERVER)..."
	@cd ussdgeth && spacetime publish -s $(SPACETIME_SERVER) -- $(SPACETIME_DB_NAME)
	@echo "✓ Database deployed"

# Run USSD client
run-ussd:
	@echo "Starting USSD client server on port $(USSD_PORT)..."
	@cd ussdclient && cargo run --release

# Run Ethereum client
run-eth:
	@echo "Starting Ethereum client..."
	@cd ethclient && cargo run --release

# Development mode
dev: build deploy-db
	@echo "Starting development env (using $(SPACETIME_SERVER))..."
	@cd ussdclient && cargo run --release &
	@cd ethclient && cargo run --release &
	@echo "✓ Development environment started"
	@echo "USSD endpoint: http://localhost:$(USSD_PORT)/ussd"
	@echo "SpacetimeDB console: https://$(SPACETIME_SERVER).spacetimedb.com"
	@if [ -n "$(SPACETIME_DB_ID)" ]; then \
		spacetime call $(SPACETIME_DB_ID) handle_ussd \
		  "1" "+254700000000" "63902" "*123#" "" \
		  -s $(SPACETIME_SERVER) || echo "⚠ Reducer call failed"; \
	else \
		echo "⚠ SPACETIME_DB_ID not set in .env"; \
	fi

# Stop dev services
stop-dev:
	@echo "Stopping services..."
	@-pkill -f "cargo run"
	@echo "✓ All services stopped"

# Logs
logs:
	@echo "=== USSD Logs ==="
	@tail -n 20 ussd_client.log 2>/dev/null || echo "No USSD logs"

# Clean
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@cd contracts && forge clean
	@rm -f *.log
	@echo "✓ Clean complete"

# Verify system health
verify: fmt-check lint check build test
	@echo "✓ Formatting: PASSED"
	@echo "✓ Linting: PASSED"
	@echo "✓ Compilation: PASSED"
	@echo "✓ Build: PASSED"
	@echo "✓ Tests: PASSED"
	@echo "Checking SpacetimeDB connection ($(SPACETIME_SERVER))..."
	@spacetime list -s $(SPACETIME_SERVER) || echo "⚠ SpacetimeDB not accessible"
	@echo "Checking reducer response..."
	@if [ -n "$(SPACETIME_DB_ID)" ]; then \
		spacetime call $(SPACETIME_DB_ID) handle_ussd \
		  "1" "+254700000000" "63902" "*123#" "" \
		  -s $(SPACETIME_SERVER) || echo "⚠ Reducer call failed"; \
	else \
		echo "⚠ SPACETIME_DB_ID not set in .env"; \
	fi
	@echo "System verification complete"

# Production build
prod: clean
	@cargo build --workspace --release
	@cd contracts && forge build
	@echo "✓ Production build complete"

# Show status
status:
	@echo "=== Project Status ==="
	@rustc --version
	@cargo --version
	@spacetime version || echo "SpacetimeDB CLI not installed"
	@cd contracts && forge --version || echo "Forge not installed"
	@echo ""
	@echo "=== Service Status ==="
	@pgrep -f "cargo run" >/dev/null && echo "✓ Rust services running" || echo "✗ No Rust services running"
	@docker ps --filter "name=stk2eth" --format "table {{.Names}}\t{{.Status}}" 2>/dev/null || echo "✗ No Docker containers running"

# ============================================================================
# Docker Commands
# ============================================================================

# Build all Docker images
docker-build: docker-build-ussd docker-build-eth docker-build-all
	@echo "✓ All Docker images built successfully"

# Build USSD client image
docker-build-ussd:
	@echo "Building USSD client Docker image..."
	@docker build --target ussdclient -t $(USSD_IMAGE) .
	@docker tag $(USSD_IMAGE) $(DOCKER_USERNAME)/$(IMAGE_NAME)-ussdclient:latest
	@echo "✓ USSD client image built: $(USSD_IMAGE)"

# Build ETH client image
docker-build-eth:
	@echo "Building ETH client Docker image..."
	@docker build --target ethclient -t $(ETH_IMAGE) .
	@docker tag $(ETH_IMAGE) $(DOCKER_USERNAME)/$(IMAGE_NAME)-ethclient:latest
	@echo "✓ ETH client image built: $(ETH_IMAGE)"

# Build all-in-one image
docker-build-all:
	@echo "Building all-in-one Docker image..."
	@docker build --target all-in-one -t $(ALL_IMAGE) .
	@docker tag $(ALL_IMAGE) $(DOCKER_USERNAME)/$(IMAGE_NAME):latest
	@echo "✓ All-in-one image built: $(ALL_IMAGE)"

# Run USSD client container
docker-run-ussd:
	@echo "Starting USSD client container..."
	@docker run -d \
		--name stk2eth-ussdclient \
		--env-file .env \
		-p $(USSD_PORT):8080 \
		--restart unless-stopped \
		$(USSD_IMAGE)
	@echo "✓ USSD client running on port $(USSD_PORT)"
	@echo "Container ID: $$(docker ps -q -f name=stk2eth-ussdclient)"

# Run ETH client container
docker-run-eth:
	@echo "Starting ETH client container..."
	@docker run -d \
		--name stk2eth-ethclient \
		--env-file .env \
		--restart unless-stopped \
		$(ETH_IMAGE)
	@echo "✓ ETH client running"
	@echo "Container ID: $$(docker ps -q -f name=stk2eth-ethclient)"

# Run all-in-one container
docker-run:
	@echo "Starting all-in-one container..."
	@docker run -d \
		--name stk2eth-all \
		--env-file .env \
		-p $(USSD_PORT):8080 \
		--restart unless-stopped \
		$(ALL_IMAGE)
	@echo "✓ All services running on port $(USSD_PORT)"
	@echo "Container ID: $$(docker ps -q -f name=stk2eth-all)"

# Stop all containers
docker-stop:
	@echo "Stopping all STK2ETH containers..."
	@docker stop $$(docker ps -q -f name=stk2eth) 2>/dev/null || echo "No running containers"
	@docker rm $$(docker ps -aq -f name=stk2eth) 2>/dev/null || echo "No containers to remove"
	@echo "✓ All containers stopped and removed"

# Clean all Docker resources
docker-clean: docker-stop
	@echo "Removing all STK2ETH Docker images..."
	@docker rmi $(USSD_IMAGE) $(ETH_IMAGE) $(ALL_IMAGE) 2>/dev/null || true
	@docker rmi $(DOCKER_USERNAME)/$(IMAGE_NAME)-ussdclient:latest 2>/dev/null || true
	@docker rmi $(DOCKER_USERNAME)/$(IMAGE_NAME)-ethclient:latest 2>/dev/null || true
	@docker rmi $(DOCKER_USERNAME)/$(IMAGE_NAME):latest 2>/dev/null || true
	@echo "✓ Docker cleanup complete"

# View container logs
docker-logs:
	@echo "=== Docker Container Logs ==="
	@docker ps -f name=stk2eth --format "{{.Names}}" | while read container; do \
		echo "\n--- $$container ---"; \
		docker logs --tail 50 $$container; \
	done

# Push images to registry
docker-push:
	@echo "Pushing images to $(DOCKER_REGISTRY)..."
	@docker push $(USSD_IMAGE)
	@docker push $(ETH_IMAGE)
	@docker push $(ALL_IMAGE)
	@echo "✓ All images pushed successfully"

# Docker Compose commands
docker-compose-up:
	@echo "Starting services with docker-compose..."
	@docker-compose up -d
	@echo "✓ All services started"
	@docker-compose ps

docker-compose-down:
	@echo "Stopping services with docker-compose..."
	@docker-compose down
	@echo "✓ All services stopped"

# Help
help:
	@echo "STK2ETH Makefile"
	@echo ""
	@echo "=== Local Development ==="
	@echo "make setup             - Install deps & setup env"
	@echo "make build             - Build workspace"
	@echo "make test              - Run tests"
	@echo "make lint              - Run clippy linter"
	@echo "make fmt               - Format code"
	@echo "make fmt-check         - Check code formatting"
	@echo "make quality           - Run all quality checks (lint + format)"
	@echo "make deploy-db         - Deploy module to SpacetimeDB"
	@echo "make dev               - Start dev env with remote SpacetimeDB"
	@echo "make verify            - Full verification (fmt + lint + build + test + DB)"
	@echo "make prod              - Production build"
	@echo "make stop-dev          - Stop running services"
	@echo "make logs              - View logs"
	@echo "make clean             - Clean build artifacts"
	@echo ""
	@echo "=== Docker Commands ==="
	@echo "make docker-build      - Build all Docker images"
	@echo "make docker-build-ussd - Build USSD client image"
	@echo "make docker-build-eth  - Build ETH client image"
	@echo "make docker-build-all  - Build all-in-one image"
	@echo "make docker-run-ussd   - Run USSD client container"
	@echo "make docker-run-eth    - Run ETH client container"
	@echo "make docker-run        - Run all-in-one container"
	@echo "make docker-stop       - Stop all containers"
	@echo "make docker-clean      - Remove all containers and images"
	@echo "make docker-logs       - View container logs"
	@echo "make docker-push       - Push images to registry"
	@echo ""
	@echo "=== Docker Compose ==="
	@echo "make docker-compose-up   - Start all services with docker-compose"
	@echo "make docker-compose-down - Stop all services"
