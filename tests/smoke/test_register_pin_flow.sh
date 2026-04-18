#!/usr/bin/env bash
# End-to-end smoke test for middleware Task 1-12 changes.
# Exercises: register_pin -> confirm_register_pin -> esim_profile + auth_7702 + user_pin rows.
# Requires: docker-compose up middleware; spacetime CLI available.
set -euo pipefail

DB="gateway2"
SESSION="smoke-$(date +%s)"
PHONE="+254712345678"
NETWORK="99999"
SERVICE="*384*6086#"

echo "=== 1. reset tables for fresh test ==="
spacetime sql "$DB" "DELETE FROM esim_profile WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "DELETE FROM user_pin     WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "DELETE FROM auth_7702    WHERE 1=1" || true  # best-effort
spacetime sql "$DB" "DELETE FROM ussd_session WHERE phone_number = '$PHONE'"

echo "=== 2. simulate dial (no text) — expect RegisterScreen ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" ""
RESP=$(spacetime sql "$DB" "SELECT response_text FROM ussd_response WHERE session_id = '$SESSION'" | tail -1)
echo "Response: $RESP"
echo "$RESP" | grep -q "Register" || { echo "FAIL: expected Register menu"; exit 1; }

echo "=== 3. pick '1. Register' ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1"

echo "=== 4. enter PIN '1379' ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*1379"

echo "=== 5. confirm PIN '1*1379*1379' ==="
spacetime call "$DB" process_ussd_step "$SESSION" "$PHONE" "$NETWORK" "$SERVICE" "1*1379*1379"

echo "=== 6. verify rows created ==="
spacetime sql "$DB" "SELECT phone_number, wallet_address FROM esim_profile WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "SELECT phone_number, locked, attempts FROM user_pin WHERE phone_number = '$PHONE'"
spacetime sql "$DB" "SELECT authority_address, status FROM auth_7702 ORDER BY created_at DESC LIMIT 1"

PROFILE_COUNT=$(spacetime sql "$DB" "SELECT COUNT(*) FROM esim_profile WHERE phone_number = '$PHONE'" | tail -1 | tr -d ' ')
[ "$PROFILE_COUNT" = "1" ] || { echo "FAIL: expected 1 esim_profile row, got $PROFILE_COUNT"; exit 1; }

PIN_COUNT=$(spacetime sql "$DB" "SELECT COUNT(*) FROM user_pin WHERE phone_number = '$PHONE'" | tail -1 | tr -d ' ')
[ "$PIN_COUNT" = "1" ] || { echo "FAIL: expected 1 user_pin row, got $PIN_COUNT"; exit 1; }

AUTH_COUNT=$(spacetime sql "$DB" "SELECT COUNT(*) FROM auth_7702" | tail -1 | tr -d ' ')
[ "$AUTH_COUNT" -ge "1" ] || { echo "FAIL: expected ≥1 auth_7702 row, got $AUTH_COUNT"; exit 1; }

echo "=== PASS ==="
