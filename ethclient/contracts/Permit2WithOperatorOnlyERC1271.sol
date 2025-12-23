// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

import "permit2/src/SignatureTransfer.sol";
import {SignatureVerification} from "permit2/src/libraries/SignatureVerification.sol";

/**
 * @title Permit2WithOperatorOnlyERC1271
 * @notice Minimal extension that only adds operator ERC-1271 support
 */
contract Permit2WithOperatorOnlyERC1271 is SignatureTransfer {
    address public immutable OPERATOR;
    bytes4 private constant MAGIC_VALUE = 0x1626ba7e;
    using SignatureVerification for bytes;

    
    constructor(address operator_) {
        require(operator_ != address(0), "Zero address operator");
        OPERATOR = operator_;
    }
    
    /**
     * @notice ERC-1271 implementation - operator only
     * @dev Uses parent verify function with operator as claimed signer
     */
    function isValidSignature(bytes32 hash, bytes calldata signature) 
        external 
        view 
        returns (bytes4) 
    {
        // Try to verify with operator as claimed signer
        signature.verify(hash, OPERATOR);
        return MAGIC_VALUE;
        
    }
}