use spacetimedb::table;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[table(name = ussd_service)]
pub struct USSDService {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub ussd_code: u64,
    pub function_name: String,
}