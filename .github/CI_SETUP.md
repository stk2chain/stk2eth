# CI/CD Pipeline Setup Guide

## Overview

This document describes the continuous integration and deployment pipeline for STK2ETH.

## Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      CI/CD Pipeline Flow                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Lint & Format Check (Fast fail)                            │
│     ├─ cargo fmt --check                                        │
│     └─ cargo clippy                                             │
│                                                                  │
│  2. Parallel Testing                                            │
│     ├─ Unit Tests (cargo test --lib --bins)                    │
│     ├─ Contract Tests (forge test)                             │
│     └─ Security Audit (cargo audit)                            │
│                                                                  │
│  3. Integration Tests                                           │
│     ├─ Send ETH Flow (Pending→InProgress→Executed→Confirmed)  │
│     └─ E2E Test (< 60s timeout requirement)                    │
│                                                                  │
│  4. Stress Tests                                                │
│     ├─ Concurrent Swap Sessions (100 sessions)                 │
│     ├─ Sustained Load Test (30s duration)                      │
│     └─ Race Condition Detection                                │
│                                                                  │
│  5. Build Verification                                          │
│     ├─ x86_64-unknown-linux-gnu                                │
│     └─ x86_64-unknown-linux-musl                               │
│                                                                  │
│  6. Coverage & Reporting                                        │
│     ├─ cargo-tarpaulin (Code coverage)                         │
│     └─ Codecov upload                                          │
│                                                                  │
│  7. Deploy to Staging (develop branch only)                    │
│     └─ SpacetimeDB deployment                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Required Secrets

Configure these secrets in GitHub repository settings:

### SpacetimeDB Secrets

- `SPACETIME_TOKEN`: Authentication token for SpacetimeDB CLI
- `SPACETIME_STAGING_SERVER`: Staging server hostname (e.g., `testnet`)
- `SPACETIME_DB_ID`: Database identifier for reducer calls

### Coverage Secrets

- `CODECOV_TOKEN`: Token for uploading coverage reports to Codecov

## Success Criteria

### 1. Code Quality

- All code must pass `cargo fmt --check`
- All code must pass `cargo clippy` with zero warnings
- No security vulnerabilities in dependencies

### 2. Testing

- 100% of unit tests must pass
- All integration tests must pass
- E2E test must complete in < 60 seconds
- Stress test success rate ≥ 95%

### 3. Build

- All workspace members must build successfully
- Release builds must complete without errors

### 4. Coverage

- Target: ≥ 70% code coverage
- Coverage reports uploaded to Codecov

## Local Testing

### Run full CI pipeline locally:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Lint
cargo clippy --all-targets --all-features -- -D warnings

# 3. Unit tests
cargo test --workspace --lib --bins

# 4. Integration tests
cargo test --workspace --test '*' -- --test-threads=1

# 5. Contract tests
cd contracts && forge test

# 6. Stress tests
CONCURRENT_SESSIONS=100 TEST_DURATION=30 cargo test --test stress_test -- --ignored

# 7. Security audit
cargo audit

# 8. Build
cargo build --release --workspace
```

### Quick local verification:

```bash
make verify
```

## Performance Benchmarks

### Target Metrics

- **Unit Tests**: Complete in < 5 minutes
- **Integration Tests**: Complete in < 10 minutes
- **Stress Tests**: Complete in < 5 minutes
- **Full Pipeline**: Complete in < 20 minutes
- **E2E Test**: Complete in < 60 seconds **Critical**

### Actual Metrics (Updated per run)

See GitHub Actions workflow run history for current metrics.

## Stress Test Configuration

### Environment Variables

- `CONCURRENT_SESSIONS`: Number of concurrent swap sessions (default: 100)
- `TEST_DURATION`: Duration of sustained load test in seconds (default: 30)
- `INTEGRATION_TEST`: Set to "true" to run integration tests

### Stress Test Scenarios

1. **Concurrent Sessions**: Tests 100 simultaneous swap operations
2. **Sustained Load**: Continuous swap operations for 30 seconds
3. **Race Conditions**: 50 iterations of rapid state transitions
4. **Session Isolation**: Verify 20 concurrent sessions maintain isolation
5. **Memory Usage**: Create 1000 sessions to test memory management

## Caching Strategy

### Cargo Cache

- Registry index and cache
- Git database
- Target directory

### Foundry Cache

- Forge cache directory
- Contract build artifacts

### Cache Keys

- Primary: `${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}`
- Restore: `${{ runner.os }}-cargo-`

## Deployment

### Staging Deployment

- **Trigger**: Push to `develop` branch
- **Target**: SpacetimeDB staging environment
- **Database**: `stk2eth-staging`
- **Notification**: Console output with deployment URL

### Production Deployment

- **Manual trigger required** via GitHub Actions UI
- Requires approval from repository maintainer

## Troubleshooting

### Common Issues

#### 1. Test Timeout

```
Error: E2E test exceeded 60s timeout
```

**Solution**: Optimize reducers or increase timeout threshold

#### 2. Clippy Warnings

```
Error: clippy found warnings
```

**Solution**: Run `cargo clippy --fix` locally and commit changes

#### 3. Format Check Failed

```
Error: Diff in .github/workflows/ci.yml
```

**Solution**: Run `cargo fmt --all` and commit changes

#### 4. Stress Test Failures

```
Error: Success rate should be >= 95%
```

**Solution**: Check SpacetimeDB connection and reducer performance

### Getting Help

- Check workflow logs in GitHub Actions
- Review [CONTRIBUTING.md](../CONTRIBUTING.md)
- Open an issue with `ci` label

## Badge Status

Add these badges to your PR description:

```markdown
[![CI Status](https://github.com/stk2chain/stk2eth/actions/workflows/ci.yml/badge.svg?branch=your-branch)](https://github.com/stk2chain/stk2eth/actions/workflows/ci.yml)
```

## Maintenance

### Weekly Tasks

- Review and update dependencies
- Check for security advisories
- Monitor CI performance metrics

### Monthly Tasks

- Review and optimize caching strategy
- Update stress test thresholds
- Review code coverage trends

## Metrics & Monitoring

Track these metrics over time:

- Build duration
- Test success rate
- Code coverage percentage
- Deployment frequency
- Mean time to recovery (MTTR)

## References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [Foundry Book](https://book.getfoundry.sh/)
- [SpacetimeDB Documentation](https://spacetimedb.com/docs)
