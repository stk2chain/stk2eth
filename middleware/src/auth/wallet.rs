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