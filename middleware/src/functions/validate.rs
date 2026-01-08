use crate::functions::{hash_pin, parse_input };
use crate::auth::list::{hashing::create_phone_permit2_authorization, auth_7702, Auth7702, AuthStatus};
use crate::auth::pin::{user_pin, UserPIN};
use crate::auth::wallet::{esim_profile, PhoneWallet, EsimProfile};
use crate::eth::tx::{eth_tx, EthTx, TxStatus, TxType, TxParams, Params};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;    
use spacetimedb::ReducerContext;

fn is_valid_e164(phone: &str) -> bool {
    // must start with +
    let bytes = phone.as_bytes();
    // if bytes.is_empty() || bytes[0] != b'+' {
    if bytes.is_empty() {
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

    // Dynamic position based on operation: OP or OP*PHONE_NUMBER
    let phone_position = if parts.len() == 2 {
        1 // First phone input: OP*PHONE_NUMBER
    } else if parts.len() > 2 {
        3 // Still in position 1 for multi-part inputs
    } else {
        return Err("Invalid input format".to_string());
    };

    let phone_number = parts[phone_position];

    if !is_valid_e164(&phone_number) {
        return Err("Invalid phone number format".to_string());
    }

    Ok(session)
}

pub fn validate_amount(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);
    
    let amount_position = if parts.len() >= 3 {
        2 // OP*PHONE*AMOUNT
    } else if parts.len() == 2 {
        1 // OP*AMOUNT (for withdraw)
    } else {
        return Err("Invalid input format".to_string());
    };
    
    let amount = parts[amount_position];

    //1*PHONE_NUMBER*AMOUNT
    // if parts.len() != 3 {
    //     return Err("Invalid input format".to_string());
    // }
    
    // let amount = parts[2];
    
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
pub fn validate_pin(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);
    
    //1*PHONE_NUMBER*AMOUNT*PIN
    if parts.len() != 4 {
        return Err("Invalid input format".to_string());
    }

    let tx_type = match TxType::from_ussd_op(parts[0]) {
        Some(st) => st,
        None => return Err("Invalid swap type".to_string()),
    };

    
    
    let phone_number = parts[1];
    let amount = parts[2];
    let pin = parts[3];
    
    // // let params = SwapParams {
    // //     to: Some(phone_number),
    // //     amount: Some(amount.parse::<u128>().unwrap()),
    // //     ..Default::default()
    // // };
    
    // let calldata = swap_type.to_tx(params).encode();
    
    if let Some(user_pin) = ctx.db.user_pin().phone_number().find(session.phone_number.clone()) {
        //NB: PIN & PHONE_NUMBER MUST ONLY BE derived from the Current Session
        let pin_hash = hash_pin(pin, &session.phone_number, &user_pin.salt);
        if user_pin.pin_hash == pin_hash {
            let mut receiver_wallet = String::new();
            if let Some(receiver_profile) = ctx.db.esim_profile().phone_number().find(phone_number.clone().to_string()) {
                receiver_wallet = receiver_profile.wallet_address.clone();
            } else {
                //TODO: Generate receiver wallet
                let (receiver_wallet_, _auth) = create_phone_permit2_authorization(
                    &phone_number,
                    0, //Universal Chain ID
                    0,
                    None,
                    None,
                );
                receiver_wallet = hex::encode(receiver_wallet_);
                //Register Receiver Esim Profile if does not exist
                ctx.db.esim_profile().insert(EsimProfile {
                    phone_number: phone_number.to_string(),
                    wallet_address: receiver_wallet.clone(),
                    // auth_hash: Some(pin_hash),
                    created_at: ctx.timestamp,
                    updated_at: ctx.timestamp,
                });

                ctx.db.auth_7702().insert(Auth7702 {
                    authority_address: receiver_wallet.clone(),
                    chain_id: _auth.chain_id,
                    delegate_to: hex::encode(_auth.address),
                    nonce: _auth.nonce,
                    v: _auth.v,
                    r: hex::encode(_auth.r),
                    s: hex::encode(_auth.s),
                    status: AuthStatus::Pending,
                    created_at: ctx.timestamp,
                    updated_at: ctx.timestamp,
                });
            }

            let params = Params {
                to: Some(&receiver_wallet.clone()),
                amount: Some(amount.parse::<u128>().unwrap()),
                ..Default::default()
            };
            
            let calldata = tx_type.to_tx(params).encode();

            let sender_wallet = ctx.db.esim_profile().phone_number().find(session.phone_number.clone()).unwrap();
            ctx.db.eth_tx().insert(EthTx {
                session_id: session.session_id.clone(),
                from: sender_wallet.wallet_address.clone(),
                to: receiver_wallet.clone(),
                value: amount.to_string(),
                data: Some(calldata),
                gas_limit: "21000".to_string(),
                status: TxStatus::Pending,
                tx_hash: None,
                created_at: ctx.timestamp,
                updated_at: ctx.timestamp,
            });
            // Add confirmation text to the session
            let confirmation_text = format!(
                "Confirm TX:\nTo: {}\nAmount: {} ETH\nFee: {}\nTotal: {}\n\n1. Confirm\n2. Cancel",
                phone_number,
                amount,
                "0.001",  // TODO: Calculate or fetch actual fee
                amount    // TODO: Add fee to amount for total
            );

            // Update session with confirmation text
            session.response_text = Some(format!("{}{}", session.response_text.unwrap_or_default(), confirmation_text));
                

            return Ok(session);
        } else {
            return Err("Invalid pin".to_string());
        }
    }
    //Should never be reached
    return Err("User Not registered".to_string());
    
    
}

pub fn validate_token(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let parts: Vec<&str> = parse_input(&session.data);
    
    //2*TOKEN*AMOUNT*RECEIVER*PIN*CANCEL_TX
    if parts.len() != 2 {
        return Err("Invalid input format".to_string());
    }
    
    let binding = parts[1].to_uppercase();
    let token_symbol = binding.trim();
    
    // List of supported tokens
    let supported_tokens = vec!["ETH", "USDC", "USDT", "DAI", "WETH"];
    
    if supported_tokens.contains(&token_symbol) {
        Ok(session)
    } else {
        Err(format!(
            "Token '{}' not supported. Available: {}",
            token_symbol,
            supported_tokens.join(", ")
        ))
    }
}



