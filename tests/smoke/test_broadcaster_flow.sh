#!/usr/bin/env bash
# Live Base Sepolia smoke test for the broadcaster.
#
# Prereqs:
#  - Local SpacetimeDB with `gateway2` published and claim_gateway_identity called
#  - Broadcaster binary running (cargo run -p broadcaster) with env pointing at
#    Alchemy Base Sepolia + the local SpacetimeDB
#  - The derived burner for +254712345678 has ≥ 0.01 ETH + gas reserve on Base Sepolia
#
# Run:  ./tests/smoke/test_broadcaster_flow.sh

set -euo pipefail

DB="gateway2"
PHONE="+254712345678"
PHONE_NORM="254712345678"
SERVICE="*384*6086#"
NETWORK="99999"
RECV="+254700000099"
PIN="1379"

echo "=== 1. ensure profile + pin exist (register if not) ==="
PROFILE=$(spacetime sql "$DB" "SELECT phone_number FROM esim_profile WHERE phone_number = '$PHONE_NORM'")
if ! echo "$PROFILE" | grep -q "$PHONE_NORM"; then
    SESSION="smoke-reg-$(date +%s)"
    spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" ""
    spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1"
    spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*$PIN"
    spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*$PIN*$PIN"
fi

BURNER=$(spacetime sql "$DB" "SELECT wallet_address FROM esim_profile WHERE phone_number = '$PHONE_NORM'" | grep -oE '[0-9a-f]{40}' | head -1)
[ -n "$BURNER" ] || { echo "FAIL: no burner derived"; exit 1; }
echo "Burner: 0x$BURNER"

echo "=== 2. send-eth flow ==="
SESSION="smoke-send-$(date +%s)"
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" ""
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1"
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*$RECV"
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*$RECV*0.01"
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*$RECV*0.01*$PIN"
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*$RECV*0.01*$PIN*1"

echo "=== 3. poll for confirmation (60s max) ==="
for i in $(seq 1 30); do
    STATUS=$(spacetime sql "$DB" "SELECT status FROM eth_tx WHERE session_id = '$SESSION'" | tail -1 || true)
    echo "t=${i}*2s  status=$STATUS"
    if echo "$STATUS" | grep -q "Confirmed"; then
        HASH=$(spacetime sql "$DB" "SELECT tx_hash FROM eth_tx WHERE session_id = '$SESSION'" | grep -oE '0x[0-9a-f]{64}' | head -1)
        echo "=== PASS === tx=$HASH"
        echo "Basescan: https://sepolia.basescan.org/tx/$HASH"
        exit 0
    fi
    if echo "$STATUS" | grep -q "Failed"; then
        echo "=== FAIL === eth_tx marked Failed"
        spacetime sql "$DB" "SELECT * FROM eth_tx WHERE session_id = '$SESSION'"
        exit 1
    fi
    sleep 2
done

echo "=== FAIL === timed out waiting for Confirmed"
exit 1
