use std::collections::HashMap;

/// Function signature for USSD functions
/// The function signature is a function that takes USSD user_input string as an argument
///
/// # Arguments
///
/// * `user_input` - The user input
///
/// # Returns
///
/// A Result value
///
/// # Example
///
/// ```
/// use ussdframework::prelude::*;
///
/// fn buy_airtime(user_input: &str) -> Result<(), String> {
///    Ok(())
/// }
pub type USSDFunction = fn(&str) -> Result<(), String>;

/// Key-value map of USSD functions
pub type FunctionMap = HashMap<String, USSDFunction>;

