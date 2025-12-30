use crate::functions::parse_input;
use crate::ussd::session::USSDSession;
use crate::eth::tx::{eth_tx, TxStatus};
use spacetimedb::Table;    
use spacetimedb::ReducerContext;
pub fn cancel_tx(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);
    
    //1*PHONE_NUMBER*AMOUNT*PIN*CANCEL_TX
    if parts.len() != 5 {
        return Err("Invalid input format".to_string());
    }

    let cancel_tx = parts[4];
    
    if cancel_tx != "1" {
        return Err("Invalid input format".to_string());
    }
    
    Ok(session)
}


/// Validates a user's choice to either confirm or cancel a pending transaction.
pub fn validate_canceltx(ctx: &ReducerContext, session_id: String, input: String) {
    let tx = ctx
        .db
        .eth_tx()
        .iter()
        .find(|s| s.session_id == session_id.clone());
    if let Some(tx) = tx {
        let mut updated_tx = tx.clone();
        if input.trim() == "2" {
            updated_tx.status = TxStatus::Cancelled;
            log::info!("Swap for session {} cancelled.", session_id);
        } else if input.trim() == "1" {
            updated_tx.status = TxStatus::Processing;
            log::info!("Swap for session {} confirmed for processing.", session_id);
        }
        ctx.db.eth_tx().id().update(updated_tx);
    }
}