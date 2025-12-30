use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum TxType {
    SendEth,
    TokenSwap,
    CashOut,
    Balance,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum TxStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled
}


