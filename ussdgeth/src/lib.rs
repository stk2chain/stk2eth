mod ussdframework;

use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp, SpacetimeType};
use ussdframework::{
    USSDMenu as FrameworkMenu,
    ScreenType as FrameworkScreenType
};

#[table(name = ussd_session)]
pub struct USSDSession {
    #[primary_key]
    sessionId: String,              // sessionId *PK
    phoneNumber: String,                  // msisdn/phoneNumber
    networkCode: String,            //networkCode
    serviceCode: String,            //serviceCode
    data: String,                   //data/text

    current_screen: String,
    visited_screens: Vec<String>,
    last_interaction_time: Timestamp,

    error_message: Option<String>,
    //displayed: HashMap<String, bool>,

    end_session: bool,
    //language: String,

    identity: Identity,
    online: bool,

    //position_in_menu
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
    #[unique]
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
    for (name, screen) in menu_screens.menus{
        let scrn = ctx.db.ussd_screen().insert( USSDScreen {
            id: 0,
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
    log::info!("USSDGETH Ininialized by, {}!", ctx.sender);
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
    log::info!("Client disconnected, {}!", ctx.sender);
}



// #[reducer]
//  pub fn get_or_create_session(ctx: &ReducerContext, sessionId: String, initial_screen: String){
//     //let retrieved_session = USSDSession::retrieve_session(&request.session_id, &cache);
//     // Retrieve Session based on sessionID, expect single instance
//     let session_retrieved = ctx.db.ussd_session().sessionID().find(sessionId);
//  }


// #[reducer]
// pub fn execute_screen(ctx: &ReducerContext, &text: String, &menu){
//     let services = ctx.db.ussd_service().ussd_menu_id_pk().filter(menu);
// }


#[reducer]
pub fn handle_ussd(ctx: &ReducerContext, sessionId: String, phoneNumber: String, networkCode: String, serviceCode:String,  text: String){


    log::info!("handle_ussd, {}:{}:{}:{}:{}:{}!", ctx.sender, sessionId, phoneNumber, networkCode, serviceCode, text);


    // Load Menus
    // let content = include_str!("../examples/data/menu.json");
    // let menus: USSDMenu = serde_json::from_str(&content).unwrap();
    // let menu = ctx.db.ussd_menu().serviceCode().find(serviceCode);


    //1. Retrieve or Generate Session
    // let mut session = get_or_create_session(ctx, &sessionId, &initial_screen);
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
