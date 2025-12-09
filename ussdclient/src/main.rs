// ussdclient/src/main.rs
// USSD webhook server (Axum) for Africa's Talking -> SpacetimeDB reducer integration.
//
// Accepts form (x-www-form-urlencoded) and JSON payloads.
// Calls SpacetimeDB reducer: POST {SPACETIME_API_URL}/call/handle_ussd
// Then queries SQL: {SPACETIME_API_URL}/sql to fetch the current screen text.
// Returns plain text (CON/END message) to Africa's Talking.

use anyhow::Result;
use axum::{
    extract::{Form, Json, State},
    response::IntoResponse,
    response::Response,
    routing::post,
    Router,
};
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info};

async fn handle_ussd(State(state): State<AppState>, body_bytes: axum::body::Bytes) -> Response {
    // Try parse as form first
    if let Ok(form_data) = serde_urlencoded::from_bytes::<AfricastalkingForm>(&body_bytes) {
        return handle_form(State(state.clone()), Form(form_data)).await;
    }

    // Try parse as JSON
    if let Ok(json_data) = serde_json::from_slice::<Value>(&body_bytes) {
        return handle_json(State(state), Json(json_data)).await;
    }

    plain_text_response("END Invalid payload format".to_string())
}

#[derive(Debug, Deserialize)]
struct AfricastalkingForm {
    #[serde(alias = "sessionId", alias = "session_id")]
    session_id: Option<String>,
    #[serde(alias = "serviceCode", alias = "service_code")]
    service_code: Option<String>,
    #[serde(alias = "phoneNumber", alias = "phone_number")]
    phone_number: Option<String>,
    #[serde(alias = "networkCode", alias = "network_code")]
    network_code: Option<String>,
    text: Option<String>,
}

#[derive(Clone)]
struct AppState {
    client: Client,
    spacetime_call: String,
    spacetime_sql: String,
    token: String,
    // optional simple in-memory rate limiter or counters
    _counter: Arc<Mutex<u64>>,
}

fn plain_text_response(body: String) -> Response {
    ([(axum::http::header::CONTENT_TYPE, "text/plain")], body).into_response()
}

async fn handle_form(
    State(s): State<AppState>,
    Form(payload): Form<AfricastalkingForm>,
) -> Response {
    let session_id = payload.session_id.unwrap_or_default();
    let service_code = payload.service_code.unwrap_or_default();
    let phone_number = payload.phone_number.unwrap_or_default();
    let network_code = payload.network_code.unwrap_or_default();
    let text = payload.text.unwrap_or_default();

    process_ussd(
        &s,
        &session_id,
        &phone_number,
        &network_code,
        &service_code,
        &text,
    )
    .await
}

async fn handle_json(State(s): State<AppState>, Json(body): Json<Value>) -> Response {
    let session_id = body
        .get("sessionId")
        .or_else(|| body.get("session_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let service_code = body
        .get("serviceCode")
        .or_else(|| body.get("service_code"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let phone_number = body
        .get("phoneNumber")
        .or_else(|| body.get("phone_number"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let network_code = body
        .get("networkCode")
        .or_else(|| body.get("network_code"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let text = body
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    process_ussd(
        &s,
        &session_id,
        &phone_number,
        &network_code,
        &service_code,
        &text,
    )
    .await
}

async fn process_ussd(
    state: &AppState,
    session_id: &str,
    phone_number: &str,
    network_code: &str,
    service_code: &str,
    text: &str,
) -> Response {
    // call reducer
    let payload = serde_json::json!({
        "session_id": session_id,
        "phone_number": phone_number,
        "network_code": network_code,
        "service_code": service_code,
        "text": text
    });

    let call_res = state
        .client
        .post(&state.spacetime_call)
        .bearer_auth(&state.token)
        .json(&payload)
        .send()
        .await;

    if let Err(e) = call_res {
        error!("Spacetime reducer call failed: {:?}", e);
        return plain_text_response("END Service temporarily unavailable".to_string());
    }

    // query SQL to get the screen text for the session
    let sql = format!(
        "SELECT s.text FROM ussd_session AS sess JOIN ussd_screen AS s ON sess.current_screen = s.name WHERE sess.session_id = '{}';",
        session_id.replace('\'', "''")
    );

    let sql_res = state
        .client
        .post(&state.spacetime_sql)
        .bearer_auth(&state.token)
        .header("Content-Type", "text/plain")
        .body(sql)
        .send()
        .await;

    if let Err(e) = sql_res {
        error!("Spacetime SQL request failed: {:?}", e);
        return plain_text_response("END Service temporarily unavailable".to_string());
    }

    let body = match sql_res.unwrap().text().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed reading SQL response text: {:?}", e);
            return plain_text_response("END Service temporarily unavailable".to_string());
        }
    };

    // parse the likely JSON shapes returned by SpacetimeDB SQL
    if let Ok(json) = serde_json::from_str::<Value>(&body) {
        // common shape: [{ "rows": [ ["Menu text"] ] }]
        if let Some(screen_text) = json
            .get(0)
            .and_then(|v| v.get("rows"))
            .and_then(|rows| rows.get(0))
            .and_then(|row| row.get(0))
            .and_then(|val| val.as_str())
        {
            return plain_text_response(screen_text.to_string());
        }

        // alternative shape: { "rows": [ ["Menu text"] ] }
        if let Some(row0) = json.get("rows").and_then(|r| r.get(0)) {
            if let Some(col0) = row0.get(0).and_then(|c| c.as_str()) {
                return plain_text_response(col0.to_string());
            }
        }
    }

    plain_text_response("END An error occurred. Try again later.".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let spacetime_api = env::var("SPACETIME_API_URL").expect("SPACETIME_API_URL must be set");
    let token = env::var("SPACETIME_AUTH_TOKEN").expect("SPACETIME_AUTH_TOKEN must be set");

    let call = format!("{}/call/handle_ussd", spacetime_api);
    let sql = format!("{}/sql", spacetime_api);

    let client = Client::builder().timeout(Duration::from_secs(6)).build()?;

    let app_state = AppState {
        client,
        spacetime_call: call,
        spacetime_sql: sql,
        token,
        _counter: Arc::new(Mutex::new(0)),
    };

    let port: u16 = env::var("USSD_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080);

    let app = Router::new()
        .route("/ussd", post(handle_ussd))
        .route("/health", post(|| async { "ok" }))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("USSD webhook listening on http://{}", addr);
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())


}


/// Load credentials from a file and connect to the database.
fn connect_to_db(host: &String, dbname: &String) -> DbConnection {
    DbConnection::builder()
        // Register our `on_connect` callback, which will save our auth token.
        // .on_connect(on_connected)
        // Register our `on_connect_error` callback, which will print a message, then exit the process.
        // .on_connect_error(on_connect_error)
        // Our `on_disconnect` callback, which will print a message, then exit the process.
        // .on_disconnect(on_disconnected)
        // If the user has previously connected, we'll have saved a token in the `on_connect` callback.
        // In that case, we'll load it and pass it to `with_token`,
        // so we can re-authenticate as the same `Identity`.
        // .with_token(creds_store().load().expect("Error loading credentials"))
        // Set the database name we chose when we called `spacetime publish`.
        .with_module_name(dbname)
        // Set the URI of the SpacetimeDB host that's running our database.
        .with_uri(host)
        // Finalize configuration and connect!
        .build()
        .expect("Failed to connect")
}


//SpacetimeDB procedures
fn creds_store() -> credentials::File {
    credentials::File::new("ussdgeth")
}

/// Our `on_connect` callback: save our credentials to a file.
fn on_connected(_ctx: &DbConnection, _identity: Identity, token: &str) {
    if let Err(e) = creds_store().save(token) {
        eprintln!("Failed to save credentials: {:?}", e);
    }
}

/// Our `on_connect_error` callback: print the error, then exit the process.
fn on_connect_error(_ctx: &ErrorContext, err: Error) {
    eprintln!("Connection error: {:?}", err);
    // std::process::exit(1);
}

/// Our `on_disconnect` callback: print a note, then exit the process.
fn on_disconnected(_ctx: &ErrorContext, err: Option<Error>) {
    if let Some(err) = err {
        eprintln!("Disconnected: {}", err);
        // std::process::exit(1);
    } else {
        println!("Disconnected.");
        // std::process::exit(0);
    }
}


async fn handle_ussd_request(ctx: Arc<DbConnection>, Form(req): Form<UssdRequest>) -> Response {
    // 1. Trigger reducer to process the USSD state transition.
    let res = ctx.reducers
        .process_ussd_step(
            req.sessionId.clone(),
            req.phoneNumber.clone(),
            req.networkCode.clone(),
            req.serviceCode.clone(),
            req.text.clone(),
        );
    
    if let Err(e) = res {
        return Response::builder()
            .status(500)
            .body(Body::from(format!("ERR: {}", e)))
            .unwrap();
    }
    
    let value = ctx.clone();
    tokio::task::block_in_place(|| {
        if let Err(e) = value.frame_tick() {
            eprintln!("Frame tick error: {}", e);
        }
    });
    //  Pull DB events deterministically.
    //    Ensures reducer effects and SQL changes are visible now.
    // if let Err(e) = ctx.frame_tick() {
    //     return Response::builder()
    //         .status(500)
    //         .body(Body::from(format!("ERR frame_tick: {}", e)))
    //         .unwrap();
    // }

    // 2. Query final return value from your USSDResponse table.
    let menu = query_ussd_output(&ctx, &req.sessionId.clone())
        .unwrap_or_else(|| "END System error".to_string());

    // 3. Return to gateway. Gateway delivers to user.
    // Respond in text/plain (required by Africa’s Talking)
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        menu,
    )
        .into_response()

}

/// SQL query USSDResponse Table.
fn query_ussd_output(ctx: &DbConnection, session_id: &String) -> Option<String> { 
    if let Some(ussd_response) = ctx.db.ussd_response().session_id().find(&session_id) {
        Some(ussd_response.response_text.clone())
    } else {
        None
    }
}
    