use spacetimedb::ReducerContext;
use crate::ussd::session::USSDSession;

use super::register::{register_pin, confirm_register_pin};
use super::validate::{validate_phone_number, validate_amount, validate_pin, validate_token};
use super::cancel::cancel_tx;

pub fn dispatch(
    function_name: &str,
    ctx: &ReducerContext,
    session: USSDSession,
) -> Result<USSDSession, String> {
    match function_name {
        "register_pin"            => register_pin(ctx, session),
        "confirm_register_pin"    => confirm_register_pin(ctx, session),
        "validate_phone_number"   => validate_phone_number(ctx, session),
        "validate_amount"         => validate_amount(ctx, session),
        "validate_pin"            => validate_pin(ctx, session),
        "validate_token"          => validate_token(ctx, session),
        "cancel_tx"               => cancel_tx(ctx, session),
        other => Err(format!("unknown USSD function '{}'", other)),
    }
}
