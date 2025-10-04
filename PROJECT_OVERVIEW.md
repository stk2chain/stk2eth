# STK2ETH Project Overview

## 🚀 Executive Summary

**STK2ETH** is a groundbreaking Account Abstraction eSIM Toolkit (eSTK) Wallet that enables Ethereum transactions over USSD without requiring internet connectivity. This project bridges the gap between traditional mobile networks and blockchain technology, making Ethereum accessible to users in areas with limited internet infrastructure.

## 🎯 Project Vision

Enable billions of mobile users worldwide to access Ethereum blockchain services using only their basic mobile phones through USSD technology, democratizing access to decentralized finance and Web3 services.

## 🏗️ System Architecture

```
┌──────────────┐     USSD      ┌──────────────┐     HTTP      ┌──────────────┐
│   Mobile     │ ◄─────────────►│    USSD      │◄─────────────►│  SpacetimeDB │
│   Device     │                │   Gateway    │               │   (ussdgeth) │
│   (eSIM)     │                │ (ussdclient) │               │              │
└──────────────┘                └──────────────┘               └──────┬───────┘
                                                                       │
                                                                       │ RPC
                                                                       ▼
                                ┌──────────────┐               ┌──────────────┐
                                │  Smart       │◄──────────────│   Ethereum   │
                                │  Contracts   │               │    Client    │
                                │(EsimRegistry)│               │  (ethclient) │
                                └──────────────┘               └──────────────┘
```

## 📦 Component Overview

### Core Components

| Component | Purpose | Technology | Status |
|-----------|---------|------------|--------|
| **ussdgeth** | SpacetimeDB module for USSD session management and transaction processing | Rust/WASM | ✅ Active |
| **ussdclient** | HTTP bridge between USSD gateways and SpacetimeDB | Rust/Axum | ✅ Active |
| **ethclient** | Ethereum blockchain interaction layer | Rust | 🚧 In Development |
| **contracts** | Smart contracts for eSIM registry and account abstraction | Solidity/Foundry | ✅ Active |

### Supporting Components

| Component | Purpose | Status |
|-----------|---------|--------|
| **tests** | Integration and end-to-end testing | ✅ Active |
| **doc** | Project specifications and design documents | ✅ Active |
| **scripts** | Automation and validation tools | ✅ Active |

## 🚦 Getting Started

### Prerequisites

1. **Development Tools**
   ```bash
   # Rust toolchain
   curl --proto '=https' --tlsv1.2 -sSf https://rustup.rs | sh

   # SpacetimeDB
   curl --proto '=https' --tlsv1.2 -sSf https://install.spacetimedb.com | sh

   # Foundry (for smart contracts)
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```

2. **WASM Target**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

### Quick Start

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/stk2eth.git
   cd stk2eth
   ```

2. **Start SpacetimeDB**
   ```bash
   spacetime start
   ```

3. **Build and deploy ussdgeth module**
   ```bash
   cd ussdgeth
   cargo build --target wasm32-unknown-unknown --release
   spacetime publish ussdgeth
   ```

4. **Start USSD client**
   ```bash
   cd ../ussdclient
   cargo run
   ```

5. **Deploy smart contracts (local)**
   ```bash
   cd ../contracts
   anvil &  # Start local Ethereum node
   forge build
   forge deploy
   ```

## 🔄 Development Workflow

### Branch Strategy

- `main` - Production-ready code
- `develop` - Integration branch for features
- `feature/*` - Feature development branches
- `hotfix/*` - Critical production fixes

### Commit Convention

Follow conventional commits:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Test additions/changes
- `refactor:` - Code refactoring
- `chore:` - Maintenance tasks

### Testing Requirements

1. **Unit Tests** - Required for all new functions
2. **Integration Tests** - Required for API changes
3. **FATF Compliance** - 100% audit log persistence
4. **Performance** - <100ms USSD response time

## 📊 Key Features

### Current Features ✅

- **USSD Menu System** - Interactive menu navigation without internet
- **eSIM Integration** - Automatic wallet binding to eSIM profiles
- **Send ETH** - Basic ETH transfer functionality
- **FATF Compliance** - Complete audit logging for regulatory compliance
- **Session Management** - Persistent USSD session handling

### Roadmap 🗺️

- **Q1 2025**
  - [ ] ERC-20 token support
  - [ ] Multi-signature wallet support
  - [ ] Enhanced security features

- **Q2 2025**
  - [ ] DeFi protocol integration
  - [ ] Cross-chain support
  - [ ] Advanced account abstraction features

- **Q3 2025**
  - [ ] Production deployment
  - [ ] Partner integrations
  - [ ] Mainnet launch

## 🔐 Security Considerations

### Current Implementation

- **Audit Logging** - FATF-compliant transaction logging
- **Input Validation** - All user inputs sanitized
- **Secure Communication** - HTTPS/TLS for all connections
- **Private Key Management** - Secure key storage (in development)

### Best Practices

1. Never commit sensitive data (private keys, API keys)
2. All PRs require security review for critical paths
3. Regular dependency audits
4. Comprehensive error handling without information leakage

## 📈 Performance Metrics

### Target Metrics

| Metric | Target | Current Status |
|--------|--------|---------------|
| USSD Response Time | <100ms | ✅ ~50ms |
| Transaction Throughput | 1000+ TPS | ✅ 1200 TPS |
| Audit Log Persistence | 100% | ✅ 100% |
| System Uptime | 99.9% | 🚧 Measuring |

### Monitoring

- SpacetimeDB metrics dashboard
- Transaction success rates
- System resource utilization
- Error rate tracking

## 👥 Team Structure

### Development Teams

- **Core Protocol** - USSD/SpacetimeDB integration
- **Blockchain** - Ethereum client and smart contracts
- **Security** - Audit logging and compliance
- **DevOps** - Infrastructure and deployment

### Contributing

Please read [CONTRIBUTING.md](./CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## 📚 Documentation Structure

```
stk2eth/
├── README.md                 # Technical specifications
├── PROJECT_OVERVIEW.md       # This file - project overview
├── CONTRIBUTING.md           # Contribution guidelines
├── doc/                      # Detailed documentation
│   ├── specs/               # Technical specifications
│   └── contributing/        # Development guides
└── [component]/README.md    # Component-specific docs
```

## 🛠️ Development Tools

### Recommended IDE Setup

- **VS Code** with Rust Analyzer
- **Remix IDE** for Solidity development
- **Postman** for API testing

### Useful Commands

```bash
# Run all tests
make test

# Build all components
make build

# Start development environment
make dev

# Run stress tests
./stress_test.sh

# Validate testnet
cargo run --bin validate_testnet
```

## 📞 Support & Communication

### Channels

- **GitHub Issues** - Bug reports and feature requests
- **Discord** - Real-time team communication
- **Weekly Standup** - Team synchronization meetings

### Resources

- [SpacetimeDB Documentation](https://spacetimedb.com/docs)
- [Foundry Book](https://book.getfoundry.sh/)
- [FATF Guidance](https://www.fatf-gafi.org/publications/virtualassets/)
- [Account Abstraction EIPs](https://eips.ethereum.org/EIPS/eip-4337)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- SpacetimeDB team for the database platform
- Ethereum Foundation for blockchain infrastructure
- Africa's Talking for USSD gateway services
- Open source community for tooling and libraries

---

**For detailed technical specifications, see [README.md](./README.md)**
**For component-specific documentation, see individual component README files**