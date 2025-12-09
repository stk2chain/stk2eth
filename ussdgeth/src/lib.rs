pub use spacetimedb::Table as SwapTable;
pub use spacetimedb::Table as USSDSessionTable;
pub(crate) mod amount_validation_tests;
mod audit_reducers;
mod audit_tests;
mod pin_validation_tests;
mod swap_tests;
mod ussdframework;

use spacetimedb::{reducer, table, Identity, ReducerContext, SpacetimeType, Table, Timestamp, ViewContext};

use anyhow::Result;

use crate::ussdframework::utils::FUNCTION_MAP;

use ussdframework::{ussd_screens, USSDMenu as FrameworkMenu};
mod controller;
mod crypto;
mod ethclient_wrapper;
mod reducers;
mod functions;

pub use reducers::keys::{delete_user_key, fetch_user_key, store_user_key};
pub use reducers::send_eth::send_eth;
pub use reducers::validate_phone::map_phone_to_wallet;
pub use reducers::validate_pin::validate_pin;
pub use functions::register_functions;


#[table(name = esim_profile)]
pub struct EsimProfile {
    #[primary_key]
    #[unique]
    phone_number: String,
    #[unique]
    wallet_address: String,
    auth_hash: Option<String>,
    created_at: Timestamp,
    updated_at: Timestamp,
}


//TODO: Make it transient
#[derive(Clone)]

//if you want to call them via /call/<reducer>.

#[table(name = phone_wallet)]
pub struct PhoneWallet {
    /// E.164 phone number (primary key)
    #[primary_key]
    pub phone_number: String,

    /// Ethereum wallet address (0x...)
    pub wallet_address: String,

    /// Creation time
    pub created_at: Timestamp,

    /// Last update time
    pub updated_at: Timestamp,
}
#[table(name = user_key)]
pub struct UserKey {
    #[primary_key]
    pub phone_number: String,
    pub encrypted_key: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
#[table(name = ussd_session)]
pub struct USSDSession {
    #[primary_key]
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    data: String,

    current_screen: String,
    // visited_screens: Vec<String>,
    last_interaction_time: Timestamp,

    // end_session: bool,
    #[index(btree)]
    client: Identity, //USSD Client identity
    // online: bool,
    authenticated: bool,
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
pub struct USSDService {
    #[primary_key]
    id: u64,
    ussd_menu: u64,
    name: String,
    function_name: String,
    function_url: Option<String>,
    data_key: String,
}




impl USSDService {
    
    fn load_function(&self) -> Box<dyn Fn(&str) -> Result<(), String> + '_> {
        // Load the function from the registered functions
        let func = FUNCTION_MAP
            .lock()
            .unwrap()
            .get(&self.function_name)
            .cloned();

        match func {
            Some(f) => {
                log::info!("Function found: {}", self.function_name);
                Box::new(f)
            }
            None => {
                log::error!("Function not found: {}", self.function_name);
                Box::new(|_input: &str| {
                    Err(format!("Function '{}' not found", self.function_name))
                })
            }
        }
    }
}


#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub enum ScreenType {
    Initial,
    Menu,
    Input,
    Function,
    Router,
    Quit,
}

impl From<ussd_screens::ScreenType> for ScreenType {
    fn from(ext: ussd_screens::ScreenType) -> Self {
        match ext {
            ussd_screens::ScreenType::Initial => ScreenType::Initial,
            ussd_screens::ScreenType::Menu => ScreenType::Menu,
            ussd_screens::ScreenType::Input => ScreenType::Input,
            ussd_screens::ScreenType::Function => ScreenType::Function,
            ussd_screens::ScreenType::Router => ScreenType::Router,
            ussd_screens::ScreenType::Quit => ScreenType::Quit,
        }
    }
}

#[derive(Clone)]
#[table(name = ussd_screen)]
pub struct USSDScreen {
    #[primary_key]
    id: u64,
    #[unique]
    name: String,
    screen_type: ScreenType,
    default_next_screen: String,
    service_code: String,
    #[index(btree)]
    ussd_menu: u64,
    text: String,
    function: Option<String>,
}

impl USSDScreen {
    /// Displays a message corresponding to the screen type.
    ///
    /// The message construction depends on the type of screen:
    /// - For an initial screen, no message is displayed.
    /// - For a menu screen, the message concatenates the screen text with the menu items.
    /// - For an input screen, the message comprises the screen text alone.
    /// - For a function screen, the message comprises the screen text alone.
    /// - For a router screen, no message is displayed.
    pub fn display(&self, ctx: &ReducerContext) -> Option<String> {
        match self.screen_type {
            ScreenType::Router => None,
            ScreenType::Quit => Some(format!("END {}", self.text.clone())),
            ScreenType::Menu => Some(format!("CON {}", self.format_menu_screen(ctx))),
            _ => Some(format!("CON {}", self.text.clone())),
        }
    }

    fn format_menu_screen(&self, ctx: &ReducerContext) -> String {
        let menu_items = self.get_sorted_menu_items(ctx);
        
        match menu_items.is_empty() {
            true => format!("{}\nNo menu items found", self.text),
            false => format!("{}{}", self.text, self.format_menu_items(&menu_items)),
        }
    }

    fn get_sorted_menu_items(&self, ctx: &ReducerContext) -> Vec<USSDMenuItem> {
        let mut items: Vec<_> = ctx.db.menu_item()
            .screen()
            .filter(self.id)
            .collect();
        
        items.sort_by_key(|item| item.option.parse::<usize>().unwrap_or(0));
        items
    }

    fn format_menu_items(&self, items: &[USSDMenuItem]) -> String {
        items
            .iter()
            .enumerate()
            .map(|(index, item)| format!("\n{}. {}", item.option, item.display_name))
            .collect::<Vec<_>>()
            .join("")
    }

    fn execute(&self, ctx: &ReducerContext, user_input: &str, session: USSDSession) -> USSDSession {
        let input = user_input.trim();

        let mut next_screen = session.current_screen.clone();

        match self.screen_type {
            ScreenType::Menu => {
                if let Ok(default_next_screen) = self.execute_menu_selection(ctx, input) {
                    next_screen = default_next_screen;
                }
            }
            ScreenType::Function => {
                if let Some(function_name) = &self.function {
                    if let Ok(_) = self.execute_function_screen(ctx, input, function_name) {
                        next_screen = self.default_next_screen.clone();
                    }
                }
            }
            _ => {
                log::warn!("Screen type {:?} not supported by execute function", self.screen_type);
            }
        }

        ctx.db.ussd_session().session_id().update(USSDSession {
            current_screen: next_screen.clone(),
            ..session
        })

    }

    fn execute_menu_selection(&self, ctx: &ReducerContext, user_input: &str) -> Result<String, String> {
        match user_input.parse::<usize>() {
            Ok(option) => if option > 0 {
                let menu_items = self.get_sorted_menu_items(ctx);
                if let Some(selected_item) = menu_items.iter().find(|item| item.option == option.to_string()) {
                    return Ok(selected_item.next_screen.clone());
                } else {
                    return Err(format!("Invalid menu option '{}' for screen '{}'", user_input, self.name));
                }
            } else {
               return Err(format!("Invalid menu option '{}' for screen '{}'", user_input, self.name));
            }
            Err(_)=> return Err("Invalid menu option".to_string())
        }
        
    }

    fn execute_function_screen(&self, ctx: &ReducerContext, user_input: &str, function_name: &str) -> Result<(), String> {
        let svc_opt = ctx.db.ussd_service().iter().find(|svc| {
                svc.name == function_name || svc.data_key == function_name || svc.function_name == function_name
            });
                
        if let Some(svc) = svc_opt {
            let loaded_function = svc.load_function();
            loaded_function(user_input)
                    
        } else {
            return Err(format!("Function not found for screen '{}'", self.name))
        }
    }

}




#[table(name = menu_item)]
pub struct USSDMenuItem {
    option: String,
    display_name: String,
    next_screen: String,
    name: String,
    #[index(btree)]
    screen: u64,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SwapStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SwapType {
    SendEth,
    TokenSwap,
    CashOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[table(name = swap, public)]
pub struct Swap {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
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

#[table(name = ussd_response, public)]
pub struct USSDResponse {
    #[primary_key]
    session_id: String,
    response_text: String,
    created_at: Timestamp,
    updated_at: Timestamp,
}

#[table(name = user_pin)]
pub struct UserPIN {
    #[primary_key]
    #[index(btree)]
    pub phone_number: String,
    pub pin_hash: String,
    pub salt: String,
    pub attempts: u32,
    pub locked: bool,
    pub last_attempt_time: Option<Timestamp>,
    pub lockout_until: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum PINValidationResult {
    Success,
    InvalidPIN,
    AccountLocked,
    UserNotFound,
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

    let menu = if let Some(existing) = ctx.db.ussd_menu().service_code().find("*384*6086#".to_string()) {
        existing
    } else {
        ctx.db.ussd_menu().insert(USSDMenu {
            id: 0,
            service_code: "*384*6086#".to_string(),
        })
    };

    for (index, (name, screen)) in menu_screens.menus.into_iter().enumerate() {
        let scrn = ctx.db.ussd_screen().insert(USSDScreen {
            id: index as u64,
            ussd_menu: menu.id,
            text: screen.text,
            screen_type: screen.screen_type.into(),
            default_next_screen: screen.default_next_screen,
            service_code: "*384*6086#".to_string(),
            function: screen.function,
            // input_identifier: screen.input_identifier,
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
            log::warn!(
                "Skipping service {} due to missing function_name or data_key",
                name
            );
            continue;
        }

        let mut max_service_id: u64 = 0;
        for s in ctx.db.ussd_service().iter() {
            if s.id > max_service_id {
                max_service_id = s.id;
            }
        }
        let new_service_id = max_service_id + 1;

        ctx.db.ussd_service().insert(USSDService {
            id: new_service_id,
            ussd_menu: menu.id,
            name: name.clone(),
            function_name: service.function_name.clone(),
            function_url: service.function_url.clone(),
            data_key: service.data_key.clone(),
        });

    }
    
    //Register all functions
    register_functions();
    
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
            authenticated: false,
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


#[reducer]
pub fn process_ussd_step(
    ctx: &ReducerContext,
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    text: String,
) {
    if let Some(_menu) = ctx.db.ussd_menu().service_code().find(service_code.clone()) {
        //Does phone number EsimProfile exist in DB?

        let _profile = _check_profile_exists(ctx, phone_number.clone());

        let _session = _check_session_exists(ctx, session_id.clone());


        // Handle USSD Session processing
        //Get or create session
        let mut current_session = session_update_or_create(
            ctx,
            session_id.clone(),
            phone_number,
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
        let display_text = current_screen.clone().expect("Screen not found ::display").display(&ctx);
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
#[reducer]
pub fn validate_canceltx(ctx: &ReducerContext, session_id: String, input: String) {
    let swap = ctx
        .db
        .swap()
        .iter()
        .find(|s| s.session_id == session_id.clone());
    if let Some(swap) = swap {
        let mut updated_swap = swap.clone();
        if input.trim() == "2" {
            updated_swap.status = SwapStatus::Cancelled;
            log::info!("Swap for session {} cancelled.", session_id);
        } else if input.trim() == "1" {
            updated_swap.status = SwapStatus::Processing;
            log::info!("Swap for session {} confirmed for processing.", session_id);
        }
        ctx.db.swap().id().update(updated_swap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
// pub fn claim_swap(ctx: &ReducerContext, id: u64) {
//     if let Some(s) = ctx.db.swap().id().find(id) {
//         if let SwapStatus::Pending = s.status {
//             let updated = Swap {
//                 status: SwapStatus::Processing,
//                 updated_at: ctx.timestamp,
//                 ..s
//             };
//             ctx.db.swap().id().update(updated);
//         }
//     }
// }

    #[allow(dead_code)]
    fn setup_common_test_db(ctx: &mut ReducerContext) {
        let menu = ctx.db.ussd_menu().insert(USSDMenu {
            id: 1,
            service_code: "*4337#".to_string(),
        });

        ctx.db.ussd_screen().insert(USSDScreen {
            id: 1,
            ussd_menu: menu.id,
            name: "EnterPin".to_string(),
            text: "Enter your PIN".to_string(),
            screen_type: ScreenType::Function,
            default_next_screen: "ConfirmPin".to_string(),
            service_code: "*4337#".to_string(),
            function: Some("validate_pin".to_string()),
            input_identifier: None,
        });

        ctx.db.ussd_service().insert(USSDServiceRow {
            id: 1,
            ussd_menu: menu.id,
            name: "validate_pin".to_string(),
            function_name: "validate_pin_function".to_string(),
            function_url: None,
            data_key: "pin".to_string(),
        });

        ctx.db.ussd_screen().insert(USSDScreen {
            id: 2,
            ussd_menu: menu.id,
            name: "QuitScreen".to_string(),
            text: "Thank you for using our service.".to_string(),
            screen_type: ScreenType::Quit,
            default_next_screen: "".to_string(),
            service_code: "*4337#".to_string(),
            function: None,
            input_identifier: None,
        });

        ctx.db.ussd_screen().insert(USSDScreen {
            id: 3,
            ussd_menu: menu.id,
            name: "ConfirmCancelTx".to_string(),
            text: "Confirm or cancel transaction".to_string(),
            screen_type: ScreenType::Function,
            default_next_screen: "TransactionResult".to_string(),
            service_code: "*4337#".to_string(),
            function: Some("validate_canceltx".to_string()),
            input_identifier: None,
        });

        ctx.db.ussd_service().insert(USSDServiceRow {
            id: 2,
            ussd_menu: menu.id,
            name: "validate_canceltx".to_string(),
            function_name: "validate_canceltx".to_string(),
            function_url: None,
            data_key: "cancel_tx".to_string(),
        });
    }

    #[test]
    fn test_ussd_session_struct_fields() {
        let session = USSDSession {
            session_id: "sess1".to_string(),
            phone_number: "+254792281871".to_string(),
            network_code: "net1".to_string(),
            service_code: "*4337#".to_string(),
            data: "testdata".to_string(),
            current_screen: "screen1".to_string(),
            visited_screens: vec!["screen0".to_string()],
            last_interaction_time: Timestamp::now(),
            end_session: false,
            sender: Identity::from_byte_array([1; 32]),
            online: true,
            authenticated: false,
        };
        assert_eq!(session.session_id, "sess1");
        assert_eq!(session.phone_number, "+254792281871");
        assert!(session.online);
        assert!(!session.end_session);
        assert_eq!(session.visited_screens.len(), 1);
    }

    #[test]
    fn test_swap_struct_fields() {
        let swap = Swap {
            id: 1,
            session_id: "sess1".to_string(),
            from_address: "0xfrom".to_string(),
            to_address: "0xto".to_string(),
            amount: "1000".to_string(),
            token_in: "ETH".to_string(),
            token_out: "USD".to_string(),
            status: SwapStatus::Pending,
            tx_hash: Some("0xhash".to_string()),
            gas_price: Some("100".to_string()),
            gas_limit: Some("21000".to_string()),
            nonce: Some(1),
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
            error_message: None,
            swap_type: SwapType::SendEth,
        };
        assert_eq!(swap.session_id, "sess1");
        assert_eq!(swap.from_address, "0xfrom");
        assert_eq!(swap.status, SwapStatus::Pending);
        assert_eq!(swap.swap_type, SwapType::SendEth);
        assert!(swap.tx_hash.is_some());
    }

    #[test]
    fn test_swap_status_enum() {
        assert_eq!(format!("{:?}", SwapStatus::Pending), "Pending");
        assert_eq!(format!("{:?}", SwapStatus::Completed), "Completed");
        assert_ne!(SwapStatus::Failed, SwapStatus::Processing);
    }

    // Pure helper to map reducer error codes to screen names. Kept pure so it can be unit tested
    // without requiring a live ReducerContext or linking SpacetimeDB native libraries.
    pub fn map_amount_error_code(code: &str) -> &'static str {
        match code {
            "amount_too_low" => "AmountTooLowScreen",
            "amount_invalid" => "AmountInvalidScreen",
            _ => "FailureScreen",
        }
    }

    #[test]
    fn test_map_amount_error_code() {
        assert_eq!(
            map_amount_error_code("amount_too_low"),
            "AmountTooLowScreen"
        );
        assert_eq!(
            map_amount_error_code("amount_invalid"),
            "AmountInvalidScreen"
        );
        assert_eq!(map_amount_error_code("unknown"), "FailureScreen");
    }

    // NOTE: Disabled test - incompatible with SpacetimeDB 1.4.0 API
    // #[test]
    // fn test_execute_function_screen_updates_current_screen() {
    //     let mut ctx = ReducerContext::__dummy();
    //     let session_id = "test_session_123".to_string();
    //     let sender = Identity::from_byte_array([0; 32]);

    //     // This test would need to be rewritten for SpacetimeDB 1.4.0
    //     // The API for testing has changed significantly
    // }
}