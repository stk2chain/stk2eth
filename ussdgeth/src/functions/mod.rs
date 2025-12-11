pub mod register;
pub mod pin;
pub mod validate;
pub mod utils;

use std::collections::HashMap;
pub use register::*;
pub use pin::{validate_pin_format, hash_pin};
pub use validate::{validate_phone_number, validate_amount, validate_pin};
pub use utils::parse_input;

use crate::ussdframework::types::{FunctionMap, USSDFunction};
use crate::ussdframework::utils::FUNCTION_MAP;

pub fn register_functions() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        let mut function_map = crate::ussdframework::utils::FUNCTION_MAP.lock().unwrap();
        
        function_map.insert("register_pin".to_string(), register_pin as USSDFunction);
        function_map.insert("confirm_register_pin".to_string(), confirm_register_pin as USSDFunction);
        function_map.insert("validate_phone_number".to_string(), validate_phone_number as USSDFunction);
        function_map.insert("validate_amount".to_string(), validate_amount as USSDFunction);
        function_map.insert("validate_pin".to_string(), validate_pin as USSDFunction);
    });
    
}
    