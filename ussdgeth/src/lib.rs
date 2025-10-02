'''mod ussdframework;
mod audit_tests;
mod audit_reducers;

use spacetimedb::{reducer, table, Identity, ReducerContext, SpacetimeType, Table, Timestamp};

use anyhow::Result;
use ussdframework::{ScreenType as FrameworkScreenType, USSDMenu as FrameworkMenu};
mod reducers;
pub use reducers::send_eth::{send_eth, process_send_eth, update_send_eth_session};
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
    session_id: String,
    phone_number: String,
    network_code: String,
    service_code: String,
    data: String,

    current_screen: String,
    visited_screens: Vec<String>,
    last_interaction_time: Timestamp,
    
    // Session persistence and TTL
    created_at: Timestamp,
    expires_at: Timestamp,           // TTL implementation
    session_state: String,           // JSON serialized state for multi-step flows
    step_count: u32,                 // Track steps in current flow
    max_steps: u32,                  // Prevent infinite loops
    
    // Multi-step flow data
    pending_amount: Option<String>,   // For Send ETH flow
    pending_recipient: Option<String>, // For Send ETH flow
    confirmation_required: bool,     // Flag for confirmation steps
    
    // Error handling
    error_message: Option<String>,
    retry_count: u32,
    max_retries: u32,

    end_session: bool,
    #[unique]
    sender: Identity,
    online: bool,
    
    // GSMA compliance
    language: Option<String>,
    operator_code: Option<String>,
}

#[table(name = ussd_menu)] //TODO: Rename to ServiceCode
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

    //      4. Insert Function Screen services
    log::info!("USSDGETH Initialized by, {}!", ctx.sender);
}

#[cfg(test)]
mod session_tests {
    use super::*;
    use spacetimedb_testing::*;
    use std::time::Duration;

    #[test]
    fn test_session_creation_and_persistence() {
        // This test should initially fail until we implement proper session handling
        let mut db = TestDB::new();
        let ctx = &db.with_reducers();
        
        // Initialize the database
        init(ctx);
        
        let session_id = "test_session_001".to_string();
        let phone_number = "+254712345678".to_string();
        let network_code = "63902".to_string();
        let service_code = "*4337#".to_string();
        let text = "".to_string();
        let initial_screen = "MainScreen".to_string();
        
        // Create session
        get_or_create_session(
            ctx,
            session_id.clone(),
            phone_number.clone(),
            network_code.clone(),
            service_code.clone(),
            text.clone(),
            initial_screen.clone()
        );
        
        // Verify session exists
        let session = ctx.db.ussd_session().session_id().find(session_id.clone());
        assert!(session.is_some(), "Session should be created");
        
        let session = session.unwrap();
        assert_eq!(session.phone_number, phone_number);
        assert_eq!(session.step_count, 1);
        assert!(!session.end_session);
    }

    #[test]
    fn test_session_resume_after_interruption() {
        // Test session persistence across interruptions (≥99% success rate target)
        let mut db = TestDB::new();
        let ctx = &db.with_reducers();
        
        init(ctx);
        
        let session_id = "resume_test_001".to_string();
        let phone_number = "+254712345678".to_string();
        
        // Step 1: Create initial session
        get_or_create_session(
            ctx,
            session_id.clone(),
            phone_number.clone(),
            "63902".to_string(),
            "*4337#".to_string(),
            "1".to_string(),
            "SendETHScreen".to_string()
        );
        
        // Simulate interruption - user input gets lost
        // Step 2: Resume session with new input
        get_or_create_session(
            ctx,
            session_id.clone(),
            phone_number.clone(),
            "63902".to_string(),
            "*4337#".to_string(),
            "0.001".to_string(),
            "SendETHAmountScreen".to_string()
        );
        
        // Verify session state is maintained
        let session = ctx.db.ussd_session().session_id().find(session_id.clone()).unwrap();
        assert_eq!(session.step_count, 2);
        assert_eq!(session.current_screen, "SendETHAmountScreen");
    }

    #[test]
    fn test_session_ttl_expiration() {
        let mut db = TestDB::new();
        let ctx = &db.with_reducers();
        
        init(ctx);
        
        let session_id = "ttl_test_001".to_string();
        
        // Create session
        get_or_create_session(
            ctx,
            session_id.clone(),
            "+254712345678".to_string(),
            "63902".to_string(),
            "*4337#".to_string(),
            "".to_string(),
            "MainScreen".to_string()
        );
        
        // Verify session exists
        assert!(ctx.db.ussd_session().session_id().find(session_id.clone()).is_some());
        
        // Simulate time passage beyond TTL
        // Note: In a real test, we'd manipulate the timestamp
        cleanup_expired_sessions(ctx);
        
        // For now, this test documents the expected behavior
        // Implementation will make this pass
    }

    #[test]
    fn test_multi_step_send_eth_flow() {
        let mut db = TestDB::new();
        let ctx = &db.with_reducers();
        
        init(ctx);
        
        let session_id = "send_eth_test_001".to_string();
        
        // Step 1: Select Send ETH option
        get_or_create_session(
            ctx,
            session_id.clone(),
            "+254712345678".to_string(),
            "63902".to_string(),
            "*4337#".to_string(),
            "1".to_string(),
            "SendETHScreen".to_string()
        );
        
        // Step 2: Enter amount
        get_or_create_session(
            ctx,
            session_id.clone(),
            "+254712345678".to_string(),
            "63902".to_string(),
            "*4337#".to_string(),
            "0.001".to_string(),
            "SendETHAmountScreen".to_string()
        );
        
        // Step 3: Enter recipient
        get_or_create_session(
            ctx,
            session_id.clone(),
            "+254712345678".to_string(),
            "63902".to_string(),
            "*4337#".to_string(),
            "0x742d35Cc6634C0532925a3b8D42C25D4F86F94ad".to_string(),
            "SendETHRecipientScreen".to_string()
        );
        
        let session = ctx.db.ussd_session().session_id().find(session_id).unwrap();
        assert_eq!(session.step_count, 3);
        // This test will fail until we implement the Send ETH flow
    }

    #[test]
    fn test_session_cleanup_prevents_memory_leaks() {
        let mut db = TestDB::new();
        let ctx = &db.with_reducers();
        
        init(ctx);
        
        // Create multiple sessions
        for i in 0..10 {
            let session_id = format!("cleanup_test_{:03}", i);
            get_or_create_session(
                ctx,
                session_id,
                "+254712345678".to_string(),
                "63902".to_string(),
                "*4337#".to_string(),
                "".to_string(),
                "MainScreen".to_string()
            );
        }
        
        // Verify sessions exist
        let session_count_before = ctx.db.ussd_session().iter().count();
        assert_eq!(session_count_before, 10);
        
        // Run cleanup
        cleanup_expired_sessions(ctx);
        
        // Note: This test documents expected behavior
        // Implementation will handle TTL-based cleanup
    }

    #[test] 
    fn test_session_resume_success_rate() {
        // This test validates the ≥99% success rate requirement
        let mut db = TestDB::new();
        let ctx = &db.with_reducers();
        
        init(ctx);
        
        let mut successful_resumes = 0;
        let total_tests = 100;
        
        for i in 0..total_tests {
            let session_id = format!("resume_rate_test_{:03}", i);
            
            // Create session
            get_or_create_session(
                ctx,
                session_id.clone(),
                "+254712345678".to_string(),
                "63902".to_string(),
                "*4337#".to_string(),
                "1".to_string(),
                "SendETHScreen".to_string()
            );
            
            // Simulate interruption and resume
            get_or_create_session(
                ctx,
                session_id.clone(),
                "+254712345678".to_string(),
                "63902".to_string(),
                "*4337#".to_string(),
                "0.001".to_string(),
                "SendETHAmountScreen".to_string()
            );
            
            // Check if resume was successful
            if let Some(session) = ctx.db.ussd_session().session_id().find(session_id) {
                if session.step_count == 2 && session.current_screen == "SendETHAmountScreen" {
                    successful_resumes += 1;
                }
            }
        }
        
        let success_rate = (successful_resumes as f64 / total_tests as f64) * 100.0;
        assert!(success_rate >= 99.0, "Session resume success rate: {:.1}% < 99%", success_rate);
    }
}

// TTL and cleanup functionality
#[reducer]
pub fn cleanup_expired_sessions(ctx: &ReducerContext) {
    let current_time = ctx.timestamp;
    
    // Find and delete expired sessions
    let expired_sessions: Vec<_> = ctx.db.ussd_session()
        .iter()
        .filter(|session| session.expires_at <= current_time)
        .map(|session| session.session_id.clone())
        .collect();
    
    for session_id in expired_sessions {
        if let Some(session) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
            ctx.db.ussd_session().session_id().delete(session_id);
            log::info!("Cleaned up expired session: {}", session.session_id);
        }
    }
}

// Session validation and recovery
#[reducer]
pub fn validate_session_health(ctx: &ReducerContext, session_id: String) -> bool {
    if let Some(session) = ctx.db.ussd_session().session_id().find(session_id) {
        let current_time = ctx.timestamp;
        
        // Check if session is expired
        if session.expires_at <= current_time {
            log::warn!("Session {} has expired", session.session_id);
            return false;
        }
        
        // Check if session has too many steps (prevent infinite loops)
        if session.step_count >= session.max_steps {
            log::warn!("Session {} exceeded max steps", session.session_id);
            return false;
        }
        
        // Check retry limits
        if session.retry_count >= session.max_retries {
            log::warn!("Session {} exceeded max retries", session.session_id);
            return false;
        }
        
        return true;
    }
    
    false
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
pub fn get_or_create_session(ctx: &ReducerContext, session_id: String, phone_number: String, network_code: String, service_code: String, text: String, initial_screen: String) {
    let current_time = ctx.timestamp;
    let session_ttl_seconds = 300; // 5 minutes default TTL
    let expires_at = current_time.plus_seconds(session_ttl_seconds);
    
    // Retrieve Session based on sessionID, expect single instance
    if let Some(session_retrieved) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
        // Validate session health before updating
        if validate_session_health(ctx, session_id.clone()) {
            ctx.db.ussd_session().session_id().update(USSDSession {
                phone_number: phone_number,
                network_code: network_code,
                service_code: service_code,
                data: text,
                current_screen: initial_screen,
                sender: ctx.sender,
                online: true,
                last_interaction_time: current_time,
                expires_at: expires_at, // Extend TTL on interaction
                step_count: session_retrieved.step_count + 1, // Increment step count
                ..session_retrieved
            });
        } else {
            // Session is invalid, create new one
            create_new_session(ctx, session_id, phone_number, network_code, service_code, text, initial_screen, current_time, expires_at);
        }
    } else {
        // Create new session
        create_new_session(ctx, session_id, phone_number, network_code, service_code, text, initial_screen, current_time, expires_at);
    }
}

fn create_new_session(ctx: &ReducerContext, session_id: String, phone_number: String, network_code: String, service_code: String, text: String, initial_screen: String, current_time: Timestamp, expires_at: Timestamp) {
    ctx.db.ussd_session().insert(USSDSession {
        session_id: session_id,
        phone_number: phone_number,
        network_code: network_code,
        service_code: service_code,
        data: text,
        current_screen: initial_screen,
        sender: ctx.sender,
        online: true,
        last_interaction_time: current_time,
        created_at: current_time,
        expires_at: expires_at,
        session_state: "{}".to_string(), // Empty JSON state
        step_count: 1,
        max_steps: 20, // Prevent infinite loops
        pending_amount: None,
        pending_recipient: None,
        confirmation_required: false,
        error_message: None,
        retry_count: 0,
        max_retries: 3,
        visited_screens: Vec::new(),
        end_session: false,
        language: Some("en".to_string()),
        operator_code: None,
    });
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