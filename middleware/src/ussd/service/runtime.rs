use spacetimedb::ReducerContext;
use super::tables::USSDService;
use crate::ussd::session::USSDSession;
use crate::functions::dispatch;

impl USSDService {
    pub fn execute_fn(
        &self,
        ctx: &ReducerContext,
        session: USSDSession,
    ) -> Result<USSDSession, String> {
        dispatch(&self.function_name, ctx, session)
    }
}
