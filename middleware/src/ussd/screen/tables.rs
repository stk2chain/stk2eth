use spacetimedb::{table, Timestamp};
use super::types::ScreenType;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[table(name = ussd_code)]
pub struct USSDCode {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    pub service_code: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[table(name = menu_item)]
pub struct USSDMenuItem {
    pub option: String,
    pub display_name: String,
    pub next_screen: String,
    pub name: String,
    #[index(btree)]
    pub screen: u64,
}

#[derive(Debug, Clone, PartialEq)]
#[table(name = ussd_screen)]
pub struct USSDScreen {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    pub name: String,
    pub screen_type: ScreenType,
    pub default_next_screen: String,
    #[index(btree)]
    pub ussd_code: u64,
    pub text: String,
    pub function: Option<String>,
}


