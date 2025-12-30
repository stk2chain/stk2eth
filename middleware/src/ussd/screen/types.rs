use spacetimedb::SpacetimeType;
use serde::{Deserialize, Serialize};

#[derive(SpacetimeType, Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ScreenType {
    Initial,
    Menu,
    Input,
    Function,
    Router,
    Quit
}