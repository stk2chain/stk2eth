// FATF Travel Rule Compliance Reducers for STK2ETH
use super::*;
use spacetimedb::{reducer, ReducerContext};

// FATF Compliance: Core reducer for logging Send ETH transactions
#[reducer]
pub fn log_send_eth_transaction(
    ctx: &ReducerContext,
    tx_hash: String,
    from_address: String,
    to_address: String,
    amount: String,
    phone_number: String,
    session_id: String
) {
    // Validate inputs
    if tx_hash.is_empty() || from_address.is_empty() || to_address.is_empty() {
        log::error!("Invalid transaction data for audit log");
        return;
    }

    // Check for duplicate transactions
    let existing_logs: Vec<_> = ctx.db.eth_audit_logs().tx_hash().filter(&tx_hash).collect();
    if !existing_logs.is_empty() {
        log::warn!("Duplicate transaction hash detected: {}", tx_hash);
        return;
    }

    // Create audit log entry
    let audit_log = EthAuditLog {
        id: 0, // auto-increment
        tx_hash: tx_hash.clone(),
        from_address,
        to_address,
        amount,
        phone_number,
        session_id,
        timestamp: ctx.timestamp,

        // FATF fields - initially None, can be updated later
        originator_name: None,
        beneficiary_name: None,
        originator_country: None,
        beneficiary_country: None,
        originator_address: None,
        beneficiary_address: None,
        originator_id: None,
        beneficiary_id: None,

        // Default compliance fields
        transaction_type: "SEND_ETH".to_string(),
        network: "ethereum".to_string(),
        gas_fee: None,
        exchange_rate: None,
        compliance_status: "PENDING".to_string(),
        risk_score: None,

        // Immutability guarantee
        is_immutable: true,
    };

    ctx.db.eth_audit_logs().insert(audit_log);

    log::info!("Audit log created for transaction: {}", tx_hash);
}

// Enhanced reducer with full FATF travel rule data
#[reducer]
pub fn log_send_eth_transaction_with_fatf(
    ctx: &ReducerContext,
    tx_hash: String,
    from_address: String,
    to_address: String,
    amount: String,
    phone_number: String,
    session_id: String,
    originator_name: Option<String>,
    beneficiary_name: Option<String>,
    originator_country: Option<String>,
    beneficiary_country: Option<String>,
    originator_address: Option<String>,
    beneficiary_address: Option<String>,
    gas_fee: Option<String>,
    exchange_rate: Option<String>
) {
    // Validate inputs
    if tx_hash.is_empty() || from_address.is_empty() || to_address.is_empty() {
        log::error!("Invalid transaction data for FATF audit log");
        return;
    }

    // Check for duplicate transactions
    let existing_logs: Vec<_> = ctx.db.eth_audit_logs().tx_hash().filter(&tx_hash).collect();
    if !existing_logs.is_empty() {
        log::warn!("Duplicate transaction hash detected: {}", tx_hash);
        return;
    }

    // Parse amount to determine risk score and compliance status
    let amount_wei: u64 = amount.parse().unwrap_or(0);
    let amount_eth = amount_wei as f64 / 1e18;

    // FATF travel rule typically applies to transactions >= $3000 USD
    // Assuming 1 ETH ≈ $3000 for simplicity
    let requires_fatf = amount_eth >= 1.0;
    let (compliance_status, risk_score) = if requires_fatf {
        match (&originator_name, &beneficiary_name) {
            (Some(_), Some(_)) => ("COMPLIANT".to_string(), Some(10)),
            _ => ("FLAGGED".to_string(), Some(80))
        }
    } else {
        ("COMPLIANT".to_string(), Some(5))
    };

    // Create comprehensive audit log entry
    let audit_log = EthAuditLog {
        id: 0, // auto-increment
        tx_hash: tx_hash.clone(),
        from_address,
        to_address,
        amount,
        phone_number,
        session_id,
        timestamp: ctx.timestamp,

        // FATF travel rule data
        originator_name,
        beneficiary_name,
        originator_country,
        beneficiary_country,
        originator_address,
        beneficiary_address,
        originator_id: None, // Could be added based on KYC data
        beneficiary_id: None,

        // Transaction metadata
        transaction_type: "SEND_ETH".to_string(),
        network: "ethereum".to_string(),
        gas_fee,
        exchange_rate,
        compliance_status,
        risk_score,

        // Immutability guarantee
        is_immutable: true,
    };

    ctx.db.eth_audit_logs().insert(audit_log);

    log::info!("FATF-compliant audit log created for transaction: {} (Risk Score: {:?})",
               tx_hash, risk_score);
}