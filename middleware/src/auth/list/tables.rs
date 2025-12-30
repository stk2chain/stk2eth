use spacetimedb::{table, Timestamp};
use super::types::AuthStatus;


#[table(name = auth_7702)]
pub struct Auth7702 {
    #[primary_key]
    #[unique]
    pub authority_address: String,

    pub chain_id: u64,
    pub delegate_to: String,
    pub nonce: u64,
    pub v: u8,
    pub r: String,
    pub s: String, //phone_salt

    
    pub status: AuthStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}