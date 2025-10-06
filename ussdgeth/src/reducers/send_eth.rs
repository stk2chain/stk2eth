use crate::{Swap, USSDSession, SwapStatus, SwapType};
use spacetimedb::Table;
use crate::swap;
use crate::ussd_session;
use spacetimedb::{reducer, ReducerContext};

fn is_valid_eth_address(address: &str) -> bool {
    address.starts_with("0x") && address.len() == 42 && address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_amount(amount: &str) -> Result<f64, String> {
    match amount.parse::<f64>() {
        Ok(amt) if amt > 0.0 => Ok(amt),
        Ok(_) => Err("Amount must be greater than zero".to_string()),
        Err(_) => Err("Invalid amount format".to_string()),
    }
}

#[reducer]
pub fn send_eth(
    ctx: &ReducerContext,
    session_id: String,
    from_address: String,
    to_address: String,
    amount: String,
) {
    if !is_valid_eth_address(&from_address) {
        log::error!("Invalid from address format: {}", from_address);
        return;
    }

    if !is_valid_eth_address(&to_address) {
        log::error!("Invalid to address format: {}", to_address);
        return;
    }

    if let Err(e) = is_valid_amount(&amount) {
        log::error!("Invalid amount: {}", e);
        return;
    }

    let session = match ctx.db.ussd_session().session_id().find(session_id.clone()) {
        Some(s) => s,
        None => {
            log::error!("Session not found: {}", session_id);
            return;
        }
    };

    ctx.db.swap().insert(Swap {
        id: 0, // This is ignored and auto-incremented
        session_id: session_id.clone(),
        from_address: from_address.clone(),
        to_address: to_address.clone(),
        amount: amount.clone(),
        token_in: "ETH".to_string(),
        token_out: "ETH".to_string(),
        status: SwapStatus::Pending,
        tx_hash: None,
        gas_price: None,
        gas_limit: Some("21000".to_string()),
        nonce: None,
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
        error_message: None,
        swap_type: SwapType::SendEth,
    });

    let mut visited_screens = session.visited_screens.clone();
    visited_screens.push(session.current_screen.clone());

    let updated_session = USSDSession {
        current_screen: "transaction_processing".to_string(),
        visited_screens,
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
