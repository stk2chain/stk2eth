import os
import logging
import pytest
from eth_utils import keccak
from ape import accounts, project, Contract
from ape_accounts import import_account_from_private_key
from scripts.core.wallet import create_phone_permit2_authorization
from scripts.core.hashing import permit_transfer_from

# ============================================================================
# CONSTANTS
# ============================================================================

# WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
WETH = "0xf531B8F309Be94191af87605CfBf600D71C2cFe0" # WETH Sepolia
WALLET_ALIAS = os.getenv("WALLET_ALIAS","operator")
WALLET_PASSPHRASE = os.getenv("WALLET_PASSPHRASE","myp455phr4s3")
WALLET_PRIVATE_KEY = os.getenv("WALLET_PRIVATE_KEY")
P2_ERC1271 = os.getenv("P2_ERC1271")

# operator = import_account_from_private_key(WALLET_ALIAS, WALLET_PASSPHRASE, WALLET_PRIVATE_KEY)
operator = accounts.load(WALLET_ALIAS)
operator.set_autosign(True)

logging.info(f"WALLET_ALIAS: {WALLET_ALIAS}")
logging.info(f"WALLET_PASSPHRASE: {WALLET_PASSPHRASE}")
logging.info(f"WALLET_PRIVATE_KEY: {WALLET_PRIVATE_KEY}")
logging.info(f"P2_ERC1271: {P2_ERC1271}")
# logging.info(f"Operator: {operator.address}")
# ============================================================================
# PURE
# ============================================================================

sign_msg = lambda op, msg: op.sign_raw_msghash(msg).encode_rsv()
magic_valid = lambda v: v.hex() == '1626ba7e'
mk_permit = lambda token, amt, spender, nonce, deadline, domain: permit_transfer_from(token, amt, spender, nonce, deadline, domain)

# ============================================================================
# FIXTURES
# ============================================================================

@pytest.fixture(scope="module")
def weth():
    return Contract(WETH)

@pytest.fixture
def op(accounts):
    # return import_account_from_private_key(WALLET_ALIAS, WALLET_PASSPHRASE, WALLET_PRIVATE_KEY)
    return operator
    # return ape.accounts.load(WALLET_ALIAS)

@pytest.fixture
def rcpt(accounts):
    return accounts[1]

@pytest.fixture
def p2(op, project):
    # return project.Permit2WithOperatorOnlyERC1271.deploy(op.address, sender=op)
    return Contract(P2_ERC1271)

@pytest.fixture(autouse=True)
def setup(op, weth, p2, rcpt):
    if weth.balanceOf(op.address) < int(10e18):
        weth.deposit(value=int(21e18), sender=rcpt)
        weth.transfer(p2.address, int(10e18), sender=rcpt)
        weth.transfer(op.address, int(10e18), sender=rcpt)
# ============================================================================
# TESTS
# ============================================================================

class TestPermit2:
    def test_operator(self, op, p2):
        assert p2.OPERATOR() == op.address

    def test_erc1271(self, op, p2):
        msg = keccak(text="Test")
        sig = sign_msg(op, msg)
        magic = p2.isValidSignature(msg, sig, sender=op)
        assert magic_valid(magic)

    def test_permit_transfer(self, op, rcpt, weth, p2):
        amt = int(1e18)
        nonce = 3
        deadline = 2**256 - 1

        # Hash → Sign → Execute
        h = mk_permit(weth.address, amt, op.address, nonce, deadline, p2.DOMAIN_SEPARATOR())
        sig = sign_msg(op, h)
        tx = p2.permitTransferFrom(
            ((weth.address, amt), nonce, deadline),
            (rcpt.address, amt),
            p2.address,
            sig,
            sender=op
        )

        # Verify
        assert tx.status == 1
        assert weth.balanceOf(rcpt.address) == amt
        assert weth.balanceOf(p2.address) == int(10e18) - amt
