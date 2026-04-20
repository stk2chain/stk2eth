// broadcaster/src/submitter.rs
//
// Builds, signs, and submits EIP-7702 type-4 transactions for each Pending
// eth_tx row, then calls writeback reducers as the state machine progresses.

use crate::abi::encode_execute;
use crate::error::BroadcasterError;
use crate::stdb::{
    mark_auth_7702_active, mark_eth_tx_broadcast, mark_eth_tx_processing, Auth7702TableAccess,
    DbConnection,
};
use crate::subscriber::PendingRow;
use alloy::consensus::{SignableTransaction, TxEip7702, TxEnvelope};
use alloy::eips::eip2718::Encodable2718;
use alloy::eips::eip7702::{Authorization, SignedAuthorization};
use alloy::network::{Ethereum, TxSignerSync};
use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::providers::{Provider, RootProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::http::{reqwest::Client, Http};
use spacetimedb_sdk::DbContext;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub const WORKER_ID: &str = "broadcaster-0";

pub type HttpProvider = RootProvider<Http<Client>, Ethereum>;

pub struct Submitter {
    pub provider: HttpProvider,
    pub operator: PrivateKeySigner,
    pub stdb: Arc<DbConnection>,
    pub chain_id: u64,
    pub burner7702_address: Address,
    pub balance_reserve: U256,
    pub operator_nonce: Arc<AtomicU64>,
    pub watcher_tx: tokio::sync::mpsc::UnboundedSender<WatcherMsg>,
}

#[derive(Debug, Clone)]
pub struct WatcherMsg {
    pub eth_tx_id: u64,
    pub tx_hash: B256,
    pub burner: Address,
    pub needs_auth: bool,
}

impl Submitter {
    pub async fn handle(&self, row: PendingRow) -> Result<(), BroadcasterError> {
        // 1. Acquire lease.
        self.stdb
            .reducers
            .mark_eth_tx_processing(row.id, WORKER_ID.to_string())
            .map_err(|e| BroadcasterError::ReducerRejected(format!("mark_processing: {e}")))?;

        // 2. Parse burner address (stored as lowercase hex without 0x).
        let burner = Address::from_str(&format!("0x{}", row.from_address))
            .map_err(|e| BroadcasterError::DerivationFailed(format!("burner parse: {e}")))?;

        // 3. Balance precheck.
        let amount = U256::from_str(&row.amount_wei)
            .map_err(|e| BroadcasterError::Config(format!("amount parse: {e}")))?;
        let balance = self
            .provider
            .get_balance(burner)
            .await
            .map_err(|e| BroadcasterError::RpcTransient(format!("get_balance: {e}")))?;
        let need = amount + self.balance_reserve;
        if balance < need {
            return Err(BroadcasterError::InsufficientBalance { have: balance, need });
        }

        // 4. needs_auth = !is_already_delegated.
        let code = self
            .provider
            .get_code_at(burner)
            .await
            .map_err(|e| BroadcasterError::RpcTransient(format!("get_code: {e}")))?;
        let expected_delegate_prefix: [u8; 3] = [0xef, 0x01, 0x00];
        let already_delegated = code.len() >= 23
            && code[..3] == expected_delegate_prefix
            && &code[3..23] == self.burner7702_address.as_slice();
        let needs_auth = !already_delegated;

        // 5. Fetch SignedAuthorization from auth_7702 table.
        let auth = if needs_auth {
            Some(self.lookup_signed_authorization(burner)?)
        } else {
            None
        };

        // 6. Build calldata: Burner7702.execute(to, value, 0x).
        let to = Address::from_str(&format!("0x{}", row.to_address))
            .map_err(|_| BroadcasterError::InvalidRecipient(row.to_address.clone()))?;
        let calldata: Bytes = encode_execute(to, amount, Bytes::new());

        // 7. Build type-4 tx.
        let nonce = self.operator_nonce.fetch_add(1, Ordering::SeqCst);
        let tx = TxEip7702 {
            chain_id: self.chain_id,
            nonce,
            gas_limit: 300_000,
            max_fee_per_gas: 2_000_000_000,
            max_priority_fee_per_gas: 1_000_000_000,
            to: burner,
            value: U256::ZERO,
            access_list: Default::default(),
            authorization_list: auth.map(|a| vec![a]).unwrap_or_default(),
            input: calldata,
        };

        // 8. Sign.
        let mut tx_signable = tx;
        let signature = self
            .operator
            .sign_transaction_sync(&mut tx_signable)
            .map_err(|e| BroadcasterError::Config(format!("sign: {e}")))?;
        let signed_tx = tx_signable.into_signed(signature);
        let envelope: TxEnvelope = signed_tx.into();

        // 9. Submit.
        let raw = envelope.encoded_2718();
        let pending = self
            .provider
            .send_raw_transaction(&raw)
            .await
            .map_err(classify_rpc_error)?;
        let hash = *pending.tx_hash();

        // 10. Writeback Broadcast.
        self.stdb
            .reducers
            .mark_eth_tx_broadcast(row.id, format!("{hash:#x}"))
            .map_err(|e| BroadcasterError::ReducerRejected(format!("mark_broadcast: {e}")))?;

        // 11. Enqueue watcher.
        let _ = self.watcher_tx.send(WatcherMsg {
            eth_tx_id: row.id,
            tx_hash: hash,
            burner,
            needs_auth,
        });

        Ok(())
    }

    fn lookup_signed_authorization(
        &self,
        burner: Address,
    ) -> Result<SignedAuthorization, BroadcasterError> {
        let addr_hex = format!("{:x}", burner);
        let row = self
            .stdb
            .db
            .auth_7702()
            .authority_address()
            .find(&addr_hex)
            .ok_or_else(|| {
                BroadcasterError::DerivationFailed(format!("no auth_7702 row for {addr_hex}"))
            })?;

        let delegate = Address::from_str(&format!("0x{}", row.delegate_to))
            .map_err(|e| BroadcasterError::Config(format!("delegate addr: {e}")))?;

        let r_bytes = hex::decode(row.r.trim_start_matches("0x"))
            .map_err(|e| BroadcasterError::Config(format!("r hex: {e}")))?;
        let s_bytes = hex::decode(row.s.trim_start_matches("0x"))
            .map_err(|e| BroadcasterError::Config(format!("s hex: {e}")))?;
        let mut r_arr = [0u8; 32];
        let mut s_arr = [0u8; 32];
        r_arr.copy_from_slice(&r_bytes);
        s_arr.copy_from_slice(&s_bytes);

        let auth = Authorization {
            chain_id: row.chain_id,
            address: delegate,
            nonce: row.nonce,
        };
        // EIP-7702 stores y_parity directly. Middleware may store legacy v (27/28)
        // or raw parity (0/1) — normalise both.
        let y_parity: u8 = match row.v {
            0 | 27 => 0,
            1 | 28 => 1,
            other => other,
        };
        Ok(SignedAuthorization::new_unchecked(
            auth,
            y_parity,
            U256::from_be_bytes(r_arr),
            U256::from_be_bytes(s_arr),
        ))
    }
}

fn classify_rpc_error(e: impl std::fmt::Display) -> BroadcasterError {
    let s = e.to_string();
    if s.contains("nonce too low") || s.contains("already known") {
        BroadcasterError::NonceAlreadyUsed
    } else if s.contains("rate limit") || s.contains("429") {
        BroadcasterError::RateLimited
    } else {
        BroadcasterError::RpcTransient(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_rpc_error_routes_nonce() {
        assert!(matches!(
            classify_rpc_error("nonce too low"),
            BroadcasterError::NonceAlreadyUsed
        ));
        assert!(matches!(
            classify_rpc_error("already known"),
            BroadcasterError::NonceAlreadyUsed
        ));
    }

    #[test]
    fn classify_rpc_error_routes_rate_limit() {
        assert!(matches!(
            classify_rpc_error("HTTP 429"),
            BroadcasterError::RateLimited
        ));
        assert!(matches!(
            classify_rpc_error("rate limit exceeded"),
            BroadcasterError::RateLimited
        ));
    }

    #[test]
    fn classify_rpc_error_defaults_to_transient() {
        assert!(matches!(
            classify_rpc_error("connection reset"),
            BroadcasterError::RpcTransient(_)
        ));
    }
}
