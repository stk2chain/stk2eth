# Smart Contracts - Account Abstraction & eSIM Registry

## Overview

The `contracts` directory contains Solidity smart contracts for the STK2ETH project, built using the **Foundry** development framework. These contracts enable account abstraction functionality and eSIM-to-wallet binding for offline Ethereum transactions.

## Architecture

This is a **Foundry project** that provides:
- eSIM profile to Ethereum wallet registry
- Account abstraction smart wallet functionality
- EIP-712 signature verification for secure operations
- Gas-efficient contract interactions

## Smart Contracts

### EsimRegistry.sol
The core contract that manages the binding between eSIM profiles and Ethereum wallets.

**Key Features:**
- **Profile Registration** - Bind eSIM profiles to Ethereum wallets
- **Wallet Updates** - Change wallet associated with eSIM profile
- **Deregistration** - Remove eSIM profile bindings
- **EIP-712 Compliance** - Secure signature-based operations
- **Event Logging** - Complete audit trail of registry changes

**Main Functions:**
```solidity
function register(bytes32 profile, address wallet) external
function update(bytes32 profile, address newWallet) external
function deregister(bytes32 profile) external
function getWallet(bytes32 profile) external view returns (address)
function getProfile(address wallet) external view returns (bytes32)
```

### Counter.sol
Template contract for Foundry testing and deployment examples.

## Development with Foundry

### Prerequisites
```bash
# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Install dependencies
forge install
```

### Building
```bash
# Compile all contracts
forge build

# Build with optimizations
forge build --optimize
```

### Testing
```bash
# Run all tests
forge test

# Run with verbose output
forge test -vvv

# Test specific contract
forge test --match-contract EsimRegistryTest

# Generate gas reports
forge test --gas-report
```

### Deployment

#### Local Deployment (Anvil)
```bash
# Start local Ethereum node
anvil

# Deploy to local network
forge script script/DeployEsimRegistry.s.sol:DeployEsimRegistryScript \
  --rpc-url http://localhost:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
  --broadcast
```

#### Testnet Deployment
```bash
# Deploy to Sepolia testnet
forge script script/DeployEsimRegistry.s.sol:DeployEsimRegistryScript \
  --rpc-url https://sepolia.infura.io/v3/YOUR_INFURA_KEY \
  --private-key $PRIVATE_KEY \
  --verify \
  --etherscan-api-key $ETHERSCAN_API_KEY \
  --broadcast
```

### Verification
```bash
# Verify contract on Etherscan
forge verify-contract \
  --chain-id 11155111 \
  --num-of-optimizations 200 \
  --constructor-args $(cast abi-encode "constructor()") \
  CONTRACT_ADDRESS \
  src/EsimRegistry.sol:EsimRegistry \
  --etherscan-api-key $ETHERSCAN_API_KEY
```

## Usage Examples

### Register eSIM Profile
```solidity
// Generate profile hash from eSIM data
bytes32 profileHash = keccak256(abi.encodePacked(iccid, imsi));

// Register profile to wallet
registry.register(profileHash, walletAddress);
```

### Query Wallet by eSIM
```solidity
// Get wallet address for eSIM profile
address wallet = registry.getWallet(profileHash);
```

### Update Wallet Binding
```solidity
// Change wallet for existing eSIM profile
registry.update(profileHash, newWalletAddress);
```

## File Structure

```
contracts/
├── src/
│   ├── EsimRegistry.sol        # Main eSIM registry contract
│   └── Counter.sol             # Example/template contract
├── test/
│   ├── EsimRegistry.t.sol      # Registry contract tests
│   └── Counter.t.sol           # Example tests
├── script/
│   ├── DeployEsimRegistry.s.sol # Deployment scripts
│   └── Counter.s.sol           # Example deployment
├── lib/                        # Foundry dependencies
├── out/                        # Compiled contracts
├── foundry.toml               # Foundry configuration
└── README.md                  # This file
```

## Configuration

### foundry.toml
```toml
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
optimizer = true
optimizer_runs = 200
via_ir = false

[profile.ci]
fuzz = { runs = 10_000 }
invariant = { runs = 1_000 }
```

### Environment Variables
```env
# Deployment
PRIVATE_KEY=0x...
RPC_URL=https://sepolia.infura.io/v3/...
ETHERSCAN_API_KEY=...

# Testing
FORK_URL=https://mainnet.infura.io/v3/...
BLOCK_NUMBER=18000000
```

## Security Considerations

### EsimRegistry Security
- **Access Control** - Only profile owners can modify bindings
- **EIP-712 Signatures** - Prevent replay attacks across chains
- **Input Validation** - Validate all profile and wallet parameters
- **Event Logging** - Complete audit trail for compliance

### Deployment Security
- **Multi-sig Ownership** - Use multi-signature wallet for contract ownership
- **Upgrade Patterns** - Consider proxy patterns for upgradability
- **Verification** - Always verify contracts on Etherscan
- **Testing** - Comprehensive test coverage before mainnet deployment

## Gas Optimization

- **Packed Structs** - Minimize storage slots
- **Batch Operations** - Group multiple operations
- **Event Indexing** - Optimize event log filtering
- **View Functions** - Use for read-only operations

## Integration with STK2ETH

### USSD Flow Integration
1. **eSIM Detection** - Extract eSIM profile from device
2. **Registry Query** - Look up associated Ethereum wallet
3. **Transaction Authorization** - Use wallet for USSD transactions
4. **State Updates** - Update registry as needed

### Account Abstraction
- **Smart Wallets** - Deploy AA wallets for eSIM profiles
- **Meta-transactions** - Gasless operations via eSIM binding
- **Social Recovery** - Recover wallets using eSIM verification
- **Batch Operations** - Multiple transactions in single call

## Testing Strategy

### Unit Tests
- Profile registration and updates
- Access control mechanisms
- EIP-712 signature verification
- Edge cases and error conditions

### Integration Tests
- End-to-end eSIM registration flow
- Multi-contract interactions
- Gas usage optimization
- Event emission verification

### Fork Tests
- Test against mainnet state
- Verify compatibility with existing contracts
- Performance testing with real data

## Related Components

- **ethclient** - Deploys and interacts with these contracts
- **ussdgeth** - May trigger contract operations via ethclient
- **ussdclient** - Initiates flows that result in contract calls
