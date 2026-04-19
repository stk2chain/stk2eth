#!/usr/bin/env bash
# Smoke test for claim_gateway_identity reducer.
# Requires: local SpacetimeDB on 127.0.0.1:3000, middleware published.
set -euo pipefail

DB="gateway2"

echo "=== 1. reset app_config gateway_identity row for a clean test ==="
spacetime sql "$DB" "DELETE FROM app_config WHERE key = 'gateway_identity'" || true

echo "=== 2. first call should succeed ==="
spacetime call "$DB" claim_gateway_identity
FIRST_ROW=$(spacetime sql "$DB" "SELECT key, value FROM app_config WHERE key = 'gateway_identity'")
echo "$FIRST_ROW" | grep -q "gateway_identity" || { echo "FAIL: first claim did not insert row"; exit 1; }

echo "=== 3. second call should be rejected (row unchanged) ==="
VALUE_BEFORE=$(spacetime sql "$DB" "SELECT value FROM app_config WHERE key = 'gateway_identity'" | tail -1)
spacetime call "$DB" claim_gateway_identity
VALUE_AFTER=$(spacetime sql "$DB" "SELECT value FROM app_config WHERE key = 'gateway_identity'" | tail -1)
[ "$VALUE_BEFORE" = "$VALUE_AFTER" ] || { echo "FAIL: second claim overwrote row"; exit 1; }

echo "=== PASS ==="
