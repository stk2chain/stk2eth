use crate::functions::{hash_pin, parse_input};
use crate::{esim_profile, EsimProfile, USSDSession};
use spacetimedb::Table;    
use spacetimedb::ReducerContext;

fn is_valid_e164(phone: &str) -> bool {
    // must start with +
    let bytes = phone.as_bytes();
    if bytes.is_empty() || bytes[0] != b'+' {
        return false;
    }

    let digits = &phone[1..];

    // length constraints: 8..=15 digits (after +)
    let len = digits.len();
    if !(8..=15).contains(&len) {
        return false;
    }

    // first digit must be 1-9
    let first = digits.as_bytes()[0];
    if !(b'1'..=b'9').contains(&first) {
        return false;
    }

    // all must be digits
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    true
}

pub fn validate_phone_number(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);

    //1*PHONE_NUMBER
    if parts.len() != 2 {
        return Err("Invalid input format".to_string());
    }

    let phone_number = parts[1];

    if !is_valid_e164(&phone_number) {
        return Err("Invalid phone number format".to_string());
    }

    Ok(session)
}

pub fn validate_amount(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);

    //1*PHONE_NUMBER*AMOUNT
    if parts.len() != 3 {
        return Err("Invalid input format".to_string());
    }
    
    let amount = parts[2];
    
    amount
        .parse::<f64>()
        .map_err(|_| "Invalid amount format".to_string())
        .and_then(|n| {
            if n > 0.0 {
                Ok(session)
            } else {
                Err("Amount must be positive".to_string())
            }
        })    
}

//Assumes ONLY called by a registerd user
pub fn validate_pin(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);
    
    //1*PHONE_NUMBER*AMOUNT*PIN
    if parts.len() != 4 {
        return Err("Invalid input format".to_string());
    }
    
    let pin = parts[3];
    
    if let Some(eprofile) = ctx.db.esim_profile().phone_number().find(session.phone_number.clone()) {
        //NB: PIN & PHONE_NUMBER MUST ONLY BE derived from the Current Session
        let pin_hash = hash_pin(pin, &session.phone_number, &eprofile.created_at.to_string());
        if let Some(auth_hash) = eprofile.auth_hash {
            if pin_hash == auth_hash {
                return Ok(session);
            } else {
                return Err("Invalid pin".to_string());
            }
        } else {
            return Err("PIN not set for user".to_string());
        }
    }
    //Should never be reached
    return Err("User Not registered".to_string());
    
    
}

