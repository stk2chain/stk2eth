# USSD Geth - SpacetimeDB Module

## Overview

The `ussdgeth` module is the core SpacetimeDB component of the STK2ETH project that handles USSD (Unstructured Supplementary Service Data) interactions for Ethereum transactions. This module enables users to perform Ethereum operations through SMS-like USSD menus without requiring internet connectivity.

## Architecture

This is a **SpacetimeDB module** compiled to WebAssembly (WASM) that provides:
- USSD session management
- Ethereum transaction processing
- FATF compliance audit logging
- Account abstraction wallet operations

## Key Components

### Tables
- `USSDSession` - Manages active USSD sessions with phone numbers and menu states
- `Swap` - Records Ethereum swap transactions and their status
- `EthAuditLog` - FATF-compliant audit trail for regulatory compliance

### Reducers (API Endpoints)
- `send_eth()` - Processes ETH transfer requests
- `log_send_eth_transaction()` - Creates audit logs for transactions
- `log_send_eth_transaction_with_fatf()` - Enhanced FATF compliance logging

### Key Features
- **USSD Menu Navigation** - State machine for USSD menu flows
- **Session Management** - Tracks user interactions across USSD sessions
- **Transaction Processing** - Handles ETH transfers and swaps
- **Audit Logging** - FATF travel rule compliance with 100% persistence
- **Immutable Records** - Tamper-proof transaction history

## Development

### Prerequisites
```bash
# Install SpacetimeDB CLI
curl --proto '=https' --tlsv1.2 -sSf https://install.spacetimedb.com | sh

# Add WASM target for Rust
rustup target add wasm32-unknown-unknown
```

### Building
```bash
# Build the WASM module
cargo build --target wasm32-unknown-unknown --release

# Deploy to SpacetimeDB
spacetime publish ussdgeth
```

### Testing
```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Stress test audit logging (1000+ transactions)
../stress_test.sh
```

## Usage

### Starting SpacetimeDB
```bash
spacetime start
```

### Calling Reducers
```bash
# Log a transaction
spacetime call ussdgeth log_send_eth_transaction "0xabc123" "0xfrom" "0xto" "1000000000000000000" "+254712345678" "session123"

# Query audit logs
spacetime sql ussdgeth "SELECT * FROM eth_audit_logs WHERE phone_number = '+254712345678'"
```

## File Structure

```
ussdgeth/
├── src/
│   ├── lib.rs              # Main module with table definitions
│   ├── audit_reducers.rs   # FATF audit logging reducers
│   └── audit_tests.rs      # Comprehensive test suite
├── Cargo.toml             # Dependencies and build config
└── README.md              # This file
```

## Dependencies

- `spacetimedb` - Core SpacetimeDB functionality
- `serde` - Serialization/deserialization
- `log` - Logging framework
- `anyhow` - Error handling
- `thiserror` - Custom error types

## Compliance

This module implements **FATF (Financial Action Task Force) travel rule** compliance:
- All transactions logged with originator/beneficiary information
- Immutable audit trail with 100% persistence
- Query performance under 30ms for 1000+ records
- Regulatory reporting capabilities

## Related Components

- **ussdclient** - HTTP bridge for USSD gateway integration
- **ethclient** - Ethereum blockchain interaction
- **contracts** - Smart contracts for account abstraction