use crate::functions::{validate_pin_format, hash_pin, parse_input};
use crate::auth::wallet::{esim_profile, EsimProfile};
use crate::auth::pin::{user_pin, UserPIN};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;    
use spacetimedb::ReducerContext;





// Assumption: register_pin ONLY EVER called 
// immediately after 1. Register
pub fn register_pin(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);

    // 1*PIN
    if parts.len() != 2 {
        return Err("*Invalid input format".to_string());
    }

    let pin = parts[1];
   
    match validate_pin_format(pin, false) {
        Ok(_) => {
            Ok(session)
        }
        Err(err) => {
            Err(err)
        }
    }
    
}

// Assumption: confirm_register_pin ONLY  EVER called 
// immediately after register_pin
// PIN was already validated in register_pin
// TODO: if user_input changes mid session ConfirmPIN != RegisterPIN
// confirm_register_pin is cooked
pub fn confirm_register_pin(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);

    // 1*PIN*PIN
    if parts.len() != 3 {
        return Err("*Invalid input format".to_string());
    }
    let pin = parts[1];
    let confirm_pin = parts[2];
    
    if pin != confirm_pin {
        return Err("*PIN do not match".to_string());
    }

    let tmstmp = ctx.timestamp;

    let pin_hash = hash_pin(pin, &session.phone_number, &tmstmp.to_string());

    // TODO: Only  insert esim_profile if phone_number is not already registered
    if !ctx.db.esim_profile().phone_number().find(&session.phone_number).is_some() {
        //Register Esim Profile and user PIN
        ctx.db.esim_profile().insert(EsimProfile {
            phone_number: session.phone_number.clone(),
            wallet_address: "".to_string(),
            // auth_hash: Some(pin_hash),
            created_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
    }

    ctx.db.user_pin().insert(UserPIN {
        phone_number: session.phone_number.clone(),
        pin_hash: pin_hash,
        salt: tmstmp.to_string(),
        attempts: 0,
        locked: false,
        last_attempt_time: None,
        lockout_until: None,
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
    });
    
    //Clear Session user_input for Main Screen
    session.data = "".to_string();
    Ok(session)
    
}
