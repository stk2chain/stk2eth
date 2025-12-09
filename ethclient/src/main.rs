use anyhow::{anyhow, Result};
use dotenv::dotenv;
use ethers_core::types::{Address, TransactionRequest, U256};
use ethers_core::utils::parse_ether;
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Signer};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::time::Duration;
use std::sync::Arc;
use spacetimedb_sdk::DbConnection;

// Generated bindings (run spacetime generate to create this module)
//   spacetime generate --lang rust --out-dir ethclient/src/module_bindings --project-path ussdgeth
mod module_bindings; use module_bindings::*;

#[derive(Debug, Deserialize, Clone)]
struct SwapRow {
    id: u64,
    session_id: String,
    from_address: String,
    to_address: String,
    amount: String,
    token_in: String,
    token_out: String,
}

async fn exec_sql(client: &Client, api_url: &str, token: &str, sql: &str) -> Result<serde_json::Value> {
    let resp = client
        .post(format!("{}/sql", api_url))
        .bearer_auth(token)
        .header("Content-Type", "text/plain")
        .body(sql.to_owned())
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!(format!("SQL error {}: {}", status, text)));
    }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    Ok(v)
}

async fn fetch_pending_swaps(client: &Client, api_url: &str, token: &str, limit: u32) -> Result<Vec<SwapRow>> {
    let sql = format!(
        "SELECT id, session_id, from_address, to_address, amount, token_in, token_out FROM swap WHERE status = 'Pending' ORDER BY id ASC LIMIT {};",
        limit
    );
    let v = exec_sql(client, api_url, token, &sql).await?;
    let mut rows = Vec::new();
    if let Some(arr) = v.as_array() {
        for obj in arr {
            if let Some(rows_arr) = obj.get("rows").and_then(|r| r.as_array()) {
                for r in rows_arr {
                    if let Some(cols) = r.as_array() {
                        if cols.len() >= 7 {
                            let row = SwapRow {
                                id: cols[0].as_u64().unwrap_or(0),
                                session_id: cols[1].as_str().unwrap_or("").to_string(),
                                from_address: cols[2].as_str().unwrap_or("").to_string(),
                                to_address: cols[3].as_str().unwrap_or("").to_string(),
                                amount: cols[4].as_str().unwrap_or("").to_string(),
                                token_in: cols[5].as_str().unwrap_or("").to_string(),
                                token_out: cols[6].as_str().unwrap_or("").to_string(),
                            };
                            rows.push(row);
                        }
                    }
                }
            }
        }
    }
    Ok(rows)
}

async fn claim_swap(client: &Client, api_url: &str, token: &str, id: u64) -> Result<bool> {
    // Call reducer claim_swap(id) -> bool
    let claimed = call_reducer(client, api_url, token, "claim_swap", json!([id])).await?;
    // Some SDKs return empty result; additionally verify via SQL
    if let Some(b) = claimed.as_bool() {
        if b { return Ok(true); }
    }
    // Fallback: check status from DB
    let v = exec_sql(client, api_url, token, &format!("SELECT status FROM swap WHERE id = {};", id)).await?;
    if let Some(arr) = v.as_array() {
        for obj in arr {
            if let Some(rows_arr) = obj.get("rows").and_then(|r| r.as_array()) {
                for r in rows_arr {
                    if let Some(cols) = r.as_array() {
                        if let Some(s) = cols.get(0).and_then(|x| x.as_str()) {
                            if s == "Processing" { return Ok(true); }
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}

// Call a SpacetimeDB reducer via HTTP
async fn call_reducer(client: &Client, api_url: &str, token: &str, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
    let url = format!("{}/call/{}", api_url, name);
    let resp = client.post(url).bearer_auth(token).json(&args).send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!(format!("Reducer {} error {}: {}", name, status, text)));
    }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    Ok(v)
}

fn parse_amount_wei(amount: &str) -> Result<U256> {
    let wei = parse_ether(amount.trim())?;
    Ok(wei)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let rpc_url = env::var("ETH_RPC_URL").expect("ETH_RPC_URL must be set");
    let priv_key = env::var("ETH_PRIVATE_KEY").expect("ETH_PRIVATE_KEY must be set");
    let api_url = env::var("SPACETIME_API_URL").expect("SPACETIME_API_URL must be set");
    let token = env::var("SPACETIME_AUTH_TOKEN").expect("SPACETIME_AUTH_TOKEN must be set");
    let host = env::var("SPACETIME_HOST").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let db_name = env::var("SPACETIME_DB_NAME").expect("SPACETIME_DB_NAME must be set");
    let network = env::var("NETWORK_NAME").unwrap_or_else(|_| "local".to_string());

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let wallet: LocalWallet = priv_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let signer = Arc::new(SignerMiddleware::new(provider.clone(), wallet.clone()));
    let http = Arc::new(Client::builder().build()?);

    // Shared app state for callbacks
    struct App { http: Arc<Client>, api_url: String, token: String, network: String, signer: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>, wallet: LocalWallet }
    let app = Arc::new(App { http: http.clone(), api_url: api_url.clone(), token: token.clone(), network: network.clone(), signer: signer.clone(), wallet: wallet.clone() });

    // Connect to SpacetimeDB via SDK
    let conn = DbConnection::builder()
        .with_token(Some(token.clone()))
        .with_module_name(&db_name)
        .with_uri(&host)
        .on_connect(|_| { println!("Connected to SpacetimeDB"); })
        .on_connect_error(|e| { eprintln!("Connect error: {:?}", e); std::process::exit(1); })
        .on_disconnect(|_| { eprintln!("Disconnected"); std::process::exit(1); })
        .build()
        .expect("Failed to connect to SpacetimeDB");

    // Register on_insert callback for swap table
    {
        let app_clone = app.clone();
        conn.db.swap().on_insert(move |_ctx, row| {
            let app = app_clone.clone();
            // Process asynchronously
            tokio::spawn(async move {
                // Fetch freshest row by id (in case fields evolve)
                let id = row.id;
                if let Ok(rows) = fetch_pending_swaps(&app.http, &app.api_url, &app.token, 100).await {
                    if let Some(swap) = rows.into_iter().find(|r| r.id == id) {
                        // Claim via reducer
                        if let Ok(true) = claim_swap(&app.http, &app.api_url, &app.token, swap.id).await {
                            // Validate and send
                            let signer_addr = app.wallet.address();
                            let from_lower = swap.from_address.to_lowercase();
                            let signer_lower = format!("0x{:x}", signer_addr).to_lowercase();
                            if from_lower != signer_lower {
                                let _ = call_reducer(&app.http, &app.api_url, &app.token, "fail_swap", json!([swap.id, "mismatched signer and from_address"])) .await; return;
                            }
                            if !(swap.token_in == "ETH" && swap.token_out == "ETH") {
                                let _ = call_reducer(&app.http, &app.api_url, &app.token, "fail_swap", json!([swap.id, "unsupported token pair"])) .await; return;
                            }
                            let to_addr: Address = swap.to_address.parse().unwrap_or_default();
                            if to_addr == Address::zero() {
                                let _ = call_reducer(&app.http, &app.api_url, &app.token, "fail_swap", json!([swap.id, "invalid to_address"])) .await; return;
                            }
                            let value = match parse_amount_wei(&swap.amount) { Ok(v) => v, Err(e) => { let _ = call_reducer(&app.http, &app.api_url, &app.token, "fail_swap", json!([swap.id, format!("invalid amount: {}", e)])).await; return; } };
                            let tx = TransactionRequest::new().to(to_addr).value(value);
                            let pending = match app.signer.send_transaction(tx, None).await { Ok(p) => p, Err(e) => { let _ = call_reducer(&app.http, &app.api_url, &app.token, "fail_swap", json!([swap.id, format!("send error: {}", e)])).await; return; } };
                            let _ = pending.await;
                            let tx_hash = format!("0x{:x}", *pending);
                            let _ = call_reducer(&app.http, &app.api_url, &app.token, "complete_swap", json!([swap.id, tx_hash, null, "21000", app.network])).await;
                        }
                    }
                }
            });
        });
    }

    // Subscribe to swaps so we get insert notifications
    conn.subscription_builder()
        .subscribe(["SELECT * FROM swap"]);

    // Keep process alive
    futures::future::pending::<()>().await;
    Ok(())
}
