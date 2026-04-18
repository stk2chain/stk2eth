
use spacetimedb::{ReducerContext, Table};
use super::tables::{USSDScreen, USSDMenuItem};
use super::types::ScreenType;
use crate::ussd::session::{ussd_session, USSDSession};
use crate::ussd::service::tables::ussd_service;
use anyhow::Result;

impl USSDScreen {
    
    pub fn execute(&self, ctx: &ReducerContext, user_input: &str, mut session: USSDSession) -> USSDSession {
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
                    match self.execute_function_screen(ctx, session.clone(), function_name) {
                        Ok(updated_session) => {
                            session = updated_session;
                            next_screen = self.default_next_screen.clone();
                            log::info!("Function executed successfully: {}", function_name);
                            log::info!("Session Data updated: {}", session.data);
                        }
                        // NB: Session MUST never be updated by a screenfn on error
                        Err(err) => {
                            session.error_text = Some(err.clone());
                            // ctx.db.ussd_session().session_id().update(session);
                            log::error!("Function execution failed: {}", err);
                        }
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

    fn execute_function_screen(&self, ctx: &ReducerContext, session: USSDSession, function_name: &str) -> Result<USSDSession, String> {
        let svc_opt = ctx.db.ussd_service().iter().find(|svc| {
                svc.function_name == function_name
            });
                
        if let Some(svc) = svc_opt {
            svc.execute_fn(ctx, session)
        } else {
            return Err(format!("Function not found for screen '{}'", self.name))
        }
    }

}