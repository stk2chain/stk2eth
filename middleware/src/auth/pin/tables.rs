use spacetimedb::{table, Timestamp};

#[table(name = user_pin)]
pub struct UserPIN {
    #[primary_key]
    #[unique]
    pub phone_number: String,
    pub pin_hash: String,
    pub salt: String,
    pub attempts: u32,
    pub locked: bool,
    pub last_attempt_time: Option<Timestamp>,
    pub lockout_until: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

