# Smart Contracts

This directory contains Solidity smart contracts that implement core functionality for the stk2eth system, including ERC-1271 signature verification and eSIM profile management.

## Contracts

### 1. Permit2WithOperatorOnlyERC1271

A minimal extension of the Permit2 contract that adds ERC-1271 signature verification support for a single operator.

**Key Features:**
- Implements ERC-1271 `isValidSignature` function
- Restricts signature verification to a single immutable operator address
- Inherits from Permit2 for token approval functionality

**Usage:**
```solidity
// Deploy with an operator address
Permit2WithOperatorOnlyERC1271 permit2 = new Permit2WithOperatorOnlyERC1271(operatorAddress);

// Verify signatures (only the operator can sign)
bytes4 magicValue = permit2.isValidSignature(hash, signature);
require(magicValue == 0x1626ba7e, "Invalid signature");
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
