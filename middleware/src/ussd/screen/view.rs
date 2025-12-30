use spacetimedb::ReducerContext;
use crate::ussd::screen::tables::{menu_item, USSDScreen, USSDMenuItem};
use crate::ussd::screen::types::ScreenType;

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

    pub fn get_sorted_menu_items(&self, ctx: &ReducerContext) -> Vec<USSDMenuItem> {
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
}
