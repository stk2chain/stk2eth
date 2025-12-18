use crate::functions::parse_input;
use crate:: USSDSession;
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