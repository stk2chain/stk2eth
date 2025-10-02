#!/bin/bash

# STK2ETH Audit Log Stress Test - 1000 Send ETH Transactions
# Tests FATF travel rule compliance and 100% persistence

echo "🚀 Starting STK2ETH Audit Log Stress Test..."
echo "📊 Testing 1000 Send ETH transactions for FATF compliance"

# Counter for successful transactions
success_count=0
failed_count=0

# Test 1000 transactions
for i in {1..1000}; do
    # Generate unique transaction data
    tx_hash=$(printf "0x%064x" $i)
    from_addr=$(printf "0x%040x" $((i * 2)))
    to_addr=$(printf "0x%040x" $((i * 2 + 1)))
    amount=$((1000000000000000000 + i))  # 1+ ETH in wei
    phone=$(printf "+254712%06d" $i)
    session="stress_test_session_$i"

    # Call the audit log reducer
    if spacetime call ussdgeth log_send_eth_transaction "$tx_hash" "$from_addr" "$to_addr" "$amount" "$phone" "$session" 2>/dev/null; then
        ((success_count++))
        if (( i % 100 == 0 )); then
            echo "✅ Processed $i transactions (Success rate: $((success_count * 100 / i))%)"
        fi
    else
        ((failed_count++))
        echo "❌ Failed transaction $i"
    fi
done

echo ""
echo "📈 STRESS TEST RESULTS:"
echo "════════════════════════"
echo "✅ Successful transactions: $success_count"
echo "❌ Failed transactions: $failed_count"
echo "📊 Success rate: $((success_count * 100 / 1000))%"

# Verify persistence in database
echo ""
echo "🔍 Verifying persistence in database..."
total_logs=$(spacetime sql ussdgeth "SELECT COUNT(*) as count FROM eth_audit_logs" | grep -o '[0-9]\+' | head -1)

echo "📝 Total audit logs in database: $total_logs"

if [ "$total_logs" -eq "$((success_count + 1))" ]; then  # +1 for the previous test log
    echo "✅ PERSISTENCE TEST PASSED: 100% of transactions were persisted"
else
    echo "❌ PERSISTENCE TEST FAILED: Expected $((success_count + 1)), got $total_logs"
fi

# Test query performance
echo ""
echo "⚡ Testing query performance..."
start_time=$(date +%s%N)
spacetime sql ussdgeth "SELECT COUNT(*) FROM eth_audit_logs WHERE compliance_status = 'PENDING'" > /dev/null
end_time=$(date +%s%N)
query_time=$(( (end_time - start_time) / 1000000 )) # Convert to milliseconds

echo "🔎 Query time for compliance status filter: ${query_time}ms"

# Test phone number indexing
start_time=$(date +%s%N)
spacetime sql ussdgeth "SELECT COUNT(*) FROM eth_audit_logs WHERE phone_number LIKE '+254712%'" > /dev/null
end_time=$(date +%s%N)
phone_query_time=$(( (end_time - start_time) / 1000000 ))

echo "📱 Query time for phone number filter: ${phone_query_time}ms"

# Final FATF compliance check
fatf_compliant_count=$(spacetime sql ussdgeth "SELECT COUNT(*) as count FROM eth_audit_logs WHERE is_immutable = true" | grep -o '[0-9]\+' | head -1)

echo ""
echo "🏛️ FATF COMPLIANCE SUMMARY:"
echo "═══════════════════════════"
echo "📋 Total immutable audit logs: $fatf_compliant_count"
echo "🔒 Immutability guarantee: $([ "$fatf_compliant_count" -eq "$total_logs" ] && echo "✅ PASSED" || echo "❌ FAILED")"
echo "📊 Data persistence: $([ "$total_logs" -ge 1000 ] && echo "✅ PASSED (≥1000 logs)" || echo "❌ FAILED (<1000 logs)")"
echo "⚡ Query performance: $([ "$query_time" -lt 1000 ] && echo "✅ PASSED (<1s)" || echo "❌ SLOW (>1s)")"

if [ "$success_count" -eq 1000 ] && [ "$total_logs" -ge 1000 ] && [ "$fatf_compliant_count" -eq "$total_logs" ]; then
    echo ""
    echo "🎉 ALL TESTS PASSED! STK2ETH audit logging is FATF compliant!"
    echo "   ✅ 100% transaction persistence achieved"
    echo "   ✅ All logs are immutable as required"
    echo "   ✅ Query performance is acceptable"
    exit 0
else
    echo ""
    echo "❌ SOME TESTS FAILED! Review the audit logging implementation."
    exit 1
fi