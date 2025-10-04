# Scripts - Automation & Validation Tools

## Overview

The `scripts` directory contains automation scripts, validation tools, and utilities for testing, deployment, and system maintenance of the STK2ETH project. These scripts help streamline development workflows and ensure system reliability.

## Scripts

### validate_testnet.rs
Comprehensive validation script for testing the send_eth reducer against local testnets.

**Purpose:**
- Validates send_eth functionality against local Ethereum testnet
- Performs balance delta validation for transaction accuracy
- Tests multiple transfer scenarios for reliability verification
- Provides automated regression testing capabilities

**Features:**
- **Testnet Integration** - Works with Anvil/Ganache local networks
- **Balance Validation** - Verifies ETH balances before/after transactions
- **Batch Testing** - Executes 100+ transfers for load testing
- **SpacetimeDB Integration** - Tests reducer calls and database updates
- **Error Reporting** - Detailed logging of failures and success rates

**Configuration:**
```rust
let spacetimedb_url = "http://localhost:3000";
let testnet_rpc = "http://localhost:8545"; // Anvil/Ganache
let test_accounts = generate_test_accounts();
let transfers_to_test = 100;
```

**Usage:**
```bash
# Prerequisites: Start testnet and SpacetimeDB
anvil &
spacetime start &

# Run validation
cargo run --bin validate_testnet

# Expected output:
# ✅ 100/100 transfers successful
# ✅ All balance deltas validated
# ✅ System performance within limits
```

**Test Scenarios:**
1. **Initial Balance Recording** - Capture baseline balances
2. **Transfer Execution** - Execute send_eth operations
3. **Balance Verification** - Validate expected balance changes
4. **Performance Metrics** - Measure transaction throughput
5. **Error Handling** - Test failure scenarios and recovery

## Usage Patterns

### Development Testing
```bash
# Quick validation during development
cargo run --bin validate_testnet

# Verbose output for debugging
RUST_LOG=debug cargo run --bin validate_testnet

# Custom transfer count
TRANSFER_COUNT=50 cargo run --bin validate_testnet
```

### Continuous Integration
```bash
# Run in CI environment
scripts/ci_validate.sh

# Generate test reports
cargo run --bin validate_testnet --release > test_report.log
```

### Load Testing
```bash
# High-volume testing
TRANSFER_COUNT=1000 cargo run --bin validate_testnet

# Concurrent validation
for i in {1..5}; do
  cargo run --bin validate_testnet &
done
wait
```

## Script Development

### Adding New Scripts

#### Rust Scripts
1. Create new `.rs` file in scripts directory
2. Add binary target to `Cargo.toml`:
```toml
[[bin]]
name = "script_name"
path = "scripts/script_name.rs"
```
3. Implement main function with error handling
4. Add documentation and usage examples

#### Shell Scripts
1. Create executable `.sh` file
2. Add shebang and set permissions: `chmod +x script.sh`
3. Follow shell scripting best practices
4. Include usage documentation in comments

### Script Standards

#### Error Handling
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Script implementation
    Ok(())
}
```

#### Logging
```rust
use log::{info, warn, error};

fn main() {
    env_logger::init();
    info!("Script starting");
    // Implementation
}
```

#### Configuration
```rust
use std::env;

let config = Config {
    spacetimedb_url: env::var("SPACETIMEDB_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string()),
    testnet_rpc: env::var("TESTNET_RPC")
        .unwrap_or_else(|_| "http://localhost:8545".to_string()),
};
```

## Common Utilities

### Environment Setup
Scripts often require specific environment setup:

```bash
# Start required services
export SPACETIMEDB_URL="http://localhost:3000"
export TESTNET_RPC="http://localhost:8545"

# Start dependencies
spacetime start &
anvil --port 8545 &
```

### Test Data Generation
```rust
fn generate_test_accounts() -> Vec<String> {
    vec![
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string(),
        "0x70997970C51812dc3A010C7d01b50e0d17dc79C8".to_string(),
        // ... more test accounts
    ]
}
```

### Balance Validation
```rust
async fn get_balances(
    rpc_url: &str,
    accounts: &[String]
) -> Result<HashMap<String, u64>, Box<dyn std::error::Error>> {
    // Implementation for balance queries
}
```

## Integration with CI/CD

### GitHub Actions Integration
```yaml
name: Script Validation
on: [push, pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
      - name: Start Services
        run: |
          spacetime start &
          anvil &
      - name: Run Validation
        run: cargo run --bin validate_testnet
```

### Pre-commit Hooks
```bash
#!/bin/sh
# .git/hooks/pre-commit
cargo run --bin validate_testnet --quiet
if [ $? -ne 0 ]; then
  echo "Validation failed. Commit aborted."
  exit 1
fi
```

## Performance Monitoring

### Metrics Collection
Scripts can collect and report performance metrics:

```rust
struct PerformanceMetrics {
    total_transfers: u32,
    successful_transfers: u32,
    failed_transfers: u32,
    average_response_time: Duration,
    total_execution_time: Duration,
}
```

### Benchmarking
```rust
use std::time::Instant;

let start = Instant::now();
// Execute operations
let duration = start.elapsed();
println!("Operation completed in: {:?}", duration);
```

## Error Scenarios

### Common Test Cases
- **Network failures** - Testnet connectivity issues
- **Invalid addresses** - Malformed Ethereum addresses
- **Insufficient balances** - Accounts with insufficient ETH
- **Concurrent operations** - Race conditions and conflicts
- **Service unavailability** - SpacetimeDB or testnet downtime

### Recovery Procedures
```rust
async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    max_retries: u32,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
{
    // Implementation with exponential backoff
}
```

## Script Maintenance

### Regular Updates
- Update test accounts and configuration
- Verify compatibility with system changes
- Update dependencies and error handling
- Maintain documentation accuracy

### Version Management
- Tag script versions with releases
- Maintain backward compatibility
- Document breaking changes
- Provide migration guides

## Related Components

Scripts interact with all project components:
- **ussdgeth** - Calls reducers and validates functionality
- **ussdclient** - Tests HTTP endpoints and integration
- **ethclient** - Validates blockchain interactions
- **contracts** - Tests smart contract deployments
- **tests** - Complements formal test suites
- **../stress_test.sh** - Performance testing coordination