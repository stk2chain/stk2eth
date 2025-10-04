# Tests - Integration & End-to-End Testing

## Overview

The `tests` directory contains comprehensive test suites for the STK2ETH project, including integration tests, end-to-end scenarios, and system validation. These tests ensure the entire system works correctly across all components.

## Test Structure

This directory contains multiple types of tests:
- **Integration Tests** - Cross-component testing
- **USSD Flow Tests** - End-to-end USSD interaction scenarios
- **Menu System Tests** - USSD menu navigation validation
- **Transaction Flow Tests** - Complete transaction processing verification

## Test Files

### test_send_eth.rs
Rust integration test for ETH sending functionality.

**Purpose:** Validates the send_eth reducer and transaction processing
**Coverage:**
- Send ETH transaction creation
- Balance validation
- Error handling for failed transactions
- Integration with ethclient component

**Usage:**
```bash
cargo test test_send_eth
```

### test_send_eth_menu.py
Python test for USSD menu system validation.

**Purpose:** Ensures USSD menu structure includes Send ETH functionality
**Coverage:**
- Menu configuration validation
- Service definition verification
- Navigation flow correctness
- MainScreen to ToNumberScreen transitions

**Usage:**
```bash
pytest test_send_eth_menu.py
```

### test_stk_flow.py
Python test for STK (SIM Toolkit) flow validation.

**Purpose:** Validates STK integration and eSIM functionality
**Coverage:** (Currently placeholder - needs implementation)

### integration/
Directory containing detailed integration test scenarios.

#### send_eth_test.rs
Comprehensive SpacetimeDB integration tests for send_eth functionality.

**Test Scenarios:**
- `test_send_eth_reducer_fails_initially()` - Validates TDD approach
- `test_send_eth_creates_swap_transaction()` - Transaction creation verification
- `test_send_eth_validates_addresses()` - Address validation testing
- `test_send_eth_validates_amount()` - Amount validation testing
- `test_send_eth_updates_session_state()` - Session state management

**Mock Objects:**
- TestReducerContext for SpacetimeDB simulation
- USSDSession creation helpers
- Transaction validation utilities

## Running Tests

### Prerequisites
```bash
# For Rust tests
cargo test

# For Python tests
pip install pytest
pip install -r requirements-test.txt

# Start SpacetimeDB (for integration tests)
spacetime start
```

### Individual Test Execution

#### Rust Integration Tests
```bash
# Run all Rust tests
cargo test

# Run specific test module
cargo test --test send_eth_test

# Run with verbose output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration
```

#### Python Tests
```bash
# Run all Python tests
pytest

# Run specific test file
pytest test_send_eth_menu.py

# Run with verbose output
pytest -v

# Run with coverage
pytest --cov=ussdgeth
```

### Test Categories

#### Unit Tests
Located within individual component directories (ussdgeth/src/, ethclient/src/, etc.)
```bash
# Run unit tests for specific component
cd ussdgeth && cargo test
cd ethclient && cargo test
```

#### Integration Tests
Located in this tests/ directory
```bash
# Run all integration tests
cargo test --test "*"
pytest
```

#### End-to-End Tests
Full system tests including multiple components
```bash
# Run complete system test
../stress_test.sh

# Manual E2E testing
spacetime start &
cargo run --bin ussdclient &
# Execute USSD flow simulation
```

## Test Configuration

### Environment Setup
```env
# Test database configuration
TEST_DB_URL=memory://
SPACETIMEDB_URL=http://localhost:3000

# Test network configuration
TEST_NETWORK=localhost
TEST_CHAIN_ID=1337

# Test wallet configuration
TEST_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
TEST_WALLET_ADDRESS=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
```

### Test Data
```json
{
  "test_session": {
    "session_id": "test_session_001",
    "phone_number": "+1234567890",
    "network_code": "TEST_NET",
    "service_code": "*ETH#"
  },
  "test_addresses": {
    "from": "0x742d35Cc6634C0532925a3b8D0A9E9B5F8C8C4C1",
    "to": "0x8ba1f109551bD432803012645Hac136c6b3d283c"
  }
}
```

## Test Scenarios

### USSD Flow Testing
1. **Menu Navigation**
   - User dials service code
   - Main menu displays correctly
   - Send ETH option available
   - Navigation to amount entry

2. **Transaction Processing**
   - Amount validation
   - Address validation
   - Transaction creation
   - Confirmation flow

3. **Error Handling**
   - Invalid inputs
   - Network failures
   - Insufficient balance
   - Timeout scenarios

### SpacetimeDB Integration
1. **Reducer Testing**
   - send_eth reducer functionality
   - Audit logging reducers
   - Session management reducers
   - Error propagation

2. **Database Operations**
   - Table insertions and updates
   - Query performance
   - Data consistency
   - Transaction atomicity

### Multi-Component Integration
1. **USSD Client → SpacetimeDB**
   - HTTP request processing
   - Reducer invocation
   - Response formatting
   - Error handling

2. **SpacetimeDB → ETH Client**
   - Transaction submission
   - Balance queries
   - Contract interactions
   - Event monitoring

## Continuous Integration

### GitHub Actions
```yaml
name: Test Suite
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
      - name: Setup Python
        uses: actions/setup-python@v3
      - name: Install SpacetimeDB
        run: curl --proto '=https' --tlsv1.2 -sSf https://install.spacetimedb.com | sh
      - name: Run Rust Tests
        run: cargo test --all
      - name: Run Python Tests
        run: pytest
```

### Test Coverage Requirements
- **Unit Tests:** >80% code coverage per component
- **Integration Tests:** All public APIs covered
- **E2E Tests:** Major user flows validated
- **Performance Tests:** Response time <100ms for USSD flows

## Mock Services

### SpacetimeDB Test Context
```rust
fn create_test_context() -> TestReducerContext {
    let mut test_db = TestDb::new();
    TestReducerContext::new(test_db, Identity::from_str("test_user").unwrap())
}
```

### USSD Session Mocks
```rust
fn create_test_session(ctx: &ReducerContext) -> USSDSession {
    USSDSession {
        session_id: "test_session_001".to_string(),
        phone_number: "+1234567890".to_string(),
        // ... other test data
    }
}
```

### Ethereum Client Mocks
```rust
struct MockEthClient {
    // Mock implementation for testing
}
```

## Performance Testing

### Load Testing
```bash
# Test 1000+ concurrent USSD sessions
./stress_test.sh

# Database performance under load
cargo test --release test_audit_log_performance
```

### Memory Testing
```bash
# Monitor memory usage during tests
valgrind --tool=memcheck cargo test

# Profile memory allocations
cargo test --profile=profiling
```

## Test Maintenance

### Adding New Tests
1. **Choose appropriate test type** (unit, integration, e2e)
2. **Follow naming conventions** (test_*, *_test.py)
3. **Add to CI pipeline** (update GitHub Actions)
4. **Document test purpose** (comments and README updates)

### Test Data Management
- Use deterministic test data
- Clean up after tests
- Avoid test interdependencies
- Mock external services

## Related Components

- **ussdgeth** - Core business logic being tested
- **ussdclient** - HTTP endpoints being validated
- **ethclient** - Blockchain interactions being mocked
- **contracts** - Smart contracts being tested
- **../stress_test.sh** - Performance testing script