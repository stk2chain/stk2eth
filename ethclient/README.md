# EthClient

A modern Python Ethereum client with WebSocket support, designed for high-performance interaction with Ethereum networks. Built with a focus on security, efficiency, and developer experience.


## Run Client
```bash
# Set up environment variables
cp .env.example .env
# Edit .env with your configuration
source .env

python ethclient_v2.py
```
## Architecture
### 0.0 Handlers
Functions subscribed to `Database Tables` via Websocket triggered on Table row `insert`/`update`/`delete` events.

| Handler | Subscription | Table | Description |
|-----------|---------|------------|--------|                                       
| **handle_esim** | `on_insert` | **`esim_profile`** | Burner-Wallet Creation |
| **handle_tx** | `on_insert`,`status=Pending` | **`eth_tx`** | Sign and Broadcast Transaction |

### 0.1 Burner-Wallet Creation

```python
def handle_esim(insert, stdb):
    data = parse_insert(insert)
    phone_number, wallet_address = data[0], data[1]
    
    if wallet_address == "":
        wallet, signed_auth, phone_salt = create_phone_burner_wallet(phone_number, 0)     
        try:
            stdb.call_reducer("map_phone_to_wallet", phone_number, wallet)
            ...
```
* Triggered **ONLY** on **NEW `esim_profile` row insert** *(User Registration)*.
* **`Burner-Wallet`**: [Nick's Method+Phone](https://medium.com/patronum-labs/nicks-method-ethereum-keyless-execution-168a6659479c) derived *(Keyless)*, [EIP-7702 SignedAuthorization](https://eips.ethereum.org/EIPS/eip-7702) delegation to a [Permit2WithOperatorOnlyERC1271](contracts/README.md) Smart Contract *(Burner Wallet)*.
* Updates `esim_profile` row with `wallet_address` on success.
### 0.2 Transaction Execution

```python
from scripts.core.wallet import create_phone_permit2_authorization
from ape import project

# Create authorization
wallet, signed_auth, _ = create_phone_permit2_authorization(
    phone_number="+254712345678",
    chain_id=1,
    nonce=0
)

permit2 = project.Permit2WithOperatorOnlyERC1271.at(wallet)
permit_hash = permit_transfer_from(
    token_address,
    amount,
    operator.address,
    operator.nonce,
    2**256 - 1,
    DOMAIN_SEPARATOR
)
signature = operator.sign_raw_msghash(permit_hash)

# Execute permit transfer
tx = permit2.permitTransferFrom(
    (
        (token_address, amount),
        operator.nonce,
        2**256 - 1,
    ),
    (recipient_address, amount),
    wallet,
    signature.encode_rsv(),
    authorization=[signed_auth],
    sender=operator
)

```
*  Triggered **ONLY** on `eth_tx` row insert where `eth_tx.status=Pending` *(Transaction Execution)*.
* Uses [Ape Framework](https://github.com/ApeWorX/ape) to sign and broadcast transactions.

## Configuration

Create a `.env` file with the following variables:

```env
# Web3 Provider (WebSocket recommended for production)
WEB3_PROVIDER_URI=wss://mainnet.infura.io/ws/v3/YOUR-PROJECT-ID

# Contract Addresses
PERMIT2_7702_ADDRESS=0x000000000022D473030F116dDEE9F6B43aC78BA3
ESIM_REGISTRY_ADDRESS=0x...

# Network Configuration
CHAIN_ID=1  # Mainnet
```


### Wallet Management

Create and manage wallets with phone-based authentication:

```python
from scripts.core.wallet import create_phone_permit2_authorization

# Create a new wallet with phone authentication
phone_number = "+254712345678"
chain_id = 1  # Mainnet
nonce = 0

# Create wallet and get authorization
wallet_address, signed_auth, phone_salt = create_phone_permit2_authorization(
    phone_number=phone_number,
    chain_id=chain_id,
    nonce=nonce
)
print(f"Created wallet: {wallet_address}")
```

### Permit2 Transactions

Execute gasless transactions using Permit2:

```python
from scripts.core.wallet import create_phone_permit2_authorization
from brownie import interface

# Initialize Permit2 contract
permit2 = interface.IPermit2(os.getenv('PERMIT2_7702_ADDRESS'))

# Create authorization
wallet, signed_auth, _ = create_phone_permit2_authorization(
    phone_number="+254712345678",
    chain_id=1,
    nonce=0
)

# Execute permit transfer
permit2.permitTransferFrom(
    signed_auth,
    {
        'to': recipient_address,
        'amount': amount,
        'token': token_address
    },
    wallet,
    {
        'from': wallet
    }
)
```

## Testing

Run the test suite:

```bash
# Install test dependencies
pip install -r requirements-test.txt

# Run tests
ape test tests/ -v DEBUG

# Run specific test
ape test tests/test_permit2_with_operator.py -v DEBUG
```
