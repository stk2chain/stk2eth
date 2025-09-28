# 🚀 STK2ETH Development Guide

## Quick Setup for Testing Session Persistence and E2E Pipeline

### Prerequisites

- Rust 1.70+
- SpacetimeDB CLI
- Docker (optional)
- Node.js (for AfricasTalking simulator)

### 1. Environment Setup

```bash
# Clone and setup
git clone <repository>
cd stk2eth

# Copy environment template
cp .env.example .env

# Install dependencies
make install
```

### 2. Configure Environment (.env)

```bash
# SpacetimeDB Configuration
SPACETIME_API_URL=http://localhost:3000/v1/database/stk2eth
SPACETIME_AUTH_TOKEN=your_auth_token_here
SPACETIME_DB_ID=stk2eth
SPACETIME_SERVER=testnet

# USSD Configuration
USSD_PORT=8080
SERVICE_CODE=*4337#

# Testing Configuration
TEST_PHONE_NUMBER=+254712345678
TEST_SESSION_ID=test_session_001
```

### 3. Run Tests (≥99% Session Resume Success Rate)

```bash
# Unit tests with session persistence validation
cargo test --package spacetime-module session_tests

# E2E pipeline tests (100% pass rate target)
cd tests
cargo test test_complete_ussd_to_ethereum_pipeline

# Session resume rate validation
cargo test test_session_resume_success_rate
```

### 4. Start Development Environment

```bash
# Terminal 1: Start SpacetimeDB
spacetime start

# Terminal 2: Deploy USSD module
make deploy-db

# Terminal 3: Start USSD client
make run-ussd

# Terminal 4: Start Ethereum client
make run-eth
```

### 5. Test AfricasTalking USSD Flow

```bash
# Manual USSD simulation
curl -X POST http://localhost:8080/ussd \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "sessionId=test_001&serviceCode=*4337#&phoneNumber=+254712345678&networkCode=63902&text="

# Expected response: "CON M-ETH Main Menu..."
```

### 6. Complete Send ETH Flow Test

```bash
# Step 1: Initial dial
curl -X POST http://localhost:8080/ussd -d "sessionId=flow_test&serviceCode=*4337#&phoneNumber=+254712345678&networkCode=63902&text="

# Step 2: Select Send ETH (option 1)
curl -X POST http://localhost:8080/ussd -d "sessionId=flow_test&serviceCode=*4337#&phoneNumber=+254712345678&networkCode=63902&text=1"

# Step 3: Enter amount
curl -X POST http://localhost:8080/ussd -d "sessionId=flow_test&serviceCode=*4337#&phoneNumber=+254712345678&networkCode=63902&text=1*0.001"

# Step 4: Enter recipient
curl -X POST http://localhost:8080/ussd -d "sessionId=flow_test&serviceCode=*4337#&phoneNumber=+254712345678&networkCode=63902&text=1*0.001*0x742d35Cc6634C0532925a3b8D42C25D4F86F94ad"

# Step 5: Confirm (option 1)
curl -X POST http://localhost:8080/ussd -d "sessionId=flow_test&serviceCode=*4337#&phoneNumber=+254712345678&networkCode=63902&text=1*0.001*0x742d35Cc6634C0532925a3b8D42C25D4F86F94ad*1"

# Step 6: Enter PIN
curl -X POST http://localhost:8080/ussd -d "sessionId=flow_test&serviceCode=*4337#&phoneNumber=+254712345678&networkCode=63902&text=1*0.001*0x742d35Cc6634C0532925a3b8D42C25D4F86F94ad*1*1234"
```

### 7. Verify Session Persistence

```bash
# Check session exists in SpacetimeDB
spacetime call stk2eth validate_session_health flow_test -s testnet

# Query session state
spacetime sql "SELECT * FROM ussd_session WHERE session_id = 'flow_test';" -s testnet

# Check transaction record
spacetime sql "SELECT * FROM swap WHERE session_id = 'flow_test';" -s testnet
```

### 8. AfricasTalking Simulator Testing

Using the [AfricasTalking USSD Simulator](https://simulator.africastalking.com:1517/):

1. Set webhook URL: `https://your-domain.com/ussd`
2. Service Code: `*4337#`
3. Test the complete flow:
   - Dial `*4337#`
   - Select `1` for Send ETH
   - Enter amount: `0.001`
   - Enter recipient: `0x742d35Cc6634C0532925a3b8D42C25D4F86F94ad`
   - Confirm: `1`
   - Enter PIN: `1234`

### 9. Session Interruption Testing

```bash
# Test session persistence across interruptions
./scripts/test_session_interruption.sh
```

### 10. Performance Validation

```bash
# Test 100 concurrent sessions (≥99% success rate)
./scripts/load_test_sessions.sh

# Verify TTL cleanup
./scripts/test_ttl_cleanup.sh
```

## Key Metrics to Validate

### Session Persistence Requirements

- ✅ Session resume success rate: **≥99%** @ 100 interrupted flows
- ✅ Session TTL: 5 minutes with automatic cleanup
- ✅ Multi-step flow state preservation
- ✅ Input validation and error handling

### E2E Pipeline Requirements

- ✅ USSD→SpacetimeDB→Ethereum pipeline: **100%** pass rate
- ✅ Complete Send ETH flow end-to-end
- ✅ Transaction recording and verification
- ✅ Session cleanup and memory management

## Troubleshooting

### Common Issues

1. **SpacetimeDB Connection Failed**

   ```bash
   # Check SpacetimeDB is running
   spacetime version
   curl http://localhost:3000/health
   ```

2. **Session Not Found**

   ```bash
   # Verify session exists
   spacetime sql "SELECT * FROM ussd_session;" -s testnet
   ```

3. **USSD Client Not Responding**

   ```bash
   # Check logs
   tail -f ussd_client.log

   # Verify port is open
   netstat -tlnp | grep 8080
   ```

4. **Tests Failing**

   ```bash
   # Run with verbose output
   cargo test -- --nocapture

   # Check environment
   env | grep SPACETIME
   ```

## Production Deployment

### 1. AfricasTalking Configuration

- Set production webhook URL in AfricasTalking dashboard
- Configure SSL certificate for HTTPS
- Set up monitoring and alerting

### 2. SpacetimeDB Production

```bash
# Deploy to production SpacetimeDB
cd ussdgeth
spacetime publish -s production.spacetimedb.com -- stk2eth_prod
```

### 3. USSD Client Deployment

```bash
# Build production release
cargo build --release --package ussdclient

# Deploy to production server
./deploy/deploy_ussd_client.sh production
```

## Monitoring and Metrics

### Key Performance Indicators

- Session creation rate: Target >100/sec
- Session resume success: ≥99%
- USSD response time: <1 second
- Database query time: <100ms
- Memory usage: <512MB per 1000 sessions
- TTL cleanup efficiency: >95%

### Dashboards and Alerts

- SpacetimeDB dashboard: Monitor session count, query performance
- Application logs: Track USSD request/response patterns
- Error tracking: Monitor failed transactions and session issues
- Load testing: Regular validation of performance targets

## Security Considerations

### Input Validation

- All USSD inputs are sanitized and validated
- Ethereum addresses verified with checksum
- Amount validation prevents overflow attacks
- PIN validation with rate limiting

### Session Security

- Session IDs are cryptographically secure
- TTL prevents session hijacking
- No sensitive data stored in session state
- All database queries use parameterized statements

### Production Security

- HTTPS required for all webhooks
- Rate limiting on USSD endpoints
- Regular security audits with `cargo audit`
- Input sanitization for all user data

---

**Target Results:**

- ✅ Session resume success rate: **≥99%** @ 100 interrupted flows
- ✅ E2E pipeline success rate: **100%** @ CI runs
- ✅ USSD→SpacetimeDB→Ethereum: Complete pipeline validated
- ✅ AfricasTalking integration: Production ready
