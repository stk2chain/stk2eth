# Middleware Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the SpacetimeDB middleware so it (a) has correct ABI/crypto, (b) carries a schema the broadcaster and SMS services can integrate against, and (c) replaces the mutex-guarded function registry with typed dispatch. Produces compiling middleware with all Rust unit tests passing and a schema ready for Plan 2.

**Architecture:** Pure-logic modules are extracted out of reducer code so they can be unit-tested without SpacetimeDB. `eth_tx` migrates to an auto-increment `id` primary key with `tx_type`, lease, and receipt fields. `FUNCTION_MAP` is removed in favor of a single `dispatch(function_name, ctx, session)` match. User input is parsed once per step into a typed `UserIntent` enum instead of `*`-split strings at every function. New tables (`withdrawal_request`, `sms_notification`, `balance_query`, `user_preferences`) and writeback reducers (`mark_eth_tx_processing`, `confirm_eth_tx`, `fail_eth_tx`, `mark_auth7702_active`, `set_gateway_identity`) land but are not yet consumed (Plan 2 wires them).

**Tech Stack:**
- Rust + SpacetimeDB 1.8 (`spacetime publish --project-path .`)
- `sha3 = 0.10.8` (already in `Cargo.toml`)
- `k256 = 0.13.4` for ECDSA (already present)
- `cargo test` for unit tests of pure-logic modules

**Prerequisites:**
- Local SpacetimeDB instance running (`spacetime start` or docker-compose)
- Reference spec: `docs/superpowers/specs/2026-04-17-stk2eth-phase1-core-redesign.md`

**Schema migration note:** This plan changes the `eth_tx` primary key. SpacetimeDB does NOT support live migrations for PK changes. The executor must `spacetime delete <module>` (or recreate the dev database) between Task 4 and republishing. No production data is at stake (the project is on `develop` with no live users).

---

## File Structure

**New files:**
- `middleware/src/eth/tx/selector.rs` — pure Keccak selector computation, unit-testable
- `middleware/src/ussd/intent.rs` — `UserIntent` enum + `parse_intent(screen_name, data)` fn
- `middleware/src/functions/dispatch.rs` — `dispatch(fn_name, ctx, session) -> Result<USSDSession, String>` match
- `middleware/tests/selector_tests.rs` — integration test for selector vectors (uses `cargo test`)
- `middleware/tests/intent_tests.rs` — integration test for `parse_intent`

**Modified files:**
- `middleware/src/eth/tx/encoding.rs` — replace fake `keccak256_simple`, use `selector.rs`
- `middleware/src/eth/tx/tables.rs` — `eth_tx` schema migration
- `middleware/src/eth/tx/types.rs` — `TxType` variants (`SendEth | WithdrawEscrow | WithdrawRefund`)
- `middleware/src/eth/tx/mapping.rs` — `from_ussd_op` with updated variants
- `middleware/src/eth/tx/mod.rs` — add `pub mod selector;`
- `middleware/src/auth/list/hashing.rs` — `nick_auth_7702` returns `Result<_, AuthGenError>` with bounded attempts
- `middleware/src/auth/wallet.rs` — delete `PhoneWallet` table
- `middleware/src/functions/mod.rs` — remove `register_functions`, add `dispatch`
- `middleware/src/functions/register.rs` — use `UserIntent`, use bounded `create_phone_permit2_authorization`
- `middleware/src/functions/validate.rs` — use `UserIntent`, fix phone_position bug
- `middleware/src/functions/cancel.rs` — use `UserIntent`, update `eth_tx.status`
- `middleware/src/ussd/service/runtime.rs` — call `dispatch` directly, no mutex
- `middleware/src/ussd/service/utils.rs` — delete `FUNCTION_MAP` (file becomes empty, delete)
- `middleware/src/ussd/service/types.rs` — delete `FunctionMap`, `USSDFunction` type aliases
- `middleware/src/ussd/service/mod.rs` — remove `utils` module
- `middleware/src/lib.rs` — new table definitions, new writeback reducers, remove commented `Swap*` code, add `gateway_identity` in `app_config`

**Deleted files:**
- `middleware/src/swap_tests.rs` (superseded: no `Swap` table anymore)

**Unchanged files:**
- `middleware/src/functions/pin.rs` — `validate_pin_format`, `hash_pin` logic stays; just gets called with typed args
- `middleware/src/functions/utils.rs` — `parse_input` becomes private / unused, left for removal at end
- `middleware/src/auth/pin/*` — no changes
- `middleware/src/ussd/screen/*` — no changes
- `middleware/src/reducers/validate_phone.rs` — no changes (separate admin reducer)

---

## Task 1: Extract Keccak selector into pure module with real hash

**Files:**
- Create: `middleware/src/eth/tx/selector.rs`
- Create: `middleware/tests/selector_tests.rs`
- Modify: `middleware/src/eth/tx/mod.rs` (add `pub mod selector;`)
- Modify: `middleware/src/eth/tx/encoding.rs` (replace `keccak256_simple` with selector call)

- [ ] **Step 1: Write the failing test**

Create `middleware/tests/selector_tests.rs`:

```rust
// Integration test — uses the middleware crate's public API for selector.
// Cannot live in src/ because SpacetimeDB modules compile to cdylib.

use middleware::eth::tx::selector::keccak_selector;

#[test]
fn transfer_selector_matches_known_vector() {
    // Known value: keccak256("transfer(address,uint256)")[0..4] = 0xa9059cbb
    let got = keccak_selector("transfer(address,uint256)");
    assert_eq!(got, [0xa9, 0x05, 0x9c, 0xbb]);
}

#[test]
fn balance_of_selector_matches_known_vector() {
    // keccak256("balanceOf(address)")[0..4] = 0x70a08231
    let got = keccak_selector("balanceOf(address)");
    assert_eq!(got, [0x70, 0xa0, 0x82, 0x31]);
}

#[test]
fn swap_exact_tokens_selector_matches_known_vector() {
    // keccak256("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)")[0..4]
    // = 0x38ed1739
    let got = keccak_selector(
        "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"
    );
    assert_eq!(got, [0x38, 0xed, 0x17, 0x39]);
}
```

Because the middleware crate is `cdylib`, we also need a companion `rlib` or a `[lib]` crate-type addition to allow integration tests. **For this plan we'll unit-test inside `src/eth/tx/selector.rs` with a `#[cfg(test)] mod tests` block** instead. Rewrite the test file accordingly — skip `tests/selector_tests.rs`, keep it as an inline test.

- [ ] **Step 2: Create `middleware/src/eth/tx/selector.rs` with failing test inline**

```rust
use sha3::{Digest, Keccak256};

/// Compute the 4-byte function selector for an ABI signature like
/// `"transfer(address,uint256)"`. Pure, no SpacetimeDB dependency.
pub fn keccak_selector(signature: &str) -> [u8; 4] {
    let hash = Keccak256::digest(signature.as_bytes());
    let mut out = [0u8; 4];
    out.copy_from_slice(&hash[..4]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_selector_matches_known_vector() {
        assert_eq!(keccak_selector("transfer(address,uint256)"),
                   [0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn balance_of_selector_matches_known_vector() {
        assert_eq!(keccak_selector("balanceOf(address)"),
                   [0x70, 0xa0, 0x82, 0x31]);
    }

    #[test]
    fn swap_exact_tokens_selector_matches_known_vector() {
        assert_eq!(
            keccak_selector(
                "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"
            ),
            [0x38, 0xed, 0x17, 0x39]
        );
    }
}
```

Delete `middleware/tests/selector_tests.rs` if it was created — skip external tests dir; inline only.

- [ ] **Step 3: Add module to `middleware/src/eth/tx/mod.rs`**

Add `pub mod selector;` in the list. Full new content:

```rust
pub mod types;
pub mod params;
pub mod encoding;
pub mod tables;
pub mod mapping;
pub mod selector;


pub use tables::*;
pub use mapping::*;
pub use params::*;
pub use encoding::*;
pub use types::*;
pub use selector::*;
```

- [ ] **Step 4: Run selector tests — verify they pass**

Run: `cd middleware && cargo test --lib selector::tests -- --nocapture`
Expected: 3 passed.

- [ ] **Step 5: Replace `keccak256_simple` in `encoding.rs`**

Open `middleware/src/eth/tx/encoding.rs`. Delete lines 3-10 (the fake `keccak256_simple` fn). Update `TxType::selector` (was at line 51 area):

```rust
impl TxType {
    pub const fn signature(&self) -> &'static str {
        match self {
            Self::SendEth => "transfer(address,uint256)",
            Self::WithdrawEscrow => "",   // no calldata; native ETH transfer
            Self::WithdrawRefund => "",   // no calldata; gateway-signed native transfer
        }
    }

    pub fn selector(&self) -> [u8; 4] {
        use crate::eth::tx::selector::keccak_selector;
        keccak_selector(self.signature())
    }
    // ...
}
```

Note: `WithdrawEscrow` and `WithdrawRefund` are native ETH transfers (no calldata, just `value`), so their "signature" is the empty string — the `selector()` result is never used for those types. The `TxParams::encode()` function will check the variant and skip selector concatenation for Withdraw types. (Task 4 adjusts `TxParams` enum.)

- [ ] **Step 6: Build and verify**

Run: `cd middleware && cargo build --release`
Expected: builds clean (may have unused-variable warnings for WithdrawEscrow/WithdrawRefund — ignore).

- [ ] **Step 7: Commit**

```bash
git add middleware/src/eth/tx/selector.rs \
        middleware/src/eth/tx/mod.rs \
        middleware/src/eth/tx/encoding.rs
git commit -m "fix(middleware): use real Keccak256 for ABI selectors

Replaces keccak256_simple (a byte-XOR accumulator, not a hash) with
sha3::Keccak256::digest()[..4] in a new pure selector module. Verified
against known vectors for transfer, balanceOf, and swapExactTokensForTokens.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 2: Bound `nick_auth_7702` attempts, return Result

**Files:**
- Modify: `middleware/src/auth/list/hashing.rs:164-188` (`nick_auth_7702`)
- Modify: `middleware/src/auth/list/hashing.rs:190-223` (`create_phone_permit2_authorization`)

- [ ] **Step 1: Write the failing test**

Add at the end of `middleware/src/auth/list/hashing.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_valid_wallet_for_real_phone() {
        let phone = "+254712345678";
        let result = create_phone_permit2_authorization(
            phone,
            84532, // Base Sepolia chain ID
            0,
            None,
            Some("0x2fDdd08Fb3e796bc68B1a26f3D1a61b073860fEf"),
        );
        let (wallet, _signed) = result.expect("derivation must succeed within attempt bound");
        assert_ne!(wallet, [0u8; 20], "wallet address must be non-zero");
    }

    #[test]
    fn determinism_same_phone_same_wallet() {
        let phone = "+254712345678";
        let (w1, _) = create_phone_permit2_authorization(phone, 84532, 0, None, None).unwrap();
        let (w2, _) = create_phone_permit2_authorization(phone, 84532, 0, None, None).unwrap();
        assert_eq!(w1, w2);
    }
}
```

- [ ] **Step 2: Run test — expect fail (current fn returns tuple, not Result)**

Run: `cd middleware && cargo test --lib auth::list::hashing::tests -- --nocapture`
Expected: FAIL — `expect` on a tuple fails to compile.

- [ ] **Step 3: Introduce `AuthGenError` and change return types**

Edit `middleware/src/auth/list/hashing.rs`. At top, add:

```rust
const NICK_MAX_ATTEMPTS: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthGenError {
    DerivationExhausted { attempts: u64 },
    InvalidDelegateAddress,
}

impl std::fmt::Display for AuthGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthGenError::DerivationExhausted { attempts } =>
                write!(f, "Nick's Method exhausted after {} attempts", attempts),
            AuthGenError::InvalidDelegateAddress =>
                write!(f, "delegate address must be 0x-prefixed 20-byte hex"),
        }
    }
}
```

Rewrite `nick_auth_7702` (was around lines 164-188):

```rust
fn nick_auth_7702(
    mut r: [u8; 32], s: [u8; 32], v: u8, msg_hash: &[u8; 32],
) -> Result<([u8; 20], [u8; 32], u8), AuthGenError> {
    let mut attempts = 0u64;
    while attempts < NICK_MAX_ATTEMPTS {
        attempts += 1;
        if let Some(address) = recover_address(&r, &s, v, msg_hash) {
            log::info!("Nick's Method derivation succeeded in {} attempts", attempts);
            return Ok((address, r, v));
        }
        // increment r by 1
        let mut carry = 1u16;
        for i in (0..32).rev() {
            let sum = r[i] as u16 + carry;
            r[i] = sum as u8;
            carry = sum >> 8;
            if carry == 0 { break; }
        }
    }
    Err(AuthGenError::DerivationExhausted { attempts })
}
```

Rewrite `create_phone_permit2_authorization` return type:

```rust
pub fn create_phone_permit2_authorization(
    phone_number: &str,
    chain_id: u64,
    nonce: u64,
    user_salt: Option<&str>,
    delegate_to: Option<&str>,
) -> Result<([u8; 20], SignedAuthorization), AuthGenError> {
    let binding = get_permit2_address();
    let delegate_address = delegate_to.unwrap_or(&binding);

    if !delegate_address.starts_with("0x") || delegate_address.len() != 42 {
        return Err(AuthGenError::InvalidDelegateAddress);
    }

    let phone_salt = phone_to_salt(phone_number, user_salt);
    let msg_hash = hash_auth7702_message(chain_id, delegate_address, nonce);

    let mut r = [0u8; 32];
    r.copy_from_slice(&msg_hash);

    let (s, v) = normalize_s(phone_salt, 27);
    let (authority_address, final_r, final_v) = nick_auth_7702(r, s, v, &msg_hash)?;

    let mut addr_bytes = [0u8; 20];
    let delegate_bytes = hex::decode(&delegate_address[2..])
        .map_err(|_| AuthGenError::InvalidDelegateAddress)?;
    addr_bytes.copy_from_slice(&delegate_bytes);

    let signed_auth = SignedAuthorization {
        chain_id,
        address: addr_bytes,
        nonce,
        v: final_v,
        r: final_r,
        s,
    };
    Ok((authority_address, signed_auth))
}
```

- [ ] **Step 4: Update callers to handle Result**

Grep for callers:

Run: `cd middleware && grep -rn "create_phone_permit2_authorization" src/`

Expected hits: `functions/register.rs`, `functions/validate.rs`.

In each caller, unwrap to `String` error to match the existing `Result<USSDSession, String>` shape. Example for `functions/register.rs:confirm_register_pin`:

```rust
let (wallet_, _auth) = create_phone_permit2_authorization(
    &session.phone_number,
    84532, // Base Sepolia — was 0 (placeholder), fix to real chain id
    0,
    None,
    None,
).map_err(|e| format!("Burner wallet derivation failed: {}", e))?;
```

Same pattern in `functions/validate.rs:validate_pin` receiver-wallet creation.

**Note:** this also changes `chain_id` from `0` (placeholder "universal chain ID") to `84532` (Base Sepolia). The spec confirms Base Sepolia as the Phase 1 target chain.

- [ ] **Step 5: Export `AuthGenError` from the module**

Edit `middleware/src/auth/list/mod.rs`. Add:

```rust
pub use hashing::{AuthGenError, create_phone_permit2_authorization};
```

- [ ] **Step 6: Run tests — expect pass**

Run: `cd middleware && cargo test --lib auth::list::hashing::tests -- --nocapture`
Expected: 2 passed.

- [ ] **Step 7: Build — verify no caller compile errors**

Run: `cd middleware && cargo build --release`
Expected: builds clean.

- [ ] **Step 8: Commit**

```bash
git add middleware/src/auth/list/hashing.rs \
        middleware/src/auth/list/mod.rs \
        middleware/src/functions/register.rs \
        middleware/src/functions/validate.rs
git commit -m "fix(middleware): bound nick_auth_7702 attempts, return Result

Caps Nick's Method derivation at 1M attempts and returns Result<_, AuthGenError>
so callers handle failure explicitly instead of risking an infinite loop.

Also fixes the placeholder chain_id = 0 in register/validate to 84532 (Base Sepolia).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 3: Delete `PhoneWallet` table (superseded by `EsimProfile`)

**Files:**
- Modify: `middleware/src/auth/wallet.rs` (remove `PhoneWallet` struct and `phone_wallet` table)
- Modify: `middleware/src/functions/register.rs` (remove `PhoneWallet` import)
- Modify: `middleware/src/functions/validate.rs` (remove `PhoneWallet` import)

- [ ] **Step 1: Grep for references**

Run: `cd middleware && grep -rn "PhoneWallet\|phone_wallet" src/`

Document the call sites. Expected: imports only, no actual usage of the `phone_wallet` table.

- [ ] **Step 2: Remove table definition**

Edit `middleware/src/auth/wallet.rs`. New content:

```rust
use spacetimedb::{table, Timestamp};

#[table(name = esim_profile, public)]
pub struct EsimProfile {
    #[primary_key]
    #[unique]
    pub phone_number: String, //E.164
    pub wallet_address: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
```

- [ ] **Step 3: Remove imports in callers**

Edit `middleware/src/functions/register.rs`. Change:

```rust
use crate::auth::wallet::{esim_profile, PhoneWallet, EsimProfile};
```

to:

```rust
use crate::auth::wallet::{esim_profile, EsimProfile};
```

Same edit in `middleware/src/functions/validate.rs`.

- [ ] **Step 4: Build — verify clean**

Run: `cd middleware && cargo build --release`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add middleware/src/auth/wallet.rs \
        middleware/src/functions/register.rs \
        middleware/src/functions/validate.rs
git commit -m "chore(middleware): remove unused PhoneWallet table

Superseded by EsimProfile. No callers used phone_wallet.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 4: Migrate `eth_tx` schema (new PK, new fields) + `TxType` enum update

**Files:**
- Modify: `middleware/src/eth/tx/tables.rs`
- Modify: `middleware/src/eth/tx/types.rs`
- Modify: `middleware/src/eth/tx/params.rs`
- Modify: `middleware/src/eth/tx/mapping.rs`
- Modify: `middleware/src/eth/tx/encoding.rs`
- Modify: `middleware/src/functions/validate.rs` (update EthTx construction)
- Delete: `middleware/src/swap_tests.rs`
- Delete: `middleware/src/amount_validation_tests.rs` (relies on legacy EthTx shape — replace in Task 9)
- Delete: `middleware/src/pin_validation_tests.rs` (same reason)

- [ ] **Step 1: Rewrite `middleware/src/eth/tx/types.rs`**

```rust
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum TxType {
    SendEth,
    WithdrawEscrow,
    WithdrawRefund,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum TxStatus {
    Pending,     // just created, awaiting user confirm
    Submitted,   // user confirmed, broadcaster should pick up
    Broadcasting, // broadcaster claimed the row
    Broadcast,   // broadcaster submitted to RPC
    Confirmed,   // on-chain
    Failed,
    Cancelled,
}
```

- [ ] **Step 2: Rewrite `middleware/src/eth/tx/tables.rs`**

```rust
use spacetimedb::{table, Timestamp};
use super::types::{TxStatus, TxType};

#[table(name = eth_tx, public)]
pub struct EthTx {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    #[index(btree)]
    pub session_id: String,

    pub tx_type: TxType,

    pub from: String,
    pub to: String,
    pub value: String,
    pub data: Option<Vec<u8>>,
    pub gas_limit: String,

    pub status: TxStatus,
    pub tx_hash: Option<String>,
    pub block_number: Option<u64>,
    pub gas_used: Option<String>,
    pub error_reason: Option<String>,

    // Broadcaster lease (row-level double-submit guard)
    pub processing_by: Option<String>,
    pub processing_since: Option<Timestamp>,

    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
```

- [ ] **Step 3: Update `middleware/src/eth/tx/params.rs`**

Remove `TokenSwap`, `CashOut`, `Balance` variants. New content:

```rust
#[derive(Debug, Default, Clone)]
pub struct Params<'a> {
    pub to: Option<&'a str>,
    pub amount: Option<u128>,
}

#[derive(Debug, Clone)]
pub enum TxParams<'a> {
    SendEth { to: &'a str, amount: u128 },
    WithdrawEscrow { to: &'a str, amount: u128 },
    WithdrawRefund { to: &'a str, amount: u128 },
}
```

- [ ] **Step 4: Update `middleware/src/eth/tx/mapping.rs`**

```rust
use super::{types::TxType, params::TxParams};

impl<'a> TxParams<'a> {
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::SendEth { .. } => TxType::SendEth,
            Self::WithdrawEscrow { .. } => TxType::WithdrawEscrow,
            Self::WithdrawRefund { .. } => TxType::WithdrawRefund,
        }
    }
}

impl TxType {
    pub fn from_ussd_op(s: &str) -> Option<Self> {
        match s {
            "1" => Some(Self::SendEth),
            "3" => Some(Self::WithdrawEscrow),
            _ => None,
        }
    }
}
```

Note: op `"2"` (Swap) no longer maps — returns `None`. op `"4"` (Balance) is handled by a separate flow (future) and not through `TxType`.

- [ ] **Step 5: Update `middleware/src/eth/tx/encoding.rs`**

For `SendEth` we call `Burner7702.execute(to, value, "")` — the actual ERC-20 transfer-style selector is NOT used here; native ETH goes via `value` on the execute() call, and `data = 0x`. Rewrite to reflect that:

```rust
use super::{types::TxType, params::{TxParams, Params}};
use super::selector::keccak_selector;

fn uint256(v: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&v.to_be_bytes());
    buf
}

fn address(addr: &str) -> [u8; 32] {
    let clean = addr.strip_prefix("0x").unwrap_or(addr);
    let mut buf = [0u8; 32];
    if clean.len() >= 40 {
        if let Ok(bytes) = hex::decode(&clean[..40]) {
            buf[12..32].copy_from_slice(&bytes);
        }
    }
    buf
}

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.iter().flat_map(|&p| p.iter().copied()).collect()
}

pub fn to_hex(data: &[u8]) -> String {
    data.iter().fold(String::from("0x"), |mut s, b| {
        s.push_str(&format!("{:02x}", b));
        s
    })
}

impl TxType {
    /// ABI signature for token-style calls. Empty for native ETH transfers
    /// (SendEth, WithdrawEscrow, WithdrawRefund all move native ETH in Phase 1).
    pub const fn signature(&self) -> &'static str {
        match self {
            Self::SendEth => "",
            Self::WithdrawEscrow => "",
            Self::WithdrawRefund => "",
        }
    }

    pub fn selector(&self) -> Option<[u8; 4]> {
        let sig = self.signature();
        if sig.is_empty() {
            None
        } else {
            Some(keccak_selector(sig))
        }
    }

    pub fn to_tx<'a>(&self, params: Params<'a>) -> TxParams<'a> {
        let to = params.to.unwrap_or("");
        let amount = params.amount.unwrap_or(0);
        match self {
            Self::SendEth => TxParams::SendEth { to, amount },
            Self::WithdrawEscrow => TxParams::WithdrawEscrow { to, amount },
            Self::WithdrawRefund => TxParams::WithdrawRefund { to, amount },
        }
    }
}

impl<'a> TxParams<'a> {
    /// For native-ETH transfers via Burner7702.execute(to, value, data),
    /// calldata is empty — value travels as the execute() argument.
    /// The broadcaster (Plan 2) constructs the full execute() ABI call.
    pub fn encode(&self) -> Vec<u8> {
        Vec::new()
    }

    pub fn to_hex(&self) -> String {
        to_hex(&self.encode())
    }
}
```

- [ ] **Step 6: Update `validate_pin` in `functions/validate.rs` — EthTx row construction**

Locate the existing `ctx.db.eth_tx().insert(EthTx { ... })` block. Replace with:

```rust
ctx.db.eth_tx().insert(EthTx {
    id: 0, // auto-inc
    session_id: session.session_id.clone(),
    tx_type: TxType::SendEth,
    from: sender_wallet.wallet_address.clone(),
    to: receiver_wallet.clone(),
    value: amount.to_string(),
    data: None, // native ETH; value goes through execute()
    gas_limit: "100000".to_string(), // includes execute() overhead
    status: TxStatus::Pending,
    tx_hash: None,
    block_number: None,
    gas_used: None,
    error_reason: None,
    processing_by: None,
    processing_since: None,
    created_at: ctx.timestamp,
    updated_at: ctx.timestamp,
});
```

- [ ] **Step 7: Delete obsolete tests**

Run:

```bash
rm middleware/src/swap_tests.rs
rm middleware/src/amount_validation_tests.rs
rm middleware/src/pin_validation_tests.rs
```

Edit `middleware/src/lib.rs`. Remove these lines:

```rust
pub(crate) mod amount_validation_tests;
mod pin_validation_tests;
mod swap_tests;
```

These tests reference the old `EthTx` shape and `Swap*` enums that no longer exist. Phase 1B will reintroduce pure-logic tests for `UserIntent` parsing (Task 6) and the new `validate_pin` flow via `cargo test` once integration harness matures.

- [ ] **Step 8: Reset dev database and republish**

SpacetimeDB does not support live PK migrations. Run:

```bash
cd middleware
spacetime delete gateway2 || true   # ok if module doesn't exist yet
cargo build --release
spacetime publish --project-path . gateway2
```

Expected: "Published successfully".

- [ ] **Step 9: Commit**

```bash
git add middleware/src/eth/tx/ \
        middleware/src/functions/validate.rs \
        middleware/src/lib.rs
git rm middleware/src/swap_tests.rs \
       middleware/src/amount_validation_tests.rs \
       middleware/src/pin_validation_tests.rs
git commit -m "refactor(middleware): migrate eth_tx schema, simplify TxType

eth_tx now uses auto-inc id as PK (was session_id), supporting multiple
txs per session (Withdraw needs escrow + refund tied to one session).

Adds: tx_type, block_number, gas_used, error_reason, processing_by,
processing_since lease fields used by the broadcaster (Plan 2).

TxType reduced to SendEth | WithdrawEscrow | WithdrawRefund. All three
move native ETH via Burner7702.execute(to, value, 0x) — no ERC-20
selector needed until Swap ships.

Deletes obsolete tests referencing removed Swap* types.

BREAKING: requires spacetime delete + republish.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 5: Add new tables (withdrawal_request, sms_notification, balance_query, user_preferences)

**Files:**
- Create: `middleware/src/tables.rs` (new module housing these 4 tables + their enums)
- Modify: `middleware/src/lib.rs` (add `mod tables;`)

- [ ] **Step 1: Create `middleware/src/tables.rs`**

```rust
use spacetimedb::{table, SpacetimeType, Timestamp};

// ============================================================
// Withdrawal request (Pretium off-ramp)
// ============================================================

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum WithdrawalStatus {
    Pending,    // row created, awaiting escrow tx confirm
    Escrowed,   // escrow eth_tx Confirmed; pretium worker should pick up
    Processing, // pretium worker claimed the row
    Fulfilled,  // pretium completed
    Failed,     // pretium rejected (triggers refund)
    Refunded,   // refund eth_tx Confirmed
}

#[table(name = withdrawal_request, public)]
pub struct WithdrawalRequest {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub phone_number: String,
    pub fiat_amount: String,
    pub currency: String,
    pub escrow_eth_tx_id: u64,
    pub refund_eth_tx_id: Option<u64>,
    pub pretium_ref: Option<String>,
    pub status: WithdrawalStatus,
    pub error_reason: Option<String>,
    pub processing_by: Option<String>,
    pub processing_since: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================
// SMS notification
// ============================================================

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SmsTemplate {
    RegSuccess,
    TxSubmitted,
    TxConfirmed,
    TxFailed,
    InboundEth,
    WithdrawInit,
    WithdrawSent,
    WithdrawFailed,
    PinLocked,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SmsStatus {
    Pending,
    Processing,
    Sent,
    Failed,
}

#[table(name = sms_notification, public)]
pub struct SmsNotification {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub phone_number: String,
    pub template: SmsTemplate,
    pub payload_json: String,
    pub status: SmsStatus,
    pub message_id: Option<String>,
    pub error_reason: Option<String>,
    pub processing_by: Option<String>,
    pub processing_since: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================
// Balance query
// ============================================================

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum QueryStatus {
    Pending,
    Done,
    Failed,
}

#[table(name = balance_query, public)]
pub struct BalanceQuery {
    #[primary_key]
    pub session_id: String,
    pub phone_number: String,
    pub wallet_address: String,
    pub status: QueryStatus,
    pub result_wei: Option<String>,
    pub error_reason: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================
// User preferences
// ============================================================

#[table(name = user_preferences, public)]
pub struct UserPreferences {
    #[primary_key]
    pub phone_number: String,
    pub base_token: String,
    pub default_withdraw_method: Option<String>,
    pub updated_at: Timestamp,
}
```

- [ ] **Step 2: Wire the module into `lib.rs`**

Edit `middleware/src/lib.rs`. Near the other `mod` declarations, add:

```rust
mod tables;
pub use tables::*;
```

- [ ] **Step 3: Build and republish**

Run:

```bash
cd middleware
cargo build --release
spacetime delete gateway2 || true
spacetime publish --project-path . gateway2
```

Expected: success.

- [ ] **Step 4: Verify tables exist**

Run:

```bash
spacetime sql gateway2 "SELECT COUNT(*) FROM withdrawal_request"
spacetime sql gateway2 "SELECT COUNT(*) FROM sms_notification"
spacetime sql gateway2 "SELECT COUNT(*) FROM balance_query"
spacetime sql gateway2 "SELECT COUNT(*) FROM user_preferences"
```

Expected: each returns 0 rows.

- [ ] **Step 5: Commit**

```bash
git add middleware/src/tables.rs middleware/src/lib.rs
git commit -m "feat(middleware): add withdrawal_request, sms_notification, balance_query, user_preferences tables

These are consumed by Plan 2 (broadcaster) and later plans (SMS service,
balance worker, Pretium worker). Tables land first with no callers so
migrations can settle before we wire flows.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 6: Introduce `UserIntent` typed parser

**Files:**
- Create: `middleware/src/ussd/intent.rs`
- Modify: `middleware/src/ussd/mod.rs` (add `pub mod intent;`)

- [ ] **Step 1: Create `middleware/src/ussd/intent.rs` with failing tests inline**

```rust
/// Typed representation of the user's accumulated USSD input for the
/// current screen. Replaces ad-hoc `split('*')` indexing in every handler.
///
/// The USSD protocol accumulates input as `part0*part1*part2*...` where
/// part0 is always the top-level menu option. `parse_intent` maps the
/// (current_screen, raw_data) pair to a typed variant.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserIntent {
    // Registration flow
    /// RegisterPinScreen: user entered their PIN for the first time.
    /// data: "1*PIN"
    RegisterPin { pin: String },

    /// ConfirmRegisterPinScreen: user re-entered PIN.
    /// data: "1*PIN*CONFIRM"
    ConfirmRegisterPin { pin: String, confirm: String },

    // Send ETH flow
    /// ToNumberScreen: receiver phone.
    /// data: "1*PHONE"
    SendEthPhone { phone: String },

    /// ToAmountScreen: amount.
    /// data: "1*PHONE*AMOUNT"
    SendEthAmount { phone: String, amount: String },

    /// PINScreen (Send ETH): PIN to authorize.
    /// data: "1*PHONE*AMOUNT*PIN"
    SendEthPin { phone: String, amount: String, pin: String },

    /// CancelTXScreen: user picks 1 (confirm) or 2 (cancel).
    /// data: "1*PHONE*AMOUNT*PIN*CONFIRM"  where CONFIRM ∈ {"1","2"}
    SendEthConfirm { phone: String, amount: String, pin: String, confirm: ConfirmDecision },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmDecision {
    Confirm, // "1"
    Cancel,  // "2"
}

pub fn parse_intent(screen_name: &str, data: &str) -> Result<UserIntent, String> {
    let parts: Vec<&str> = data.split('*').collect();

    match screen_name {
        "RegisterPinScreen" => {
            if parts.len() != 2 { return Err("expected 1*PIN".to_string()); }
            Ok(UserIntent::RegisterPin { pin: parts[1].to_string() })
        }
        "ConfirmRegisterPinScreen" => {
            if parts.len() != 3 { return Err("expected 1*PIN*CONFIRM".to_string()); }
            Ok(UserIntent::ConfirmRegisterPin {
                pin: parts[1].to_string(),
                confirm: parts[2].to_string(),
            })
        }
        "ToNumberScreen" => {
            if parts.len() != 2 { return Err("expected 1*PHONE".to_string()); }
            Ok(UserIntent::SendEthPhone { phone: parts[1].to_string() })
        }
        "ToAmountScreen" => {
            if parts.len() != 3 { return Err("expected 1*PHONE*AMOUNT".to_string()); }
            Ok(UserIntent::SendEthAmount {
                phone: parts[1].to_string(),
                amount: parts[2].to_string(),
            })
        }
        "PINScreen" => {
            if parts.len() != 4 { return Err("expected 1*PHONE*AMOUNT*PIN".to_string()); }
            Ok(UserIntent::SendEthPin {
                phone: parts[1].to_string(),
                amount: parts[2].to_string(),
                pin: parts[3].to_string(),
            })
        }
        "CancelTXScreen" => {
            if parts.len() != 5 {
                return Err("expected 1*PHONE*AMOUNT*PIN*CONFIRM".to_string());
            }
            let confirm = match parts[4] {
                "1" => ConfirmDecision::Confirm,
                "2" => ConfirmDecision::Cancel,
                other => return Err(format!("invalid confirm value '{}'", other)),
            };
            Ok(UserIntent::SendEthConfirm {
                phone: parts[1].to_string(),
                amount: parts[2].to_string(),
                pin: parts[3].to_string(),
                confirm,
            })
        }
        other => Err(format!("no intent parser for screen '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_register_pin() {
        let got = parse_intent("RegisterPinScreen", "1*1234").unwrap();
        assert_eq!(got, UserIntent::RegisterPin { pin: "1234".to_string() });
    }

    #[test]
    fn parses_confirm_register_pin() {
        let got = parse_intent("ConfirmRegisterPinScreen", "1*1234*1234").unwrap();
        assert_eq!(got, UserIntent::ConfirmRegisterPin {
            pin: "1234".to_string(),
            confirm: "1234".to_string(),
        });
    }

    #[test]
    fn parses_send_eth_pin() {
        let got = parse_intent("PINScreen", "1*254712345678*0.5*9876").unwrap();
        assert_eq!(got, UserIntent::SendEthPin {
            phone: "254712345678".to_string(),
            amount: "0.5".to_string(),
            pin: "9876".to_string(),
        });
    }

    #[test]
    fn parses_send_eth_confirm_accept() {
        let got = parse_intent("CancelTXScreen", "1*254712345678*0.5*9876*1").unwrap();
        match got {
            UserIntent::SendEthConfirm { confirm: ConfirmDecision::Confirm, .. } => {}
            other => panic!("expected Confirm, got {:?}", other),
        }
    }

    #[test]
    fn parses_send_eth_confirm_cancel() {
        let got = parse_intent("CancelTXScreen", "1*254712345678*0.5*9876*2").unwrap();
        match got {
            UserIntent::SendEthConfirm { confirm: ConfirmDecision::Cancel, .. } => {}
            other => panic!("expected Cancel, got {:?}", other),
        }
    }

    #[test]
    fn rejects_wrong_part_count() {
        assert!(parse_intent("PINScreen", "1*254712345678*0.5").is_err());
    }

    #[test]
    fn rejects_unknown_screen() {
        assert!(parse_intent("NonexistentScreen", "1").is_err());
    }

    #[test]
    fn rejects_invalid_confirm_value() {
        assert!(parse_intent("CancelTXScreen", "1*254*0.5*1234*9").is_err());
    }
}
```

- [ ] **Step 2: Wire into ussd module**

Edit `middleware/src/ussd/mod.rs`. Add line:

```rust
pub mod intent;
```

- [ ] **Step 3: Run tests**

Run: `cd middleware && cargo test --lib ussd::intent::tests -- --nocapture`
Expected: 8 passed.

- [ ] **Step 4: Build — verify integrates**

Run: `cd middleware && cargo build --release`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add middleware/src/ussd/intent.rs middleware/src/ussd/mod.rs
git commit -m "feat(middleware): add UserIntent typed parser

Replaces scattered split('*') indexing in function handlers with a
single parse_intent(screen, data) returning a typed enum. Covered
by 8 unit tests (happy path + arity + unknown screen + invalid values).

Handlers migrate to consume UserIntent in Task 7-10.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 7: Replace `FUNCTION_MAP` with compile-time match dispatch

**Files:**
- Create: `middleware/src/functions/dispatch.rs`
- Modify: `middleware/src/functions/mod.rs`
- Modify: `middleware/src/ussd/service/runtime.rs`
- Delete: `middleware/src/ussd/service/utils.rs`
- Delete: `middleware/src/ussd/service/types.rs` (if it only holds FunctionMap/USSDFunction)
- Modify: `middleware/src/ussd/service/mod.rs`

- [ ] **Step 1: Create dispatch module**

Create `middleware/src/functions/dispatch.rs`:

```rust
use spacetimedb::ReducerContext;
use crate::ussd::session::USSDSession;

use super::register::{register_pin, confirm_register_pin};
use super::validate::{validate_phone_number, validate_amount, validate_pin, validate_token};
use super::cancel::cancel_tx;

/// Compile-time dispatch for USSD function screens.
/// Replaces the previous lazy_static Mutex<HashMap<String, fn>>.
pub fn dispatch(
    function_name: &str,
    ctx: &ReducerContext,
    session: USSDSession,
) -> Result<USSDSession, String> {
    match function_name {
        "register_pin"            => register_pin(ctx, session),
        "confirm_register_pin"    => confirm_register_pin(ctx, session),
        "validate_phone_number"   => validate_phone_number(ctx, session),
        "validate_amount"         => validate_amount(ctx, session),
        "validate_pin"            => validate_pin(ctx, session),
        "validate_token"          => validate_token(ctx, session),
        "cancel_tx"               => cancel_tx(ctx, session),
        other => Err(format!("unknown USSD function '{}'", other)),
    }
}
```

- [ ] **Step 2: Update `middleware/src/functions/mod.rs`**

Replace entire file contents with:

```rust
pub mod register;
pub mod pin;
pub mod validate;
pub mod utils;
pub mod cancel;
pub mod dispatch;

pub use register::{register_pin, confirm_register_pin};
pub use pin::{validate_pin_format, hash_pin};
pub use validate::{validate_phone_number, validate_amount, validate_pin, validate_token};
pub use utils::parse_input;
pub use cancel::cancel_tx;
pub use dispatch::dispatch;
```

- [ ] **Step 3: Rewrite `middleware/src/ussd/service/runtime.rs`**

Replace entire file with:

```rust
use spacetimedb::ReducerContext;
use super::tables::USSDService;
use crate::ussd::session::USSDSession;
use crate::functions::dispatch;

impl USSDService {
    /// Execute the function bound to this service row. Returns the
    /// updated session on success, or an error message to be shown
    /// as USSD error text.
    pub fn execute_fn(
        &self,
        ctx: &ReducerContext,
        session: USSDSession,
    ) -> Result<USSDSession, String> {
        dispatch(&self.function_name, ctx, session)
    }
}
```

- [ ] **Step 4: Update `middleware/src/ussd/screen/executor.rs` call site**

Locate `execute_function_screen` (around line 72). Replace its body:

```rust
fn execute_function_screen(&self, ctx: &ReducerContext, session: USSDSession, function_name: &str) -> Result<USSDSession, String> {
    let svc_opt = ctx.db.ussd_service().iter().find(|svc| {
        svc.function_name == function_name
    });

    if let Some(svc) = svc_opt {
        svc.execute_fn(ctx, session)
    } else {
        Err(format!("Function not found for screen '{}'", self.name))
    }
}
```

(The change: `load_function()(ctx, session)` → `svc.execute_fn(ctx, session)`.)

- [ ] **Step 5: Delete the old mutex machinery**

Run:

```bash
rm middleware/src/ussd/service/utils.rs
rm middleware/src/ussd/service/types.rs
```

Edit `middleware/src/ussd/service/mod.rs`. Remove:

```rust
pub mod utils;
pub mod types;
```

Leave `pub mod tables;`, `pub mod runtime;`.

- [ ] **Step 6: Remove `lazy_static` import chain**

Edit `middleware/src/lib.rs`. Remove the import `use crate::ussd::service::utils::FUNCTION_MAP;` (currently around line 18-19 of the file).

Remove `lazy_static` from `middleware/Cargo.toml` `[dependencies]`:

```toml
# delete this line:
lazy_static = "1.4.0"
```

Run: `cd middleware && cargo build --release`

Expected: builds clean. If there are lingering `lazy_static!` or `FUNCTION_MAP` references, grep and remove them:

```bash
grep -rn "FUNCTION_MAP\|lazy_static" middleware/src/
```

Expected: no hits.

- [ ] **Step 7: Republish and smoke test**

```bash
cd middleware
spacetime delete gateway2 || true
spacetime publish --project-path . gateway2
```

Expected: module publishes. `init` reducer runs without panics.

Verify init ran:

```bash
spacetime logs gateway2 | tail -20
```

Expected: see `USSDGETH Ininialized by, ...` log line (typo preserved from original code).

- [ ] **Step 8: Commit**

```bash
git add middleware/
git rm middleware/src/ussd/service/utils.rs \
       middleware/src/ussd/service/types.rs
git commit -m "refactor(middleware): replace FUNCTION_MAP mutex with compile-time dispatch

The lazy_static Mutex<HashMap<String, fn>> was cleared and rebuilt on
every USSD step — wasteful and race-prone. Replaced with a single
match statement in functions::dispatch::dispatch(fn_name, ctx, session).

Drops the lazy_static crate dependency.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 8: Migrate `register_pin` + `confirm_register_pin` to `UserIntent`

**Files:**
- Modify: `middleware/src/functions/register.rs`

- [ ] **Step 1: Rewrite `register_pin`**

Full new file contents:

```rust
use crate::functions::{validate_pin_format, hash_pin};
use crate::auth::list::{
    AuthGenError,
    hashing::create_phone_permit2_authorization,
    auth_7702, Auth7702, AuthStatus,
};
use crate::auth::pin::{user_pin, UserPIN};
use crate::auth::wallet::{esim_profile, EsimProfile};
use crate::ussd::intent::{parse_intent, UserIntent};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;
use spacetimedb::ReducerContext;

const BASE_SEPOLIA_CHAIN_ID: u64 = 84532;

pub fn register_pin(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("RegisterPinScreen", &session.data)?;
    let pin = match intent {
        UserIntent::RegisterPin { pin } => pin,
        _ => return Err("expected RegisterPin intent".to_string()),
    };
    validate_pin_format(&pin, false)?;
    Ok(session)
}

pub fn confirm_register_pin(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("ConfirmRegisterPinScreen", &session.data)?;
    let (pin, confirm) = match intent {
        UserIntent::ConfirmRegisterPin { pin, confirm } => (pin, confirm),
        _ => return Err("expected ConfirmRegisterPin intent".to_string()),
    };

    if pin != confirm {
        return Err("PIN does not match".to_string());
    }

    let ts = ctx.timestamp;
    let pin_hash = hash_pin(&pin, &session.phone_number, &ts.to_string());

    if ctx.db.esim_profile().phone_number().find(&session.phone_number).is_none() {
        let (wallet, auth) = create_phone_permit2_authorization(
            &session.phone_number,
            BASE_SEPOLIA_CHAIN_ID,
            0,
            None,
            None,
        ).map_err(|e: AuthGenError| format!("Burner derivation failed: {}", e))?;

        let wallet_address = hex::encode(wallet);

        ctx.db.esim_profile().insert(EsimProfile {
            phone_number: session.phone_number.clone(),
            wallet_address: wallet_address.clone(),
            created_at: ts,
            updated_at: ts,
        });

        ctx.db.auth_7702().insert(Auth7702 {
            authority_address: wallet_address,
            chain_id: auth.chain_id,
            delegate_to: hex::encode(auth.address),
            nonce: auth.nonce,
            v: auth.v,
            r: hex::encode(auth.r),
            s: hex::encode(auth.s),
            status: AuthStatus::Pending,
            created_at: ts,
            updated_at: ts,
        });
    }

    ctx.db.user_pin().insert(UserPIN {
        phone_number: session.phone_number.clone(),
        pin_hash,
        salt: ts.to_string(),
        attempts: 0,
        locked: false,
        last_attempt_time: None,
        lockout_until: None,
        created_at: ts,
        updated_at: ts,
    });

    session.data = "".to_string();
    Ok(session)
}
```

- [ ] **Step 2: Build**

Run: `cd middleware && cargo build --release`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add middleware/src/functions/register.rs
git commit -m "refactor(middleware): register handlers consume UserIntent

register_pin and confirm_register_pin now call parse_intent and pattern-
match on the typed variant instead of indexing session.data.split('*').

Uses BASE_SEPOLIA_CHAIN_ID = 84532 constant for 7702 derivation.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 9: Migrate validation handlers to `UserIntent`, fix `validate_pin` bug

**Files:**
- Modify: `middleware/src/functions/validate.rs`

- [ ] **Step 1: Rewrite `middleware/src/functions/validate.rs`**

```rust
use crate::functions::hash_pin;
use crate::auth::list::{
    AuthGenError,
    hashing::create_phone_permit2_authorization,
    auth_7702, Auth7702, AuthStatus,
};
use crate::auth::pin::{user_pin};
use crate::auth::wallet::{esim_profile, EsimProfile};
use crate::eth::tx::{eth_tx, EthTx, TxStatus, TxType};
use crate::ussd::intent::{parse_intent, UserIntent};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;
use spacetimedb::ReducerContext;

const BASE_SEPOLIA_CHAIN_ID: u64 = 84532;

fn is_valid_e164(phone: &str) -> bool {
    let digits = phone.strip_prefix('+').unwrap_or(phone);
    let len = digits.len();
    if !(8..=15).contains(&len) { return false; }
    let first = digits.as_bytes()[0];
    if !(b'1'..=b'9').contains(&first) { return false; }
    digits.chars().all(|c| c.is_ascii_digit())
}

pub fn validate_phone_number(_ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("ToNumberScreen", &session.data)?;
    let phone = match intent {
        UserIntent::SendEthPhone { phone } => phone,
        _ => return Err("expected SendEthPhone intent".to_string()),
    };
    if !is_valid_e164(&phone) {
        return Err("Invalid phone number format".to_string());
    }
    Ok(session)
}

pub fn validate_amount(_ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("ToAmountScreen", &session.data)?;
    let amount = match intent {
        UserIntent::SendEthAmount { amount, .. } => amount,
        _ => return Err("expected SendEthAmount intent".to_string()),
    };
    let parsed: f64 = amount.parse().map_err(|_| "Invalid amount format".to_string())?;
    if parsed <= 0.0 {
        return Err("Amount must be positive".to_string());
    }
    Ok(session)
}

pub fn validate_pin(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("PINScreen", &session.data)?;
    let (phone, amount, pin) = match intent {
        UserIntent::SendEthPin { phone, amount, pin } => (phone, amount, pin),
        _ => return Err("expected SendEthPin intent".to_string()),
    };

    let user_pin_row = ctx.db.user_pin().phone_number()
        .find(session.phone_number.clone())
        .ok_or_else(|| "User not registered".to_string())?;

    let computed = hash_pin(&pin, &session.phone_number, &user_pin_row.salt);
    if computed != user_pin_row.pin_hash {
        return Err("Invalid PIN".to_string());
    }

    // Look up or derive receiver wallet
    let receiver_wallet = if let Some(p) = ctx.db.esim_profile().phone_number().find(phone.clone()) {
        p.wallet_address
    } else {
        let (w, auth) = create_phone_permit2_authorization(
            &phone, BASE_SEPOLIA_CHAIN_ID, 0, None, None,
        ).map_err(|e: AuthGenError| format!("Receiver derivation failed: {}", e))?;
        let wallet_address = hex::encode(w);
        ctx.db.esim_profile().insert(EsimProfile {
            phone_number: phone.clone(),
            wallet_address: wallet_address.clone(),
            created_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
        ctx.db.auth_7702().insert(Auth7702 {
            authority_address: wallet_address.clone(),
            chain_id: auth.chain_id,
            delegate_to: hex::encode(auth.address),
            nonce: auth.nonce,
            v: auth.v,
            r: hex::encode(auth.r),
            s: hex::encode(auth.s),
            status: AuthStatus::Pending,
            created_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
        wallet_address
    };

    let sender = ctx.db.esim_profile().phone_number()
        .find(session.phone_number.clone())
        .ok_or_else(|| "Sender profile missing".to_string())?;

    ctx.db.eth_tx().insert(EthTx {
        id: 0,
        session_id: session.session_id.clone(),
        tx_type: TxType::SendEth,
        from: sender.wallet_address,
        to: receiver_wallet,
        value: amount.clone(),
        data: None,
        gas_limit: "100000".to_string(),
        status: TxStatus::Pending,
        tx_hash: None,
        block_number: None,
        gas_used: None,
        error_reason: None,
        processing_by: None,
        processing_since: None,
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
    });

    session.response_text = Some(format!(
        "Confirm TX:\nTo: {}\nAmount: {} ETH\n\n1. Confirm\n2. Cancel",
        phone, amount
    ));
    Ok(session)
}

pub fn validate_token(_ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    // Phase 1: token swaps out of scope; token screen leads to a ComingSoon screen.
    // Keep this as a no-op that always succeeds so menu navigation still works.
    Ok(session)
}
```

- [ ] **Step 2: Build**

Run: `cd middleware && cargo build --release`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add middleware/src/functions/validate.rs
git commit -m "refactor(middleware): validation handlers use UserIntent, fix phone_position bug

validate_pin/phone/amount/token all consume typed UserIntent variants
from parse_intent instead of indexing parts[N]. This fixes the
phone_position=3 bug that addressed the wrong index when the cumulative
input had only 2 segments.

validate_token becomes a no-op until Swap is in scope.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 10: Wire `cancel_tx` to update `eth_tx.status`

**Files:**
- Modify: `middleware/src/functions/cancel.rs`

- [ ] **Step 1: Rewrite `middleware/src/functions/cancel.rs`**

```rust
use crate::eth::tx::{eth_tx, EthTx, TxStatus};
use crate::ussd::intent::{parse_intent, UserIntent, ConfirmDecision};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;
use spacetimedb::ReducerContext;

pub fn cancel_tx(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("CancelTXScreen", &session.data)?;
    let confirm = match intent {
        UserIntent::SendEthConfirm { confirm, .. } => confirm,
        _ => return Err("expected SendEthConfirm intent".to_string()),
    };

    // Locate the pending eth_tx for this session
    let pending = ctx.db.eth_tx().iter()
        .filter(|t| t.session_id == session.session_id && matches!(t.status, TxStatus::Pending))
        .next()
        .ok_or_else(|| format!("No pending eth_tx for session {}", session.session_id))?;

    let new_status = match confirm {
        ConfirmDecision::Confirm => TxStatus::Submitted,
        ConfirmDecision::Cancel  => TxStatus::Cancelled,
    };

    ctx.db.eth_tx().id().update(EthTx {
        status: new_status.clone(),
        updated_at: ctx.timestamp,
        ..pending
    });

    log::info!(
        "cancel_tx: session {} → eth_tx.status = {:?}",
        session.session_id, new_status
    );

    Ok(session)
}
```

- [ ] **Step 2: Build**

Run: `cd middleware && cargo build --release`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add middleware/src/functions/cancel.rs
git commit -m "feat(middleware): cancel_tx updates eth_tx.status

ConfirmDecision::Confirm → TxStatus::Submitted (broadcaster picks up)
ConfirmDecision::Cancel  → TxStatus::Cancelled

Previously only logged.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 11: Add broadcaster-facing reducers

**Files:**
- Create: `middleware/src/reducers/broadcaster.rs`
- Modify: `middleware/src/reducers/mod.rs` (add `pub mod broadcaster;`)

- [ ] **Step 1: Create `middleware/src/reducers/broadcaster.rs`**

```rust
use crate::eth::tx::{eth_tx, EthTx, TxStatus};
use crate::auth::list::{auth_7702, Auth7702, AuthStatus};
use crate::AppConfig;
use spacetimedb::{reducer, ReducerContext, Identity, Table};

const GATEWAY_IDENTITY_KEY: &str = "gateway_identity";

fn require_gateway(ctx: &ReducerContext) -> Result<(), String> {
    let cfg = ctx.db.app_config().key().find(GATEWAY_IDENTITY_KEY.to_string())
        .ok_or_else(|| "gateway identity not configured".to_string())?;
    let expected = cfg.value;
    let sender = format!("{}", ctx.sender);
    if sender != expected {
        return Err(format!("unauthorized: {} != {}", sender, expected));
    }
    Ok(())
}

#[reducer]
pub fn set_gateway_identity(ctx: &ReducerContext, identity_str: String) {
    // Allow only module-owner sender — pattern from SpacetimeDB docs:
    // require ctx.sender == ctx.identity (module identity).
    if ctx.sender != ctx.identity() {
        log::error!("set_gateway_identity: unauthorized sender {}", ctx.sender);
        return;
    }
    if let Some(existing) = ctx.db.app_config().key().find(GATEWAY_IDENTITY_KEY.to_string()) {
        ctx.db.app_config().key().update(AppConfig {
            value: identity_str,
            ..existing
        });
    } else {
        ctx.db.app_config().insert(AppConfig {
            key: GATEWAY_IDENTITY_KEY.to_string(),
            value: identity_str,
        });
    }
    log::info!("gateway identity set");
}

#[reducer]
pub fn mark_eth_tx_processing(ctx: &ReducerContext, eth_tx_id: u64, worker_id: String) {
    if let Err(e) = require_gateway(ctx) {
        log::error!("mark_eth_tx_processing: {}", e); return;
    }
    let row = match ctx.db.eth_tx().id().find(eth_tx_id) {
        Some(r) => r,
        None => { log::error!("eth_tx {} not found", eth_tx_id); return; }
    };
    // Reject if already leased in last 5 min by another worker.
    if let (Some(_other), Some(since)) = (&row.processing_by, row.processing_since) {
        let elapsed = ctx.timestamp.saturating_duration_since(since);
        if elapsed < spacetimedb::TimeDuration::from_micros(5 * 60 * 1_000_000) {
            log::warn!("eth_tx {} already leased", eth_tx_id);
            return;
        }
    }
    ctx.db.eth_tx().id().update(EthTx {
        status: TxStatus::Broadcasting,
        processing_by: Some(worker_id),
        processing_since: Some(ctx.timestamp),
        updated_at: ctx.timestamp,
        ..row
    });
}

#[reducer]
pub fn mark_eth_tx_broadcast(ctx: &ReducerContext, eth_tx_id: u64, tx_hash: String) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.eth_tx().id().find(eth_tx_id) {
        ctx.db.eth_tx().id().update(EthTx {
            status: TxStatus::Broadcast,
            tx_hash: Some(tx_hash),
            updated_at: ctx.timestamp,
            ..row
        });
    }
}

#[reducer]
pub fn confirm_eth_tx(
    ctx: &ReducerContext,
    eth_tx_id: u64,
    tx_hash: String,
    block_number: u64,
    gas_used: String,
) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.eth_tx().id().find(eth_tx_id) {
        ctx.db.eth_tx().id().update(EthTx {
            status: TxStatus::Confirmed,
            tx_hash: Some(tx_hash),
            block_number: Some(block_number),
            gas_used: Some(gas_used),
            updated_at: ctx.timestamp,
            ..row
        });
    }
}

#[reducer]
pub fn fail_eth_tx(ctx: &ReducerContext, eth_tx_id: u64, tx_hash: Option<String>, reason: String) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.eth_tx().id().find(eth_tx_id) {
        ctx.db.eth_tx().id().update(EthTx {
            status: TxStatus::Failed,
            tx_hash,
            error_reason: Some(reason),
            updated_at: ctx.timestamp,
            ..row
        });
    }
}

#[reducer]
pub fn mark_auth7702_active(ctx: &ReducerContext, authority_address: String) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.auth_7702().authority_address().find(authority_address) {
        ctx.db.auth_7702().authority_address().update(Auth7702 {
            status: AuthStatus::Broadcasted,
            updated_at: ctx.timestamp,
            ..row
        });
    }
}
```

- [ ] **Step 2: Wire module**

Edit `middleware/src/reducers/mod.rs`. Add:

```rust
pub mod broadcaster;
```

- [ ] **Step 3: Build and republish**

```bash
cd middleware
cargo build --release
spacetime delete gateway2 || true
spacetime publish --project-path . gateway2
```

Expected: publish succeeds.

- [ ] **Step 4: Smoke-test a reducer call fails from non-gateway sender**

Run:

```bash
spacetime call gateway2 mark_eth_tx_processing 1 "test_worker"
spacetime logs gateway2 | tail -5
```

Expected: log line "mark_eth_tx_processing: gateway identity not configured" (since we haven't set it yet — which is correct: unauth sender gets rejected).

- [ ] **Step 5: Set gateway identity (module owner only)**

```bash
# Get the current cli identity
spacetime identity list
# Use it as gateway identity for now (test only)
MY_ID=$(spacetime identity list | tail -1 | awk '{print $1}')
spacetime call gateway2 set_gateway_identity "$MY_ID"
spacetime sql gateway2 "SELECT * FROM app_config WHERE key = 'gateway_identity'"
```

Expected: one row with `value = <your identity>`.

- [ ] **Step 6: Commit**

```bash
git add middleware/src/reducers/broadcaster.rs \
        middleware/src/reducers/mod.rs
git commit -m "feat(middleware): add broadcaster writeback reducers

- set_gateway_identity (module-owner only) registers the broadcaster's
  SpacetimeDB identity in app_config
- mark_eth_tx_processing (with 5-min row lease), mark_eth_tx_broadcast,
  confirm_eth_tx, fail_eth_tx handle the eth_tx state machine
- mark_auth7702_active flips auth_7702.status once the SetCode tx
  confirms on-chain

All reducers require ctx.sender == gateway_identity from app_config.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 12: Clean up `lib.rs` — remove commented Swap code, tighten imports

**Files:**
- Modify: `middleware/src/lib.rs`

- [ ] **Step 1: Strip commented `Swap*` code**

Open `middleware/src/lib.rs`. Remove every block that is entirely within `// ... //` comments referring to `Swap`, `SwapStatus`, `SwapType`. Use the following as a guide — search for each landmark comment and delete the block:

- The big commented block starting `// #[derive(Debug, Clone, PartialEq, Eq)]` and ending with `// }` after `pub swap_type: SwapType,`
- The `// #[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]` block for `pub enum SwapStatus`
- The `// #[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]` block for `pub enum SwapType`
- The commented `claim_swap` reducer stub inside `mod tests`
- The top-level `pub use spacetimedb::Table as SwapTable;` line
- The top-level `pub use spacetimedb::Table as USSDSessionTable;` line (unused)

Also remove these inline helpers that are only called once — inline them at the call site:

- `_check_profile_exists` → used only in `process_ussd_step` (one call)
- `_check_session_exists` → same

After inlining, the usage in `process_ussd_step` becomes:

```rust
let _profile = ctx.db.esim_profile().phone_number()
    .find(normalize_phone_number(&phone_number.clone()));
let _session = ctx.db.ussd_session().session_id().find(session_id.clone());
```

- [ ] **Step 2: Build**

Run: `cd middleware && cargo build --release`
Expected: clean.

- [ ] **Step 3: Verify grep shows no Swap or commented-block leftovers**

```bash
grep -n "Swap\|claim_swap" middleware/src/lib.rs
```

Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add middleware/src/lib.rs
git commit -m "chore(middleware): remove commented Swap code and unused helpers

Deletes commented-out Swap struct, SwapStatus/SwapType enums, claim_swap
reducer stub, and unused SwapTable/USSDSessionTable aliases.

Inlines _check_profile_exists and _check_session_exists (one call each).

Git history preserves the deleted code for reference.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 13: End-to-end middleware smoke test

**Files:**
- Create: `tests/smoke/test_register_pin_flow.sh`

- [ ] **Step 1: Create the smoke test script**

```bash
mkdir -p tests/smoke
```

Create `tests/smoke/test_register_pin_flow.sh`:

```bash
#!/usr/bin/env bash
# End-to-end smoke test for middleware Task 1-12 changes.
# Exercises: register_pin → confirm_register_pin → esim_profile + auth_7702 + user_pin rows.
# Requires: docker-compose up middleware; spacetime CLI available.
set -euo pipefail

DB="gateway2"
SESSION="smoke-$(date +%s)"
PHONE="+254712345678"
NETWORK="99999"
SERVICE="*384*6086#"

echo "=== 1. reset tables for fresh test ==="
spacetime sql "$DB" "DELETE FROM esim_profile WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "DELETE FROM user_pin     WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "DELETE FROM auth_7702    WHERE 1=1" || true  # best-effort
spacetime sql "$DB" "DELETE FROM ussd_session WHERE phone_number = '$PHONE'"

echo "=== 2. simulate dial (no text) — expect RegisterScreen ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" ""
RESP=$(spacetime sql "$DB" "SELECT response_text FROM ussd_response WHERE session_id = '$SESSION'" | tail -1)
echo "Response: $RESP"
echo "$RESP" | grep -q "Register" || { echo "FAIL: expected Register menu"; exit 1; }

echo "=== 3. pick '1. Register' ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1"

echo "=== 4. enter PIN '1379' ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*1379"

echo "=== 5. confirm PIN '1*1379*1379' ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*1379*1379"

echo "=== 6. verify rows created ==="
spacetime sql "$DB" "SELECT phone_number, wallet_address FROM esim_profile WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "SELECT phone_number, locked, attempts FROM user_pin WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "SELECT authority_address, status FROM auth_7702 ORDER BY created_at DESC LIMIT 1"

PROFILE_COUNT=$(spacetime sql "$DB" "SELECT COUNT(*) FROM esim_profile WHERE phone_number = '$PHONE'" | tail -1 | tr -d ' ')
[ "$PROFILE_COUNT" = "1" ] || { echo "FAIL: expected 1 esim_profile row, got $PROFILE_COUNT"; exit 1; }

PIN_COUNT=$(spacetime sql "$DB" "SELECT COUNT(*) FROM user_pin WHERE phone_number = '$PHONE'" | tail -1 | tr -d ' ')
[ "$PIN_COUNT" = "1" ] || { echo "FAIL: expected 1 user_pin row, got $PIN_COUNT"; exit 1; }

AUTH_COUNT=$(spacetime sql "$DB" "SELECT COUNT(*) FROM auth_7702" | tail -1 | tr -d ' ')
[ "$AUTH_COUNT" -ge "1" ] || { echo "FAIL: expected ≥1 auth_7702 row, got $AUTH_COUNT"; exit 1; }

echo "=== PASS ==="
```

Make executable:

```bash
chmod +x tests/smoke/test_register_pin_flow.sh
```

- [ ] **Step 2: Run the smoke test**

Prerequisite: middleware published (Task 11 step 3 already did this).

Run: `./tests/smoke/test_register_pin_flow.sh`
Expected: ends with `=== PASS ===`, shows one row in each of esim_profile, user_pin, auth_7702.

If any step fails, inspect `spacetime logs gateway2 | tail -30` for the failing reducer's error.

- [ ] **Step 3: Commit**

```bash
git add tests/smoke/test_register_pin_flow.sh
git commit -m "test(smoke): end-to-end register flow smoke test

Exercises process_ussd_step through the full registration path (initial
dial → pick Register → enter PIN → confirm PIN) and asserts that
esim_profile, user_pin, and auth_7702 rows are created.

Runs against a live SpacetimeDB instance — not part of cargo test.
Prereq: middleware published via 'spacetime publish'.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Plan self-review

After all 13 tasks complete, verify:

1. **Spec coverage** — every middleware cleanup item from the spec is addressed:
   - ✅ Real Keccak256 (Task 1)
   - ✅ `validate_pin` phone_position (Task 9)
   - ✅ `nick_auth_7702` bound (Task 2)
   - ✅ Phone normalization (deferred — ussdclient handles input normalization; middleware assumes E.164 via `is_valid_e164`)
   - ✅ `cancel_tx` wiring (Task 10)
   - ✅ `FUNCTION_MAP` removal (Task 7)
   - ✅ `UserIntent` typed parsing (Tasks 6, 8, 9, 10)
   - ✅ `PhoneWallet` deletion (Task 3)
   - ✅ Test cleanup (Task 4 deletes, Task 13 smoke replaces)
   - ✅ New tables (Task 5)
   - ✅ Broadcaster reducers (Task 11)

2. **Deferred from Phase 1A** (will land in Plan 2 or later):
   - ussdclient phone normalization at entry (Plan 3)
   - Menu.json update for SwapComingSoon screen (Plan 3)
   - FATF audit log calls on Send ETH (Phase 2)

3. **No placeholders** — every step has exact file paths, full code blocks, and exact commands.

4. **Type consistency** — `UserIntent` variant names match across Tasks 6, 8, 9, 10; `TxType` variants match across Tasks 4, 9, 10, 11.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-17-middleware-foundation.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
