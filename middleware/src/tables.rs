use spacetimedb::{table, SpacetimeType, Timestamp};

// ============================================================
// Withdrawal request (Pretium off-ramp)
// ============================================================

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum WithdrawalStatus {
    Pending,
    Escrowed,
    Processing,
    Fulfilled,
    Failed,
    Refunded,
}

#[table(name = withdrawal_request, public)]
pub struct WithdrawalRequest {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub phone_number: String,
    pub fiat_amount: String,
    pub currency: String,
    pub escrow_eth_tx_id: u64,
    pub refund_eth_tx_id: Option<u64>,
    pub pretium_ref: Option<String>,
    pub status: WithdrawalStatus,
    pub error_reason: Option<String>,
    pub processing_by: Option<String>,
    pub processing_since: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================
// SMS notification
// ============================================================

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SmsTemplate {
    RegSuccess,
    TxSubmitted,
    TxConfirmed,
    TxFailed,
    InboundEth,
    WithdrawInit,
    WithdrawSent,
    WithdrawFailed,
    PinLocked,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum SmsStatus {
    Pending,
    Processing,
    Sent,
    Failed,
}

#[table(name = sms_notification, public)]
pub struct SmsNotification {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub phone_number: String,
    pub template: SmsTemplate,
    pub payload_json: String,
    pub status: SmsStatus,
    pub message_id: Option<String>,
    pub error_reason: Option<String>,
    pub processing_by: Option<String>,
    pub processing_since: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================
// Balance query
// ============================================================

#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq)]
pub enum QueryStatus {
    Pending,
    Done,
    Failed,
}

#[table(name = balance_query, public)]
pub struct BalanceQuery {
    #[primary_key]
    pub session_id: String,
    pub phone_number: String,
    pub wallet_address: String,
    pub status: QueryStatus,
    pub result_wei: Option<String>,
    pub error_reason: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ============================================================
// User preferences
// ============================================================

#[table(name = user_preferences, public)]
pub struct UserPreferences {
    #[primary_key]
    pub phone_number: String,
    pub base_token: String,
    pub default_withdraw_method: Option<String>,
    pub updated_at: Timestamp,
}
