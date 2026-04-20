use crate::functions::hash_pin;
use crate::auth::list::{
    hashing::create_phone_permit2_authorization,
    auth_7702, Auth7702, AuthStatus,
};
use crate::auth::pin::user_pin;
use crate::auth::wallet::{esim_profile, EsimProfile};
use crate::eth::tx::{eth_tx, EthTx, TxStatus, TxType};
use crate::ussd::intent::{parse_intent, UserIntent};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;
use spacetimedb::ReducerContext;

const BASE_SEPOLIA_CHAIN_ID: u64 = 84532;

fn is_valid_e164(phone: &str) -> bool {
    let digits = phone.strip_prefix('+').unwrap_or(phone);
    let len = digits.len();
    if !(8..=15).contains(&len) { return false; }
    let first = digits.as_bytes()[0];
    if !(b'1'..=b'9').contains(&first) { return false; }
    digits.chars().all(|c| c.is_ascii_digit())
}

pub fn validate_phone_number(_ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("ToNumberScreen", &session.data)?;
    let phone = match intent {
        UserIntent::SendEthPhone { phone } => phone,
        _ => return Err("expected SendEthPhone intent".to_string()),
    };
    if !is_valid_e164(&phone) {
        return Err("Invalid phone number format".to_string());
    }
    Ok(session)
}

pub fn validate_amount(_ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("ToAmountScreen", &session.data)?;
    let amount = match intent {
        UserIntent::SendEthAmount { amount, .. } => amount,
        _ => return Err("expected SendEthAmount intent".to_string()),
    };
    let parsed: f64 = amount.parse().map_err(|_| "Invalid amount format".to_string())?;
    if parsed <= 0.0 {
        return Err("Amount must be positive".to_string());
    }
    eth_decimal_to_wei(&amount)?;
    Ok(session)
}

fn eth_decimal_to_wei(amount: &str) -> Result<String, String> {
    let (int_part, frac_part) = match amount.split_once('.') {
        Some((i, f)) => (i, f),
        None => (amount, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err("Invalid amount format".to_string());
    }
    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return Err("Invalid amount format".to_string());
    }
    if frac_part.len() > 18 {
        return Err("Amount precision exceeds 18 decimals".to_string());
    }
    let frac_padded = format!("{:0<18}", frac_part);
    let combined = format!("{}{}", int_part, frac_padded);
    let trimmed = combined.trim_start_matches('0');
    Ok(if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    })
}

pub fn validate_pin(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("PINScreen", &session.data)?;
    let (phone, amount, pin) = match intent {
        UserIntent::SendEthPin { phone, amount, pin } => (phone, amount, pin),
        _ => return Err("expected SendEthPin intent".to_string()),
    };

    let user_pin_row = ctx.db.user_pin().phone_number()
        .find(session.phone_number.clone())
        .ok_or_else(|| "User not registered".to_string())?;

    let computed = hash_pin(&pin, &session.phone_number, &user_pin_row.salt);
    if computed != user_pin_row.pin_hash {
        return Err("Invalid PIN".to_string());
    }

    let receiver_wallet = if let Some(p) = ctx.db.esim_profile().phone_number().find(phone.clone()) {
        p.wallet_address
    } else {
        let (w, auth) = create_phone_permit2_authorization(
            &phone, BASE_SEPOLIA_CHAIN_ID, 0, None, None,
        ).map_err(|e| {
            log::error!("receiver wallet derivation failed: {}", e);
            "Service temporarily unavailable. Please try again.".to_string()
        })?;
        let wallet_address = hex::encode(w);
        ctx.db.esim_profile().insert(EsimProfile {
            phone_number: phone.clone(),
            wallet_address: wallet_address.clone(),
            created_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
        ctx.db.auth_7702().insert(Auth7702 {
            authority_address: wallet_address.clone(),
            chain_id: auth.chain_id,
            delegate_to: hex::encode(auth.address),
            nonce: auth.nonce,
            v: auth.v,
            r: hex::encode(auth.r),
            s: hex::encode(auth.s),
            status: AuthStatus::Pending,
            created_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
        wallet_address
    };

    let sender = ctx.db.esim_profile().phone_number()
        .find(session.phone_number.clone())
        .ok_or_else(|| "Sender profile missing".to_string())?;

    let amount_wei = eth_decimal_to_wei(&amount)?;

    ctx.db.eth_tx().insert(EthTx {
        id: 0,
        session_id: session.session_id.clone(),
        tx_type: TxType::SendEth,
        from: sender.wallet_address,
        to: receiver_wallet,
        value: amount_wei,
        data: None,
        gas_limit: "100000".to_string(),
        status: TxStatus::Pending,
        tx_hash: None,
        block_number: None,
        gas_used: None,
        error_reason: None,
        processing_by: None,
        processing_since: None,
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
    });

    session.response_text = Some(format!(
        "Confirm TX:\nTo: {}\nAmount: {} ETH\n\n1. Confirm\n2. Cancel",
        phone, amount
    ));
    Ok(session)
}

pub fn validate_token(_ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    Ok(session)
}
