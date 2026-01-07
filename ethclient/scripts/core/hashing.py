from eth_abi import encode as abi_encode
from eth_utils import keccak

# Type hashes (constants)
DOMAIN_TYPEHASH = keccak(text="EIP712Domain(string name,uint256 chainId,address verifyingContract)")
TOKEN_PERMISSIONS_TYPEHASH = keccak(text="TokenPermissions(address token,uint256 amount)")
PERMIT_TRANSFER_FROM_TYPEHASH = keccak(text="PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)")

# Core functions
hash_bytes = lambda data: keccak(abi_encode(*data))
to_hex = lambda h: '0x' + h.hex()
lower = lambda addr: addr.lower()

domain_separator = lambda chain_id, permit2: hash_bytes((
    ['bytes32', 'bytes32', 'uint256', 'address'],
    [DOMAIN_TYPEHASH, keccak(text="Permit2"), chain_id, lower(permit2)]
))

token_permissions_hash = lambda token, amount: hash_bytes((
    ['bytes32', 'address', 'uint256'],
    [TOKEN_PERMISSIONS_TYPEHASH, lower(token), amount]
))

hash_struct = lambda token, amount, spender, nonce, deadline: hash_bytes((
    ['bytes32', 'bytes32', 'address', 'uint256', 'uint256'],
    [PERMIT_TRANSFER_FROM_TYPEHASH, token_permissions_hash(token, amount), lower(spender), nonce, deadline]
))

eip712_hash = lambda domain_sep, struct: keccak(b'\x19\x01' + domain_sep + struct)

# Main function
def permit_transfer_from(token, amount, spender, nonce, deadline, domain_sep):
    """Generate EIP-712 hash for Permit2 permitTransferFrom."""
    return eip712_hash(
        domain_sep,
        hash_struct(token, amount, spender, nonce, deadline)
    )
