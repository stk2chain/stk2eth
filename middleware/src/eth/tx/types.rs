use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum TxType {
    SendEth,
    WithdrawEscrow,
    WithdrawRefund,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum TxStatus {
    Pending,
    Submitted,
    Broadcasting,
    Broadcast,
    Confirmed,
    Failed,
    Cancelled,
}
