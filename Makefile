.PHONY: all build check test clean run-ussd run-eth dev logs install setup help

# Load variables from .env
ifneq (,$(wildcard .env))
    include .env
    export $(shell sed 's/=.*//' .env)
endif

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
	@cd ussdgeth && spacetime publish -s $(SPACETIME_SERVER) -- $(SPACETIME_DB_NAME) --clear-database
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
verify: check build test
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

# Help
help:
	@echo "STK2ETH Makefile"
	@echo ""
	@echo "make setup        - Install deps & setup env"
	@echo "make build        - Build workspace"
	@echo "make test         - Run tests"
	@echo "make deploy-db    - Deploy module to SpacetimeDB"
	@echo "make dev          - Start dev env with remote SpacetimeDB"
	@echo "make verify       - Verify build + DB connection"
	@echo "make prod         - Production build"
	@echo "make stop-dev     - Stop running services"
	@echo "make logs         - View logs"
	@echo "make clean        - Clean build artifacts"
