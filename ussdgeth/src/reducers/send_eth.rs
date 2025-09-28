use spacetimedb::{ReducerContext, reducer};
use crate::{Swap, USSDSession, ussd_session, swap};
use crate::{SwapStatus, SwapType};
use spacetimedb::Table;
use serde_json::{json, Value};

/// Validates Ethereum address format
fn is_valid_eth_address(address: &str) -> bool {
    // Basic validation: starts with 0x and has 42 characters total
    address.starts_with("0x") && 
    address.len() == 42 && 
    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validates amount string can be parsed as positive decimal
fn is_valid_amount(amount: &str) -> Result<f64, String> {
    match amount.parse::<f64>() {
        Ok(amt) if amt > 0.0 => Ok(amt),
        Ok(_) => Err("Amount must be greater than zero".to_string()),
        Err(_) => Err("Invalid amount format".to_string()),
    }
}

/// Send ETH reducer - creates a Swap transaction representing ETH transfer
/// This implements "Send ETH" as ETH->ETH swap with different recipient addresses
#[reducer]
pub fn send_eth(
    ctx: &ReducerContext,
    session_id: String,
    from_address: String,
    to_address: String,
    amount: String,
) {
    // Validate inputs
    if !is_valid_eth_address(&from_address) {
        panic!("Invalid from address format");
    }
    
    if !is_valid_eth_address(&to_address) {
        panic!("Invalid to address format");
    }
    
    if let Err(e) = is_valid_amount(&amount) {
        panic!("Invalid amount: {}", e);
    }
    
    // Find and validate session exists
    let session = match ctx.db.ussd_session().session_id().find(session_id.clone()) {
        Some(s) => s,
        None => {
            panic!("Session not found: {}", session_id);
        }
    };
    
    // Create Swap transaction (Send ETH is modeled as ETH->ETH swap)
    let swap = Swap {
        id: 0, // auto-increment
        session_id: session_id.clone(),
        from_address: from_address.clone(),
        to_address: to_address.clone(),
        amount: amount.clone(),
        token_in: "ETH".to_string(),
        token_out: "ETH".to_string(),
        status: SwapStatus::Pending,
        tx_hash: None,
        gas_price: None,
        gas_limit: Some("21000".to_string()), // Standard ETH transfer gas limit
        nonce: None, // Will be set by blockchain client
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
        error_message: None,
        swap_type: SwapType::SendEth,
    };
    
    // Insert the swap transaction
    let _ = ctx.db.swap().insert(swap);
    
    // Update session state to show transaction is processing
    let updated_session = USSDSession {
        current_screen: "transaction_processing".to_string(),
        visited_screens: {
            let mut visited = session.visited_screens;
            visited.push(session.current_screen);
            visited
        },
        last_interaction_time: ctx.timestamp,
        ..session
    };
    
    ctx.db.ussd_session().session_id().update(updated_session);
    
    log::info!(
        "Created ETH send transaction: {} ETH from {} to {} for session {}", 
        amount, from_address, to_address, session_id
    );
}

/// Process Send ETH - validates session state and processes the transaction
#[reducer]
pub fn process_send_eth(ctx: &ReducerContext, session_id: String) -> Value {
    // Find session
    let session = match ctx.db.ussd_session().session_id().find(session_id.clone()) {
        Some(s) => s,
        None => {
            return json!({
                "status": "error",
                "message": "Session not found",
                "error_code": "SESSION_NOT_FOUND"
            });
        }
    };
    
    // Parse session state to get transaction details
    let session_state: Value = match serde_json::from_str(&session.session_state) {
        Ok(state) => state,
        Err(_) => json!({})
    };
    
    // Validate required fields from session state
    let eth_amount = session.pending_amount.as_ref().unwrap_or(&"0".to_string()).clone();
    let recipient_address = session.pending_recipient.as_ref().unwrap_or(&"".to_string()).clone();
    
    if eth_amount == "0" || recipient_address.is_empty() {
        return json!({
            "status": "failed",
            "message": "Missing transaction details. Please start over.",
            "error_code": "MISSING_DETAILS"
        });
    }
    
    // Validate inputs
    if !is_valid_eth_address(&recipient_address) {
        return json!({
            "status": "failed", 
            "message": "Invalid recipient address format",
            "error_code": "INVALID_ADDRESS"
        });
    }
    
    if let Err(e) = is_valid_amount(&eth_amount) {
        return json!({
            "status": "failed",
            "message": format!("Invalid amount: {}", e),
            "error_code": "INVALID_AMOUNT"
        });
    }
    
    // Check if user has sufficient balance (mock for now)
    let user_balance = 1.0; // TODO: Get actual balance from blockchain
    let amount_f64 = eth_amount.parse::<f64>().unwrap();
    
    if amount_f64 > user_balance {
        return json!({
            "status": "failed",
            "message": "Insufficient balance",
            "error_code": "INSUFFICIENT_BALANCE"
        });
    }
    
    // Simulate transaction processing
    let mock_tx_hash = format!("0x{:064x}", rand_hash());
    let gas_fee = calculate_gas_fee(&eth_amount);
    
    // Create swap record for tracking
    let swap = Swap {
        id: 0,
        session_id: session_id.clone(),
        from_address: "0x1234567890123456789012345678901234567890".to_string(), // TODO: Get from session
        to_address: recipient_address.clone(),
        amount: eth_amount.clone(),
        token_in: "ETH".to_string(),
        token_out: "ETH".to_string(),
        status: SwapStatus::Completed, // Mock success
        tx_hash: Some(mock_tx_hash.clone()),
        gas_price: Some(gas_fee.clone()),
        gas_limit: Some("21000".to_string()),
        nonce: Some(1),
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
        error_message: None,
        swap_type: SwapType::SendEth,
    };
    
    ctx.db.swap().insert(swap);
    
    // Update session to mark transaction as complete
    let updated_session = USSDSession {
        end_session: true,
        step_count: session.step_count + 1,
        last_interaction_time: ctx.timestamp,
        ..session
    };
    
    ctx.db.ussd_session().session_id().update(updated_session);
    
    log::info!("Processed ETH send: {} ETH to {} for session {}", eth_amount, recipient_address, session_id);
    
    json!({
        "status": "success",
        "message": "Transaction completed successfully",
        "tx_hash": mock_tx_hash,
        "eth_amount": eth_amount,
        "recipient_address": recipient_address,
        "gas_fee": gas_fee,
        "timestamp": ctx.timestamp.to_string()
    })
}

/// Updates session with pending transaction details
#[reducer]
pub fn update_send_eth_session(ctx: &ReducerContext, session_id: String, field: String, value: String) {
    if let Some(session) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
        let mut updated_session = session;
        
        match field.as_str() {
            "eth_amount" => {
                // Validate amount
                if let Err(_) = is_valid_amount(&value) {
                    log::warn!("Invalid amount provided: {}", value);
                    return;
                }
                updated_session.pending_amount = Some(value);
            },
            "recipient_address" => {
                // Validate address
                if !is_valid_eth_address(&value) {
                    log::warn!("Invalid address provided: {}", value);
                    return;
                }
                updated_session.pending_recipient = Some(value);
            },
            _ => {
                log::warn!("Unknown field for send_eth session update: {}", field);
                return;
            }
        }
        
        updated_session.last_interaction_time = ctx.timestamp;
        updated_session.step_count += 1;
        
        ctx.db.ussd_session().session_id().update(updated_session);
        
        log::info!("Updated session {} field {} with value {}", session_id, field, "***");
    }
}

// Helper functions
fn rand_hash() -> u64 {
    // Simple pseudo-random hash for demo purposes
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

fn calculate_gas_fee(amount: &str) -> String {
    // Mock gas fee calculation
    let gas_price_gwei = 20; // 20 gwei
    let gas_limit = 21000;
    let gas_fee_gwei = gas_price_gwei * gas_limit;
    let gas_fee_eth = gas_fee_gwei as f64 / 1_000_000_000.0; // Convert to ETH
    
    format!("{:.6}", gas_fee_eth)
}