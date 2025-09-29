use axum::{
    extract::Form,
    response::{IntoResponse, Response},
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
    // Build JSON payload for SpaceTimeDB reducer
    let json_payload = serde_json::json!({
        "sessionId": payload.session_id,
        "phoneNumber": payload.phone_number,
        "networkCode": payload.network_code,
        "serviceCode": payload.service_code,
        "text": payload.text,
    });

    let mut reply_text: String = "END Service unavailable".to_string();

    // Call SpaceTimeDB reducer over HTTP (assuming reducer is exposed at /call/handle_ussd)
    let client = Client::new();
    //let spacetime_url = "http://127.0.0.1:3000/v1/database/gateway/call/handle_ussd"; // adjust for your SpacetimeDB instance
    //let spacetime_sql_url = "http://127.0.0.1:3000/v1/database/gateway/sql";

    let spacetime_url = std::env::var("SPACETIME_API_URL").unwrap() + "/call/handle_ussd";
    let spacetime_sql_url = std::env::var("SPACETIME_API_URL").unwrap() + "/sql";
    let token = std::env::var("SPACETIME_AUTH_TOKEN").unwrap();

    let _response = client.post(spacetime_url).json(&json_payload).send().await;

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
