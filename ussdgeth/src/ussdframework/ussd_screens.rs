use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// Define types of screens
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub enum ScreenType {
    #[default]
    Initial,
    Menu,
    Input,
    Function,
    Router,
    Quit,
}

impl fmt::Display for ScreenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScreenType::Initial => write!(f, "Initial"),
            ScreenType::Menu => write!(f, "Menu"),
            ScreenType::Input => write!(f, "Input"),
            ScreenType::Function => write!(f, "Function"),
            ScreenType::Router => write!(f, "Router"),
            ScreenType::Quit => write!(f, "Quit"),
        }
    }
}

impl ScreenType {
    pub fn from_string(screen_type: &str) -> ScreenType {
        match screen_type {
            "Initial" => ScreenType::Initial,
            "Menu" => ScreenType::Menu,
            "Input" => ScreenType::Input,
            "Function" => ScreenType::Function,
            "Router" => ScreenType::Router,
            "Quit" => ScreenType::Quit,
            _ => {
                log::error!("Invalid screen type");
                ScreenType::Initial
            }
        }
    }
}

// Define structure for a screen
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct USSDScreen {
    pub text: String,
    pub screen_type: ScreenType,
    pub default_next_screen: String,
    #[serde(default)]
    pub service_code: Option<String>,
    #[serde(default)]
    pub menu_items: Option<HashMap<String, USSDMenuItems>>,
    #[serde(default)]
    pub function: Option<String>,
    #[serde(default)]
    pub router_options: Option<Vec<USSDRouterOption>>,
    #[serde(default)]
    pub input_identifier: Option<String>,
    #[serde(default)]
    pub input_type: Option<String>,
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
    pub fn display(&self) -> Option<String> {
        let mut message = String::new();

        match self.screen_type {
            ScreenType::Initial => None,
            ScreenType::Menu => {
                message.push_str(&self.text);

                if let Some(menu_items) = &self.menu_items {
                    let mut sorted_menu_items: Vec<(&String, &USSDMenuItems)> =
                        menu_items.iter().collect();
                    // Sort the menu items by their option number
                    sorted_menu_items
                        .sort_by_key(|(_, item)| item.option.parse::<usize>().unwrap());

                    for (index, (_, value)) in sorted_menu_items.iter().enumerate() {
                        message.push_str(&format!("\n{}. {}", index + 1, value.display_name));
                    }
                } else {
                    message.push_str("\nNo menu items found");
                }

                Some(message)
            }
            ScreenType::Input => {
                message.push_str(&self.text);
                Some(message)
            }
            ScreenType::Function => {
                message.push_str(&self.text);
                Some(message)
            }
            ScreenType::Router => None,
            ScreenType::Quit => {
                message.push_str(&self.text);
                Some(message)
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct USSDMenuItems {
    pub option: String,
    pub display_name: String,
    pub next_screen: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct USSDRouterOption {
    pub router_option: String,
    pub next_screen: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_function_screen() {
        let screen = USSDScreen {
            text: "Enter your PIN".to_string(),
            screen_type: ScreenType::Function,
            default_next_screen: "next_screen".to_string(),
            ..Default::default()
        };

        assert_eq!(screen.display(), Some("Enter your PIN".to_string()));
    }
}
