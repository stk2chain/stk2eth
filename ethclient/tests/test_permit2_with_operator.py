import os
import pytest
from eth_utils import to_checksum_address, encode_hex, to_hex
from eth_account import Account
from eth_account.messages import encode_defunct
from eth_abi import encode as abi_encode
from eth_abi.packed import encode_packed
from eth_utils import keccak, to_bytes
from ape import accounts, project, networks, Contract


from scripts.core.wallet import create_phone_burner_wallet, create_phone_permit2_authorization
from scripts.core.hashing import permit_transfer_from


# Constants
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"


@pytest.fixture(scope="module")
def weth():
    """ WETH contract """
    return Contract(WETH_ADDRESS)


@pytest.fixture
def operator(accounts):
    return accounts[0]

@pytest.fixture
def recipient(accounts):
    return accounts[1]

@pytest.fixture
def permit2_with_operator(operator, project):
    """ Deploy Permit2WithOperatorOnlyERC1271 """
    return project.Permit2WithOperatorOnlyERC1271.deploy(operator.address, sender=operator)

@pytest.fixture(autouse=True)
def setup(operator, weth, permit2_with_operator):
    """ Test setup """
    if weth.balanceOf(operator.address) < int(10e18):
        weth.deposit(value=int(11e18), sender=operator)
        print(permit2_with_operator.address)
        weth.transfer(permit2_with_operator.address, int(10e18), sender=operator)

class TestPermit2WithOperator:
    def test_permit2_with_operator_erc1271_operator(self, operator, permit2_with_operator):
        """Test Permit2WithOperatorOnlyERC1271 operator"""
        
        assert permit2_with_operator.OPERATOR() == operator.address

    def test_permit2_with_operator_erc1271(self, operator, recipient, weth, permit2_with_operator):
        """Test Permit2WithOperatorOnlyERC1271 ERC-1271 signature verification"""
        # 1. Get wallet instance
        wallet = permit2_with_operator


        # CORRECT:
        message_hash = keccak(text="Test Message")
        signature = operator.sign_raw_msghash(message_hash)
        magic_value = permit2_with_operator.isValidSignature(message_hash, signature.encode_rsv(), sender=operator)
        # 2. Create a test message and sign it with the operator
        # message = keccak(text="Test Message")#encode_defunct(text="Test Message")
        # signature = operator.sign_message(encode_defunct(hexstr=message.hex()))
        print(f"message: {message_hash}")
        print(f"Signature: {signature.encode_rsv()}")
        # sig = signature.r+signature.s+signature.v.to_bytes(1, 'big')
        # print(f"len(sig): {len(sig)}")
        # 3. Verify signature through ERC-1271
        # magic_value = permit2_with_operator.isValidSignature(message, signature.encode_rsv(), sender=recipient)
        print(f"magic_value: {magic_value}")
        assert magic_value.hex() == '0x1626ba7e'  # Magic value for valid signature

    def test_permit_transfer_from(self, operator, recipient, weth, permit2_with_operator):
        """Test token transfer using permitTransferFrom"""

        print(f"Accounts: {accounts}")
        print(f"Operator: {operator}")
        # 1. Delegate wallet using phone-based authorization
        wallet_address, signed_auth, phone_salt = create_phone_permit2_authorization(
            "+254712345678",  # phone number
            1,  # chain_id (mainnet)
            0,  # nonce
            None,  # user_salt
            permit2_with_operator.address  # delegate_to
        )
        
        # 2. Create permit data
        amount = int(1e18)  # 1 ETH
        
        p2wop_domain_sep = permit2_with_operator.DOMAIN_SEPARATOR()

        print(f"Domain Separator: {p2wop_domain_sep}")

        # 3. Generate permit hash
        permit_hash = permit_transfer_from(
            weth.address,
            amount,
            operator.address,
            3,
            2**256 - 1,
            p2wop_domain_sep
        )
        
        print(f"Permit Hash: {permit_hash}")
        # 4. Sign the permit with the operator's private key
        signature = operator.sign_raw_msghash(permit_hash)
        
        print(f"Signature: {signature.encode_rsv()}")
        # 5. Prepare transfer details
        # sig = signature.r+signature.s+bytes([signature.v-27])
        
        # 6. Execute permitTransferFrom through the wallet contract
        tx = permit2_with_operator.permitTransferFrom(
            (
                (weth.address, amount),
                3,
                2**256 - 1,
            ),
            (recipient.address, amount),
            permit2_with_operator.address,
            signature.encode_rsv(),
            sender=operator
        )
        print(f"Transaction: {tx}")
        
        # 7. Verify the transfer
        assert tx.receipt.status == 1, "Transaction failed"
        assert weth.balanceOf(recipient.address) == amount, "Incorrect token transfer amount"
        assert weth.balanceOf(permit2_with_operator.address) == int(10e18) - amount, "Incorrect sender balance"
        # assert weth.balanceOf(not_operator) == 2e18