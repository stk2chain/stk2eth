use crate::{app_config, AppConfig};
use spacetimedb::{reducer, ReducerContext, Table};

const GATEWAY_IDENTITY_KEY: &str = "gateway_identity";

#[reducer]
pub fn claim_gateway_identity(ctx: &ReducerContext) {
    if ctx.db.app_config().key().find(GATEWAY_IDENTITY_KEY.to_string()).is_some() {
        log::error!("claim_gateway_identity: already claimed, rejected {}", ctx.sender);
        return;
    }
    ctx.db.app_config().insert(AppConfig {
        key: GATEWAY_IDENTITY_KEY.to_string(),
        value: format!("{}", ctx.sender),
    });
    log::info!("gateway identity claimed: {}", ctx.sender);
}
