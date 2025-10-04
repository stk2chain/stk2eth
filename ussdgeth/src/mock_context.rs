// Minimal mock_context module for tests
use crate::{USSDSession, USSDScreen, USSDMenu, USSDServiceRow};
use spacetimedb::{Identity, Timestamp};

pub struct MockReducerContext {
    pub db: MockDatabase,
    pub sender: Identity,
    pub timestamp: Timestamp,
}

impl MockReducerContext {
    pub fn new() -> Self {
        Self {
            db: MockDatabase::new(),
            sender: Identity::from_byte_array([0; 32]),
            timestamp: Timestamp::now(),
        }
    }
}

impl Default for MockReducerContext {
    fn default() -> Self {
        Self::new()
    }
}
pub struct MockDatabase;

impl Default for MockDatabase {
    fn default() -> Self {
        MockDatabase::new()
    }
}

impl MockDatabase {
    pub fn new() -> Self { Self }
    pub fn ussd_session(&self) -> MockTable<USSDSession> { MockTable::new() }
    pub fn ussd_screen(&self) -> MockTable<USSDScreen> { MockTable::new() }
    pub fn ussd_menu(&self) -> MockTable<USSDMenu> { MockTable::new() }
    pub fn ussd_service(&self) -> MockTable<USSDServiceRow> { MockTable::new() }
    pub fn create_tables(&self) {}
}

pub struct MockTable<T> {
    data: Vec<T>,
}

impl<T> MockTable<T> {
    pub fn new() -> Self { Self { data: Vec::new() } }
    pub fn insert(&mut self, item: T) { self.data.push(item); }
    pub fn session_id(&mut self) -> &mut Self { self }
    pub fn find(&self, _id: String) -> Option<&T> { self.data.first() }
}
