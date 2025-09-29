mod ussdframework;

use spacetimedb::{reducer, table, Identity, ReducerContext, SpacetimeType, Table, Timestamp};

use anyhow::Result;
use ussdframework::{ScreenType as FrameworkScreenType, USSDMenu as FrameworkMenu};
mod reducers;
pub use reducers::send_eth::send_eth;
// #[table(name = ussd_session)]
// pub struct USSDSession {
//     #[primary_key]
//     sessionId: String,              // sessionId *PK
//     phoneNumber: String,                  // msisdn/phoneNumber
//     networkCode: String,            //networkCode
//     serviceCode: String,            //serviceCode
//     data: String,                   //data/text

//     current_screen: String,
//     visited_screens: Vec<String>,
//     last_interaction_time: Timestamp,

//     error_message: Option<String>,
//     //displayed: HashMap<String, bool>,

//     end_session: bool,
//     //language: String,

//     identity: Identity,
//     online: bool,

//     //position_in_menu
// }
#[table(name = ussd_session)]
pub struct USSDSession {
    #[primary_key]
    session_id: String, // sessionId *PK
    phone_number: String, // msisdn/phoneNumber
    network_code: String, //networkCode
    service_code: String, //serviceCode
    data: String,         //data/text

    current_screen: String,
    visited_screens: Vec<String>,
    last_interaction_time: Timestamp,

    // error_message: Option<String>,
    //displayed: HashMap<String, bool>,
    end_session: bool,
    //language: String,
    #[unique]
    sender: Identity,
    online: bool,
    //position_in_menu
}

#[table(name = ussd_menu)] //TODO: Rename to ServiceCode
pub struct USSDMenu {
    #[primary_key]
    #[auto_inc]
    id: u64,
    #[unique]
    service_code: String, //*4337# V1
                          //networkCode: String,          //99999 V2
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

#[derive(SpacetimeType)]
pub enum ScreenType {
    //#[default]
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
    //menu_items
    function: Option<String>,
    //router_options
    input_identifier: Option<String>,
    name: String, //Screen Name
}

#[table(name = menu_item)]
pub struct USSDMenuItem {
    option: String,
    display_name: String,
    next_screen: String,
    name: String, //MenuItem name
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
    pub amount: String,    // Store as string to avoid precision issues
    pub token_in: String,  // "ETH", "USDC", etc.
    pub token_out: String, // "ETH", "USDC", etc.

    pub status: SwapStatus, // "pending", "processing", "completed", "failed"
    pub tx_hash: Option<String>, // Ethereum transaction hash once submitted
    pub gas_price: Option<String>,
    pub gas_limit: Option<String>,
    pub nonce: Option<u64>,

    pub created_at: Timestamp,
    pub updated_at: Timestamp,

    pub error_message: Option<String>, // If failed, store error details
    pub swap_type: SwapType,           // "send_eth", "token_swap", "cash_out", etc.
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

// Initialize SessionID USSD Menu from json
// USSDMenu: List of USSDScreens
//      ScreenTypes: (Initial, Menu, Input, Function, Router, Quit)
//      USSDScreen:
//          text
//          screen_type
//          default_next_screen
//          service_code
//          menu_items -> Menu
//          function
//          router_options -> Router
//          input_identifier -> Input
//          input_type
#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Called when the module is initially published (constructor)
    // Fetch USSD Menu ABI
    let content = include_str!("./data/menu.json");
    let menu_screens: FrameworkMenu = match serde_json::from_str(content) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Failed to parse ussd menu json: {:?}", e);
            return;
        }
    };

    // log::info!("USSDGETH Menu Screens, {:?}!", menu_screens);

    // Write to DB: (Insert DB Rows)
    //      1. Insert USSDMenu Row with ID: TODO: Seperate Service Code
    // If the menu already exists (re-publish or re-init) reuse it instead of inserting
    let menu = if let Some(existing) = ctx.db.ussd_menu().service_code().find("*4337#".to_string())
    {
        existing
    } else {
        ctx.db.ussd_menu().insert(USSDMenu {
            id: 0,
            service_code: "*4337#".to_string(),
        })
    };

    //      2. Insert USSDScreen Rows linked to USSD Menu(ServiceCode)
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

        //      3. Insert Other Sub USSDScreen Rows linked to USSDScreen
        //      3.1 menu_items
        if let Some(menu_items) = screen.menu_items {
            // If screen.type == Menu
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

        //      3.2 router_options
        if let Some(router_options) = screen.router_options {
            // If screen.type == Router
            for option in router_options {
                ctx.db.router_option().insert(USSDRouterOption {
                    router_option: option.router_option,
                    next_screen: option.next_screen,
                });
            }
        }
    }
    //      4. Insert Function Screen services
    for (name, service) in menu_screens.services.into_iter() {
        // defensive: ensure function_name and data_key exist
        if service.function_name.trim().is_empty() || service.data_key.trim().is_empty() {
            log::warn!(
                "Skipping service {} due to missing function_name or data_key",
                name
            );
            continue;
        }

        // Avoid primary-key collisions by allocating a new id based on the current max id
        let mut max_service_id: u64 = 0;
        for s in ctx.db.ussd_service().iter() {
            if s.id > max_service_id {
                max_service_id = s.id;
            }
        }
        let new_service_id = max_service_id + 1;

        // Directly insert, since catch_unwind cannot be used with &ReducerContext
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    // Called everytime a new client connects
    // we set online to true
    log::info!("Client  Connected, {}!", ctx.sender);
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    // Called everytime a client disconnects
    // we set online to false
    // Get the current session to continue processing
    if let Some(session_retrieved) = ctx.db.ussd_session().sender().find(ctx.sender) {
        //ctx.db.ussd_session().sender().update( USSDSession {online:false, ..session_retrieved} );
        // Basic implementation - just log for now
        log::info!(
            "Processing USSD for session: {}",
            session_retrieved.session_id
        );
        // log::info!("Client disconnected, {:?}@{:?}!", ctx.sender, ctx.timestamp);
    } else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to disconnect without connecting first.
        log::warn!(
            "Disconnect event for unknown user with identity {:?}@{:?}",
            ctx.sender,
            ctx.timestamp
        );
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
    //let retrieved_session = USSDSession::retrieve_session(&request.session_id, &cache);
    // Retrieve Session based on sessionID, expect single instance
    // we set online to true
    if let Some(session_retrieved) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
        ctx.db.ussd_session().session_id().update(USSDSession {
            phone_number,
            network_code,
            service_code,
            data: text,
            current_screen: initial_screen,
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

//  #[reducer]
pub fn get_initial_screen(ctx: &ReducerContext) -> String {
    for screen in ctx.db.ussd_screen().iter() {
        if let ScreenType::Initial = screen.screen_type {
            return screen.name.clone();
        }
    }
    "InitialScreen".to_string() // Change panic to return default
}

// #[reducer]
// pub fn execute_screen(ctx: &ReducerContext, &text: String, &menu){
//     let services = ctx.db.ussd_service().ussd_menu_id_pk().filter(menu);
// }

#[reducer]
pub fn handle_ussd(
    ctx: &ReducerContext,
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    text: String,
) {
    // log::info!("handle_ussd, {}:{}:{}:{}:{}:{}!", ctx.sender, sessionId, phoneNumber, networkCode, serviceCode, text);

    // Load Menus
    if let Some(menu) = ctx.db.ussd_menu().service_code().find(service_code.clone()) {
        let _screens = ctx.db.ussd_screen().ussd_menu().filter(menu.id);

        let initial_screen = get_initial_screen(ctx);

        //1. Retrieve or Generate Session
        get_or_create_session(
            ctx,
            session_id.clone(),
            phone_number,
            network_code,
            service_code,
            text.clone(),
            initial_screen.clone(),
        );

        // fetch the session we just created/updated so we can inspect current_screen
        let session = match ctx.db.ussd_session().session_id().find(session_id.clone()) {
            Some(s) => s,
            None => {
                log::error!("Failed to retrieve session after create for {}", session_id);
                return;
            }
        };

        // If the current screen is a Function screen or has a function, enqueue a USSDRequest
        // find the screen definition
        let mut screen_opt: Option<USSDScreen> = None;
        for s in ctx.db.ussd_screen().iter() {
            if s.name == session.current_screen {
                screen_opt = Some(s);
                break;
            }
        }

        if let Some(screen_def) = screen_opt {
            if let ScreenType::Function = screen_def.screen_type {
                // determine function name
                if let Some(func_name) = screen_def.function.clone() {
                    // find service by name or data_key
                    let mut svc_opt = None;
                    for svc in ctx.db.ussd_service().iter() {
                        if svc.name == func_name
                            || svc.data_key == func_name
                            || svc.function_name == func_name
                        {
                            svc_opt = Some(svc);
                            break;
                        }
                    }

                    if let Some(svc) = svc_opt {
                        // allocate id for new request
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

                        // Call the appropriate function based on service
                        if svc.function_name == "send_eth" {
                            // For send_eth, we need to extract the data from the session
                            // This is a simplified implementation - in practice, you'd parse the session data
                            // to extract from_address, to_address, and amount from the user inputs
                            let from_address =
                                "0x0000000000000000000000000000000000000000".to_string(); // Placeholder
                            let to_address =
                                "0x0000000000000000000000000000000000000000".to_string(); // Placeholder
                            let amount = "0.0".to_string(); // Placeholder

                            // Call the send_eth reducer
                            send_eth(
                                ctx,
                                session.session_id.clone(),
                                from_address,
                                to_address,
                                amount,
                            );
                        }

                        log::info!(
                            "Enqueued USSDRequest {} for service {} and called function",
                            new_req_id,
                            svc.name
                        );
                    } else {
                        log::warn!("No service found for function {}", func_name);
                    }
                }
            }
        }
    } else {
        // This branch should be unreachable,
        log::warn!("Unknown Menu serviceCode {}", service_code);
    }

    // let initial_screen = screens.get_initial_screen();

    //let mut session = USSDSession::get_or_create_session(request, &initial_screen, session_cache);
    // let retrieved_session = USSDSession::retrieve_session(&request.session_id, &cache);

    //2. Create a response object
    // let mut response: USSDResponse = USSDResponse {
    //     msisdn: phoneNumber.clone(),
    //     session_id: sessionId.clone(),
    //     end_session: session.end_session,
    //     message: "Something went wrong, please try again later".to_string(),
    // };

    //2.1 Display screen history
    // session.display_screen_history();

    //3. Get current screent
    // let mut current_screen = session.current_screen.clone();
}
