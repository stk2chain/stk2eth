mod module_bindings;
use module_bindings::*;

use spacetimedb_sdk::{credentials, DbContext, Error, Event, Identity, Status, Table, TableWithPrimaryKey};

use axum::{
    extract::Form,
    response::{IntoResponse, Response},
    routing::post,
    body::Body,
    Router,
};
use dotenv::dotenv;
use serde::Deserialize;
use std::{env, net::SocketAddr};

use std::sync::Arc;

use hyper::StatusCode;


// --- Form payload from Africa's Talking ---
// --- HTTP POST request of Content-Type application/x-www-form-urlencoded ---
#[derive(Debug, Deserialize)]
struct UssdRequest {
    sessionId: String,
    phoneNumber: String,
    networkCode: String,
    serviceCode: String,
    text: String,
}




#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load environment variables from .env
    dotenv().ok();

    // Read env vars
    // The URI of the SpacetimeDB instance hosting our chat database and module.
    let spacetime_host =
        env::var("SPACETIME_HOST").expect("SPACETIME_HOST must be set in .env");
    // The database name we chose when we published our module.
    let spacetime_dbname =
        env::var("SPACETIME_DBNAME").expect("SPACETIME_DBNAME must be set in .env");
    // The port number for the USSD Client.
    let port: u16 = env::var("USSDCLIENT_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .expect("USSDCLIENT_PORT must be a valid number");
    
    // 1. Create one SpacetimeDB database Websocket connection for the entire process.
    // Connect to the SpacetimeDB database
    let ctx = connect_to_db(&spacetime_host, &spacetime_dbname);
    let ctx = Arc::new(ctx);

    let ctx_for_http = Arc::clone(&ctx);
    
    // 2. Build the HTTP server that receives USSD gateway POSTs.
    // Handles Africa's Talking HTTP POST request of Content-Type application/x-www-form-urlencoded
    let app = Router::new()
        .route("/ussdclient", post(move |form: Form<UssdRequest>| {
        handle_ussd_request(Arc::clone(&ctx_for_http), form)
    }));

    // 3. Start the USSD Client HTTP server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("USSD server running on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

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
    