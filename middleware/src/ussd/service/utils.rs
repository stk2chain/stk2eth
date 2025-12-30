use super::types::{FunctionMap, USSDFunction};
// use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};



lazy_static::lazy_static! {
    // Initialized on first use, not at program start
    // Define a lazy static variable to store registered functions
    // pub static ref FUNCTION_MAP: Arc<Mutex<FunctionMap>> = Arc::new(Mutex::new(FunctionMap::new()));
    pub static ref FUNCTION_MAP: Arc<Mutex<FunctionMap>> = {
        let map = FunctionMap::new();
        Arc::new(Mutex::new(map))
    };
    // pub static ref REGISTERED_FUNCTIONS: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

}

