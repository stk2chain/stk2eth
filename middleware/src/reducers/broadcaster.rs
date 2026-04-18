use crate::eth::tx::{eth_tx, EthTx, TxStatus};
use crate::auth::list::{auth_7702, Auth7702, AuthStatus};
use crate::{app_config, AppConfig};
use spacetimedb::{reducer, ReducerContext, Table};

const GATEWAY_IDENTITY_KEY: &str = "gateway_identity";

fn is_terminal(s: &TxStatus) -> bool {
    matches!(s, TxStatus::Confirmed | TxStatus::Failed | TxStatus::Cancelled)
}

fn require_gateway(ctx: &ReducerContext) -> Result<(), String> {
    let cfg = ctx.db.app_config().key().find(GATEWAY_IDENTITY_KEY.to_string())
        .ok_or_else(|| "gateway identity not configured".to_string())?;
    let expected = cfg.value;
    let sender = format!("{}", ctx.sender);
    if sender != expected {
        return Err(format!("unauthorized: {} != {}", sender, expected));
    }
    Ok(())
}

#[reducer]
pub fn set_gateway_identity(ctx: &ReducerContext, identity_str: String) {
    if ctx.sender != ctx.identity() {
        log::error!("set_gateway_identity: unauthorized sender {}", ctx.sender);
        return;
    }
    if let Some(existing) = ctx.db.app_config().key().find(GATEWAY_IDENTITY_KEY.to_string()) {
        ctx.db.app_config().key().update(AppConfig {
            value: identity_str,
            ..existing
        });
    } else {
        ctx.db.app_config().insert(AppConfig {
            key: GATEWAY_IDENTITY_KEY.to_string(),
            value: identity_str,
        });
    }
    log::info!("gateway identity set");
}

#[reducer]
pub fn mark_eth_tx_processing(ctx: &ReducerContext, eth_tx_id: u64, worker_id: String) {
    if let Err(e) = require_gateway(ctx) {
        log::error!("mark_eth_tx_processing: {}", e); return;
    }
    let row = match ctx.db.eth_tx().id().find(eth_tx_id) {
        Some(r) => r,
        None => { log::error!("eth_tx {} not found", eth_tx_id); return; }
    };
    if is_terminal(&row.status) {
        log::warn!("eth_tx {} already in terminal state {:?}", eth_tx_id, row.status);
        return;
    }
    if let (Some(_other), Some(since)) = (&row.processing_by, row.processing_since) {
        let elapsed = ctx.timestamp.duration_since(since).unwrap_or_default();
        if elapsed < std::time::Duration::from_secs(5 * 60) {
            log::warn!("eth_tx {} already leased", eth_tx_id);
            return;
        }
    }
    ctx.db.eth_tx().id().update(EthTx {
        status: TxStatus::Broadcasting,
        processing_by: Some(worker_id),
        processing_since: Some(ctx.timestamp),
        updated_at: ctx.timestamp,
        ..row
    });
}

#[reducer]
pub fn mark_eth_tx_broadcast(ctx: &ReducerContext, eth_tx_id: u64, tx_hash: String) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.eth_tx().id().find(eth_tx_id) {
        if is_terminal(&row.status) {
            log::warn!("eth_tx {} already terminal, ignoring mark_eth_tx_broadcast", eth_tx_id);
            return;
        }
        ctx.db.eth_tx().id().update(EthTx {
            status: TxStatus::Broadcast,
            tx_hash: Some(tx_hash),
            updated_at: ctx.timestamp,
            ..row
        });
    }
}

#[reducer]
pub fn confirm_eth_tx(
    ctx: &ReducerContext,
    eth_tx_id: u64,
    tx_hash: String,
    block_number: u64,
    gas_used: String,
) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.eth_tx().id().find(eth_tx_id) {
        if is_terminal(&row.status) {
            log::warn!("eth_tx {} already terminal, ignoring confirm_eth_tx", eth_tx_id);
            return;
        }
        ctx.db.eth_tx().id().update(EthTx {
            status: TxStatus::Confirmed,
            tx_hash: Some(tx_hash),
            block_number: Some(block_number),
            gas_used: Some(gas_used),
            updated_at: ctx.timestamp,
            ..row
        });
    }
}

#[reducer]
pub fn fail_eth_tx(ctx: &ReducerContext, eth_tx_id: u64, tx_hash: Option<String>, reason: String) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.eth_tx().id().find(eth_tx_id) {
        if is_terminal(&row.status) {
            log::warn!("eth_tx {} already terminal, ignoring fail_eth_tx", eth_tx_id);
            return;
        }
        ctx.db.eth_tx().id().update(EthTx {
            status: TxStatus::Failed,
            tx_hash,
            error_reason: Some(reason),
            updated_at: ctx.timestamp,
            ..row
        });
    }
}

#[reducer]
pub fn mark_auth7702_active(ctx: &ReducerContext, authority_address: String) {
    if let Err(e) = require_gateway(ctx) { log::error!("{}", e); return; }
    if let Some(row) = ctx.db.auth_7702().authority_address().find(authority_address) {
        ctx.db.auth_7702().authority_address().update(Auth7702 {
            status: AuthStatus::Broadcasted,
            updated_at: ctx.timestamp,
            ..row
        });
    }
}
