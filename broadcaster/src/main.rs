// broadcaster/src/main.rs
//
// Wires Config → signer → provider → SpacetimeDB subscriber → submitter +
// watcher, runs reconcile once, then serves the submit loop until ctrl_c.

mod abi;
mod config;
mod error;
mod reconcile;
mod signer;
mod stdb;
mod submitter;
mod subscriber;
mod watcher;

use crate::config::Config;
use crate::error::BroadcasterError;
use crate::signer::load_operator_signer;
use crate::stdb::AppConfigTableAccess;
use crate::submitter::{Submitter, WatcherMsg};
use crate::subscriber::Subscriber;
use crate::watcher::Watcher;
use alloy::providers::{Provider, RootProvider};
use spacetimedb_sdk::DbContext;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), BroadcasterError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    tracing::info!(chain_id = cfg.chain_id, "broadcaster starting");

    let operator = load_operator_signer(
        &cfg.operator_keystore_path,
        &cfg.operator_keystore_passphrase,
    )?;
    tracing::info!(operator = %operator.address(), "operator keystore decrypted");

    let rpc_url: reqwest::Url = cfg
        .rpc_url
        .parse()
        .map_err(|e| BroadcasterError::Config(format!("RPC_URL parse: {e}")))?;
    let provider = RootProvider::new_http(rpc_url);

    let onchain_chain_id = provider
        .get_chain_id()
        .await
        .map_err(|e| BroadcasterError::RpcTransient(format!("get_chain_id: {e}")))?;
    if onchain_chain_id != cfg.chain_id {
        return Err(BroadcasterError::Config(format!(
            "chain_id mismatch: env={} rpc={}",
            cfg.chain_id, onchain_chain_id
        )));
    }
    tracing::info!(chain_id = onchain_chain_id, "RPC chain_id verified");

    let subscriber = Subscriber::connect(
        &cfg.spacetime_host,
        &cfg.spacetime_db_name,
        secrecy::ExposeSecret::expose_secret(&cfg.spacetime_auth_token),
    )
    .await?;
    let stdb = subscriber.stdb.clone();

    subscriber
        .stdb
        .subscription_builder()
        .subscribe(vec![
            "SELECT * FROM app_config".to_string(),
            "SELECT * FROM auth_7702".to_string(),
            "SELECT * FROM eth_tx".to_string(),
        ]);

    let mut expected_identity = None;
    for _ in 0..50 {
        if let Some(id) = stdb.try_identity() {
            expected_identity = Some(id);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let expected_identity = expected_identity.ok_or_else(|| {
        BroadcasterError::Config("SpacetimeDB identity not received within 5s".to_string())
    })?;
    let expected_hex = format!("{}", expected_identity.to_hex());
    wait_for_gateway_identity(&stdb, &expected_hex).await?;

    let initial_nonce = provider
        .get_transaction_count(operator.address())
        .pending()
        .await
        .map_err(|e| BroadcasterError::RpcTransient(format!("get_tx_count: {e}")))?;
    let operator_nonce = Arc::new(AtomicU64::new(initial_nonce));
    tracing::info!(nonce = initial_nonce, "operator nonce bootstrapped");

    let (watcher_tx, watcher_rx) = tokio::sync::mpsc::unbounded_channel::<WatcherMsg>();

    let submitter = Submitter {
        provider: provider.clone(),
        operator,
        stdb: stdb.clone(),
        chain_id: cfg.chain_id,
        burner7702_address: cfg.burner7702_address,
        balance_reserve: cfg.balance_reserve_wei,
        operator_nonce: operator_nonce.clone(),
        watcher_tx: watcher_tx.clone(),
    };

    reconcile::reconcile(&submitter, &watcher_tx, cfg.reconcile_scan_blocks).await?;

    let watcher = Watcher {
        ws_url: cfg.rpc_ws_url.clone(),
        stdb: stdb.clone(),
        rx: watcher_rx,
    };
    let watcher_handle = tokio::spawn(async move {
        if let Err(e) = watcher.run().await {
            tracing::error!(error = %e, "watcher exited with error");
        }
    });

    let mut rx = subscriber.rx;
    let _subscriber_keepalive = subscriber.stdb;
    let submit_handle = tokio::spawn(async move {
        while let Some(row) = rx.recv().await {
            let id = row.id;
            if let Err(e) = submitter.handle(row).await {
                tracing::error!(eth_tx_id = id, error = %e, "submitter handle failed");
            }
        }
    });

    tokio::signal::ctrl_c()
        .await
        .map_err(|e| BroadcasterError::Config(format!("signal install: {e}")))?;
    tracing::info!("ctrl_c received, shutting down");
    submit_handle.abort();
    watcher_handle.abort();
    Ok(())
}

async fn wait_for_gateway_identity(
    stdb: &stdb::DbConnection,
    expected_hex: &str,
) -> Result<(), BroadcasterError> {
    for _ in 0..50 {
        if let Some(row) = stdb
            .db
            .app_config()
            .key()
            .find(&"gateway_identity".to_string())
        {
            if row.value.eq_ignore_ascii_case(expected_hex) {
                tracing::info!(identity = %row.value, "gateway identity confirmed");
                return Ok(());
            }
            return Err(BroadcasterError::Config(format!(
                "gateway_identity mismatch: module={} ours={expected_hex}",
                row.value
            )));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err(BroadcasterError::Config(
        "gateway_identity not seen within 5s — is claim_gateway_identity wired?".to_string(),
    ))
}
