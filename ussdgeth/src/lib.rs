'''mod ussdframework;
mod audit_tests;
mod audit_reducers;

use spacetimedb::{reducer, table, Identity, ReducerContext, SpacetimeType, Table, Timestamp};

use anyhow::Result;
use ussdframework::{ScreenType as FrameworkScreenType, USSDMenu as FrameworkMenu};
mod reducers;
pub use reducers::send_eth::send_eth;

#[table(name = ussd_session)]
pub struct USSDSession {
    #[primary_key]
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    data: String,

    current_screen: String,
    visited_screens: Vec<String>,
    last_interaction_time: Timestamp,

    end_session: bool,
    #[unique]
    sender: Identity,
    online: bool,
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

#[table(name = ussd_menu)]
pub struct USSDMenu {
    #[primary_key]
    #[auto_inc]
    id: u64,
    #[unique]
    service_code: String,
}

#[table(name = ussd_service)]
pub struct USSDServiceRow {
    #[primary_key]
    id: u64,
    ussd_menu: u64,
    name: String,
    function_name: String,
    function_url: Option<String>,
    data_key: String,
}

#[derive(SpacetimeType, Clone)]
pub enum ScreenType {
    Initial,
    Menu,
    Input,
    Function,
    Router,
    Quit,
}

impl From<FrameworkScreenType> for ScreenType {
    fn from(ext: FrameworkScreenType) -> Self {
        match ext {
            FrameworkScreenType::Initial => ScreenType::Initial,
            FrameworkScreenType::Menu => ScreenType::Menu,
            FrameworkScreenType::Input => ScreenType::Input,
            FrameworkScreenType::Function => ScreenType::Function,
            FrameworkScreenType::Router => ScreenType::Router,
            FrameworkScreenType::Quit => ScreenType::Quit,
        }
    }
}

#[table(name = ussd_screen)]
pub struct USSDScreen {
    #[primary_key]
    id: u64,
    #[index(btree)]
    ussd_menu: u64,
    text: String,
    screen_type: ScreenType,
    default_next_screen: String,
    service_code: String,
    function: Option<String>,
    input_identifier: Option<String>,
    name: String,
}

#[table(name = menu_item)]
pub struct USSDMenuItem {
    option: String,
    display_name: String,
    next_screen: String,
    name: String,
    screen: u64,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SwapStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SwapType {
    SendEth,
    TokenSwap,
    CashOut,
}

#[table(name = swap)]
pub struct Swap {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub session_id: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub token_in: String,
    pub token_out: String,
    pub status: SwapStatus,
    pub tx_hash: Option<String>,
    pub gas_price: Option<String>,
    pub gas_limit: Option<String>,
    pub nonce: Option<u64>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub error_message: Option<String>,
    pub swap_type: SwapType,
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

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    let content = include_str!("./data/menu.json");
    let menu_screens: FrameworkMenu = match serde_json::from_str(content) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Failed to parse ussd menu json: {:?}", e);
            return;
        }
    };

    let menu = if let Some(existing) = ctx.db.ussd_menu().service_code().find("*4337#".to_string()) {
        existing
    } else {
        ctx.db.ussd_menu().insert(USSDMenu {
            id: 0,
            service_code: "*4337#".to_string(),
        })
    };

    for (index, (name, screen)) in menu_screens.menus.into_iter().enumerate() {
        let scrn = ctx.db.ussd_screen().insert(USSDScreen {
            id: index as u64,
            ussd_menu: menu.id,
            text: screen.text,
            screen_type: screen.screen_type.into(),
            default_next_screen: screen.default_next_screen,
            service_code: "*4337#".to_string(),
            function: screen.function,
            input_identifier: screen.input_identifier,
            name: name.to_string(),
        });

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

        if let Some(router_options) = screen.router_options {
            for option in router_options {
                ctx.db.router_option().insert(USSDRouterOption {
                    router_option: option.router_option,
                    next_screen: option.next_screen,
                });
            }
        }
    }

    for (name, service) in menu_screens.services.into_iter() {
        if service.function_name.trim().is_empty() || service.data_key.trim().is_empty() {
            log::warn!("Skipping service {} due to missing function_name or data_key", name);
            continue;
        }

        let mut max_service_id: u64 = 0;
        for s in ctx.db.ussd_service().iter() {
            if s.id > max_service_id {
                max_service_id = s.id;
            }
        }
        let new_service_id = max_service_id + 1;

        ctx.db.ussd_service().insert(USSDServiceRow {
            id: new_service_id,
            ussd_menu: menu.id,
            name: name.clone(),
            function_name: service.function_name.clone(),
            function_url: service.function_url.clone(),
            data_key: service.data_key.clone(),
        });
    }
    log::info!("USSDGETH Ininialized by, {}!", ctx.sender);
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    log::info!("Client Connected, {}!", ctx.sender);
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(session_retrieved) = ctx.db.ussd_session().sender().find(ctx.sender) {
        log::info!("Processing USSD for session: {}", session_retrieved.session_id);
    } else {
        log::warn!("Disconnect event for unknown user with identity {:?}@{:?}", ctx.sender, ctx.timestamp);
    }
}

#[reducer]
pub fn get_or_create_session(
    ctx: &ReducerContext,
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    text: String,
    initial_screen: String,
) {
    if let Some(session_retrieved) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
        ctx.db.ussd_session().session_id().update(USSDSession {
            phone_number,
            network_code,
            service_code,
            data: text,
            current_screen: session_retrieved.current_screen.clone(),
            sender: ctx.sender,
            online: true,
            last_interaction_time: ctx.timestamp,
            ..session_retrieved
        });
    } else {
        ctx.db.ussd_session().insert(USSDSession {
            session_id,
            phone_number,
            network_code,
            service_code,
            data: text,
            current_screen: initial_screen,
            sender: ctx.sender,
            online: true,
            last_interaction_time: ctx.timestamp,
            visited_screens: Vec::new(),
            end_session: false,
        });
    }
}

pub fn get_initial_screen(ctx: &ReducerContext) -> String {
    for screen in ctx.db.ussd_screen().iter() {
        if let ScreenType::Initial = screen.screen_type {
            return screen.name.clone();
        }
    }
    "InitialScreen".to_string()
}

#[reducer]
pub fn execute_screen(ctx: &ReducerContext, session_id: String, text: String) {
    let session = match ctx.db.ussd_session().session_id().find(session_id.clone()) {
        Some(s) => s,
        None => {
            log::error!("execute_screen failed: Session not found for {}", session_id);
            return;
        }
    };

    let screen_def = match ctx.db.ussd_screen().iter().find(|s| s.name == session.current_screen) {
        Some(s) => s,
        None => {
            log::error!("execute_screen failed: Screen definition not found for {}", session.current_screen);
            return;
        }
    };

    match screen_def.screen_type.clone() {
        ScreenType::Function => {
            if let Some(func_name) = screen_def.function.clone() {
                let svc_opt = ctx.db.ussd_service().iter().find(|svc| {
                    svc.name == func_name || svc.data_key == func_name || svc.function_name == func_name
                });

                if let Some(svc) = svc_opt {
                    let mut max_req_id: u64 = 0;
                    for r in ctx.db.ussd_request().iter() {
                        if r.id > max_req_id {
                            max_req_id = r.id
                        }
                    }
                    let new_req_id = max_req_id + 1;

                    ctx.db.ussd_request().insert(USSDRequest {
                        id: new_req_id,
                        ussd_menu: svc.ussd_menu,
                        session_id: session.session_id.clone(),
                        raw_data: text.clone(),
                        status: "queued".to_string(),
                        created_by: ctx.sender,
                        created_at: ctx.timestamp,
                    });

                    if svc.function_name == "send_eth" {
                        let from_address = "0x0000000000000000000000000000000000000000".to_string();
                        let to_address = "0x0000000000000000000000000000000000000000".to_string();
                        let amount = "0.0".to_string();
                        send_eth(ctx, session.session_id.clone(), from_address, to_address, amount);
                    }
                    log::info!("Enqueued USSDRequest {} for service {}", new_req_id, svc.name);
                } else {
                    log::warn!("No service found for function {}", func_name);
                }
            }
            // Update the current screen to the next screen
            ctx.db.ussd_session().session_id().update(USSDSession {
                current_screen: screen_def.default_next_screen,
                ..session
            });
        }
        _ => {
            log::info!("Executing screen type: {:?}", screen_def.screen_type);
            // Handle other screen types here in the future
        }
    }
}

#[reducer]
pub fn handle_ussd(
    ctx: &ReducerContext,
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    text: String,
) {
    if let Some(_menu) = ctx.db.ussd_menu().service_code().find(service_code.clone()) {
        let initial_screen = get_initial_screen(ctx);

        get_or_create_session(
            ctx,
            session_id.clone(),
            phone_number,
            network_code,
            service_code,
            text.clone(),
            initial_screen,
        );

        execute_screen(ctx, session_id, text);

    } else {
        log::warn!("Unknown Menu serviceCode {}", service_code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacetimedb::ReducerContext;

    fn setup_test_db(ctx: &ReducerContext) {
        // Function to initialize DB with test data
        let menu = ctx.db.ussd_menu().insert(USSDMenu {
            id: 1,
            service_code: "*123#".to_string(),
        }).unwrap();

        ctx.db.ussd_screen().insert(USSDScreen {
            id: 1,
            ussd_menu: menu.id,
            name: "EnterPin".to_string(),
            text: "Enter your PIN".to_string(),
            screen_type: ScreenType::Function,
            default_next_screen: "ConfirmPin".to_string(),
            service_code: "*123#".to_string(),
            function: Some("validate_pin".to_string()),
            input_identifier: None,
        }).unwrap();

        ctx.db.ussd_service().insert(USSDServiceRow {
            id: 1,
            ussd_menu: menu.id,
            name: "validate_pin".to_string(),
            function_name: "validate_pin_function".to_string(),
            function_url: None,
            data_key: "pin".to_string(),
        }).unwrap();
    }

    #[test]
    fn menu_json_contains_send_eth_service() {
        let content = include_str!("./data/menu.json");
        let menu: ussdframework::USSDMenu =
            serde_json::from_str(content).expect("failed to parse menu.json");
        assert!(
            menu.services.contains_key("send_eth"),
            "menu.json should contain a send_eth service"
        );
    }

    #[test]
    fn test_execute_function_screen_updates_current_screen() {
        let mut ctx = ReducerContext::new();
        ctx.db.create_tables();
        setup_test_db(&ctx);

        let session_id = "test_session_123".to_string();
        let sender = Identity::new(&[0; 32]);

        ctx.db.ussd_session().insert(USSDSession {
            session_id: session_id.clone(),
            phone_number: "12345".to_string(),
            network_code: "9999".to_string(),
            service_code: "*123#".to_string(),
            data: "".to_string(),
            current_screen: "EnterPin".to_string(),
            visited_screens: vec![],
            last_interaction_time: Timestamp::now(),
            end_session: false,
            sender,
            online: true,
        }).unwrap();

        // Execute the screen with some user input
        execute_screen(&ctx, session_id.clone(), "1234".to_string());

        // Verify that the current screen was updated
        let updated_session = ctx.db.ussd_session().session_id().find(session_id).unwrap();
        assert_eq!(updated_session.current_screen, "ConfirmPin");
    }
}
''