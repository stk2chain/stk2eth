pub use spacetimedb::Table as SwapTable;
pub use spacetimedb::Table as USSDSessionTable;
pub use std::collections::HashMap;
mod audit_reducers;
mod audit_tests;
mod tables;
mod ussd;
mod auth;
mod eth;

pub use tables::*;
use serde::{Deserialize, Serialize};
use spacetimedb::{reducer, table, Identity, ReducerContext, SpacetimeType, Table, Timestamp, ViewContext};

use anyhow::Result;

use ussd::*;
use auth::*;
use eth::*;

mod reducers;
mod functions;



#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MenuItem {
    pub option: String,
    pub display_name: String,
    pub next_screen: String,
}



#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Screen {
    pub text: String,
    pub screen_type: ScreenType,
    pub default_next_screen: String,
    #[serde(default)]
    pub service_code: Option<String>,
    #[serde(default)]
    pub menu_items: Option<HashMap<String, MenuItem>>,
    #[serde(default)]
    pub function: Option<String>,
    // #[serde(default)]
    // pub router_options: Option<Vec<USSDRouterOption>>,
    #[serde(default)]
    pub input_identifier: Option<String>,
    #[serde(default)]
    pub input_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Service {
    pub function_name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct USSDMenu {
    pub menus: HashMap<String, Screen>,
    pub services: HashMap<String, Service>,
}




#[table(name = user_key)]
pub struct UserKey {
    #[primary_key]
    pub phone_number: String,
    pub encrypted_key: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}




#[table(name = eth_audit_logs)]
pub struct EthAuditLog {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    #[index(btree)]
    pub phone_number: String,
    pub session_id: String,
    pub timestamp: Timestamp,
    pub originator_name: Option<String>,
    pub beneficiary_name: Option<String>,
    pub originator_country: Option<String>,
    pub beneficiary_country: Option<String>,
    pub originator_address: Option<String>,
    pub beneficiary_address: Option<String>,
    pub originator_id: Option<String>,
    pub beneficiary_id: Option<String>,
    pub transaction_type: String,
    pub network: String,
    pub gas_fee: Option<String>,
    pub exchange_rate: Option<String>,
    pub compliance_status: String,
    pub risk_score: Option<u32>,
    pub is_immutable: bool,
}









// #[derive(Debug, Clone, PartialEq, Eq)]
// #[table(name = swap, public)]
// pub struct Swap {
//     #[primary_key]
//     #[auto_inc]
//     pub id: u64,
//     pub session_id: String,
//     pub from_address: String,
//     pub to_address: String,
//     pub amount: String,
//     pub token_in: String,
//     pub token_out: String,
//     pub status: SwapStatus,
//     pub tx_hash: Option<String>,
//     pub gas_price: Option<String>,
//     pub gas_limit: Option<String>,
//     pub nonce: Option<u64>,
//     pub created_at: Timestamp,
//     pub updated_at: Timestamp,
//     pub error_message: Option<String>,
//     pub swap_type: SwapType,
// }

// #[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
// pub enum SwapStatus {
//     Pending,
//     Processing,
//     Completed,
//     Failed,
//     Cancelled,
// }

// #[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
// pub enum SwapType {
//     SendEth,
//     TokenSwap,
//     CashOut,
// }

#[table(name = app_config)]
pub struct AppConfig {
    #[primary_key]
    key: String,
    value: String,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum AmountValidationResult {
    Valid,
    TooLow,
    Invalid,
}

#[table(name = router_option)]
pub struct USSDRouterOption {
    router_option: String,
    next_screen: String,
}

#[table(name = ussd_request)]
pub struct USSDRequest {
    #[primary_key]
    id: u64,
    ussd_menu: u64,
    session_id: String,
    raw_data: String,
    status: String,
    created_by: Identity,
    created_at: Timestamp,
}





#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum PINValidationResult {
    Success,
    InvalidPIN,
    AccountLocked,
    UserNotFound,
}


#[table(name = ussd_response, public)]
pub struct USSDResponse {
    #[primary_key]
    session_id: String,
    response_text: String,
    created_at: Timestamp,
    updated_at: Timestamp,
}


#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    let content = include_str!("./data/menu.json");
    let menu_screens: USSDMenu = match serde_json::from_str(content) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Failed to parse ussd menu json: {:?}", e);
            return;
        }
    };

    let menu = if let Some(existing) = ctx.db.ussd_code().service_code().find("*384*6086#".to_string()) {
        existing
    } else {
        ctx.db.ussd_code().insert(USSDCode {
            id: 0,
            service_code: "*384*6086#".to_string(),
        })
    };

    //Insert USSD Screens
    for (name, screen) in menu_screens.menus.into_iter() {
        let scrn = ctx.db.ussd_screen().insert(USSDScreen {
            id: 0, 
            ussd_code: menu.id,
            text: screen.text,
            screen_type: screen.screen_type.into(),
            default_next_screen: screen.default_next_screen,
            function: screen.function,
            name: name.to_string(),
        });

        //Insert USSD Screen Menu Items
        if let Some(menu_items) = screen.menu_items {
            for (name, item) in menu_items {
                ctx.db.menu_item().insert(USSDMenuItem {
                    option: item.option,
                    display_name: item.display_name,
                    next_screen: item.next_screen,
                    name,
                    screen: scrn.id,
                });
            }
        }

    }

    //Insert USSD Services
    for (name, service) in menu_screens.services.into_iter() {
        if service.function_name.trim().is_empty() {
            log::warn!(
                "Skipping service {} due to missing function_name or data_key",
                name
            );
            continue;
        }
            

        ctx.db.ussd_service().insert(USSDService {
            id: 0, 
            ussd_code: menu.id,
            function_name: service.function_name.clone(),
        });

    }
    log::info!("USSDGETH Ininialized by, {}!", ctx.sender);
}

/// Logs a message when a client connects.
#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    log::info!("Client Connected, {:?}@{:?}!", ctx.sender, ctx.timestamp);
}

/// Logs a message when a client disconnects.
#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    // if let Some(session_retrieved) = ctx.db.ussd_session().client().find(ctx.sender) {
    //     log::info!("Processing USSD for session: {}", session_retrieved.session_id);
    // } else {
    log::warn!("Client Disconnected, {:?}@{:?}!", ctx.sender, ctx.timestamp);
    // }
}


fn _check_profile_exists(ctx: &ReducerContext, phone_number: String) -> Option<EsimProfile> {
    ctx.db.esim_profile().phone_number().find(phone_number.clone())
}

fn _check_session_exists(ctx: &ReducerContext, session_id: String) -> Option<USSDSession> {
    ctx.db.ussd_session().session_id().find(session_id.clone())
}

fn session_update_or_create(
    ctx: &ReducerContext,
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    text: String,
    initial_screen: String,
    session_retrieved: Option<USSDSession>,
) -> USSDSession {
    if let Some(session_retrieved) = session_retrieved {
        ctx.db.ussd_session().session_id().update(USSDSession {
            // phone_number,
            // network_code,
            // service_code,
            data: text,
            // current_screen: session_retrieved.current_screen.clone(),
            client: ctx.sender, //client identity
            // online: true,
            last_interaction_time: ctx.timestamp,
            response_text: None,
            error_text: None,
            // end_session: false,
            ..session_retrieved
        })
    } else {
        ctx.db.ussd_session().insert(USSDSession {
            session_id,
            phone_number,
            network_code,
            service_code,
            data: text,
            current_screen: initial_screen,
            client: ctx.sender, //client identity
            response_text: None,
            error_text: None,
            last_interaction_time: ctx.timestamp,
        })
    }
}


fn response_update_or_create(ctx: &ReducerContext, session_id: String, response_text: String){
    if let Some(response_retrieved) = ctx.db.ussd_response().session_id().find(session_id.clone()) {
        ctx.db.ussd_response().session_id().update(USSDResponse {
            response_text: response_text.clone(),
            updated_at: ctx.timestamp,
            ..response_retrieved
        });
    } else {
        ctx.db.ussd_response().insert(USSDResponse {
            session_id,
            response_text: response_text.clone(),
            updated_at: ctx.timestamp,
            created_at: ctx.timestamp,
        });
    }
}


pub fn get_initial_screen(ctx: &ReducerContext, profile: Option<EsimProfile>) -> String {
    if let Some(profile) = profile {
        "MainScreen".to_string()
    } else {
        "RegisterScreen".to_string()
    }
}


//TODO: ONLY the USSD Client should be able to call this function
#[reducer]
pub fn process_ussd_step(
    ctx: &ReducerContext,
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    text: String,
) {
    // TODO: Check Client is whitelisted USSD Client
    if let Some(_code) = ctx.db.ussd_code().service_code().find(service_code.clone()) {
        //Does phone number EsimProfile exist in DB?

        let _profile = _check_profile_exists(ctx, normalize_phone_number(&phone_number.clone()));

        let _session = _check_session_exists(ctx, session_id.clone());


        // Handle USSD Session processing
        //Get or create session
        let mut current_session = session_update_or_create(
            ctx,
            session_id.clone(),
            normalize_phone_number(&phone_number),
            network_code,
            service_code,
            text.clone(),
            get_initial_screen(ctx, _profile),
            _session.clone(),
        );
        
        // Handle USSD SCreen processing
        let mut current_screen = ctx.db.ussd_screen().name().find(current_session.current_screen.clone());
        
        if _session.is_some() {    
            // Execute screen logic
            current_session = current_screen.clone().expect("Screen not found ::execute").execute(&ctx, &text, current_session.clone());
        }

        // Update USSD Response
        // Get updated current screen after execution
        current_screen = ctx.db.ussd_screen().name().find(current_session.current_screen.clone());
        let mut display_text = current_screen.clone().expect("Screen not found ::display").display(&ctx);
        if let Some(error_text) = current_session.error_text {
            display_text = format!("{}\n{}", display_text.unwrap_or_default(), error_text).into();
        }
        if let Some(response_text) = current_session.response_text {
            display_text = format!("{}\n{}", display_text.unwrap_or_default(), response_text).into();
        }
        response_update_or_create(ctx, session_id.clone(), display_text.clone().expect("Display text not found"));

        
    } else {
        log::warn!("Unknown Menu serviceCode {}", service_code);
    }
    
}




/// Marks a USSD session as ended and offline.
// #[reducer]
// pub fn cleanup_session(ctx: &ReducerContext, session_id: String) {
//     if let Some(session) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
//         let updated_session = USSDSession {
//             online: false,
//             end_session: true,
//             ..session
//         };
//         ctx.db.ussd_session().session_id().update(updated_session);
//         log::info!("Session {} cleaned up.", session_id);
//     }
// }

/// Validates a user's choice to either confirm or cancel a pending transaction.


#[cfg(test)]
mod tests {
    // Pure helper to map reducer error codes to screen names. Kept pure so it can be unit tested
    // without requiring a live ReducerContext or linking SpacetimeDB native libraries.
    fn map_amount_error_code(code: &str) -> &'static str {
        match code {
            "amount_too_low" => "AmountTooLowScreen",
            "amount_invalid" => "AmountInvalidScreen",
            _ => "FailureScreen",
        }
    }

    #[test]
    fn test_map_amount_error_code() {
        assert_eq!(map_amount_error_code("amount_too_low"), "AmountTooLowScreen");
        assert_eq!(map_amount_error_code("amount_invalid"), "AmountInvalidScreen");
        assert_eq!(map_amount_error_code("unknown"), "FailureScreen");
    }
}