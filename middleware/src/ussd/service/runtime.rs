use spacetimedb::ReducerContext;
use super::tables::USSDService;
use super::utils::FUNCTION_MAP;
use crate::ussd::session::USSDSession;
use crate::functions::register_functions;


impl USSDService {
    
    pub fn load_function(&self) -> Box<dyn Fn(&ReducerContext, USSDSession) -> Result<USSDSession, String> + '_> {
        //Ensure functions are registered
        register_functions();
        
        // Load the function from the registered functions
        let func = { 
            let map = FUNCTION_MAP.lock().unwrap();
            log::info!("Function map: {:?}", map);
            map.get(&self.function_name).cloned()
        };

        match func {
            Some(f) => {
                log::info!("Function found: {}", self.function_name);
                Box::new(f)
            }
            None => {
                log::error!("Function not found: {}", self.function_name);
                Box::new(|_ctx: &ReducerContext, _session: USSDSession| {
                    Err(format!("Function '{}' not found", self.function_name))
                })
            }
        }
    }
}

