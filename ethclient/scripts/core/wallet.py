import os

from eth_utils import to_bytes
from scripts.core.auth_7702 import _hash_auth7702Message, _auth_sig, SignedAuthorization

from scripts.core.utils import phone_to_salt

from typing import Optional, Tuple
import hashlib

PERMIT2_ADDRESS = os.getenv("PERMIT2_7702_ADDRESS", "0x000000000022D473030F116dDEE9F6B43aC78BA3")


def _nick_auth_7702(r: int, s: int, v: int, msg_hash: bytes) -> Tuple[str, int]:
    print(f"Searching for valid signature with phone-derived salt...")
    attempts = 0
    while True:
        try:
            attempts += 1
            auth_sig = _auth_sig(r, s, v)
            pubk = auth_sig.recover_public_key_from_msg(msg_hash)
            authority_address = pubk.to_checksum_address()
            break
        except Exception:
            r += 1
            if attempts % 1000 == 0:
                print(f"  ... {attempts} attempts")
    
    print(f"✓ Found valid signature after {attempts} attempts")
    
    
    return (authority_address, r)

def create_phone_permit2_authorization(phone_number: str, chain_id: int, nonce: int , user_salt: Optional[str] = None, delegate_to: str = PERMIT2_ADDRESS) -> Tuple[str, SignedAuthorization, bytes]:

    phone_salt = phone_to_salt(phone_number, user_salt)
    
    msg_hash = _hash_auth7702Message(chain_id, delegate_to, nonce)
    
    r = int.from_bytes(msg_hash, byteorder='big')
    s = int.from_bytes(phone_salt, byteorder='big')
    v = 27
    
    (authority_address, r) = _nick_auth_7702(r, s, v, msg_hash)
    
    signed_auth = SignedAuthorization(
        chain_id=chain_id,
        address=to_bytes(hexstr=delegate_to),
        nonce=nonce,
        v=v,
        r=r,
        s=s
    )   

    return (authority_address, signed_auth, phone_salt)


def create_phone_burner_wallet(phone_number: str, nonce: int):
    chain_id = 0 # Chain Agnostic
    permit2_wallet = create_phone_permit2_authorization(phone_number, chain_id, nonce)
    return permit2_wallet