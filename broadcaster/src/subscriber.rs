// broadcaster/src/subscriber.rs
//
// Connects to SpacetimeDB and subscribes to eth_tx rows in Pending status.
// Pushes each row onto an unbounded channel consumed by the submitter task.

use crate::error::BroadcasterError;
use crate::stdb::{DbConnection, EthTxTableAccess, TxStatus};
use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey};
use tokio::sync::mpsc;

/// A lightweight view of an `eth_tx` row for the submitter.
/// Field names follow our semantic conventions; source fields on the generated
/// `EthTx` struct are `from`, `to`, and `value`.
#[derive(Debug, Clone)]
pub struct PendingRow {
    pub id: u64,
    pub from_address: String,
    pub to_address: String,
    pub amount_wei: String,
    pub tx_type: String,
}

pub struct Subscriber {
    pub rx: mpsc::UnboundedReceiver<PendingRow>,
    _conn: DbConnection,
    _driver: std::thread::JoinHandle<()>,
}

impl Subscriber {
    pub async fn connect(
        host: &str,
        db_name: &str,
        auth_token: &str,
    ) -> Result<Self, BroadcasterError> {
        let (tx, rx) = mpsc::unbounded_channel();

        let conn = DbConnection::builder()
            .with_uri(host)
            .with_module_name(db_name)
            .with_token(Some(auth_token.to_string()))
            .on_connect(|_ctx, identity, _token| {
                tracing::info!(identity = ?identity, "connected to SpacetimeDB");
            })
            .on_connect_error(|_ctx, err| {
                tracing::error!(?err, "SpacetimeDB connect error");
            })
            .build()
            .map_err(|e| BroadcasterError::Config(format!("STDB connect: {e}")))?;

        let tx_on_insert = tx.clone();
        conn.db.eth_tx().on_insert(move |_ctx, row| {
            if matches!(row.status, TxStatus::Pending) {
                let pending = PendingRow {
                    id: row.id,
                    from_address: row.from.clone(),
                    to_address: row.to.clone(),
                    amount_wei: row.value.clone(),
                    tx_type: format!("{:?}", row.tx_type),
                };
                let _ = tx_on_insert.send(pending);
            }
        });

        let tx_on_update = tx.clone();
        conn.db.eth_tx().on_update(move |_ctx, _old, new| {
            if matches!(new.status, TxStatus::Pending) {
                let pending = PendingRow {
                    id: new.id,
                    from_address: new.from.clone(),
                    to_address: new.to.clone(),
                    amount_wei: new.value.clone(),
                    tx_type: format!("{:?}", new.tx_type),
                };
                let _ = tx_on_update.send(pending);
            }
        });

        conn.subscription_builder()
            .on_applied(|_ctx| tracing::info!("eth_tx Pending subscription applied"))
            .subscribe(vec!["SELECT * FROM eth_tx WHERE status = 'Pending'".to_string()]);

        let driver = conn.run_threaded();

        Ok(Subscriber {
            rx,
            _conn: conn,
            _driver: driver,
        })
    }
}
