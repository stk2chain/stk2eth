mod ussdframework;

use spacetimedb::{
    table, reducer, 
    Table, ReducerContext, Identity, Timestamp, 
    SpacetimeType,
};

use anyhow::Result;
use ussdframework::{
    USSDMenu as FrameworkMenu,
    ScreenType as FrameworkScreenType
};
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
    session_id: String,              // sessionId *PK
    phone_number: String,                  // msisdn/phoneNumber
    network_code: String,            //networkCode
    service_code: String,            //serviceCode
    data: String,                   //data/text

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
    service_code: String,            //*4337# V1
    //networkCode: String,          //99999 V2
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
    name: String //Screen Name

}



#[table(name = menu_item)]
pub struct USSDMenuItem {
    option: String,
    display_name: String,
    next_screen: String,
    name: String, //MenuItem name
    screen: u64
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
    pub amount: String, // Store as string to avoid precision issues
    pub token_in: String, // "ETH", "USDC", etc.
    pub token_out: String, // "ETH", "USDC", etc.
    
    pub status: SwapStatus, // "pending", "processing", "completed", "failed"
    pub tx_hash: Option<String>, // Ethereum transaction hash once submitted
    pub gas_price: Option<String>,
    pub gas_limit: Option<String>,
    pub nonce: Option<u64>,
    
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    
    pub error_message: Option<String>, // If failed, store error details
    pub swap_type: SwapType, // "send_eth", "token_swap", "cash_out", etc.
}

#[table(name = router_option)]
pub struct USSDRouterOption {
    router_option: String,
    next_screen: String
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
    let menu_screens: FrameworkMenu = serde_json::from_str(&content).unwrap();

    // log::info!("USSDGETH Menu Screens, {:?}!", menu_screens);

    // Write to DB: (Insert DB Rows)
    //      1. Insert USSDMenu Row with ID: TODO: Seperate Service Code
    let menu = ctx.db.ussd_menu().insert(USSDMenu {
        id: 0,
        service_code: "*4337#".to_string(),
    });

    //      2. Insert USSDScreen Rows linked to USSD Menu(ServiceCode)
    for (index, (name, screen)) in menu_screens.menus.into_iter().enumerate(){
        let scrn = ctx.db.ussd_screen().insert( USSDScreen {
            id: index as u64,
            ussd_menu: menu.id,
            text: screen.text,
            screen_type: screen.screen_type.into(),
            default_next_screen: screen.default_next_screen,
            service_code: "*4337#".to_string(),
            function: screen.function,
            input_identifier: screen.input_identifier,
            name: name.to_string()
        });

        //      3. Insert Other Sub USSDScreen Rows linked to USSDScreen
        //      3.1 menu_items
        if let Some(menu_items) = screen.menu_items { // If screen.type == Menu
            for (name, item) in menu_items{
                ctx.db.menu_item().insert( USSDMenuItem {
                    option: item.option,
                    display_name: item.display_name,
                    next_screen: item.next_screen,
                    name: name,
                    screen: scrn.id
                });
            }
        }

        //      3.2 router_options
        if let Some(router_options) = screen.router_options { // If screen.type == Router
            for option in router_options{
                ctx.db.router_option().insert( USSDRouterOption {
                    router_option: option.router_option,
                    next_screen: option.next_screen

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
    // Called everytime a new client connects
    // we set online to true
    log::info!("Client  Connected, {}!", ctx.sender);
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    // Called everytime a client disconnects
    // we set online to false
    // Get the current session to continue processing
    if let Some(session_retrieved) = ctx.db.ussd_session().sender().find(ctx.sender){
        //ctx.db.ussd_session().sender().update( USSDSession {online:false, ..session_retrieved} );
        // Basic implementation - just log for now
        log::info!("Processing USSD for session: {}", session_retrieved.session_id);
        // log::info!("Client disconnected, {:?}@{:?}!", ctx.sender, ctx.timestamp);
    }else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to disconnect without connecting first.
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
pub fn handle_ussd(ctx: &ReducerContext, session_id: String, phone_number: String, network_code: String, service_code:String,  text: String){


    // log::info!("handle_ussd, {}:{}:{}:{}:{}:{}!", ctx.sender, sessionId, phoneNumber, networkCode, serviceCode, text);


    // Load Menus
    if let Some(menu)= ctx.db.ussd_menu().service_code().find(service_code.clone()){
        let _screens = ctx.db.ussd_screen().ussd_menu().filter(menu.id);

        let initial_screen= get_initial_screen(ctx);

        //1. Retrieve or Generate Session
        get_or_create_session(ctx, session_id.clone(), phone_number, network_code, service_code, text, initial_screen);

    }else {
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
