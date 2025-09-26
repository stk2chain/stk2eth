.PHONY: all build check test clean run-ussd run-eth run-gateway dev logs install setup help

# Default target
all: check build test

# Install required dependencies
install:
	@echo "Installing Rust if not present..."
	@which rustc || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	@echo "Installing spacetimedb CLI..."
	@which spacetime || cargo install spacetimedb-cli
	@echo "Dependencies installed"

# Setup project environment
setup: install
	@echo "Setting up SpacetimeDB..."
	@spacetime version || echo "SpacetimeDB CLI not found - install manually"
	@echo "Creating local SpacetimeDB instance..."
	@-spacetime local start 2>/dev/null || echo "SpacetimeDB already running or needs manual setup"
	@echo "Setup complete"

# Check all components compile
check:
	@echo "Checking workspace compilation..."
	@cargo check --workspace
	@echo "Checking individual components..."
	@cd ussdgeth && cargo check
	@cd ussdclient && cargo check
	@cd ethclient && cargo check
	@echo "✓ All components compile successfully"

# Build all components
build:
	@echo "Building workspace..."
	@cargo build --workspace
	@echo "Building release versions..."
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
	@echo "Running contract tests..."
	@cd contracts && forge test
	@echo "✓ All tests passed"

# Deploy SpacetimeDB module
deploy-db:
	@echo "Deploying ussdgeth module to SpacetimeDB..."
	@cd ussdgeth && spacetime publish --clear-database ussdgeth
	@echo "✓ Database module deployed"

# Run USSD client server
run-ussd:
	@echo "Starting USSD client server on port 8080..."
	@cd ussdclient && cargo run --release

# Run Ethereum client
run-eth:
	@echo "Starting Ethereum client..."
	@cd ethclient && cargo run --release

# Run SpacetimeDB gateway (development)
run-gateway:
	@echo "Starting SpacetimeDB gateway..."
	@spacetime local start

# Development mode - run all services
dev: build deploy-db
	@echo "Starting development environment..."
	@echo "Starting SpacetimeDB gateway..."
	@-spacetime local start &
	@sleep 2
	@echo "Deploying database module..."
	@cd ussdgeth && spacetime publish --clear-database ussdgeth || echo "Module deployment failed - check SpacetimeDB setup"
	@echo "Starting USSD client server..."
	@cd ussdclient && cargo run --release &
	@echo "✓ Development environment started"
	@echo "USSD endpoint: http://localhost:8080/ussd"
	@echo "SpacetimeDB console: http://localhost:3000"

# Stop all development services
stop-dev:
	@echo "Stopping development services..."
	@-pkill -f "cargo run"
	@-spacetime local stop
	@echo "✓ All services stopped"

# View logs from running services
logs:
	@echo "Showing recent logs..."
	@echo "=== USSD Client Logs ==="
	@tail -n 20 ussd_client.log 2>/dev/null || echo "No USSD client logs found"
	@echo "=== SpacetimeDB Logs ==="
	@tail -n 20 ~/.spacetimedb/logs/* 2>/dev/null || echo "No SpacetimeDB logs found"

# Clean all build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@cd contracts && forge clean
	@rm -f *.log
	@echo "✓ Clean complete"

# Verify entire system health
verify: check build test
	@echo "Running system verification..."
	@echo "✓ Compilation: PASSED"
	@echo "✓ Build: PASSED" 
	@echo "✓ Tests: PASSED"
	@echo "Checking SpacetimeDB connection..."
	@spacetime list || echo "⚠ SpacetimeDB not accessible"
	@echo "System verification complete"

# Quick development cycle
quick: check
	@echo "Quick build and test cycle..."
	@cargo build --workspace
	@cargo test --workspace --lib
	@echo "✓ Quick cycle complete"

# Production build
prod: clean
	@echo "Building for production..."
	@cargo build --workspace --release
	@cd contracts && forge build
	@echo "✓ Production build complete"

# Monitor running services
monitor:
	@echo "Monitoring running services..."
	@echo "=== Process Status ==="
	@ps aux | grep -E "(spacetime|cargo run|ussd)" | grep -v grep || echo "No services running"
	@echo "=== Port Usage ==="
	@lsof -i :8080 2>/dev/null || echo "Port 8080 not in use"
	@lsof -i :3000 2>/dev/null || echo "Port 3000 not in use" 
	@echo "=== Recent Logs ==="
	@tail -n 5 ~/.spacetimedb/logs/* 2>/dev/null || echo "No SpacetimeDB logs"

# Reset development environment
reset: stop-dev clean setup
	@echo "Resetting development environment..."
	@spacetime local clear || echo "Could not clear SpacetimeDB"
	@echo "✓ Environment reset complete"

# Show project status
status:
	@echo "=== Project Status ==="
	@echo "Rust version: $$(rustc --version)"
	@echo "Cargo version: $$(cargo --version)"
	@echo "SpacetimeDB CLI: $$(spacetime version 2>/dev/null || echo 'Not installed')"
	@echo "Forge version: $$(cd contracts && forge --version 2>/dev/null || echo 'Not installed')"
	@echo ""
	@echo "=== Build Status ==="
	@cargo check --workspace --quiet && echo "✓ Workspace compiles" || echo "✗ Compilation errors"
	@echo ""
	@echo "=== Service Status ==="
	@pgrep -f spacetime >/dev/null && echo "✓ SpacetimeDB running" || echo "✗ SpacetimeDB not running"
	@pgrep -f "cargo run" >/dev/null && echo "✓ Rust services running" || echo "✗ No Rust services running"

# Help target
help:
	@echo "STK2ETH Project Makefile"
	@echo ""
	@echo "Setup Commands:"
	@echo "  install        - Install required dependencies (Rust, SpacetimeDB CLI)"
	@echo "  setup          - Setup project environment"
	@echo "  reset          - Reset development environment"
	@echo ""
	@echo "Build Commands:"
	@echo "  check          - Check all components compile"
	@echo "  build          - Build all components"
	@echo "  build-contracts- Build smart contracts"
	@echo "  prod           - Production build"
	@echo "  quick          - Quick development cycle"
	@echo ""
	@echo "Run Commands:"
	@echo "  dev            - Start development environment (all services)"
	@echo "  run-ussd       - Run USSD client server only"
	@echo "  run-eth        - Run Ethereum client only"
	@echo "  run-gateway    - Run SpacetimeDB gateway only"
	@echo "  deploy-db      - Deploy database module to SpacetimeDB"
	@echo ""
	@echo "Maintenance Commands:"
	@echo "  test           - Run all tests"
	@echo "  clean          - Clean build artifacts"
	@echo "  verify         - Verify entire system health"
	@echo "  status         - Show project and service status"
	@echo "  monitor        - Monitor running services"
	@echo "  logs           - View recent logs"
	@echo "  stop-dev       - Stop all development services"
	@echo ""
	@echo "Usage Examples:"
	@echo "  make setup && make dev    # First time setup and run"
	@echo "  make quick               # Quick development cycle"
	@echo "  make verify              # Check everything works"