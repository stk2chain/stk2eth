# Middleware

SpacetimeDB WASM module - USSD session logic + Swap processing + FATF audit

>**USSD (Unstructured Supplementary Service Data)** - a communications protocol used by GSM mobile phones to interact with a service provider's computers in real-time.

## Build & Deploy

```bash
spacetime publish -c --server local --project-path middleware gateway2
```
## Architecture
### 0.0 USSD Session

```rust

{
  sessionId: String,       // unique per session
  phoneNumber: String,     // user mobile number E.164 format
  networkCode: String,     // telco network
  serviceCode: String,     // USSD code for your app
  text: String             // user input, concatenated with "*" for multi-step
}
```

* `text` contains **opCode** and **parameters**, e.g., *"1\*0.5\*0xRecipient"*.

### 0.1 Session Data (`"text"`)

Each USSD **opCode** maps to a **primitive** (basis function):

| opCode | Primitive  | Parameters                                     | Flow |
| ------ | ---------- | ---------------------------------------------- |-------|
| **`1`**  | **Register**   | `pin`, `confirm_pin`                              | **RegisterScreen** |
| **`1`**  | **SendETH**   | `phone_number`, `amount`, `pin`                              | **MainScreen** |
| **`2`**  | **Swap**       | `token_out`, `phone_number`, `amount_in`, `pin` | **MainScreen** |
| **`3`**  | **Withdraw**   | `token`, `amount`, `pin`                                  | **MainScreen** |
| **`4`**  | **Balance** | `token`, `pin`                                          | **MainScreen** |

**`SendETH` is treated as SendWETH** → always a **[WETH](https://etherscan.io/address/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2) contract call**, no direct ETH transfer.

### 0.2 User Registration
* `is_registered[phone_number]` determines **screen flow**:
  * `false` → *RegisterScreen* - registration flow (PIN entry & confirmation)
  * `true` → *MainScreen* - main transaction menu

* Registration is **mandatory** before executing **`MainScreen`** primitives.

### 0.3 Transaction Encoding
*  Each primitive converts to an **Ethereum transaction** (`EthTx`):
```rust
{
  session_id: String,
  to: String, // permit2 / Uniswap / Token
  value: String,
  data: String, // ABI-encoded function call (transfer, swapExactTokensForTokens, etc.)
  gas_limit: String,
  status: EthTxStatus,    
  created_at: Timestamp,
  updated_at: Timestamp,
}
```
* **Shannon perspective**: minimal bits encode only necessary parameters.
* **Fourier perspective**: each primitive is a deterministic basis function
<!-- TODO: Account -->
### 0.4 Flow

1. Parse USSD input → `opCode + parameters`
2. Determine screen via `is_registered[msg.sender]`
3. Map to primitive → parameters
4. Encode primitive → EthTx (`to`, `value`, `data`)
5. Sign & broadcast → deterministic on-chain execution


## Tables

### `ussd_session`
```rust
session_id: String,          // Primary key
current_screen: String,      // Menu state
response_text: String,       // CON ... or END ...
```

### `pin`
```rust
phone_number: String,
pin_hash: String,
salt: String,
```

### `phone_wallet`
```rust
phone_number: String,
wallet_address: String,
```

### `eth_tx`
```rust
session_id: String,
to: String,          
value: String,       
data: String,          
gas_limit: String,
status: EthTxStatus,          // Pending/Processing/Completed/Failed/Canceled
tx_hash: Option<String>
```
### `swap`
```rust
session_id: String,
token_in: String,
token_out: String,
amount_in: String,
amount_out: String,
recipient: String,
```

### `eth_audit_logs`
```rust
tx_hash: String,
from_address: String,
to_address: String,
amount: String,
data: String,
timestamp: Timestamp,
```

## Endpoints *(Reducers)*

#### `process_ussd_step(session_id, phone, network, service, text)`
#### `execute_ussd(session_id, phone, network, service, text)`
#### `map_phone_to_wallet(phone, wallet)`

## Session Data Format


```rust
"opCode*parameter1*parameter2*parameter3"

Register : "1*pin*confirm_pin"
SendETH  : "1*phone_number*amount*pin"
Swap     : "2*token_out*amount_in*recipient"
Withdraw : "3*token*amount*pin"
Balance  : "4*token*pin"
```

## Query

```bash
# Get session
spacetime sql gateway2 "SELECT * FROM ussd_session WHERE session_id = 'AT123'"

# Get transaction
spacetime sql gateway2 "SELECT * FROM eth_tx WHERE session_id = 'AT123'"

# Get wallet
spacetime sql gateway2 "SELECT * FROM phone_wallet WHERE phone_number = '+254712345678'"
```
## Test

```bash
cargo test                    # Unit tests
cargo test --test integration # Integration
../stress_test.sh             # 1000+ TPS audit test
```


