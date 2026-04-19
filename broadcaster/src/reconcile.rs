// broadcaster/src/reconcile.rs
//
// Startup recovery. Runs once before submitter/watcher loops.
//  - Broadcasting rows: retry handle(); on NonceAlreadyUsed, walk recent blocks
//    for the operator's tx and recover the hash.
//  - Broadcast rows: put back onto the watcher track-list.

use crate::error::BroadcasterError;
use crate::stdb::{
    fail_eth_tx, mark_eth_tx_broadcast, Auth7702TableAccess, AuthStatus, EthTxTableAccess, TxStatus,
};
use crate::submitter::{Submitter, WatcherMsg};
use alloy::consensus::Transaction as _;
use alloy::network::TransactionResponse as _;
use alloy::primitives::{Address, B256};
use alloy::providers::Provider;
use alloy::rpc::types::eth::BlockNumberOrTag;
use spacetimedb_sdk::Table;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn reconcile(
    submitter: &Submitter,
    watcher_tx: &mpsc::UnboundedSender<WatcherMsg>,
    scan_blocks: u64,
) -> Result<(), BroadcasterError> {
    let budget = Duration::from_secs(60);
    let start = std::time::Instant::now();

    let broadcasting: Vec<_> = submitter
        .stdb
        .db
        .eth_tx()
        .iter()
        .filter(|r| matches!(r.status, TxStatus::Broadcasting))
        .collect();

    for row in broadcasting {
        if start.elapsed() > budget {
            tracing::error!("reconcile: budget exceeded, aborting");
            break;
        }
        let burner = parse_burner(&row.from)?;
        let pending = crate::subscriber::PendingRow {
            id: row.id,
            from_address: row.from.clone(),
            to_address: row.to.clone(),
            amount_wei: row.value.clone(),
            tx_type: format!("{:?}", row.tx_type),
        };
        match submitter.handle(pending).await {
            Ok(_) => {}
            Err(BroadcasterError::NonceAlreadyUsed) => {
                match recover_broadcast_hash(submitter, scan_blocks, burner).await? {
                    Some(hash) => {
                        let _ = submitter
                            .stdb
                            .reducers
                            .mark_eth_tx_broadcast(row.id, format!("{hash:#x}"));
                        let needs_auth = auth_needs_attached(submitter, burner)?;
                        let _ = watcher_tx.send(WatcherMsg {
                            eth_tx_id: row.id,
                            tx_hash: hash,
                            burner,
                            needs_auth,
                        });
                    }
                    None => {
                        let _ = submitter.stdb.reducers.fail_eth_tx(
                            row.id,
                            None,
                            "reconcile: no hash recovered".into(),
                        );
                    }
                }
            }
            Err(e) => {
                let _ = submitter
                    .stdb
                    .reducers
                    .fail_eth_tx(row.id, None, format!("reconcile: {e}"));
            }
        }
    }

    let broadcast: Vec<_> = submitter
        .stdb
        .db
        .eth_tx()
        .iter()
        .filter(|r| matches!(r.status, TxStatus::Broadcast))
        .collect();

    for row in broadcast {
        let Some(tx_hash_str) = &row.tx_hash else { continue };
        let Ok(hash) = B256::from_str(tx_hash_str.trim_start_matches("0x").trim_start_matches("0X"))
        else {
            continue;
        };
        let burner = parse_burner(&row.from)?;
        let needs_auth = auth_needs_attached(submitter, burner)?;
        let _ = watcher_tx.send(WatcherMsg {
            eth_tx_id: row.id,
            tx_hash: hash,
            burner,
            needs_auth,
        });
    }

    let pending_nonce = submitter
        .provider
        .get_transaction_count(submitter.operator.address())
        .pending()
        .await
        .map_err(|e| BroadcasterError::RpcTransient(format!("get_tx_count: {e}")))?;
    submitter
        .operator_nonce
        .store(pending_nonce, std::sync::atomic::Ordering::SeqCst);
    tracing::info!(nonce = pending_nonce, "operator nonce rebootstrapped");

    Ok(())
}

fn parse_burner(raw: &str) -> Result<Address, BroadcasterError> {
    Address::from_str(&format!("0x{}", raw.trim_start_matches("0x")))
        .map_err(|e| BroadcasterError::DerivationFailed(format!("burner parse: {e}")))
}

fn auth_needs_attached(submitter: &Submitter, burner: Address) -> Result<bool, BroadcasterError> {
    let addr_hex = format!("{:x}", burner);
    let row = submitter
        .stdb
        .db
        .auth_7702()
        .authority_address()
        .find(&addr_hex);
    Ok(match row {
        Some(r) => !matches!(r.status, AuthStatus::Broadcasted),
        None => true,
    })
}

async fn recover_broadcast_hash(
    submitter: &Submitter,
    scan_blocks: u64,
    burner: Address,
) -> Result<Option<B256>, BroadcasterError> {
    let latest = submitter
        .provider
        .get_block_number()
        .await
        .map_err(|e| BroadcasterError::RpcTransient(format!("get_block_number: {e}")))?;
    let op = submitter.operator.address();
    for n in (latest.saturating_sub(scan_blocks)..=latest).rev() {
        let block = submitter
            .provider
            .get_block_by_number(BlockNumberOrTag::Number(n), true.into())
            .await
            .map_err(|e| BroadcasterError::RpcTransient(format!("get_block: {e}")))?;
        let Some(block) = block else { continue };
        for tx in block.transactions.txns() {
            if tx.from == op && tx.to() == Some(burner) {
                return Ok(Some(tx.tx_hash()));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_needs_attached_signature_stable() {
        let _: fn(&Submitter, Address) -> Result<bool, BroadcasterError> = auth_needs_attached;
    }

    #[test]
    fn parse_burner_accepts_raw_and_prefixed() {
        let a1 = parse_burner("4089a1e74f202d1c28dfc341078a615fd02f91fa").unwrap();
        let a2 = parse_burner("0x4089a1e74f202d1c28dfc341078a615fd02f91fa").unwrap();
        assert_eq!(a1, a2);
    }
}
