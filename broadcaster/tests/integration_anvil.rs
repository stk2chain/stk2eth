// broadcaster/tests/integration_anvil.rs
// End-to-end broadcaster tests against an anvil fork of Base Sepolia and a
// local SpacetimeDB running the `gateway2` module.
//
// Run with: cargo test -p broadcaster --test integration_anvil -- --ignored --test-threads=1
//
// Prereqs: anvil on PATH, spacetime CLI on PATH, SpacetimeDB listening on 127.0.0.1:3000.

mod helpers;

use helpers::{anvil_available, Anvil};

#[tokio::test]
#[ignore]
async fn test_end_to_end_send_eth_flow() {
    if !anvil_available() {
        eprintln!("SKIP: anvil not on PATH");
        return;
    }
    let _anvil = Anvil::spawn_base_sepolia_fork();

    // The full flow exercises: publish middleware, claim identity, register phone,
    // submit send-eth, assert Confirmed. Scaffold here is minimal; the implementer
    // expands in Task 18.
    //
    // Steps:
    // 1. Reset SpacetimeDB: `spacetime delete gateway2; spacetime publish ...`
    // 2. `spacetime call gateway2 claim_gateway_identity`
    // 3. Drive USSD register flow via spacetime calls (mirrors test_register_pin_flow.sh)
    // 4. Fund the derived burner via anvil `anvil_setBalance`
    // 5. Start broadcaster pointing at anvil + local STDB
    // 6. Insert a Pending eth_tx row (via send_eth USSD flow)
    // 7. Wait up to 30s for status = Confirmed
    // 8. Assert recipient received the ETH on anvil

    assert!(true, "scaffold placeholder — concrete test added in Task 18");
}
