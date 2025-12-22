# Smart Contracts

This directory contains Solidity smart contracts that implement core functionality for the stk2eth system, including ERC-1271 signature verification and eSIM profile management.

## Contracts

### 1. Permit2WithOperatorOnlyERC1271

A specialized wallet contract that combines Permit2 functionality with ERC-1271 signature verification, designed to work with 7702 authorization delegation. This contract acts as a secure, non-custodial burner wallet that can be delegated to by an EOA (Externally Owned Account) using Nick's method for signature generation.

**Core Purpose:**
- Serves as a delegated wallet that can be controlled by a designated operator
- Implements ERC-1271 to validate signatures from the operator
- Inherits Permit2's `permitTransferFrom` functionality for gasless token transfers
- Functions as a one-time use burner wallet when combined with Nick's method

**Key Features:**
- **Delegated Control**: Only the designated operator can initiate transfers
- **Single-Use Design**: When used with Nick's method, each wallet is effectively a burner
- **Gasless Transactions**: Inherits Permit2's meta-transaction capabilities
- **Secure Delegation**: Uses 7702 authorization for secure EOA delegation

**Authorization Flow:**
1. EOA generates a one-time authorization using Nick's method (as seen in `wallet.py`)
2. The operator deploys `Permit2WithOperatorOnlyERC1271` with themselves as the operator
3. The contract acts as if it were the EOA for token transfers
4. `isValidSignature` verifies the operator's signature
5. `permitTransferFrom` executes transfers on behalf of the EOA

**Usage:**
```solidity
// Deploy with an operator address
Permit2WithOperatorOnlyERC1271 wallet = new Permit2WithOperatorOnlyERC1271(operatorAddress);

// Operator can now initiate transfers that will be signed by the contract
// The contract will verify the operator's signature via ERC-1271
// and execute the transfer using Permit2's permitTransferFrom
```

### 2. EsimRegistry

A registry contract for managing eSIM profile to wallet address bindings with EIP-712 typed data support.

**Key Features:**
- Maps eSIM profiles to wallet addresses and vice versa
- Implements EIP-712 for secure off-chain message signing
- Emits events for all state changes
- Supports profile registration, updates, and deregistration

**Events:**
- `Register`: Emitted when a new profile is registered
- `Update`: Emitted when a profile's wallet binding is updated
- `Deregister`: Emitted when a profile is removed

**Usage:**
```solidity
// Register a new profile
registry.register(esimProfile, walletAddress);

// Update a profile's wallet
registry.update(esimProfile, newWalletAddress);

// Deregister a profile
registry.deregister(esimProfile);
```

## Dependencies

- Solidity ^0.8.13
- Permit2 (imported as a dependency)

## Security Considerations

- Both contracts implement access control mechanisms
- `Permit2WithOperatorOnlyERC1271` restricts signing to a single operator
- `EsimRegistry` includes checks to prevent unauthorized access to profile management

## Testing

Run the test suite with:
```bash
pytest tests/
```

## License

MIT
