use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    Pending,
    Broadcasted,
    Failed,
}