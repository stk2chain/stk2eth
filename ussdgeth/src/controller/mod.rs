// ussdgeth/src/controller/mod.rs
use crate::ethclient_wrapper::EthClientWrapper;
use anyhow::Result;
use ethers::types::Address;
use reqwest::Client;
use serde_json::Value;
use std::{env, time::Duration};
use tokio::time::sleep;
use tracing::{error, info, warn}; //provide a small wrapper

//Controller main loop. Call this from your droplet main (or spawn in background).
#[allow(dead_code)]
pub async fn start_controller_loop() -> Result<()> {
    tracing::info!("Controller loop starting...");

    let spacetime_api = env::var("SPACETIME_API_URL")?;
    let spacetime_token = env::var("SPACETIME_AUTH_TOKEN")?;
    let sql_url = format!("{}/sql", spacetime_api);

    let infura = env::var("INFURA_URL")?;
    let master_key = env::var("MASTER_PRIVATE_KEY")?;

    let http = Client::builder().timeout(Duration::from_secs(8)).build()?;
    let eth = EthClientWrapper::new(&infura, &master_key).await?;

    loop {
        // 1) Query queued swaps
        let sql = "SELECT id, session_id, from_address, to_address, amount FROM swap WHERE status = 'Pending' LIMIT 10;";
        let res = http
            .post(&sql_url)
            .bearer_auth(&spacetime_token)
            .header("Content-Type", "text/plain")
            .body(sql)
            .send()
            .await;

        match res {
            Ok(resp) => {
                match resp.text().await {
                    Ok(text) => {
                        // parse JSON
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            // Expect either array as earlier code, or object with rows
                            // Try to read rows
                            let mut rows: Vec<Vec<Value>> = Vec::new();
                            if let Some(arr0) = json.get(0) {
                                if let Some(r) = arr0.get("rows") {
                                    if let Some(rarr) = r.as_array() {
                                        for row in rarr {
                                            // row is array
                                            if let Some(rowarr) = row.as_array() {
                                                rows.push(rowarr.clone());
                                            }
                                        }
                                    }
                                }
                            } else if let Some(r) = json.get("rows") {
                                if let Some(rarr) = r.as_array() {
                                    for row in rarr {
                                        if let Some(rowarr) = row.as_array() {
                                            rows.push(rowarr.clone());
                                        }
                                    }
                                }
                            }

                            for row in rows {
                                // Expect row columns: id, session_id, from_address, to_address, amount
                                if row.len() < 5 {
                                    warn!("Swap row has insufficient cols: {:?}", row);
                                    continue;
                                }
                                let id = row[0].as_u64().unwrap_or(0);
                                let session_id = row[1].as_str().unwrap_or_default();
                                let _from_address = row[2].as_str().unwrap_or_default();
                                let to_address = row[3].as_str().unwrap_or_default();
                                let amount_str = row[4].as_str().unwrap_or("0");

                                tracing::info!(
                                    "Found swap id={} session={} to={} amount={}",
                                    id,
                                    session_id,
                                    to_address,
                                    amount_str
                                );

                                // parse to address and amount
                                let to_addr: Address = match to_address.parse() {
                                    Ok(a) => a,
                                    Err(e) => {
                                        error!(
                                            "Invalid to address for swap {}: {}. Marking failed.",
                                            id, e
                                        );
                                        let _ = update_swap_failed(
                                            &http,
                                            &sql_url,
                                            &spacetime_token,
                                            id,
                                            format!("Invalid to address: {}", e),
                                        )
                                        .await;
                                        continue;
                                    }
                                };

                                let amount_eth: f64 = match amount_str.parse::<f64>() {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!(
                                            "Invalid amount for swap {}: {}. Marking failed.",
                                            id, e
                                        );
                                        let _ = update_swap_failed(
                                            &http,
                                            &sql_url,
                                            &spacetime_token,
                                            id,
                                            format!("Invalid amount: {}", e),
                                        )
                                        .await;
                                        continue;
                                    }
                                };

                                // Send transaction
                                match eth.send_eth(to_addr, amount_eth).await {
                                    Ok(tx_hash) => {
                                        info!("Swap {} sent tx {}", id, tx_hash);
                                        // update swap: set tx_hash and status
                                        let update_sql = format!(
                                            "UPDATE swap SET tx_hash = '{}', status = 'Processing' WHERE id = {};",
                                            tx_hash, id
                                        );
                                        let _ = http
                                            .post(&sql_url)
                                            .bearer_auth(&spacetime_token)
                                            .header("Content-Type", "text/plain")
                                            .body(update_sql)
                                            .send()
                                            .await;
                                    }
                                    Err(e) => {
                                        error!("Failed to send tx for swap {}: {:?}", id, e);
                                        let _ = update_swap_failed(
                                            &http,
                                            &sql_url,
                                            &spacetime_token,
                                            id,
                                            format!("{:?}", e),
                                        )
                                        .await;
                                    }
                                }
                            }
                        } else {
                            warn!("SQL response not JSON: {}", text);
                        }
                    }
                    Err(e) => {
                        warn!("Failed reading SQL response text: {:?}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Spacetime SQL request failed: {:?}", e);
            }
        }

        // sleep before next poll
        sleep(Duration::from_secs(5)).await;
    }
}

//small helper to update swap status to failed with a message
#[allow(dead_code)]
async fn update_swap_failed(
    client: &Client,
    sql_url: &str,
    token: &str,
    swap_id: u64,
    msg: String,
) -> Result<()> {
    let sql = format!(
        "UPDATE swap SET status = 'Failed', error_message = '{}' WHERE id = {};",
        msg.replace('\'', "''"),
        swap_id
    );
    let _ = client
        .post(sql_url)
        .bearer_auth(token)
        .header("Content-Type", "text/plain")
        .body(sql)
        .send()
        .await?;
    Ok(())
}
