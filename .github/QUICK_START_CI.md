# Quick Start: CI/CD Pipeline

## Activate Your CI Pipeline in 5 Minutes

### Step 1: Configure GitHub Secrets

Go to your repository → Settings → Secrets and variables → Actions

Add these secrets:

```bash
# SpacetimeDB Configuration
SPACETIME_TOKEN=<your-spacetime-token>
SPACETIME_STAGING_SERVER=testnet
SPACETIME_DB_ID=<your-database-id>

# Optional: Code Coverage
CODECOV_TOKEN=<your-codecov-token>
```

### Step 2: Enable GitHub Actions

1. Go to repository → Actions tab
2. Click "I understand my workflows, go ahead and enable them"
3. The CI pipeline will run automatically on next push

### Step 3: Push to Trigger CI

```bash
git add .
git commit -m "feat(ci): Enable CI/CD pipeline"
git push origin develop
```

### Step 4: Monitor First Build

1. Go to Actions tab
2. Click on the running workflow
3. Watch jobs execute in parallel
4. Wait for all checks to pass

## Verification Checklist

After first successful build:

- [ ] All CI jobs show green checkmarks
- [ ] README.md badges display correctly
- [ ] Coverage report uploaded (if Codecov configured)
- [ ] No security vulnerabilities found
- [ ] Build artifacts uploaded

## What Happens on Each Push

### Automatic Checks (Every Push/PR):

1. **Lint & Format** (~2 min)

   - Code style verification
   - Clippy linting

2. **Tests** (~10 min)

   - Unit tests with coverage
   - Integration tests
   - Contract tests

3. **Stress Tests** (~3 min)

   - 100 concurrent sessions
   - Performance metrics

4. **Build** (~6 min)

   - Multi-target compilation
   - Artifact generation

5. **Security** (~2 min)
   - Dependency audit
   - Vulnerability scanning

### On `develop` Branch:

- All above checks PLUS
- **Automatic deployment to staging**

## Local Testing Before Push

Save time by running checks locally:

```bash
# Quick check (recommended before every commit)
cargo fmt --all -- --check && cargo clippy --workspace

# Full verification (recommended before pushing)
make verify

# Run specific test suites
cargo test --workspace --lib                    # Unit tests (~2 min)
cargo test --workspace --test '*'               # Integration tests (~5 min)
CONCURRENT_SESSIONS=10 cargo test --test stress_test -- --ignored  # Stress tests (~1 min)
```

## Understanding CI Results

### All Green (Success)

```
Lint & Format
Unit Tests
Contract Tests
Integration Tests
Stress Tests
Build
Security Audit
```

**Action**: Your PR is ready to merge!

### Some Red (Failure)

#### Format Check Failed

```bash
# Fix it:
cargo fmt --all
git add . && git commit --amend --no-edit && git push --force
```

#### Clippy Warnings

```bash
# Fix it:
cargo clippy --workspace --fix --allow-dirty
git add . && git commit -m "fix: Address clippy warnings" && git push
```

#### Test Failures

```bash
# Debug it:
cargo test --workspace -- --nocapture
# Fix the failing test, then:
git add . && git commit -m "fix: Fix failing tests" && git push
```

#### E2E Timeout (> 60s)

```bash
# This is critical - optimize your code
# Check which operations are slow:
RUST_LOG=debug cargo test --test integration_send_eth -- send_eth_flow_test
```

## 🔍 Monitoring & Badges

### Status Badges

Add to your PR description:

```markdown
[![CI](https://github.com/stk2chain/stk2eth/actions/workflows/ci.yml/badge.svg?branch=your-branch)](https://github.com/stk2chain/stk2eth/actions/workflows/ci.yml)
```

### View Detailed Results

- Click on the badge in README.md
- Or go to Actions tab → Select workflow run
- Click on failed job to see logs

## 🎓 Best Practices

### Before Creating PR:

1. ✅ Run `make verify` locally
2. ✅ Ensure all tests pass
3. ✅ Check that formatting is correct
4. ✅ Run security audit

### During PR Review:

1. ✅ Monitor CI results
2. ✅ Fix any failures immediately
3. ✅ Check coverage hasn't decreased
4. ✅ Verify E2E test is < 60s

### After PR Merge:

1. ✅ Verify staging deployment succeeded
2. ✅ Run smoke tests on staging
3. ✅ Monitor for any issues

## 🚨 Troubleshooting

### CI is not running

- Check that GitHub Actions is enabled
- Verify workflow file syntax
- Check branch protection rules

### Secrets not working

- Verify secret names match exactly
- Secrets are case-sensitive
- Re-create secrets if unsure

### Slow CI builds

- Check cache is working (see build logs)
- Reduce parallelism if hitting rate limits
- Consider self-hosted runners for large projects

### Flaky tests

- Use `--test-threads=1` for serial execution
- Add proper waits/timeouts in integration tests
- Mock external dependencies

## 📚 Further Reading

- [Full CI Setup Guide](.github/CI_SETUP.md)
- [Implementation Summary](.github/CI_IMPLEMENTATION_SUMMARY.md)
- [Contributing Guidelines](../CONTRIBUTING.md)

## 🆘 Need Help?

1. Check workflow logs for error details
2. Review [CI_SETUP.md](CI_SETUP.md) for configuration
3. Open an issue with `ci` label
4. Tag maintainers for urgent issues

---

**Ready to go!** Push your code and watch the magic happen! 🎉
