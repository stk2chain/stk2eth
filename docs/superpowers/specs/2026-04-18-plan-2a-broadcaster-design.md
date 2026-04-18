# Plan 2a: Broadcaster + ABI Encoding Design

**Date:** 2026-04-18
**Scope:** Rust broadcaster binary that turns `eth_tx(Pending)` rows into confirmed on-chain transactions on Base Sepolia (chain_id 84532), plus the middleware changes required to unblock it.
**Out of scope (Plan 2b):** PIN lockout wiring, random PIN salt, broadcaster lease-holder validation, gas-bump for stuck txs.

---

## Goal

Ship a Rust broadcaster that subscribes to `eth_tx(Pending)` via SpacetimeDB WebSocket, builds and signs EIP-7702 type-4 transactions (authorization + execute bundled into a single tx per send), submits them to Base Sepolia via Alchemy, watches receipts via `newHeads` subscription, and calls the existing writeback reducers (`mark_eth_tx_processing`, `mark_eth_tx_broadcast`, `confirm_eth_tx`, `fail_eth_tx`, `mark_auth7702_active`).

Secondary deliverables (required to unblock the broadcaster):
- One-time `claim_gateway_identity()` reducer replacing the unreachable `set_gateway_identity`.
- Real ABI encoding via alloy `sol!` macro in the broadcaster; delete `TxType::signature()`, `TxType::selector()`, and `TxParams::encode()` stubs in the middleware.
- `.env` / middleware-default fix-up: use `BURNER7702_ADDRESS=0x1ff0A24D1145d58Ea6F190569C10916E1a78F013` (Base Sepolia) instead of `0x2fDdd08Fb3e796bc68B1a26f3D1a61b073860fEf` (Ethereum Sepolia, wrong chain).

## Locked decisions

| # | Decision |
|---|---|
| 1 | Every send is a single EIP-7702 type-4 tx carrying `authorization_list` (set-code) + top-level `execute()` call. No standalone set-code txs — lazy per-send delegation to save gas. |
| 2 | Operator key loaded from encrypted keystore JSON (`OPERATOR_KEYSTORE_PATH` + `OPERATOR_KEYSTORE_PASSPHRASE`). |
| 3 | Writeback auth via one-time `claim_gateway_identity()` trust-on-first-use reducer. |
| 4 | Single broadcaster worker; lease calls preserved for future multi-worker. |
| 5 | Pending-tx discovery via SpacetimeDB WebSocket subscription. |
| 6 | All-Rust broadcaster, alloy-rs (`alloy`, `alloy-signer-local`, `alloy-provider`, `alloy-rpc-types-eth`). |
| 7 | RPC provider: Alchemy Base Sepolia (free tier sufficient). |
| 8 | Receipt watching: separate tokio task subscribing to `eth_subscribe("newHeads")`. |
| 9 | Burner balance pre-check in broadcaster (`eth_getBalance`) before submit. |
| 10 | ABI encoding in broadcaster via alloy `sol!`; middleware stubs deleted. |
| 11 | In-memory operator nonce (bootstrapped from RPC), at-least-once recovery via reconcile-on-startup, no gas-bump for stuck txs (deferred). |
| 12 | Burner7702 ABI sourced from verified contract at https://sepolia.basescan.org/address/0x1ff0A24D1145d58Ea6F190569C10916E1a78F013#code. |

---

## 1. Crate layout

`Cargo.toml` at repo root becomes a workspace. The broadcaster is a new crate; middleware structure unchanged.

```
stk2eth/
├── Cargo.toml                       NEW: [workspace] members = ["middleware", "broadcaster"]
├── middleware/                      existing Rust WASM module
├── broadcaster/                     NEW
│   ├── Cargo.toml                   bin target
│   └── src/
│       ├── main.rs                  parse env, connect RPC + STDB, spawn tasks
│       ├── config.rs                env-var loading, keystore decrypt, startup validation
│       ├── signer.rs                operator keystore → alloy LocalSigner
│       ├── subscriber.rs            SpacetimeDB subscription to eth_tx(Pending)
│       ├── submitter.rs             build + sign + submit type-4 tx
│       ├── watcher.rs               newHeads subscription + receipt matching
│       ├── abi.rs                   alloy sol!{} Burner7702 bindings
│       ├── reconcile.rs             startup recovery for in-flight rows
│       └── error.rs                 BroadcasterError enum
├── contracts/
│   └── abi/Burner7702.json          NEW: pulled from Basescan
└── tests/                           existing smoke + e2e
```

**Rationale.** Workspace (not separate repo) because the broadcaster imports middleware row types via `spacetimedb-sdk` bindings generated from the published module; separate repos force copy-paste or a codegen step. One file per concern keeps each module 50–150 lines with a single boundary — `submitter` doesn't know about subscriptions, `watcher` doesn't know how txs get submitted. They coordinate only through `eth_tx` row state.

Broadcaster tests:
```
broadcaster/tests/
├── abi_encoding.rs              sol!-derived selector == known-good keccak
├── nonce_recovery.rs            simulated crash mid-submit
└── integration_anvil.rs         end-to-end against anvil --fork base-sepolia
```

## 2. Broadcaster state machine

Two tokio tasks run concurrently. In-process shared state: operator nonce (atomic `u64`) and the operator `LocalSigner`. All other coordination via `eth_tx` rows.

**`eth_tx.status` transitions:**

```
  Pending ─► Broadcasting ─► Broadcast ─► Confirmed
     │            │              │
     │            │              └─► Failed (reverted on-chain)
     │            └─► Failed (pre-submit failure: insufficient balance, bad build)
     └─► Cancelled (set by USSD cancel_tx; broadcaster never writes Cancelled)
```

Terminal states: `Confirmed | Failed | Cancelled`. The existing `is_terminal()` helper in `reducers/broadcaster.rs` guards all writebacks.

**Submitter task:**

```rust
loop {
    let row = subscriber.next_pending().await;
    if let Err(e) = handle(row).await {
        if e.is_terminal() {
            let _ = reducer.fail_eth_tx(row.id, None, &e.to_string()).await;
        } else {
            log::warn!("transient on row {}: {e} — will retry", row.id);
            // leave status as Pending; next subscription push or reconcile retries
        }
    }
}

async fn handle(row: EthTx) -> Result<(), BroadcasterError> {
    reducer.mark_eth_tx_processing(row.id, WORKER_ID).await?;
    let burner = derive_burner(&row.from_phone)?;
    let balance = rpc.get_balance(burner.address).await?;
    if balance < row.amount + BALANCE_RESERVE {
        return Err(BroadcasterError::InsufficientBalance {
            have: balance, need: row.amount + BALANCE_RESERVE,
        });
    }
    let needs_auth = !is_already_delegated(burner.address).await?;
    let tx = build_type4(&row, &burner, needs_auth, operator_nonce.next())?;
    let signed = operator_signer.sign_transaction(tx).await?;
    let hash = rpc.send_raw_transaction(signed.envelope_bytes()).await?;
    reducer.mark_eth_tx_broadcast(row.id, hash).await?;
    watcher_tx.send((row.id, hash, burner.address, needs_auth)).await?;
    Ok(())
}
```

**Watcher task:**

```rust
let mut blocks = rpc.subscribe_blocks().await?;
let mut tracked: HashMap<TxHash, Tracked> = HashMap::new();
loop {
    tokio::select! {
        Some(item) = watcher_rx.recv() => { tracked.insert(item.hash, item); }
        Some(_block) = blocks.next() => {
            for (hash, item) in tracked.clone() {
                match rpc.get_transaction_receipt(hash).await? {
                    Some(r) if r.status() => {
                        reducer.confirm_eth_tx(item.id, format!("{hash:#x}"),
                            r.block_number.unwrap(), r.gas_used.to_string()).await?;
                        if item.needs_auth {
                            reducer.mark_auth7702_active(format!("{:#x}", item.burner)).await?;
                        }
                        tracked.remove(&hash);
                    }
                    Some(_) => {
                        reducer.fail_eth_tx(item.id, Some(format!("{hash:#x}")),
                            "execute reverted".into()).await?;
                        tracked.remove(&hash);
                    }
                    None => {} // still pending; try again next block
                }
            }
        }
    }
}
```

**Signed fields per tx (EIP-7702 type-4):**
- Outer signer: operator (env keystore). Pays gas.
- `authorization_list = [ SignedAuthorization { chain_id: 84532, address: 0x1ff0…F013, nonce: burner_nonce, signature: burner.sign(auth_tuple) } ]` — present iff `needs_auth` (i.e., on-chain code at burner ≠ `0xef0100 || Burner7702`).
- `to = burner_address`, `value = 0`.
- Calldata = `Burner7702.execute(recipient, amount, 0x)` — generated by alloy `sol!`.

## 3. Reconcile-on-startup

Runs once before the submitter/watcher loops accept work. Purpose: clean up rows left in non-terminal states by the previous run.

**Startup handling by status:**

| Status | Action |
|---|---|
| `Pending` | No-op. Submitter picks up via subscription on first push. |
| `Broadcasting` | Investigate. Lease was held but writeback didn't complete. Retry `handle(row)`; on `NonceAlreadyUsed`, walk operator's recent txs to recover the hash. |
| `Broadcast` | Enqueue on watcher's track-list. |
| `Confirmed` / `Failed` / `Cancelled` | Skip. |

**Broadcasting recovery:**

```rust
for row in stdb.query("SELECT * FROM eth_tx WHERE status = 'Broadcasting'").await? {
    match submitter.handle(row.clone()).await {
        Ok(_) => {}                                                 // successful re-submit
        Err(BroadcasterError::NonceAlreadyUsed) => {
            // prior submit landed; walk last N blocks for operator's tx
            if let Some(hash) = recover_broadcast_hash(&row, RECONCILE_SCAN_BLOCKS).await? {
                reducer.mark_eth_tx_broadcast(row.id, hash).await?;
                watcher_tx.send((row.id, hash, burner, needs_auth)).await?;
            } else {
                reducer.fail_eth_tx(row.id, None,
                    "reconcile: broadcasting row with no recoverable hash".into()).await?;
            }
        }
        Err(e) => reducer.fail_eth_tx(row.id, None, &format!("reconcile: {e}")).await?,
    }
}

for row in stdb.query("SELECT * FROM eth_tx WHERE status = 'Broadcast'").await? {
    let burner = derive_burner(&row.from_phone)?.address;
    // needs_auth is recovered from auth_7702 table, not from the eth_tx row
    let needs_auth = match stdb.query_one(
        "SELECT status FROM auth_7702 WHERE authority_address = ?", burner.to_string()).await?
    {
        Some(r) if r.status == AuthStatus::Broadcasted => false,
        _ => true,
    };
    watcher_tx.send((row.id, row.tx_hash.unwrap(), burner, needs_auth)).await?;
}
```

The submitter's in-flight path has the same requirement, but there `needs_auth` is computed from a fresh `eth_getCode(burner)` call — so no state loss at that point. Only the startup path needs this fallback lookup.

**`recover_broadcast_hash`:** walks `eth_getBlockByNumber` for the last `RECONCILE_SCAN_BLOCKS` (default 20) blocks, filters for txs with `from == operator` and `to == burner`, decodes calldata via the alloy `Burner7702::execute` decoder, matches on `(recipient, amount)`. Returns the matching `TxHash` or `None`.

**Nonce rebootstrap.** After reconcile completes, `operator_nonce = rpc.get_transaction_count(operator_addr, "pending")`. This absorbs any duplicate submits performed during reconcile.

**Timeout.** Reconcile has a 60s hard budget. On timeout, log and proceed — the watcher still picks up `Broadcast` rows; `Broadcasting` stragglers can be cleaned by a future admin reducer (not in 2a).

**Why not auto-fail all `Broadcasting` rows at startup:** if the prior broadcaster submitted a user's send and the chain accepted it but writeback was lost, marking it Failed causes the user to retry → double-spend from the burner. The 20-block walk is cheap insurance.

## 4. Error taxonomy

One enum classifies every failure by handling policy.

```rust
#[derive(Debug, thiserror::Error)]
pub enum BroadcasterError {
    // TERMINAL — mark row Failed, never retry
    #[error("insufficient burner balance: have {have} need {need}")]
    InsufficientBalance { have: U256, need: U256 },
    #[error("invalid recipient address: {0}")]
    InvalidRecipient(String),
    #[error("tx reverted on chain: {0}")]
    Reverted(String),
    #[error("burner derivation failed: {0}")]
    DerivationFailed(String),

    // RETRYABLE — leave row in Pending / Broadcasting
    #[error("RPC unavailable: {0}")]
    RpcTransient(String),
    #[error("rate limited by provider")]
    RateLimited,
    #[error("nonce already used")]
    NonceAlreadyUsed,

    // WRITEBACK — log and continue; reducers are idempotent
    #[error("SpacetimeDB reducer rejected: {0}")]
    ReducerRejected(String),

    // PROGRAMMER ERROR — crash loud
    #[error("config error: {0}")]
    Config(String),
}

impl BroadcasterError {
    pub fn is_terminal(&self) -> bool {
        use BroadcasterError::*;
        matches!(self,
            InsufficientBalance {..} | InvalidRecipient(_) | Reverted(_) | DerivationFailed(_))
    }
    pub fn is_retryable(&self) -> bool {
        use BroadcasterError::*;
        matches!(self, RpcTransient(_) | RateLimited | NonceAlreadyUsed)
    }
}
```

**Retry policy.** A retryable error leaves the row status unchanged; next subscription push or next reconcile retries. No in-loop busy retry — starves other rows.

**Rate-limit handling.** On `RateLimited`, the submitter loop pauses with exponential backoff (1s → 2s → 4s, cap at `RATE_LIMIT_BACKOFF_MAX_SECS=30`). Subscription events queue (bounded 256; overflow logs a warning). Loop resets backoff on next successful RPC call.

**Writeback failure.** The on-chain side already committed; log and continue. Reconcile-on-startup or the watcher's hash scan catches orphans. Crashing here would create more orphans than it prevents.

**Not included (deferred):** `Stuck` variant for underpriced txs. Add when observed.

## 5. Test strategy

Three layers, no chain mocks (anvil fork is cheap).

**Layer 1: unit (in-tree, `cargo test -p broadcaster --lib`):**
- `abi.rs::tests::test_execute_selector_matches_keccak` — verify alloy `sol!` selector == `0xb61d27f6`.
- `abi.rs::tests::test_encode_execute_roundtrip` — encode → decode == input.
- `abi.rs::tests::test_encode_matches_reference` — golden-file bytes compared against a canonical web3.py-generated payload.
- `signer.rs::tests::test_keystore_decrypt_correct_passphrase`.
- `signer.rs::tests::test_keystore_decrypt_wrong_passphrase_errors`.
- `signer.rs::tests::test_sign_authorization_tuple_roundtrip` — signed auth → ecrecover returns burner address.
- `error.rs::tests::test_classification_matrix` — every variant lands in one of terminal/retryable/writeback.

**Layer 2: integration (`broadcaster/tests/integration_anvil.rs`, gated `#[ignore]`):**
- `test_end_to_end_send_eth_flow` — anvil fork, fund burner, insert Pending row, assert Confirmed + recipient balance.
- `test_needs_auth_first_send_attaches_authorization_list`.
- `test_second_send_skips_authorization` (burner code is `0xef0100..` after first send).
- `test_underfunded_burner_marks_row_failed_without_broadcasting`.
- `test_recipient_reverts_marks_row_failed_after_receipt`.
- `test_crash_mid_submit_reconciles_on_restart`.

Run locally with `cargo test -p broadcaster -- --ignored`. Anvil is a dev-dep; CI opts in only on PRs touching `broadcaster/**` or `middleware/src/eth/**`.

**Layer 3: live smoke (`tests/smoke/test_broadcaster_flow.sh`):**
- Mirror of existing `test_register_pin_flow.sh`. Real Alchemy key, pre-funded burner on actual Base Sepolia.
- Dial USSD → send-ETH → confirm → assert `eth_tx.status = Confirmed` within 60s; verify on Basescan.
- Not in CI; manual release-gate via `workflow_dispatch`.

**Not tested in 2a:** broadcaster-against-broadcaster lease races (2b), gas-bump (deferred), stress benchmarks.

## 6. Config & secrets

All broadcaster config via env vars. `.env.example` committed; real `.env` gitignored.

```bash
# Ethereum
RPC_URL=https://base-sepolia.g.alchemy.com/v2/<key>          # required
RPC_WS_URL=wss://base-sepolia.g.alchemy.com/v2/<key>         # required (newHeads)
CHAIN_ID=84532                                                # required; wrong-chain guard
BURNER7702_ADDRESS=0x1ff0A24D1145d58Ea6F190569C10916E1a78F013 # required

# Operator key
OPERATOR_KEYSTORE_PATH=/var/lib/stk2eth/operator-keystore.json  # required
OPERATOR_KEYSTORE_PASSPHRASE=<passphrase>                     # required
# Legacy WALLET_PRIVATE_KEY accepted only when OPERATOR_KEYSTORE_PATH unset.
# Emits startup warning. Removed in Plan 2b.

# SpacetimeDB
SPACETIME_HOST=http://127.0.0.1:3000                          # required
SPACETIME_DB_NAME=gateway2                                    # required
SPACETIME_AUTH_TOKEN=<operator's STDB token>                  # required; must match claim_gateway_identity() caller

# Tuning (optional, with defaults)
BALANCE_RESERVE_WEI=500000000000000                           # 0.0005 ETH reserve for execute() overhead
RECONCILE_SCAN_BLOCKS=20
RATE_LIMIT_BACKOFF_MAX_SECS=30
LOG_LEVEL=info
```

**Keystore.** Standard Ethereum keystore JSON (scrypt-encrypted, geth/cast/alloy-compatible). Decrypted once at startup:

```rust
let keystore = eth_keystore::decrypt_key(path, passphrase)?;
let signer = LocalSigner::from_bytes(&keystore.into())?;
log::info!("operator loaded: {}", signer.address());    // log address, never key
```

**.env fix-up in Plan 2a:**
- Remove `BURNER_7702=0x2fDdd...` (Ethereum Sepolia, wrong chain).
- Add `BURNER7702_ADDRESS=0x1ff0A24D1145d58Ea6F190569C10916E1a78F013` (Base Sepolia).
- Keep `WALLET_PRIVATE_KEY` with a deprecation log; removed in 2b.
- `P2_ERC1271` unchanged (unrelated to 7702).

**Startup validation.** `config.rs` runs before any task is spawned. Refuse to start if:
- Any required env var missing.
- `CHAIN_ID` env ≠ `eth_chainId` from RPC.
- `OPERATOR_KEYSTORE_PATH` unreadable or undecryptable.
- SpacetimeDB connection fails 3 consecutive attempts.
- `app_config` has no `gateway_identity` row (must call `claim_gateway_identity()` first).

Failure = `std::process::exit(1)` with stderr message.

**Secrets handling.**
- Passphrase in `secrecy::SecretString`, zeroized on drop.
- `LocalSigner` opaque; `Debug` suppressed.
- Code review verifies no `{:?}` on `LocalSigner` or `SignedAuthorization` in any `log::` / `tracing::` call.

## 7. Middleware changes

**7.1 New reducer: `claim_gateway_identity`.**

New file `middleware/src/reducers/gateway_identity.rs`. Replaces `set_gateway_identity` (unreachable today). Trust-on-first-use — whoever calls first wins.

```rust
// middleware/src/reducers/gateway_identity.rs
use crate::{app_config, AppConfig};
use spacetimedb::{reducer, ReducerContext, Table};

const GATEWAY_IDENTITY_KEY: &str = "gateway_identity";

#[reducer]
pub fn claim_gateway_identity(ctx: &ReducerContext) {
    if ctx.db.app_config().key().find(GATEWAY_IDENTITY_KEY.to_string()).is_some() {
        log::error!("claim_gateway_identity: already claimed, rejected {}", ctx.sender);
        return;
    }
    ctx.db.app_config().insert(AppConfig {
        key: GATEWAY_IDENTITY_KEY.to_string(),
        value: format!("{}", ctx.sender),
    });
    log::info!("gateway identity claimed: {}", ctx.sender);
}
```

Register in `middleware/src/reducers/mod.rs`: `pub mod gateway_identity;`.

**Deploy runbook:**
```bash
spacetime publish gateway2
spacetime call gateway2 claim_gateway_identity    # as the operator identity
```

**7.2 Deletions.**
- `middleware/src/reducers/broadcaster.rs`: delete the `set_gateway_identity` reducer. Keep `require_gateway` helper.
- `middleware/src/eth/tx.rs`: delete `TxType::signature()`, `TxType::selector()`, `TxParams::encode()` stubs. Keep `TxType` enum and `TxParams` struct (intent fields are still meaningful).

**7.3 Config default fix.** Grep middleware for `0x2fDdd08Fb3e796bc68B1a26f3D1a61b073860fEf`; replace any occurrence with `0x1ff0A24D1145d58Ea6F190569C10916E1a78F013`. Most likely in a bootstrap/default-config reducer if one exists.

**7.4 Unchanged.**
- `eth_tx` and `auth_7702` schemas.
- Writeback reducers: `mark_eth_tx_processing`, `mark_eth_tx_broadcast`, `confirm_eth_tx`, `fail_eth_tx`, `mark_auth7702_active`.
- Lease TTL (5 minutes) in `mark_eth_tx_processing`.
- All USSD-side reducers (`process_ussd_step`, `register_pin`, `cancel_tx`, etc.).

**7.5 Explicitly NOT in Plan 2a** (these are Plan 2b):
- PIN lockout wiring (`MAX_PIN_ATTEMPTS`, `is_rate_limited`, `calculate_lockout_time` remain defined but uncalled).
- Random salt for `hash_pin` (currently `Timestamp::to_string()`).
- Lease-holder check in follow-up writeback reducers.
- Admin reducer for forcing stuck `Broadcasting` rows to a terminal state.
- Gas-bump for underpriced txs.
