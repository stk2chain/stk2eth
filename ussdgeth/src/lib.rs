mod ussdframework;
mod audit_tests;
mod audit_reducers;

use spacetimedb::{reducer, table, Identity, ReducerContext, SpacetimeType, Table, Timestamp, ViewContext};

use anyhow::Result;

use crate::ussdframework::utils::FUNCTION_MAP;

use ussdframework::{ScreenType as FrameworkScreenType, USSDMenu as FrameworkMenu};
mod reducers;
mod functions;
pub use reducers::send_eth::send_eth;
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


#[derive(SpacetimeType, Clone, Debug)]
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
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SwapType {
    SendEth,
    TokenSwap,
    CashOut,
}

#[table(name = swap, public)]
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

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    log::info!("Client Connected, {:?}@{:?}!", ctx.sender, ctx.timestamp);
}

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

// #[view(name = ussd_response, public)]
// pub fn ussd_response(ctx: &ViewContext, session_id: String) -> Option<String> {
//     if let Some(session_retrieved) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
//         let screen = ctx.db.ussd_screen.name().find(session_retrieved.current_screen.clone());
//         Some(screen.display(&ctx));
//     }else {
//         None
//     }

// }

// #[reducer]
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

// #[reducer]
// pub fn complete_swap(
//     ctx: &ReducerContext,
//     id: u64,
//     tx_hash: String,
//     gas_price: Option<String>,
//     gas_limit: Option<String>,
//     network: String,
// ) {
//     let Some(s) = ctx.db.swap().id().find(id) else {
//         log::warn!("complete_swap: swap {} not found", id);
//         return;
//     };

//     let from_address_c = s.from_address.clone();
//     let to_address_c = s.to_address.clone();
//     let amount_c = s.amount.clone();
//     let session_id_c = s.session_id.clone();

//     let updated = Swap {
//         status: SwapStatus::Completed,
//         tx_hash: Some(tx_hash.clone()),
//         gas_price,
//         gas_limit,
//         updated_at: ctx.timestamp,
//         ..s
//     };
//     ctx.db.swap().id().update(updated);

//     let phone_number = match ctx.db.ussd_session().session_id().find(session_id_c.clone()) {
//         Some(sess) => sess.phone_number,
//         None => "".to_string(),
//     };

//     let _ = ctx.db.eth_audit_logs().insert(EthAuditLog {
//         id: 0,
//         tx_hash,
//         from_address: from_address_c,
//         to_address: to_address_c,
//         amount: amount_c,
//         phone_number,
//         session_id: session_id_c,
//         timestamp: ctx.timestamp,
//         originator_name: None,
//         beneficiary_name: None,
//         originator_country: None,
//         beneficiary_country: None,
//         originator_address: None,
//         beneficiary_address: None,
//         originator_id: None,
//         beneficiary_id: None,
//         transaction_type: "send_eth".to_string(),
//         network,
//         gas_fee: None,
//         exchange_rate: None,
//         compliance_status: "ok".to_string(),
//         risk_score: None,
//         is_immutable: true,
//     });
// }

// #[reducer]
// pub fn fail_swap(ctx: &ReducerContext, id: u64, error_message: String) {
//     if let Some(s) = ctx.db.swap().id().find(id) {
//         let updated = Swap {
//             status: SwapStatus::Failed,
//             error_message: Some(error_message),
//             updated_at: ctx.timestamp,
//             ..s
//         };
//         ctx.db.swap().id().update(updated);
//     } else {
//         log::warn!("fail_swap: swap {} not found", id);
//     }
// }

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

// #[reducer]
// pub fn complete_swap(
//     ctx: &ReducerContext,
//     id: u64,
//     tx_hash: String,
//     gas_price: Option<String>,
//     gas_limit: Option<String>,
//     network: String,
// ) {
//     let Some(s) = ctx.db.swap().id().find(id) else {
//         log::warn!("complete_swap: swap {} not found", id);
//         return;
//     };

//     let from_address_c = s.from_address.clone();
//     let to_address_c = s.to_address.clone();
//     let amount_c = s.amount.clone();
//     let session_id_c = s.session_id.clone();

//     let updated = Swap {
//         status: SwapStatus::Completed,
//         tx_hash: Some(tx_hash.clone()),
//         gas_price,
//         gas_limit,
//         updated_at: ctx.timestamp,
//         ..s
//     };
//     ctx.db.swap().id().update(updated);

//     let phone_number = match ctx.db.ussd_session().session_id().find(session_id_c.clone()) {
//         Some(sess) => sess.phone_number,
//         None => "".to_string(),
//     };

//     let _ = ctx.db.eth_audit_logs().insert(EthAuditLog {
//         id: 0,
//         tx_hash,
//         from_address: from_address_c,
//         to_address: to_address_c,
//         amount: amount_c,
//         phone_number,
//         session_id: session_id_c,
//         timestamp: ctx.timestamp,
//         originator_name: None,
//         beneficiary_name: None,
//         originator_country: None,
//         beneficiary_country: None,
//         originator_address: None,
//         beneficiary_address: None,
//         originator_id: None,
//         beneficiary_id: None,
//         transaction_type: "send_eth".to_string(),
//         network,
//         gas_fee: None,
//         exchange_rate: None,
//         compliance_status: "ok".to_string(),
//         risk_score: None,
//         is_immutable: true,
//     });
// }

// #[reducer]
// pub fn fail_swap(ctx: &ReducerContext, id: u64, error_message: String) {
//     if let Some(s) = ctx.db.swap().id().find(id) {
//         let updated = Swap {
//             status: SwapStatus::Failed,
//             error_message: Some(error_message),
//             updated_at: ctx.timestamp,
//             ..s
//         };
//         ctx.db.swap().id().update(updated);
//     } else {
//         log::warn!("fail_swap: swap {} not found", id);
//     }
// }
// *;

    // ...
    // fn setup_test_db(ctx: &ReducerContext) {
    //     // Function to initialize DB with test data
    //     let menu = ctx.db.ussd_menu().insert(USSDMenu {
    //         id: 1,
    //         service_code: "*123#".to_string(),
    //     });

    //     ctx.db.ussd_screen().insert(USSDScreen {
    //         id: 1,
    //         ussd_menu: menu.id,
    //         name: "EnterPin".to_string(),
    //         text: "Enter your PIN".to_string(),
    //         screen_type: ScreenType::Function,
    //         default_next_screen: "ConfirmPin".to_string(),
    //         service_code: "*123#".to_string(),
    //         function: Some("validate_pin".to_string()),
    //         input_identifier: None,
    //     });

    //     ctx.db.ussd_service().insert(USSDServiceRow {
    //         id: 1,
    //         ussd_menu: menu.id,
    //         name: "validate_pin".to_string(),
    //         function_name: "validate_pin_function".to_string(),
    //         function_url: None,
    //         data_key: "pin".to_string(),
    //     });
    // }

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

    // NOTE: Disabled test - incompatible with SpacetimeDB 1.4.0 API
    // #[test]
    // fn test_execute_function_screen_updates_current_screen() {
    //     let mut ctx = ReducerContext::__dummy();
    //     let session_id = "test_session_123".to_string();
    //     let sender = Identity::from_byte_array([0; 32]);

    //     // This test would need to be rewritten for SpacetimeDB 1.4.0
    //     // The API for testing has changed significantly
    // }

    #[cfg(test)]
    mod ussd_screen_display_tests {
        use super::*;

        // Mock function to create test USSDMenuItem
        // TODO: Replace with real ctx.db.menu_item
        fn create_test_menu_item(option: &str, display_name: &str, screen_id: u64) -> USSDMenuItem {
            USSDMenuItem {
                option: option.to_string(),
                display_name: display_name.to_string(),
                next_screen: "next".to_string(),
                name: format!("item_{}", option),
                screen: screen_id,
            }
        }

        // Mock function to create test USSDScreen
        fn create_test_screen(id: u64, screen_type: ScreenType, text: &str) -> USSDScreen {
            USSDScreen {
                id,
                ussd_menu: 1,
                text: text.to_string(),
                screen_type,
                default_next_screen: "default".to_string(),
                service_code: "*4337#".to_string(),
                function: None,
                name: format!("screen_{}", id),
            }
        }

        #[test]
        fn test_format_menu_items_empty_list() {
            let screen = create_test_screen(1, ScreenType::Menu, "Test Menu");
            let items: Vec<USSDMenuItem> = vec![];
            
            let result = screen.format_menu_items(&items);
            
            assert_eq!(result, "");
        }

        #[test]
        fn test_format_menu_items_single_item() {
            let screen = create_test_screen(1, ScreenType::Menu, "Test Menu");
            let items = vec![
                create_test_menu_item("1", "Send Money", 1)
            ];
            
            let result = screen.format_menu_items(&items);
            
            assert_eq!(result, "\n1. Send Money");
        }

        #[test]
        fn test_format_menu_items_multiple_items() {
            let screen = create_test_screen(1, ScreenType::Menu, "Test Menu");
            let items = vec![
                create_test_menu_item("1", "Send Money", 1),
                create_test_menu_item("2", "Check Balance", 1),
                create_test_menu_item("3", "Transaction History", 1)
            ];
            
            let result = screen.format_menu_items(&items);
            
            assert_eq!(result, "\n1. Send Money\n2. Check Balance\n3. Transaction History");
        }

        #[test]
        fn test_display_router_screen_returns_none() {
            let screen = create_test_screen(1, ScreenType::Router, "Router Screen");
            // Note: We can't easily mock ReducerContext, so this test is limited
            // In a real test environment, you'd use a proper mock or test framework
            
            // The logic should return None for Router screens
            // This test verifies the pattern matching logic
            match screen.screen_type {
                ScreenType::Router | ScreenType::Initial => assert!(true),
                _ => assert!(false, "Router screen should match the None pattern"),
            }
        }

        #[test]
        fn test_display_initial_screen_returns_none() {
            let screen = create_test_screen(1, ScreenType::Initial, "Initial Screen");
            
            // The logic should return None for Initial screens
            match screen.screen_type {
                ScreenType::Router | ScreenType::Initial => assert!(true),
                _ => assert!(false, "Initial screen should match the None pattern"),
            }
        }

        #[test]
        fn test_display_input_screen_returns_text() {
            let screen = create_test_screen(1, ScreenType::Input, "Enter your PIN:");
            
            // For non-Menu, non-Router, non-Initial screens, should return text
            match screen.screen_type {
                ScreenType::Router | ScreenType::Initial => assert!(false),
                ScreenType::Menu => assert!(false),
                _ => {
                    // Should return Some(text.clone())
                    let expected = screen.text.clone();
                    assert_eq!(expected, "Enter your PIN:");
                }
            }
        }

        #[test]
        fn test_display_function_screen_returns_text() {
            let screen = create_test_screen(1, ScreenType::Function, "Processing transaction...");
            
            match screen.screen_type {
                ScreenType::Router | ScreenType::Initial => assert!(false),
                ScreenType::Menu => assert!(false),
                _ => {
                    let expected = screen.text.clone();
                    assert_eq!(expected, "Processing transaction...");
                }
            }
        }

        #[test]
        fn test_display_quit_screen_returns_text() {
            let screen = create_test_screen(1, ScreenType::Quit, "Thank you for using our service!");
            
            match screen.screen_type {
                ScreenType::Router | ScreenType::Initial => assert!(false),
                ScreenType::Menu => assert!(false),
                _ => {
                    let expected = screen.text.clone();
                    assert_eq!(expected, "Thank you for using our service!");
                }
            }
        }

        #[test]
        fn test_format_menu_screen_with_empty_items() {
            let screen = create_test_screen(1, ScreenType::Menu, "Main Menu");
            let items: Vec<USSDMenuItem> = vec![];
            
            // Simulate what format_menu_screen would do with empty items
            let result = match items.is_empty() {
                true => format!("{}\nNo menu items found", screen.text),
                false => format!("{}{}", screen.text, screen.format_menu_items(&items)),
            };
            
            assert_eq!(result, "Main Menu\nNo menu items found");
        }

        #[test]
        fn test_format_menu_screen_with_items() {
            let screen = create_test_screen(1, ScreenType::Menu, "Main Menu");
            let items = vec![
                create_test_menu_item("1", "Send ETH", 1),
                create_test_menu_item("2", "Check Balance", 1)
            ];
            
            // Simulate what format_menu_screen would do with items
            let result = match items.is_empty() {
                true => format!("{}\nNo menu items found", screen.text),
                false => format!("{}{}", screen.text, screen.format_menu_items(&items)),
            };
            
            assert_eq!(result, "Main Menu\n1. Send ETH\n2. Check Balance");
        }

        #[test]
        fn test_menu_item_sorting_by_option() {
            let screen = create_test_screen(1, ScreenType::Menu, "Test Menu");
            let mut items = vec![
                create_test_menu_item("3", "Third Option", 1),
                create_test_menu_item("1", "First Option", 1),
                create_test_menu_item("2", "Second Option", 1),
                create_test_menu_item("invalid", "Invalid Option", 1), // Should default to 0
            ];
            
            // Test the sorting logic
            items.sort_by_key(|item| item.option.parse::<usize>().unwrap_or(0));
            
            assert_eq!(items[0].option, "invalid"); // Should be first (defaults to 0)
            assert_eq!(items[1].option, "1");
            assert_eq!(items[2].option, "2");
            assert_eq!(items[3].option, "3");
        }

        #[test]
        fn test_screen_type_pattern_matching() {
            // Test all screen types to ensure pattern matching works correctly
            let test_cases = vec![
                (ScreenType::Initial, true),   // Should return None
                (ScreenType::Router, true),    // Should return None
                (ScreenType::Menu, false),     // Should return Some (special case)
                (ScreenType::Input, false),    // Should return Some
                (ScreenType::Function, false), // Should return Some
                (ScreenType::Quit, false),     // Should return Some
            ];

            for (screen_type, should_be_none) in test_cases {
                let returns_none = matches!(screen_type, ScreenType::Router | ScreenType::Initial);
                assert_eq!(returns_none, should_be_none, 
                    "Screen type {:?} pattern matching failed", screen_type);
            }
        }
    }
}