pub mod register;
pub mod pin;
pub mod validate;
pub mod utils;
pub mod cancel;

use std::collections::HashMap;
pub use register::*;
pub use pin::{validate_pin_format, hash_pin};
pub use validate::{validate_phone_number, validate_amount, validate_pin, validate_token};
pub use utils::parse_input;
pub use cancel::cancel_tx;

use crate::ussd::service::types::{FunctionMap, USSDFunction};
use crate::ussd::service::utils::FUNCTION_MAP;

pub fn register_functions() {
    let mut function_map = crate::ussd::service::utils::FUNCTION_MAP.lock().unwrap();
    
    //Clear exisiting functions to avoid duplicates
    function_map.clear();
    
    //Re-register functions
    function_map.insert("register_pin".to_string(), register_pin as USSDFunction);
    function_map.insert("confirm_register_pin".to_string(), confirm_register_pin as USSDFunction);
    function_map.insert("validate_phone_number".to_string(), validate_phone_number as USSDFunction);
    function_map.insert("validate_amount".to_string(), validate_amount as USSDFunction);
    function_map.insert("validate_pin".to_string(), validate_pin as USSDFunction);
    function_map.insert("cancel_tx".to_string(), cancel_tx as USSDFunction);
    function_map.insert("validate_token".to_string(), validate_token as USSDFunction);
}
    