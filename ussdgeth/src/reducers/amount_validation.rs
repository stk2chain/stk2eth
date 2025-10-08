use spacetimedb::ReducerContext;

use crate::app_config;
use crate::AmountValidationResult;
use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;

/// Validates that the amount is a valid number and meets the minimum threshold
/// # Arguments
/// * `amount` - The amount string to validate
/// * `min_amount` - The minimum allowable amount
/// # Returns
/// * `AmountValidationResult` - Enum indicating if the amount is valid, too low, or invalid
/// # Examples
/// ```
/// let result = validate_amount_value("0.0005", 0.00025);
/// assert_eq!(result, AmountValidationResult::Valid);
/// ```
pub fn validate_amount_value(amount: &str, min_amount: Decimal) -> AmountValidationResult {
    if amount.trim().is_empty() {
        return AmountValidationResult::Invalid;
    }

    match Decimal::from_str(amount.trim()) {
        Ok(amount_dec) => {
            if amount_dec < min_amount {
                AmountValidationResult::TooLow
            } else {
                AmountValidationResult::Valid
            }
        }
        Err(_) => AmountValidationResult::Invalid,
    }
}

#[spacetimedb::reducer]
pub fn validate_amount(ctx: &ReducerContext, amount: String) -> Result<(), String> {
    // Use Decimal for precise financial calculations. Default: 0.00025
    let mut min_amount = Decimal::from_str("0.00025").unwrap();

    if let Some(cfg) = ctx
        .db
        .app_config()
        .key()
        .find("min_transfer_amount".to_string())
    {
        if let Ok(parsed) = Decimal::from_str(&cfg.value) {
            min_amount = parsed;
        }
    } else if let Ok(env_min) = std::env::var("MIN_TRANSFER_AMOUNT") {
        if let Ok(parsed) = Decimal::from_str(&env_min) {
            min_amount = parsed;
        }
    }

    let res = validate_amount_value(&amount, min_amount);
    match res {
        AmountValidationResult::Valid => Ok(()),
        AmountValidationResult::TooLow => Err("amount_too_low".to_string()),
        AmountValidationResult::Invalid => Err("amount_invalid".to_string()),
    }
}
