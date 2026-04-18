use crate::functions::{validate_pin_format, hash_pin};
use crate::auth::list::{hashing::create_phone_permit2_authorization, auth_7702, Auth7702, AuthStatus};
use crate::auth::pin::{user_pin, UserPIN};
use crate::auth::wallet::{esim_profile, EsimProfile};
use crate::ussd::intent::{parse_intent, UserIntent};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;
use spacetimedb::ReducerContext;

const BASE_SEPOLIA_CHAIN_ID: u64 = 84532;

pub fn register_pin(_ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("RegisterPinScreen", &session.data)?;
    let pin = match intent {
        UserIntent::RegisterPin { pin } => pin,
        _ => return Err("expected RegisterPin intent".to_string()),
    };
    validate_pin_format(&pin, false)?;
    Ok(session)
}

pub fn confirm_register_pin(ctx: &ReducerContext, mut session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("ConfirmRegisterPinScreen", &session.data)?;
    let (pin, confirm) = match intent {
        UserIntent::ConfirmRegisterPin { pin, confirm } => (pin, confirm),
        _ => return Err("expected ConfirmRegisterPin intent".to_string()),
    };

    if pin != confirm {
        return Err("*PIN do not match".to_string());
    }

    let ts = ctx.timestamp;
    let pin_hash = hash_pin(&pin, &session.phone_number, &ts.to_string());

    if ctx.db.esim_profile().phone_number().find(&session.phone_number).is_none() {
        let (wallet, auth) = create_phone_permit2_authorization(
            &session.phone_number,
            BASE_SEPOLIA_CHAIN_ID,
            0,
            None,
            None,
        ).map_err(|e| {
            log::error!("wallet derivation failed for register: {}", e);
            "Service temporarily unavailable. Please try again.".to_string()
        })?;

        let wallet_address = hex::encode(wallet);

        ctx.db.esim_profile().insert(EsimProfile {
            phone_number: session.phone_number.clone(),
            wallet_address: wallet_address.clone(),
            created_at: ts,
            updated_at: ts,
        });

        ctx.db.auth_7702().insert(Auth7702 {
            authority_address: wallet_address,
            chain_id: auth.chain_id,
            delegate_to: hex::encode(auth.address),
            nonce: auth.nonce,
            v: auth.v,
            r: hex::encode(auth.r),
            s: hex::encode(auth.s),
            status: AuthStatus::Pending,
            created_at: ts,
            updated_at: ts,
        });
    }

    ctx.db.user_pin().insert(UserPIN {
        phone_number: session.phone_number.clone(),
        pin_hash,
        salt: ts.to_string(),
        attempts: 0,
        locked: false,
        last_attempt_time: None,
        lockout_until: None,
        created_at: ts,
        updated_at: ts,
    });

    session.data = "".to_string();
    Ok(session)
}
