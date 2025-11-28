pub mod register;

use std::collections::HashMap;
pub use register::*;

use crate::ussdframework::types::{FunctionMap, USSDFunction};
use crate::ussdframework::utils::FUNCTION_MAP;

pub fn register_functions() {
    let mut function_map = crate::ussdframework::utils::FUNCTION_MAP.lock().unwrap();
    
    function_map.insert("register_pin".to_string(), register_pin as USSDFunction);
    function_map.insert("confirm_register_pin".to_string(), confirm_register_pin as USSDFunction);
    
}
    