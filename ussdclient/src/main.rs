use axum::{
    extract::Form,
    response::{IntoResponse, R    // Respond in text/plain (required by Africa's Talking)
    ([(axum::http::header::CONTENT_TYPE, "text/plain")], reply_text).into_response()
}

async fn handle_user_input(client: &Client, token: &str, session_id: &str, input: &str) -> Result<(), String> {
    // Get current session state
    let sql_query = format!(
        "SELECT current_screen FROM ussd_session WHERE session_id = '{}';",
        session_id
    );
    
    let spacetime_sql_url = std::env::var("SPACETIME_API_URL").unwrap_or_else(|_| "http://localhost:3000/v1/database/stk2eth".to_string()) + "/sql";
    
    let response = client.post(&spacetime_sql_url)
        .bearer_auth(token)
        .header("Content-Type", "text/plain")
        .body(sql_query)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body = response.text().await.map_err(|e| e.to_string())?;
    
    if let Ok(json) = serde_json::from_str::<Value>(&body) {
        if let Some(current_screen) = json
            .get(0)
            .and_then(|v| v.get("rows"))
            .and_then(|rows| rows.get(0))
            .and_then(|row| row.get(0))
            .and_then(|v| v.as_str())
        {
            // Handle input based on current screen
            match current_screen {
                "SendETHAmountScreen" => {
                    // Update session with ETH amount
                    update_session_field(client, token, session_id, "eth_amount", input).await?;
                },
                "SendETHRecipientScreen" => {
                    // Update session with recipient address
                    update_session_field(client, token, session_id, "recipient_address", input).await?;
                },
                "SendETHPINScreen" => {
                    // Validate PIN and process transaction
                    if input.len() >= 4 {
                        call_process_send_eth(client, token, session_id).await?;
                    }
                },
                _ => {
                    // Handle menu navigation
                    println!("Menu input {} for screen {}", input, current_screen);
                }
            }
        }
    }
    
    Ok(())
}

async fn update_session_field(client: &Client, token: &str, session_id: &str, field: &str, value: &str) -> Result<(), String> {
    let spacetime_url = std::env::var("SPACETIME_API_URL").unwrap_or_else(|_| "http://localhost:3000/v1/database/stk2eth".to_string()) + "/call/update_send_eth_session";
    
    let payload = serde_json::json!({
        "session_id": session_id,
        "field": field,
        "value": value
    });
    
    let response = client.post(&spacetime_url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to update session field: {}", response.status()));
    }
    
    println!("Updated session {} field {} with value {}", session_id, field, "***");
    Ok(())
}

async fn call_process_send_eth(client: &Client, token: &str, session_id: &str) -> Result<(), String> {
    let spacetime_url = std::env::var("SPACETIME_API_URL").unwrap_or_else(|_| "http://localhost:3000/v1/database/stk2eth".to_string()) + "/call/process_send_eth";
    
    let payload = serde_json::json!({
        "session_id": session_id
    });
    
    let response = client.post(&spacetime_url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to process send ETH: {}", response.status()));
    }
    
    println!("Processed send ETH for session {}", session_id);
    Ok(())
}

async fn process_template_variables(text: &str, session_id: &str, client: &Client, token: &str) -> String {
    // Simple template variable replacement
    let mut processed = text.to_string();
    
    // Replace common variables with session data
    if processed.contains("{{") {
        // Get session data
        let sql_query = format!(
            "SELECT pending_amount, pending_recipient FROM ussd_session WHERE session_id = '{}';",
            session_id
        );
        
        let spacetime_sql_url = std::env::var("SPACETIME_API_URL").unwrap_or_else(|_| "http://localhost:3000/v1/database/stk2eth".to_string()) + "/sql";
        
        if let Ok(response) = client.post(&spacetime_sql_url)
            .bearer_auth(token)
            .header("Content-Type", "text/plain")
            .body(sql_query)
            .send()
            .await
        {
            if let Ok(body) = response.text().await {
                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(row) = json
                        .get(0)
                        .and_then(|v| v.get("rows"))
                        .and_then(|rows| rows.get(0))
                    {
                        if let (Some(amount), Some(recipient)) = (
                            row.get(0).and_then(|v| v.as_str()),
                            row.get(1).and_then(|v| v.as_str()),
                        ) {
                            processed = processed.replace("{{eth_amount}}", amount);
                            processed = processed.replace("{{recipient_address}}", recipient);
                            processed = processed.replace("{{gas_fee}}", "0.000420"); // Mock gas fee
                        }
                    }
                }
            }
        }
    }
    
    processed
}

// --- Main server ---se},
    routing::post,
    Json, Router,
};
use dotenv::dotenv;
use serde::Deserialize;
use std::{env, net::SocketAddr};
// use tokio::net::TcpListener;
use reqwest::Client;
use serde_json::Value;

// --- Form payload from Africa's Talking ---
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct UssdRequest {
    session_id: String,
    service_code: String,
    phone_number: String,
    network_code: String,
    text: String,
}

// --- Handler ---
#[allow(dead_code)]
async fn ussd_handler(Form(payload): Form<UssdRequest>) -> Response {
    println!("Received USSD request: {:?}", payload);
    
    // Build JSON payload for SpaceTimeDB reducer
    let json_payload = serde_json::json!({
        "sessionId": payload.session_id,
        "phoneNumber": payload.phone_number,
        "networkCode": payload.network_code,
        "serviceCode": payload.service_code,
        "text": payload.text,
    });

    let mut reply_text: String = "END Service unavailable".to_string();
    let mut response_prefix = "CON"; // Continue session by default

    let client = Client::new();
    let spacetime_url = std::env::var("SPACETIME_API_URL").unwrap_or_else(|_| "http://localhost:3000/v1/database/stk2eth".to_string()) + "/call/handle_ussd";
    let spacetime_sql_url = std::env::var("SPACETIME_API_URL").unwrap_or_else(|_| "http://localhost:3000/v1/database/stk2eth".to_string()) + "/sql";
    let token = std::env::var("SPACETIME_AUTH_TOKEN").unwrap_or_else(|_| "".to_string());

    // Step 1: Call handle_ussd reducer to process the USSD request
    let response = client.post(&spacetime_url)
        .bearer_auth(&token)
        .json(&json_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Successfully called handle_ussd reducer");
            } else {
                println!("Failed to call handle_ussd reducer: {}", resp.status());
            }
        },
        Err(e) => {
            println!("Error calling handle_ussd reducer: {}", e);
            return ([(axum::http::header::CONTENT_TYPE, "text/plain")], "END Service temporarily unavailable").into_response();
        }
    }

    // Step 2: Handle different screen types and user input
    let user_input = payload.text.split('*').last().unwrap_or("").trim();
    
    // Step 3: Update session based on user input and current screen
    if !user_input.is_empty() {
        let session_update_result = handle_user_input(&client, &token, &payload.session_id, user_input).await;
        if let Err(e) = session_update_result {
            println!("Error updating session: {}", e);
        }
    }

    let sql_query = format!(
        "SELECT s.text \
        FROM ussd_session AS sess \
        JOIN ussd_screen AS s \
        ON sess.current_screen = s.name \
        WHERE sess.session_id = '{}';",
        payload.session_id
    );

    let rpl = client
        .post(spacetime_sql_url)
        .bearer_auth(token) // load from creds
        .header("Content-Type", "text/plain") // 👈 tell it this is raw SQL
        .body(sql_query) // 👈 just the SQL text
        .send()
        .await;

    // let rply = rpl.unwrap();
    let body = rpl.unwrap().text().await.unwrap();

    println!("Response {:?}. ", body);

    if let Ok(json) = serde_json::from_str::<Value>(&body) {
        if let Some(row_value) = json
            .get(0) // outer array
            .and_then(|v| v.get("rows"))
            .and_then(|rows| rows.get(0)) // first row
            .and_then(|row| row.get(0)) // first column
            .and_then(|val| val.as_str())
        {
            reply_text = row_value.to_string();
        }
    }

    // Respond in text/plain (required by Africa’s Talking)
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        reply_text,
    )
        .into_response()
}

// --- Main server ---
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env
    dotenv().ok();

    // Read env vars
    let spacetime_api =
        env::var("SPACETIME_API_URL").expect("SPACETIME_API_URL must be set in .env");
    let spacetime_token =
        env::var("SPACETIME_AUTH_TOKEN").expect("SPACETIME_AUTH_TOKEN must be set in .env");
    let port: u16 = env::var("USSD_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .expect("USSD_PORT must be a valid number");

    // Build URLs
    let spacetime_url = format!("{}/call/handle_ussd", spacetime_api);
    let _spacetime_sql_url = format!("{}/sql", spacetime_api);

    // Prepare HTTP client with bearer token
    let client = Client::builder().build()?;

    // Example: store client & token in app state
    let app = Router::new().route(
        "/ussd",
        post(move |Json(body): Json<Value>| {
            let client = client.clone();
            let spacetime_url = spacetime_url.clone();
            let spacetime_token = spacetime_token.clone();
            async move {
                let res = client
                    .post(&spacetime_url)
                    .bearer_auth(&spacetime_token)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Error: {:?}", e))?;

                Ok::<_, String>(res.text().await.unwrap_or_default())
            }
        }),
    );

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("USSD server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
