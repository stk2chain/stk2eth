import ape
import pytest

from scripts.core.wallet import create_phone_burner_wallet, create_phone_permit2_authorization



class TestUniswap():
    @pytest.fixture
    def operator(accounts):
        return accounts[0]

    @pytest.fixture
    def not_operator(accounts):
        return accounts[1]

        @pytest.fixture(scope="session")
    def weth(Contract):
        """ WETH """
        yield Contract("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
    
    @pytest.fixture(scope="session")
    def uniswap_router_v4(Contract):
        """ Uniswap Router V4 """
        yield Contract("0x66a9893cC07D91D95644AEDD05D03f95e1dBA8Af")


    @pytest.fixture(scope="session")
    def uniswap_pool_manager_v4(Contract):
        """ Uniswap Pool Manager V4 """
        yield Contract("0x000000000004444c5dc75cB358380D2e3dE08A90")    
    
    @pytest.fixture(scope="session")
    def permit2_with_operator(operator, project):
        """ Deploy Permit2WithOperatorOnlyERC1271 """
        return operator.deploy(project.Permit2WithOperatorOnlyERC1271, operator)

    @pytest.fixture(scope="session")
    def esim_registry(operator, project):
        """ Deploy Esim Registry """
        return operator.deploy(project.EsimRegistry)
    

    @pytest.fixture(scope="module", autouse=True)
    def shared_setup(operator, weth):
        #amnt = 11579208923731619542357098500868790785326998466564056403945758400791312963993
        # accounts.add(private_key=wallet_private_key) # Create LocalAccount capable of signing messages
        weth.deposit({"from": operator, "value": "10 ether"})   # Deposit 10 WETH for accounts[0]

        # weth.transfer(accounts[0], 5e18, {"from" : accounts[0]}) # Transfer 5 WETH to LocalAccount



    @pytest.mark.require_network("mainnet-fork")
    def test_permit2_with_operator(accounts, web3, uniswap_router_v4, weth, uniswap_pool_manager_v4, permit2_with_operator, operator):
        """ Tests Swap Token Transfer on Uniswap Router V4 """
        #Gernerate permit2 wallet
        # burner_wallet = create_phone_burner_wallet("+254712345678", 0)

        #1. Deploy permit2

        #2. delegate wallet
        permit2_7702_wallet, signed_auth, phone_salt = create_phone_permit2_authorization("+254712345678", 0,0,None, permit2_with_operator.address)
        
        #3. Compute Permit hash

        #4. Call permit2_7702_wallet.permitTransferFrom via a 7702 transaction
        
        permit2_7702_wallet_contract = project.Permit2WithOperatorOnlyERC1271.at(permit2_7702_wallet)
        permit2_7702_wallet_contract.permitTransferFrom(

            sender=operator    
        )

        # assert weth.balanceOf(permit2_7702_wallet) == 3e18
        # assert weth.balanceOf(not_operator) == 2e18