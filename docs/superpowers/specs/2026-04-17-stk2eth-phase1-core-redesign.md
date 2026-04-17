# STK2ETH — First-Principles Core Redesign

**Date:** 2026-04-17
**Status:** Design approved, pending implementation plan
**Target:** Base Sepolia testnet demo, end-to-end all screens

---

## Goal

Three ordered outcomes:

1. **Phase 1 — Working E2E demo on Base Sepolia.** All MainScreen options function end-to-end except Swap (which becomes "Coming Soon"). Users can register, send ETH, check balance, withdraw to M-Pesa via Pretium, and access the Account menu.
2. **Phase 2 — Crypto and compliance hardening.** Swap the env-var hot key for a pluggable signer interface, wire FATF/Travel Rule fields, add PIN recovery and rate limiting, audit the Nick's Method derivation.
3. **Phase 3 — Architecture refinement.** Separate deployable services, multi-instance broadcaster, observability, operator dashboard, DR runbook.

This spec defines Phase 1 completely and describes Phase 2/3 at the level of goals plus candidate tasks (not full designs).

---

## Trust model (confirmed)

- **Burner wallet** — phone-derived EOA produced via Nick's Method. No private key exists for this address; the signature used during derivation is a synthetic one crafted to satisfy `ecrecover`.
- **EIP-7702 authorization** — pre-signed delegation from the burner wallet to the `Burner7702` smart contract. Stored in `auth_7702` table with `status=Pending`. Broadcast lazily, attached as `authorization_list` on the user's first outbound transaction.
- **`Burner7702` contract** — `execute(target, value, data)` and `executeBatch(...)`, gated by `onlyOperator`. Immutable `OPERATOR` set at construction.
- **Gateway key = OPERATOR** — the only address that can make a burner wallet do anything. Holds `GATEWAY_PRIVATE_KEY` (Phase 1: env var) and pays all gas.
- **User PIN over USSD** — how the user authorizes the gateway to sign on their behalf. PIN hash is `Keccak256(phone || pin || domain_separator || salt)`, verified constant-time.
- **`Permit2WithOperatorOnlyERC1271`** — separate contract for ERC-20 approvals via ERC-1271 smart-contract signatures. Used in future flows; not required for Phase 1 Send ETH / Withdraw.

---

## Architecture

Four components, three trust domains, one shared state layer:

```
 ┌──────────────────┐      HTTP POST /ussdeth          ┌──────────────────────┐
 │ Africa's Talking │ ───────────────────────────────▶ │  ussdclient (Python) │
 │   USSD Gateway   │                                   │  Flask HTTP bridge   │
 └──────────────────┘                                   └──────────┬───────────┘
                                                                   │ SpacetimeDB WS
                                                                   ▼
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │                   middleware (Rust / SpacetimeDB module)                    │
 │   Pure state + screen FSM. No RPC, no hot keys, no external I/O.            │
 │   Tables: ussd_session, ussd_response, user_pin, esim_profile,              │
 │           auth_7702, eth_tx, withdrawal_request, balance_query,             │
 │           sms_notification, user_preferences, eth_audit_logs                │
 └──────────┬──────────────────────────────────────────────────┬───────────────┘
            │ SpacetimeDB subscription                          │ subscription
            ▼                                                   ▼
 ┌────────────────────────────────────┐      ┌─────────────────────────────┐
 │     broadcaster (Python svc)       │      │     smsclient (Python svc)  │
 │  auth7702 │ ethtx │ pretium │      │      │     Termii API worker       │
 │  balance  workers                  │      │                             │
 │  Holds: GATEWAY_PRIVATE_KEY,       │      │  Holds: TERMII_API_KEY,     │
 │         PRETIUM_API_KEY            │      │         TERMII_SENDER_ID    │
 └──────────────┬─────────────────────┘      └──────────────┬──────────────┘
                │                                            │
                ▼                                            ▼
   Base Sepolia RPC + Pretium API               Termii SMS API
```

**Component responsibilities:**

| Component | Holds | External I/O | Purpose |
|-----------|-------|--------------|---------|
| `ussdclient` | — | HTTP in, SpacetimeDB out | USSD protocol bridge. No business logic. |
| `middleware` | SpacetimeDB tables | — | State + screen FSM. Pure functions, DB writes only. |
| `broadcaster` | Gateway signing key, Pretium API key | Base Sepolia RPC, Pretium API | Signs + submits on-chain txs, off-ramp fiat, balance queries. |
| `smsclient` | Termii API key | Termii SMS API | Sends confirmation/alert SMS. |

**Invariant:** every cross-component interaction goes through SpacetimeDB tables. No direct process-to-process calls. Restarting any service never requires coordinating with the others.

---

## Data flows (Phase 1)

### Flow A — Registration

1. User dials, no `esim_profile` for phone → middleware shows `RegisterScreen`
2. User enters PIN twice → middleware runs `confirm_register_pin`:
   - Validates PIN format (4-6 digits, not weak)
   - Hashes PIN with Keccak256 + domain separator + timestamp salt
   - Calls `create_phone_permit2_authorization(phone, chain_id=BASE_SEPOLIA)` → derives burner wallet address + signed 7702 authorization
   - Inserts `esim_profile`, `auth_7702(status=Pending)`, `user_pin`
   - Inserts `sms_notification(template=RegSuccess, wallet_address)`
3. User sees "Registration Complete!" (END)
4. `smsclient` drains the SMS row and sends Termii welcome SMS
5. `broadcaster.auth7702_worker` does NOT broadcast yet — waits for first outbound tx (lazy)

### Flow B — Send ETH

1. User picks "1. Send ETH", enters receiver phone, amount, PIN
2. Middleware `validate_pin`:
   - Parses input by screen (not by raw `*` split)
   - Verifies PIN hash constant-time
   - Auto-registers receiver if needed (creates `esim_profile` + `auth_7702` for their burner wallet)
   - Inserts `eth_tx(session_id, tx_type=SendEth, from=sender_burner, to=receiver_burner, value, data=0x, status=Pending)`
   - Sets `session.response_text` to the confirm screen
3. User sees `Confirm TX: ... 1.Confirm 2.Cancel`
4. User picks 1 → `cancel_tx`:
   - Updates `eth_tx.status = Submitted`
   - Inserts `sms_notification(template=TxSubmitted)`
5. `broadcaster.ethtx_worker` sees `eth_tx.status=Submitted`:
   - Checks if sender burner wallet already has 7702 code on-chain
   - If NOT: attaches `authorization_list=[auth_7702 row]`, sets tx `type=4`
   - Builds `Burner7702.execute(to=receiver_wallet, value=amount, data=0x)` calldata
   - Signs with gateway key, submits
   - On confirm: updates `eth_tx.status=Confirmed`, `auth_7702.status=Active` (if first use), inserts `eth_audit_logs`, inserts two SMS rows (sender `TxConfirmed`, receiver `InboundEth`)

### Flow C — Withdraw (Pretium off-ramp)

1. User picks "3. Withdraw", enters fiat amount (e.g., 500 KES), PIN
2. Middleware `validate_pin`:
   - Verifies PIN
   - Inserts `eth_tx(session_id, tx_type=WithdrawEscrow, from=user_burner, to=gateway_escrow, value=eth_equivalent, status=Pending)`
   - Inserts `withdrawal_request(phone, fiat_amount=500, currency=KES, escrow_eth_tx_id=<that id>, status=Pending)`
3. User confirms → `eth_tx.status=Submitted` → broadcaster submits escrow tx
4. On `eth_tx.status=Confirmed`:
   - Middleware `confirm_eth_tx` reducer advances linked `withdrawal_request.status = Escrowed`
5. `broadcaster.pretium_worker` sees `withdrawal_request.status=Escrowed`:
   - Calls Pretium API: `POST /off-ramp { amount, currency, recipient_phone }`
   - On success: `withdrawal_request.status=Fulfilled`, stores `pretium_ref`, inserts `WithdrawSent` SMS
   - On failure: `withdrawal_request.status=Failed`; middleware `refund_withdrawal` reducer inserts new `eth_tx(tx_type=WithdrawRefund, from=gateway_escrow, to=user_burner, ...)`, sets `withdrawal_request.refund_eth_tx_id`, inserts `WithdrawFailed` SMS

### Flow D — Balance

1. User picks "4. Balance", enters PIN
2. Middleware `validate_pin`:
   - Verifies PIN
   - Inserts `balance_query(session_id, phone, status=Pending)`
   - Leaves `ussd_response` as "Checking balance..." placeholder
3. Africa's Talking polls response; USSD session stays open
4. `broadcaster.balance_worker`:
   - Calls `eth_getBalance` on burner wallet
   - Reducer `complete_balance_query` writes result + updates `ussd_response.response_text` to `"Balance: {eth} ETH"`
5. Next USSD poll sees updated response → displays to user

Typical latency: ~2s, acceptable for USSD.

### Flow E — Account menu

- **Address**: pure DB lookup → display `esim_profile.wallet_address`
- **Base Token**: settings screen writing to `user_preferences.base_token`
- **Instant Withdraw**: settings screen writing to `user_preferences.default_withdraw_method`

### Flow F — Swap (Phase 1 placeholder)

`SwapTokenScreen` → replaced by `SwapComingSoonScreen` (Quit type): `"Swap coming soon. Dial *384*6086# to send ETH."`.

---

## Middleware cleanup

### Correctness fixes (must-have for Phase 1)

| Issue | Fix | Location |
|-------|-----|----------|
| `keccak256_simple` is fake (byte-sum XOR, not Keccak) | `sha3::Keccak256::digest(sig.as_bytes())[..4]` | `middleware/src/eth/tx/encoding.rs:3-10` |
| `validate_pin` phone_position bug | Use length-based match: `len==2 → 1`, `len==4 → 1` | `middleware/src/functions/validate.rs:48-58` |
| `nick_auth_7702` unbounded loop | `const MAX_ATTEMPTS = 1_000_000`, return `Result<_, AuthGenError>` | `middleware/src/auth/list/hashing.rs:164-188` |
| Phone normalization inconsistency | Normalize to E.164-with-`+` at ussdclient entry; middleware assumes E.164 throughout; delete `normalize_phone_number` from hot path | `middleware/src/eth/tx/encoding.rs` + `lib.rs:318` |
| `cancel_tx` stub doesn't update `eth_tx` | `parts[4]=="1"` → set `eth_tx.status = Submitted` + insert SMS; `"2"` → `status = Cancelled` | `middleware/src/functions/cancel.rs` |

### Structural cleanups

| Issue | Fix |
|-------|-----|
| `FUNCTION_MAP` mutex cleared + re-registered on every `load_function()` call | Replace with a single dispatch: `match function_name { "register_pin" => register_pin(ctx, session), ... }`. Delete `FUNCTION_MAP`, `register_functions()`, `load_function()`, `lazy_static`. |
| `session.data` split by `*` in every function, no type safety | Introduce `enum UserIntent` parsed once per step via `parse_intent(current_screen, data) -> Result<UserIntent, String>`. Each function takes a typed variant. |
| Old `Swap` table code commented out everywhere | Delete (git keeps history). |
| `PhoneWallet` table superseded by `EsimProfile` | Delete `PhoneWallet` table. |
| Tests disabled due to SpacetimeDB 1.4 API change | Extract pure logic into db-free modules. Unit-test those. Integration tests via E2E (see Testing). |

### Schema changes to existing tables

**`eth_tx`** — the current primary key is `session_id`, making it 1:1 with USSD sessions. This blocks (a) Withdraw, which needs both an escrow tx and a refund tx linked to the same session, and (b) future features like retry txs.

Migration:

```rust
#[table(name = eth_tx, public)]
pub struct EthTx {
    #[primary_key] #[auto_inc] pub id: u64,            // NEW
    #[index(btree)] pub session_id: String,            // was primary_key
    pub tx_type: TxType,                                // NEW (SendEth | WithdrawEscrow | WithdrawRefund)
    pub from: String,                                   // burner wallet or gateway escrow
    pub to: String,                                     // burner wallet, gateway escrow, or external
    pub value: String,
    pub data: Option<Vec<u8>>,
    pub gas_limit: String,
    pub status: TxStatus,
    pub tx_hash: Option<String>,
    pub block_number: Option<u64>,                      // NEW
    pub gas_used: Option<String>,                       // NEW
    pub error_reason: Option<String>,                   // NEW
    pub processing_by: Option<String>,                  // NEW (broadcaster lease)
    pub processing_since: Option<Timestamp>,            // NEW
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
```

The `TxType` enum (existing in `eth/tx/types.rs` as `SendEth | TokenSwap | CashOut | Balance`) is updated to `SendEth | WithdrawEscrow | WithdrawRefund`. `TokenSwap` and `CashOut` / `Balance` variants are removed — they were unused by the actual flow (Balance is a separate `balance_query` table; Swap is deferred; CashOut was a pre-Pretium abstraction).

**Wallet-address conventions in `eth_tx`:**

| tx_type | from | to |
|---------|------|-----|
| `SendEth` | sender's burner wallet | receiver's burner wallet |
| `WithdrawEscrow` | user's burner wallet | gateway escrow address (single address in env `GATEWAY_ESCROW_ADDRESS`) |
| `WithdrawRefund` | gateway escrow | user's burner wallet |

All three types invoke `Burner7702.execute(to, value, data)` on the `from` address (which has 7702-delegated code). For `WithdrawRefund`, the `from` is gateway-owned so no 7702 auth is needed — a plain EOA signed tx.

### New tables

```rust
#[table(name = withdrawal_request)]
pub struct WithdrawalRequest {
    #[primary_key] #[auto_inc] id: u64,
    #[index(btree)] phone_number: String,
    fiat_amount: String,
    currency: String,
    escrow_eth_tx_id: u64,                 // FK to eth_tx.id, the escrow leg
    refund_eth_tx_id: Option<u64>,         // FK to eth_tx.id if refund was issued
    pretium_ref: Option<String>,
    status: WithdrawalStatus, // Pending | Escrowed | Fulfilled | Failed | Refunded
    created_at: Timestamp,
    updated_at: Timestamp,
}

#[table(name = sms_notification)]
pub struct SmsNotification {
    #[primary_key] #[auto_inc] id: u64,
    #[index(btree)] phone_number: String,
    template: SmsTemplate,
    payload_json: String,
    status: SmsStatus,        // Pending | Processing | Sent | Failed
    message_id: Option<String>,
    created_at: Timestamp,
    updated_at: Timestamp,
}

#[table(name = balance_query)]
pub struct BalanceQuery {
    #[primary_key] session_id: String,
    phone_number: String,
    status: QueryStatus,      // Pending | Done | Failed
    result_wei: Option<String>,
    created_at: Timestamp,
}

#[table(name = user_preferences)]
pub struct UserPreferences {
    #[primary_key] phone_number: String,
    base_token: String,       // "ETH"
    default_withdraw_method: Option<String>,
    updated_at: Timestamp,
}
```

### New reducers (called by broadcaster and smsclient)

```
mark_eth_tx_processing(eth_tx_id, processing_by)  // Pending/Submitted → Broadcasting, sets lease
mark_eth_tx_broadcast(eth_tx_id, tx_hash)         // Broadcasting → Broadcast (tx submitted, awaiting confirm)
confirm_eth_tx(eth_tx_id, tx_hash, block, gas)    // Broadcast → Confirmed; advances linked withdrawal_request if Escrow
fail_eth_tx(eth_tx_id, tx_hash, reason)           // → Failed

fulfill_withdrawal(request_id, pretium_ref)       // Escrowed → Fulfilled
fail_withdrawal(request_id, reason)               // → Failed; triggers refund eth_tx
refund_withdrawal(request_id)                     // internal, issues new eth_tx gateway → burner

mark_auth7702_active(authority_address)           // Pending → Active

complete_balance_query(session_id, result_wei)    // Pending → Done, updates ussd_response

mark_sms_processing(id)                           // Pending → Processing
mark_sms_sent(id, message_id)                     // Processing → Sent
mark_sms_failed(id, reason)                       // Processing → Failed

set_gateway_identity(identity)                    // ops reducer — whitelists broadcaster's SpacetimeDB Identity
set_smsclient_identity(identity)                  // ops reducer — whitelists smsclient's Identity
```

**Reducer authorization:** each broadcaster/smsclient reducer checks `ctx.sender == <expected_identity>` from `app_config`. Mirrors the existing TODO in `process_ussd_step` for the USSD client whitelist.

### Deletions (YAGNI)

- All commented-out `Swap`, `SwapStatus`, `SwapType` code
- `_check_profile_exists` / `_check_session_exists` helpers (used once, inline them)
- `src/lib.rs` top-level `pub use` aliases (`SwapTable`, `USSDSessionTable`)
- `middleware/src/swap_tests.rs`
- FATF audit reducers: keep the table, remove from Phase 1 flow calls — deferred to Phase 2

---

## Broadcaster service

### Layout

```
broadcaster/
├── main.py                   # entrypoint, spins up workers
├── stdb.py                   # SpacetimeDB subscribe + reducer-call client
├── chain.py                  # web3 wrapper: nonce mgr, gas estimation, signing
├── gateway_key.py            # loads GATEWAY_PRIVATE_KEY, exposes sign() — swap point for KMS in Phase 2
├── abi/
│   └── burner7702.json       # compiled from contracts/src/Burner7702.sol
└── workers/
    ├── auth7702_worker.py    # lazy 7702 auth attachment
    ├── ethtx_worker.py       # signs + submits Burner7702.execute()
    ├── pretium_worker.py     # Pretium off-ramp API
    └── balance_worker.py     # eth_getBalance queries
```

### Worker pattern (shared across all four)

```
1. Subscribe to filtered SpacetimeDB query (status = 'Submitted' or 'Pending')
2. On event:
   a. Reducer mark_*_processing (sets processing_by = this worker's ID,
      processing_since = now). Rejects if already claimed within last 5 min.
   b. Perform external work
   c. Reducer writeback (confirm / fulfill / fail)
3. Transient failure: exponential backoff 3× (2s, 8s, 30s)
4. Terminal failure: status=Failed + error_reason
5. Crash recovery: on startup, clear stale processing_* claims (> 5 min old)
   owned by this worker ID, resubscribe.
```

### Ethtx worker — hot path

```python
async def process_eth_tx(tx: EthTx):
    calldata = burner7702.encodeABI(
        fn_name="execute",
        args=[tx.to, int(tx.value), bytes.fromhex(tx.data or "")]
    )

    authority_has_code = await chain.eth.get_code(tx.from_address) != b""

    tx_params = {
        "to": tx.from_address,
        "data": calldata,
        "value": 0,
        "gas": await chain.estimate_gas(...),
        "maxFeePerGas": ...,
        "maxPriorityFeePerGas": ...,
        "nonce": nonce_mgr.next(GATEWAY_ADDRESS),
        "chainId": BASE_SEPOLIA_CHAIN_ID,
    }

    if not authority_has_code:
        auth_row = await stdb.query_one(
            "SELECT * FROM auth_7702 WHERE authority_address = ?",
            tx.from_address
        )
        tx_params["authorizationList"] = [build_auth_tuple(auth_row)]
        tx_params["type"] = 4

    signed = gateway_key.sign_tx(tx_params)
    tx_hash = await chain.eth.send_raw_transaction(signed.rawTransaction)

    await stdb.call_reducer("mark_eth_tx_broadcast", tx.id, tx_hash.hex())
    receipt = await chain.wait_for_receipt(tx_hash, timeout=120)

    if receipt.status == 1:
        await stdb.call_reducer(
            "confirm_eth_tx",
            tx.id, tx_hash.hex(), receipt.blockNumber, str(receipt.gasUsed),
        )
    else:
        await stdb.call_reducer("fail_eth_tx", tx.id, tx_hash.hex(), "revert")
```

### Key decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Nonce management | In-memory counter per gateway address, seeded from `get_transaction_count(pending)` on startup | One signer → one nonce source. Simpler than on-chain lookup per tx. |
| 7702 auth broadcasting | **Lazy**: attached to first user tx via `authorization_list` | Saves one user-funded tx. One tx handles both SetCode + first `execute()`. |
| Gas estimation | `eth_estimateGas` + 20% buffer | Base Sepolia gas is cheap; overshoot is fine. |
| Gas payment | Gateway always pays | Users have 0 ETH — core premise of the UX. Track in `eth_audit_logs.gas_fee`. |
| Retry | Exp backoff 3× (2s, 8s, 30s); then Failed | No silent retries — a silently-failed tx is better than a double-spend. |
| Double-submission guard | `mark_processing` reducer sets `processing_by` + `processing_since`. Other instances skip rows claimed in last 5 min. | Enables future multi-instance broadcaster without external coordination. |
| Pretium auth | Per Pretium docs, `PRETIUM_API_KEY` env var | Standard REST pattern. |
| Reducer authorization | `ctx.sender == gateway_identity` from `app_config` | Matches existing whitelist pattern. |

---

## SMS service

### Layout

```
smsclient/
├── main.py
├── stdb.py                  # subset of broadcaster's stdb.py (shared lib in Phase 3)
├── termii.py                # Termii API wrapper
└── templates.py             # template registry + renderer
```

### Template registry

```python
TEMPLATES: dict[SmsTemplate, SmsTemplateDef] = {
    RegSuccess:     ("Welcome to M-ETH! Your wallet: {wallet}\nDial *384*6086# to send ETH.",
                     ["wallet"]),
    TxSubmitted:    ("Tx submitted: sending {amount} ETH to {to_phone}. Tracking: {tx_hash}",
                     ["amount", "to_phone", "tx_hash"]),
    TxConfirmed:    ("Tx confirmed! {amount} ETH sent to {to_phone}. Block: {block}",
                     ["amount", "to_phone", "block"]),
    TxFailed:       ("Tx failed: sending {amount} ETH to {to_phone}. Reason: {reason}",
                     ["amount", "to_phone", "reason"]),
    InboundEth:     ("You received {amount} ETH from {from_phone}.",
                     ["amount", "from_phone"]),
    WithdrawInit:   ("Withdraw initiated: {fiat_amount} {currency}. Processing via M-Pesa...",
                     ["fiat_amount", "currency"]),
    WithdrawSent:   ("Withdrawal complete! {fiat_amount} {currency} sent to M-Pesa {msisdn}. Ref: {ref}",
                     ["fiat_amount", "currency", "msisdn", "ref"]),
    WithdrawFailed: ("Withdrawal failed: {fiat_amount} {currency}. Funds refunded. Contact support.",
                     ["fiat_amount", "currency"]),
    PinLocked:      ("Security alert: 3 failed PIN attempts. Account locked for 15 minutes.",
                     []),
}
```

**Validation:** the middleware reducer that inserts `sms_notification` rows validates that `payload_json` has all required keys for the template. SMS service never discovers rendering errors at send time.

### Drain loop

```python
async def drain():
    while True:
        pending = await stdb.query(
            "SELECT * FROM sms_notification WHERE status = 'Pending' "
            "ORDER BY created_at ASC LIMIT 50"
        )
        if not pending:
            await asyncio.sleep(2)
            continue
        for row in pending:
            await process_one(row)

async def process_one(row: SmsNotification):
    await stdb.call_reducer("mark_sms_processing", row.id)
    rendered = templates.render(row.template, json.loads(row.payload_json))
    try:
        msg_id = await termii.send_sms(to=row.phone_number, body=rendered)
        await stdb.call_reducer("mark_sms_sent", row.id, msg_id)
    except TermiiError as e:
        await stdb.call_reducer("mark_sms_failed", row.id, str(e))
```

### Key decisions

| Decision | Choice |
|----------|--------|
| Delivery guarantee | At-least-once |
| Retry policy | 3 attempts, exp backoff (2s, 8s, 30s); then Failed |
| Rate limiting | Batch ≤ 50 per cycle, 2s sleep (~25/s cap, well under Termii limits) |
| Language | English only for Phase 1. Swahili/Luganda = Phase 2. |
| Sender ID | `TERMII_SENDER_ID` env var |
| Cost tracking | Store Termii cost from API response — useful for Phase 2 billing |
| DLR | Phase 2 (Termii webhook) |

---

## Error handling

| Layer | Failure surface | Response |
|-------|----------------|----------|
| USSD bridge | Timeout, malformed, session expiry | Return `END Error`; no retry |
| Middleware reducer | Invalid input, wrong PIN, bad format | Write `session.error_text`; screen stays; user re-prompted. Session never advances on reducer error. |
| Broadcaster / SMS worker | RPC/API flake | Exp backoff 3×; terminal → `status=Failed + reason` |
| On-chain revert | Contract rejects | `eth_tx.status=Failed`; `TxFailed` SMS; receipt retained |
| Escrow stuck | ETH in escrow, Pretium permanently failed | `refund_withdrawal` reducer → new `eth_tx` gateway → burner; `WithdrawFailed` SMS |

**Cross-layer invariant:** ETH only moves via `eth_tx` rows. Every movement including refunds is an auditable row with a tx hash. No side channels.

---

## Testing

### Unit tests

**Middleware (pure logic, db-free modules):**
- PIN hashing, weak-PIN detection, format validation
- Amount parsing, phone normalization
- ABI encoding with real Keccak
- `UserIntent` parsing (all screens, all valid + invalid inputs)
- Template payload validation
- Nick's Method derivation (deterministic test vector: known phone → known wallet)
- Target: ≥ 80% coverage on pure logic

**Broadcaster:**
- Chain wrapper (mocked web3)
- Nonce manager
- Retry logic with backoff
- Mock SpacetimeDB + Pretium

**SMS service:**
- Template rendering (all templates × valid/invalid payloads)
- Drain-loop logic
- Mock Termii

### Integration tests

- **Contracts**: Foundry tests against Burner7702 + EsimRegistry. Fork Base Sepolia for cross-contract scenarios.
- **Middleware**: deferred until SpacetimeDB testing API stabilizes post-1.4. E2E compensates.

### E2E tests (scripted Base Sepolia)

- `tests/e2e/test_registration.py` — POST as Africa's Talking, assert DB + on-chain result
- `tests/e2e/test_send_eth.py` — register A, register B, A sends to B, verify on-chain balance change + SMS sent
- `tests/e2e/test_withdraw.py` — initiate withdraw, assert escrow tx + Pretium mock called
- `tests/e2e/test_pin_lockout.py` — 3 wrong PINs → lockout
- `tests/e2e/test_balance.py` — balance query round-trip ≤ 5s

### CI pipeline

- **PR**: unit tests + Foundry tests (fast, < 2 min)
- **Merge to develop**: full E2E suite against Anvil fork of Base Sepolia (~5 min)
- **Manual trigger**: E2E against real Base Sepolia + Pretium sandbox

---

## Phase ordering

### Phase 1 — Demo

1. Deploy Burner7702 + EsimRegistry + Permit2WithOperator to Base Sepolia
2. Middleware correctness fixes (real keccak, validate_pin, nick_auth bound, phone norm, cancel_tx)
3. Middleware structural: FUNCTION_MAP → match dispatch; typed UserIntent; delete Swap/PhoneWallet
4. New tables: withdrawal_request, sms_notification, balance_query, user_preferences
5. New reducers for broadcaster/SMS writeback
6. Broadcaster service: auth7702 + ethtx workers, lazy 7702 broadcasting
7. SMS service: Termii integration, template registry, drain loop
8. Pretium worker + withdraw flow
9. Balance worker
10. Menu.json update: SwapComingSoon, Account sub-items
11. E2E test suite + CI
12. Ship

### Phase 2 — Crypto and compliance hardening

- Gateway key abstraction: swap env var for pluggable signer (KMS-ready interface)
- FATF / Travel Rule reducers wired into Send ETH and Withdraw (populate `eth_audit_logs`)
- PIN rotation reducer + "forgot PIN" out-of-band recovery
- Reducer-layer rate limiting (per-phone + per-identity)
- Termii DLR webhook
- Swahili SMS templates + per-user language pref
- `nick_auth_7702` deterministic benchmark + attempt-count SLA
- Independent signature audit (Nick's Method + 7702 auth construction)

### Phase 3 — Architecture refinement

- Extract broadcaster into independent deployable (own Dockerfile + compose profile)
- Multi-instance broadcaster (row-lease protocol already present in Phase 1 design; just add coordination)
- Observability: structured JSON logs, Prometheus metrics, trace IDs across services
- Shared Python lib for SpacetimeDB client (stop copy-paste)
- Operator dashboard (simple web UI reading middleware tables)
- DR runbook: key compromise, SpacetimeDB corruption, Pretium API change
- Re-enable middleware integration tests once SpacetimeDB testing story matures

---

## Deployment (Phase 1)

Extend existing `docker-compose.yml`:

```yaml
services:
  middleware:     # SpacetimeDB + compiled WASM module (existing)
  ussdclient:     # Flask bridge (existing, Python)
  broadcaster:    # NEW — env: GATEWAY_PRIVATE_KEY, RPC_URL, PRETIUM_API_KEY
  smsclient:      # NEW — env: TERMII_API_KEY, TERMII_SENDER_ID
```

**Secrets Phase 1:** `.env` on host, file perms 600, documented in `doc/SECRETS_SETUP.md`. Phase 2 migrates to a secret manager.

DigitalOcean deploy workflow (`.github/workflows/deploy.yml`) extends to build and push the two new service images.

---

## Non-goals (Phase 1)

- Not mainnet-ready
- Not supporting multiple gateway keys (single OPERATOR per deploy)
- Not supporting non-SIM-bound wallets (recovery = Phase 2)
- Not supporting currencies beyond KES for Withdraw
- Not building a web UI

---

## Open items deferred to implementation plan

- Exact Pretium endpoint shape (API docs needed)
- Exact gas cost baselines on Base Sepolia (benchmark during implementation)
- Whether `balance_query` polling interval is acceptable over real MNO USSD latency (measure)
- Whether `sms_notification` drain loop is replaced by SpacetimeDB push subscription vs SQL polling (measure; polling is fine for Phase 1)
