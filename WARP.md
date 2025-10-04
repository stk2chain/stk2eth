# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

STK2ETH is an innovative Account Abstraction eSIM Toolkit (eSTK) wallet that enables ETH transactions over USSD without requiring internet connectivity. The system bridges blockchain functionality with traditional telecom infrastructure, leveraging eSIM secure elements as Trusted Execution Environments (TEE).

### Core Components

1. **Java Card Applet (eSTK)** - Secure key generation and transaction signing within eSIM secure elements
2. **USSD Gateway (ussdgeth)** - SpaceTimeDB-based module handling USSD menu flows and session management
3. **USSD Client Bridge** - Axum-based HTTP server bridging Africa's Talking USSD API to SpaceTimeDB
4. **Smart Contracts** - ERC-4337 Account Abstraction contracts with eSIM registry functionality
5. **ETH Client** - Basic Ethereum client implementation (early stage)

### Key Architecture Principles

- **Hardware Root of Trust**: Private keys generated and stored exclusively within eSIM secure elements
- **No Seed Phrases**: Keys never leave the secure element, ensuring non-custodial security
- **Offline Transactions**: USSD-based interface enables blockchain interactions without internet
- **Account Abstraction**: ERC-4337 compatible wallet infrastructure

## Development Commands

### Smart Contracts (Foundry)
```bash
cd contracts/

# Build contracts
forge build

# Run tests
forge test

# Run tests with verbosity
forge test -vvv

# Deploy locally (requires anvil running)
forge script script/Counter.s.sol --rpc-url http://localhost:8545 --broadcast

# Clean build artifacts
forge clean
```

### USSD Gateway (SpaceTimeDB Module)
```bash
cd ussdgeth/

# Build the WASM module
cargo build --release

# Publish to local SpaceTimeDB instance
spacetime publish --project-path . spacetime-module

# View database tables
spacetime sql "SELECT * FROM ussd_session"
spacetime sql "SELECT * FROM ussd_menu"
spacetime sql "SELECT * FROM ussd_screen"
```

### USSD Client Bridge
```bash
cd ussdclient/

# Build the client
cargo build

# Run the HTTP bridge server (connects to SpaceTimeDB)
cargo run

# Test USSD flow with sample payload
curl -X POST http://localhost:8080/ussd \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "sessionId=test123&serviceCode=*4337*&phoneNumber=+123456789&networkCode=99999&text="
```

### ETH Client
```bash
cd ethclient/

# Build (currently minimal)
cargo build

# Run
cargo run
```

### Running Tests
```bash
# Smart contract tests
cd contracts && forge test

# Python integration tests (TODO - currently placeholder)
python tests/test_stk_flow.py
```

## Code Architecture

### USSD Flow Architecture
The USSD system follows a three-layer architecture:

1. **Telco Layer**: Interfaces with Africa's Talking USSD API
2. **USSD Framework Layer**: Processes menu navigation using SpaceTimeDB
3. **Service Layer**: Handles business logic (transaction processing, balance queries, etc.)

### Menu System
USSD menus are configured via JSON (`ussdgeth/src/data/menu.json`) with support for:
- **Initial Screens**: Entry points that auto-navigate
- **Menu Screens**: Display options to users
- **Input Screens**: Collect user data
- **Function Screens**: Execute business logic
- **Router Screens**: Conditional navigation based on results
- **Quit Screens**: End session with messages

### Smart Contract Structure
- `EsimRegistry.sol`: Manages eSIM profile to wallet address mappings with EIP-712 signatures
- `Counter.sol`: Template contract for testing
- ERC-4337 compatible architecture planned

### Database Schema (SpaceTimeDB)
- `ussd_session`: Tracks user sessions with phone numbers, current screen state
- `ussd_menu`: Service code definitions (*4337# etc.)
- `ussd_screen`: Individual screen definitions with types and navigation rules
- `menu_item`: Options within menu screens
- `router_option`: Conditional navigation rules

## Branch Naming Convention

Follow strict branch naming enforced by git hooks:
```
<type>/<scope>-<issueID>-<description>
```

**Types**: `feat|fix|enhance|chore|docs|test|style`
**Scopes**: `javacard|gateway|contract|euicc|ci|benchmark`

Examples:
- `feat/gateway-42-implement-balance-query`
- `fix/contract-15-resolve-registry-overflow`

## Commit Message Format

Commits must follow structured format with measurable outcomes:

```
<type>(<scope>): <observable code change>
--
<intent: objective outcome>
<metric>: <operator><value> [@ <test>]
--
BREAKING|DEPRECATED: <instruction>
--
Ref|Fixes|Closes #<issueID>
```

Example:
```
enhance(gateway): batch USSD session queries
--
Reduce database load for concurrent users
Query time: ≤200ms @ 100 concurrent sessions
--
Ref #77
```

## Key Development Notes

### SpaceTimeDB Integration
- The `ussdgeth` module compiles to WASM and runs within SpaceTimeDB
- Database operations use the SpaceTimeDB SDK with tables defined via `#[table]` macros
- Reducers handle state transitions triggered by USSD requests

### Security Considerations
- eSIM applet follows BIP-32/BIP-44 key derivation with secp256r1 (P-256) curves
- Private keys generated exclusively within Java Card VM isolated execution environment
- EIP-712 signatures used for registry operations

### USSD Protocol Compliance
References ETSI technical specifications:
- TS 131 111 (USAT/STK) for proactive commands
- TS 123 038 for GSM 7-bit alphabet and USSD packing
- TS 102 223 for Card Application Toolkit

### Service Integration
- USSD client bridges Africa's Talking webhook format to SpaceTimeDB reducers
- Authentication handled via bearer tokens for SpaceTimeDB SQL queries
- Menu navigation persisted across USSD session interactions

## File Structure Context

- `contracts/` - Foundry-based Solidity contracts
- `ussdgeth/` - SpaceTimeDB module for USSD logic
- `ussdclient/` - HTTP bridge server
- `ethclient/` - Ethereum client (early development)
- `doc/specs/` - Technical specifications for each component
- `.githooks/` - Enforced branch and commit validation
- `tests/` - Integration tests (planned)