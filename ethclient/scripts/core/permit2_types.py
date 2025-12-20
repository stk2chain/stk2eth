from dataclasses import dataclass
from typing import List, Optional
from eth_typing import ChecksumAddress, HexStr
from eth_utils import to_checksum_address
from eth_abi import encode as abi_encode
from eth_abi.packed import encode_packed
from eth_utils import keccak

@dataclass
class TokenPermissions:
    token: ChecksumAddress
    amount: int

@dataclass
class PermitTransferFrom:
    permitted: TokenPermissions
    nonce: int
    deadline: int

def hash_permit_transfer(permit: 'PermitTransferFrom', domain_separator: bytes, operator: ChecksumAddress) -> bytes:
    """
    Hashes the PermitTransferFrom struct according to EIP-712
    """
    PERMIT_TRANSFER_FROM_TYPEHASH = keccak(
        text='PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)'
    )
    
    token_permissions_hash = keccak(
        abi_encode(
            ['bytes32', 'address', 'uint256'],
            [
                keccak(text='TokenPermissions(address token,uint256 amount)'),
                permit.permitted.token,
                permit.permitted.amount
            ]
        )
    )
    
    data_hash = keccak(
        abi_encode(
            ['bytes32', 'bytes32', 'address', 'uint256', 'uint256'],
            [
                PERMIT_TRANSFER_FROM_TYPEHASH,
                token_permissions_hash,
                operator,  # spender
                permit.nonce,
                permit.deadline
            ]
        )
    )
    
    return keccak(encode_packed(['bytes1', 'bytes1', 'bytes32', 'bytes32'], [b'\x19', b'\x01', domain_separator, data_hash]))
