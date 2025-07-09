mod ussdframework;

use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp, SpacetimeType};
use ussdframework::{
    USSDMenu as FrameworkMenu,
    ScreenType as FrameworkScreenType
};

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
    if let Some(session_retrieved) = ctx.db.ussd_session().sender().find(ctx.sender){
        ctx.db.ussd_session().sender().update( USSDSession {online:false, ..session_retrieved} );
        log::info!("Client disconnected, {:?}@{:?}!", ctx.sender, ctx.timestamp);
    }else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to disconnect without connecting first.
        log::warn!("Disconnect event for unknown user with identity {:?}@{:?}", ctx.sender, ctx.timestamp);
    }
}



#[reducer]
 pub fn get_or_create_session(ctx: &ReducerContext, session_id: String, phone_number: String, network_code: String, service_code:String,  text: String, initial_screen: String){
    //let retrieved_session = USSDSession::retrieve_session(&request.session_id, &cache);
    // Retrieve Session based on sessionID, expect single instance
    // we set online to true
    if let Some(session_retrieved) = ctx.db.ussd_session().session_id().find(session_id.clone()){
        ctx.db.ussd_session().session_id().update( USSDSession {
            phone_number: phone_number,
            network_code: network_code,
            service_code: service_code,
            data: text,
            current_screen: initial_screen,
            sender: ctx.sender,
            online:true,
            last_interaction_time: ctx.timestamp,
            ..session_retrieved
        });

    }else {
        ctx.db.ussd_session().insert( USSDSession {
            session_id: session_id,
            phone_number: phone_number,
            network_code: network_code,
            service_code: service_code,
            data: text,
            current_screen: initial_screen,
            sender: ctx.sender,
            online:true,
            last_interaction_time: ctx.timestamp,
            visited_screens: Vec::new(),
            end_session: false

        });
    }
 }


//  #[reducer]
pub fn get_initial_screen(ctx: &ReducerContext) -> String {
    for screen in ctx.db.ussd_screen().iter() {
        if let ScreenType::Initial = screen.screen_type {
            return screen.name;
        }
    }
    panic!("No initial screen found!");
}

// #[reducer]
// pub fn execute_screen(ctx: &ReducerContext, &text: String, &menu){
//     let services = ctx.db.ussd_service().ussd_menu_id_pk().filter(menu);
// }


#[reducer]
pub fn handle_ussd(ctx: &ReducerContext, sessionId: String, phoneNumber: String, networkCode: String, serviceCode:String,  text: String){


    // log::info!("handle_ussd, {}:{}:{}:{}:{}:{}!", ctx.sender, sessionId, phoneNumber, networkCode, serviceCode, text);


    // Load Menus
    if let Some(menu)= ctx.db.ussd_menu().service_code().find(serviceCode.clone()){
        let screens = ctx.db.ussd_screen().ussd_menu().filter(menu.id);

        let initial_screen= get_initial_screen(ctx);

        //1. Retrieve or Generate Session
        let mut session = get_or_create_session(ctx, sessionId.clone(), phoneNumber, networkCode, serviceCode, text, initial_screen);

    }else {
        // This branch should be unreachable,
        log::warn!("Unknown Menu serviceCode {}", serviceCode);

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
