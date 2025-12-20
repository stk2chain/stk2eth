import rlp
from eth_utils import keccak, to_bytes
from rlp import encode as rlp_encode
from rlp.sedes import big_endian_int, binary


from eth_keys import keys


class Authorization(rlp.Serializable):
    """
    RLP structure for EIP-7702 Authorization
    """
    fields = [
        ('chain_id', big_endian_int),
        ('address', binary),
        ('nonce', big_endian_int),
    ]

class SignedAuthorization(rlp.Serializable):
    """
    RLP structure for signed EIP-7702 Authorization
    """
    fields = [
        ('chain_id', big_endian_int),
        ('address', binary),
        ('nonce', big_endian_int),
        ('v', big_endian_int),
        ('r', big_endian_int),
        ('s', big_endian_int),
    ]




def _auth_sig(r: int, s: int, v: int) -> keys.Signature:
    """
    Returns Signature from r, s, v
    """
    auth_sig_bytes = r.to_bytes(32, 'big') + s.to_bytes(32, 'big') + bytes([v - 27])

    return keys.Signature(signature_bytes=auth_sig_bytes)


def _hash_auth7702Message(chain_id: int, delegate_to: str, nonce: int, magic: str = "0x05") -> str:
    """
    MAGIC : 0x05
    msg = keccak(MAGIC || rlp([chain_id, address, nonce]))

    Returns 7702 Authorization List Message Hash
    """
    auth = Authorization(chain_id, to_bytes(hexstr=delegate_to), nonce)
    auth_encoded = rlp_encode(auth)
    
    return keccak(to_bytes(hexstr=magic)+auth_encoded)



'''
7702 pemrit2:
    full token ability
    ?require approve/permit for spender
    ?permitSingle spender?
        ?allowance[owner][token][msg.sender]
         -? allows spending actual token
    ?allowaance?
        -?x require second vrs sig on nick's

7702 hook?
    ?

permit.permit<Single/Batch>:
    vrs = permitSingle
    allowance[owner][token][spender]
    allowance only on permit2 not on token
    fails on token.call
    ?unless is pool hook? and uniswap pool call?
        ? Would still require pool_manager.setOperator

approve:
    allowance[msg.sender][token][spender]
    spends permit2 tokens


pool_manager.setOperator:
    currency.
        mint  
        transfer
        burn321 



_take : flash_loan
_settle: currency_reserves delta after take

_mint: delta like _take without flash_loan but with minted tokens
_burn: delter like _settle but require minted tokens


pool_manager.setOperator

_mint + _settle :: Deposit
_burn + _take  :: Withdraw

Permit2WithOperatorOnlyERC1271
'''