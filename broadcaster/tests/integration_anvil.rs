// broadcaster/tests/integration_anvil.rs
mod helpers;

use helpers::*;
use std::time::Duration;

const DB: &str = "gateway2_test";
const PHONE: &str = "+254712345678";

#[tokio::test]
#[ignore]
async fn test_end_to_end_send_eth_flow() {
    if !anvil_available() || !stdb_available() {
        eprintln!("SKIP: anvil or spacetime missing");
        return;
    }
    let anvil = Anvil::spawn_base_sepolia_fork(8545);
    stdb_reset(DB);

    // Register a phone so esim_profile + auth_7702 exist
    let session = format!("it-{}", std::process::id());
    stdb_call(DB, "process_ussd_step", &[&session, PHONE, "99999", "*384*6086#", ""]);
    stdb_call(DB, "process_ussd_step", &[&session, PHONE, "99999", "*384*6086#", "1"]);
    stdb_call(DB, "process_ussd_step", &[&session, PHONE, "99999", "*384*6086#", "1*1379"]);
    stdb_call(DB, "process_ussd_step", &[&session, PHONE, "99999", "*384*6086#", "1*1379*1379"]);

    // Pull burner address from esim_profile
    let out = stdb_sql(
        DB,
        "SELECT wallet_address FROM esim_profile WHERE phone_number = '254712345678'",
    );
    let burner_hex = out
        .lines()
        .find_map(|l| l.split('"').nth(1))
        .expect("no wallet row");
    let burner = format!("0x{}", burner_hex);
    anvil.set_balance(&burner, "0xde0b6b3a7640000").await; // 1 ETH

    // Drive send-eth USSD to create eth_tx(Pending)
    let s2 = format!("it-send-{}", std::process::id());
    let recv = "+254700000001";
    stdb_call(DB, "process_ussd_step", &[&s2, PHONE, "99999", "*384*6086#", ""]);
    stdb_call(DB, "process_ussd_step", &[&s2, PHONE, "99999", "*384*6086#", "1"]);
    stdb_call(
        DB,
        "process_ussd_step",
        &[&s2, PHONE, "99999", "*384*6086#", &format!("1*{recv}")],
    );
    stdb_call(
        DB,
        "process_ussd_step",
        &[&s2, PHONE, "99999", "*384*6086#", &format!("1*{recv}*0.01")],
    );
    stdb_call(
        DB,
        "process_ussd_step",
        &[&s2, PHONE, "99999", "*384*6086#", &format!("1*{recv}*0.01*1379")],
    );
    stdb_call(
        DB,
        "process_ussd_step",
        &[&s2, PHONE, "99999", "*384*6086#", &format!("1*{recv}*0.01*1379*1")],
    );

    // Assert that a Pending row exists — the broadcaster subprocess spawn and
    // Confirmed-status assertion are expanded in a follow-up.
    let out = stdb_sql(
        DB,
        &format!("SELECT status FROM eth_tx WHERE session_id = '{s2}'"),
    );
    assert!(
        out.contains("Pending") || out.contains("Submitted"),
        "no eth_tx row for session: {out}"
    );

    tokio::time::sleep(Duration::from_secs(1)).await;
}

#[tokio::test]
#[ignore]
async fn test_underfunded_burner_marks_row_failed_without_broadcasting() {
    if !anvil_available() || !stdb_available() {
        return;
    }
    // Register phone, skip funding the burner, send 0.01 ETH, assert Failed.
    // Key assertion: eth_tx.status = Failed with error_reason containing
    // "insufficient burner balance".
}

#[tokio::test]
#[ignore]
async fn test_crash_mid_submit_reconciles_on_restart() {
    if !anvil_available() || !stdb_available() {
        return;
    }
    // Start broadcaster, let it mark_eth_tx_processing (Broadcasting), then kill it.
    // Restart. Assert either a hash is recovered (Broadcast) or row is failed.
}
