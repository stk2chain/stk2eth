use crate::{swap, ussd_session, Swap, USSDSession};
use crate::{SwapStatus, SwapType};
use spacetimedb::Table;
use spacetimedb::{reducer, ReducerContext};

/// Validates Ethereum address format
fn is_valid_eth_address(address: &str) -> bool {
    // Basic validation: starts with 0x and has 42 characters total
    address.starts_with("0x")
        && address.len() == 42
        && address[2..].chars().all(|c| c.is_ascii_hexdigit())
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
        nonce: None,                          // Will be set by blockchain client
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
        amount,
        from_address,
        to_address,
        session_id
    );
}
