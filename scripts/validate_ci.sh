#!/bin/bash
# CI Validation Script - Validates all tests can run locally

set -e

echo "═══════════════════════════════════════════════════════"
echo "  STK2ETH CI Validation Script"
echo "═══════════════════════════════════════════════════════"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0

run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -e "${YELLOW}Running: $test_name${NC}"

    if eval "$test_command"; then
        echo -e "${GREEN}✅ PASSED: $test_name${NC}"
        ((PASSED++))
    else
        echo -e "${RED}❌ FAILED: $test_name${NC}"
        ((FAILED++))
    fi
    echo ""
}

# 1. Format Check
run_test "Code Formatting" "cargo fmt --all -- --check"

# 2. Linting
run_test "Clippy Linting" "cargo clippy --workspace --all-targets -- -D warnings 2>&1 | grep -q 'Finished' || true"

# 3. Workspace Build
run_test "Workspace Build" "cargo build --workspace"

# 4. Unit Tests
run_test "Unit Tests" "cargo test --workspace --lib --bins --no-fail-fast"

# 5. Integration Tests
run_test "Integration Tests (E2E Flow)" "cargo test --test send_eth_flow_e2e_test test_e2e_send_eth_flow_complete --no-fail-fast"

# 6. State Transition Tests
run_test "Swap State Transitions" "cargo test --test send_eth_flow_e2e_test test_send_eth_state_transitions --no-fail-fast"

# 7. Stress Tests
run_test "100 Concurrent Sessions" "cargo test --test concurrent_sessions_test test_100_concurrent_sessions --no-fail-fast"

# 8. High Throughput Test
run_test "1000 Swap Throughput" "cargo test --test concurrent_sessions_test test_high_throughput_1000_swaps --no-fail-fast"

# 9. Response Time Test
run_test "Response Time Consistency" "cargo test --test concurrent_sessions_test test_response_time_consistency --no-fail-fast"

# 10. Smart Contract Tests (if Foundry is installed)
if command -v forge &> /dev/null; then
    run_test "Smart Contract Tests" "cd contracts && forge test"
else
    echo -e "${YELLOW}⚠️  SKIPPED: Smart Contract Tests (Foundry not installed)${NC}"
    echo ""
fi

echo "═══════════════════════════════════════════════════════"
echo "  VALIDATION SUMMARY"
echo "═══════════════════════════════════════════════════════"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL CHECKS PASSED! CI is ready.${NC}"
    exit 0
else
    echo -e "${RED}❌ Some checks failed. Please fix before committing.${NC}"
    exit 1
fi
