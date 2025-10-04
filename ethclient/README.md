# ETH Client - Ethereum Blockchain Interface

## Overview

The `ethclient` is a Rust library that provides a clean interface for interacting with the Ethereum blockchain. It handles wallet operations, transaction signing, and smart contract interactions for the STK2ETH project's account abstraction functionality.

## Architecture

This is a **Rust library crate** that provides:
- Ethereum JSON-RPC client functionality
- Wallet and key management
- Transaction building and signing
- Smart contract interaction utilities

## Key Features

### Blockchain Interaction
- **JSON-RPC Client** - Connects to Ethereum nodes (Geth, Infura, etc.)
- **Transaction Management** - Build, sign, and broadcast transactions
- **Block Monitoring** - Listen for new blocks and transaction confirmations
- **Gas Estimation** - Dynamic gas price calculation and optimization

### Wallet Operations
- **Key Management** - Generate and manage Ethereum private keys
- **Account Abstraction** - Support for smart contract wallets
- **Multi-signature** - Coordinate multi-sig wallet operations
- **Hardware Wallet** - Integration with hardware security modules

### Smart Contract Integration
- **Contract Deployment** - Deploy new smart contracts
- **Contract Calls** - Interact with existing contracts
- **Event Monitoring** - Listen for smart contract events
- **ABI Encoding/Decoding** - Handle contract interface encoding

## Development Status

⚠️ **Note**: This module is currently a **placeholder/stub** implementation. The core functionality needs to be implemented based on project requirements.

### Planned Features
- [ ] Ethereum node connection management
- [ ] Transaction pool monitoring
- [ ] Account abstraction wallet support
- [ ] ERC-20 token operations
- [ ] Smart contract deployment utilities
- [ ] Gas optimization strategies
- [ ] Multi-network support (mainnet, testnets)

## Development

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://rustup.rs | sh

# For development against local node
# Install and run Ganache CLI or similar
npm install -g ganache-cli
ganache-cli --port 8545
```

### Building
```bash
# Build the library
cargo build

# Run tests
cargo test

# Build with all features
cargo build --all-features
```

### Testing
```bash
# Unit tests
cargo test

# Integration tests (requires running Ethereum node)
cargo test --test integration -- --ignored

# Test against local testnet
cargo test --features="testnet"
```

## Usage

### Basic Setup
```rust
use ethclient::EthClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Ethereum node
    let client = EthClient::new("http://localhost:8545")?;

    // Get latest block
    let block = client.get_latest_block().await?;
    println!("Latest block: {}", block.number);

    Ok(())
}
```

### Wallet Operations
```rust
use ethclient::{EthClient, Wallet};

async fn create_wallet() -> Result<(), Box<dyn std::error::Error>> {
    let client = EthClient::new("http://localhost:8545")?;

    // Create new wallet
    let wallet = Wallet::new_random();

    // Get balance
    let balance = client.get_balance(wallet.address()).await?;
    println!("Balance: {} ETH", balance);

    Ok(())
}
```

### Send Transaction
```rust
use ethclient::{EthClient, Transaction};
use ethers::types::{U256, Address};

async fn send_eth() -> Result<(), Box<dyn std::error::Error>> {
    let client = EthClient::new("http://localhost:8545")?;

    let tx = Transaction {
        to: "0x742d35Cc6634C0532925a3b8D0A9E9B5F8C8C4C1".parse()?,
        value: U256::from(1_000_000_000_000_000_000u64), // 1 ETH
        gas_limit: 21000,
        ..Default::default()
    };

    let tx_hash = client.send_transaction(tx).await?;
    println!("Transaction sent: {}", tx_hash);

    Ok(())
}
```

## Configuration

### Environment Variables
```env
# Ethereum node endpoints
ETH_RPC_URL=http://localhost:8545
ETH_WS_URL=ws://localhost:8545

# Network configuration
CHAIN_ID=1337
NETWORK_NAME=localhost

# Gas configuration
MAX_GAS_PRICE=100000000000  # 100 gwei
GAS_MULTIPLIER=1.2

# Wallet configuration
WALLET_PRIVATE_KEY=0x...
WALLET_MNEMONIC="word1 word2 ..."
```

## File Structure

```
ethclient/
├── src/
│   ├── lib.rs              # Main library interface
│   ├── client.rs           # Ethereum JSON-RPC client
│   ├── wallet.rs           # Wallet and key management
│   ├── transaction.rs      # Transaction building and signing
│   ├── contract.rs         # Smart contract interaction
│   └── types.rs            # Common types and utilities
├── tests/
│   ├── integration/        # Integration tests
│   └── unit/               # Unit tests
├── examples/               # Usage examples
├── Cargo.toml             # Dependencies and build config
└── README.md              # This file
```

## Dependencies

- `tokio` - Async runtime
- `anyhow` - Error handling
- `ethers` (planned) - Ethereum library
- `secp256k1` (planned) - Cryptographic operations
- `serde` (planned) - Serialization
- `hex` (planned) - Hex encoding/decoding

## Integration with STK2ETH

### USSD Flow Integration
1. **User Request** - USSD client receives transaction request
2. **SpacetimeDB** - ussdgeth module processes business logic
3. **Ethereum Execution** - ethclient executes blockchain transaction
4. **Confirmation** - Transaction hash returned to USSD client

### Account Abstraction
- **Smart Wallet** - Deploy and manage AA wallets
- **Meta-transactions** - Gasless transactions for users
- **Batch Operations** - Multiple operations in single transaction
- **Social Recovery** - Wallet recovery mechanisms

## Security Considerations

- **Private Key Management** - Secure storage and handling
- **Transaction Validation** - Verify all transaction parameters
- **Gas Limit Protection** - Prevent gas limit attacks
- **Nonce Management** - Handle transaction ordering
- **Network Validation** - Verify correct network connection

## Testing Strategy

### Unit Tests
- Key generation and validation
- Transaction building and encoding
- Contract ABI encoding/decoding
- Gas estimation algorithms

### Integration Tests
- End-to-end transaction flow
- Smart contract deployment and interaction
- Multi-signature wallet operations
- Network failure handling

### Load Tests
- High-frequency transaction processing
- Concurrent wallet operations
- Memory usage under load
- Connection pool management

## Related Components

- **ussdgeth** - SpacetimeDB module that calls ethclient functions
- **ussdclient** - HTTP bridge that may trigger ethclient operations
- **contracts** - Smart contracts that ethclient will deploy and interact with