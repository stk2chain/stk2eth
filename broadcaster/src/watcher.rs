// broadcaster/src/watcher.rs
//
// Subscribes to Base Sepolia newHeads and polls receipts for tracked tx hashes.
// On receipt match, calls confirm_eth_tx or fail_eth_tx + mark_auth_7702_active.

use crate::error::BroadcasterError;
use crate::stdb::{confirm_eth_tx, fail_eth_tx, mark_auth_7702_active, DbConnection};
use crate::submitter::WatcherMsg;
use alloy::primitives::{Address, B256};
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use futures::StreamExt;
use spacetimedb_sdk::DbContext;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct Watcher {
    pub ws_url: String,
    pub stdb: Arc<DbConnection>,
    pub rx: mpsc::UnboundedReceiver<WatcherMsg>,
}

struct Tracked {
    id: u64,
    burner: Address,
    needs_auth: bool,
}

impl Watcher {
    pub async fn run(mut self) -> Result<(), BroadcasterError> {
        let provider = ProviderBuilder::new()
            .on_ws(WsConnect::new(&self.ws_url))
            .await
            .map_err(|e| BroadcasterError::RpcTransient(format!("ws connect: {e}")))?;

        let mut stream = provider
            .subscribe_blocks()
            .await
            .map_err(|e| BroadcasterError::RpcTransient(format!("subscribe_blocks: {e}")))?
            .into_stream();

        let mut tracked: HashMap<B256, Tracked> = HashMap::new();

        loop {
            tokio::select! {
                Some(msg) = self.rx.recv() => {
                    tracked.insert(msg.tx_hash, Tracked {
                        id: msg.eth_tx_id,
                        burner: msg.burner,
                        needs_auth: msg.needs_auth,
                    });
                }
                Some(_block) = stream.next() => {
                    let snapshot: Vec<(B256, Tracked)> = tracked.drain().collect();
                    for (hash, t) in snapshot {
                        match provider.get_transaction_receipt(hash).await {
                            Ok(Some(receipt)) => {
                                if receipt.status() {
                                    let _ = self.stdb.reducers.confirm_eth_tx(
                                        t.id,
                                        format!("{hash:#x}"),
                                        receipt.block_number.unwrap_or(0),
                                        receipt.gas_used.to_string(),
                                    );
                                    if t.needs_auth {
                                        let _ = self.stdb.reducers.mark_auth_7702_active(
                                            format!("{:x}", t.burner),
                                        );
                                    }
                                } else {
                                    let _ = self.stdb.reducers.fail_eth_tx(
                                        t.id,
                                        Some(format!("{hash:#x}")),
                                        "execute reverted".into(),
                                    );
                                }
                            }
                            Ok(None) => {
                                tracked.insert(hash, t);
                            }
                            Err(e) => {
                                tracing::warn!(?e, ?hash, "receipt fetch error; will retry next block");
                                tracked.insert(hash, t);
                            }
                        }
                    }
                }
                else => { break; }
            }
        }
        Ok(())
    }
}
