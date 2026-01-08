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
    
    if cancel_tx == "1" {
        log::info!("ETH Transaction for session {} confirmed for processing.", session.session_id);
        // return Err("Invalid input format".to_string());
    }else if cancel_tx == "2" {
        log::info!("ETH Transaction for session {} cancelled.", session.session_id);
    }
    
    Ok(session)
}

