use spacetimedb::{table, Timestamp, Identity};

#[derive(Debug, Clone)]
#[table(name = ussd_session)]
pub struct USSDSession {
    #[primary_key]
    pub session_id: String,
    pub phone_number: String,
    pub network_code: String,
    pub service_code: String,
    pub data: String,

    pub current_screen: String,
    pub response_text: Option<String>,
    pub error_text: Option<String>,
    
    pub last_interaction_time: Timestamp,

    #[index(btree)]
    pub client: Identity, //USSD Client identity
}