use spacetimedb::{table, Timestamp};
use super::types::{TxStatus, TxType};

#[table(name = eth_tx, public)]
pub struct EthTx {
    #[primary_key]
    #[unique]
    pub session_id: String,
    pub from: String, 
    pub to: String,
    pub value: String,
    pub data: Option<Vec<u8>>,
    pub gas_limit: String,

    pub status: TxStatus,
        
    pub tx_hash: Option<String>,
    
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}




