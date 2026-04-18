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

    pub processing_by: Option<String>,
    pub processing_since: Option<Timestamp>,

    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
