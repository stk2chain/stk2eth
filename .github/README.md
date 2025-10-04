# CI/CD Pipeline Documentation

## 📁 Documentation Structure

```
.github/
├── README.md                       ← You are here
├── workflows/
│   └── ci.yml                      ← Main CI/CD pipeline configuration
├── CI_SETUP.md                     ← Detailed setup and configuration guide
├── CI_IMPLEMENTATION_SUMMARY.md    ← Implementation details and metrics
├── QUICK_START_CI.md               ← 5-minute activation guide
└── PULL_REQUEST_TEMPLATE.md        ← PR template with CI checklist
```

## 🎯 Quick Links

### For Developers
- **[Quick Start Guide](QUICK_START_CI.md)** - Get CI running in 5 minutes
- **[PR Template](PULL_REQUEST_TEMPLATE.md)** - Use this when creating PRs
- Local testing: Run `make verify` before pushing

### For DevOps / Maintainers
- **[CI Setup Guide](CI_SETUP.md)** - Complete pipeline architecture
- **[Implementation Summary](CI_IMPLEMENTATION_SUMMARY.md)** - Metrics and status
- **[Workflow Configuration](workflows/ci.yml)** - GitHub Actions YAML

## 🚀 What Does This CI Do?

### Automated Quality Checks
✅ Code formatting (rustfmt)
✅ Linting (clippy)
✅ Unit tests with coverage
✅ Integration tests
✅ Contract tests (Foundry)
✅ Stress tests (100 concurrent sessions)
✅ Security audits
✅ Multi-platform builds

### Performance Requirements
⏱️ E2E test must complete in < 60 seconds
📊 Success rate ≥ 95% for stress tests
🎯 100% passing required for merge

### Automatic Deployment
🚀 Auto-deploy to staging on `develop` branch

## 📊 Pipeline Visualization

```
┌─────────────────────────────────────────────────────────┐
│                    CI/CD Pipeline                        │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  On Push/PR:                                            │
│  ├── Lint & Format (2 min)        ┐                    │
│  ├── Unit Tests (5 min)           ├─ Parallel          │
│  ├── Contract Tests (2 min)       ├─ Execution         │
│  └── Security Audit (2 min)       ┘                    │
│                                                          │
│  After Basic Checks Pass:                               │
│  ├── Integration Tests (8 min)                         │
│  ├── Stress Tests (3 min)                              │
│  └── Multi-target Builds (6 min)                       │
│                                                          │
│  On Success (develop branch only):                      │
│  └── Deploy to Staging (2 min)                         │
│                                                          │
│  Total Time: ~20 minutes                                │
└─────────────────────────────────────────────────────────┘
```

## 🎓 Documentation Index

### Getting Started
1. **[Quick Start](QUICK_START_CI.md)** - Start here if you're new
2. **[CI Setup](CI_SETUP.md)** - Complete configuration guide
3. **[Implementation Summary](CI_IMPLEMENTATION_SUMMARY.md)** - What's been built

### Reference
- **Workflow File**: [workflows/ci.yml](workflows/ci.yml)
- **Configuration**: `rustfmt.toml`, `.clippy.toml`, `deny.toml` (in repo root)
- **Tests**: `tests/integration_send_eth.rs`, `tests/stress_test.rs`

### Templates
- **[Pull Request Template](PULL_REQUEST_TEMPLATE.md)** - Use when creating PRs

## 🏆 Success Criteria (Issue #18)

| Requirement | Status | Notes |
|------------|--------|-------|
| Automate tests | ✅ Complete | Full test suite automation |
| Automate deployments | ✅ Complete | Staging auto-deploy on develop |
| 100% passing required | ✅ Complete | All jobs must succeed |
| E2E test < 60s | ✅ Complete | Enforced via timeout |
| Unit tests | ✅ Complete | With coverage reporting |
| Integration tests | ✅ Complete | Full Send ETH flow |
| Lint & format | ✅ Complete | rustfmt + clippy |
| Dependency caching | ✅ Complete | Cargo + Foundry |
| Status badge | ✅ Complete | In README.md |
| Stress tests | ✅ Complete | 100 concurrent sessions |
| Coverage reports | ✅ Complete | Codecov integration |

## 📈 Monitoring

### Status Badges
![CI Status](https://github.com/stk2chain/stk2eth/actions/workflows/ci.yml/badge.svg)

### Key Metrics to Track
- Build duration (target: < 20 min)
- Test success rate (target: 100%)
- Code coverage (target: ≥ 70%)
- E2E test duration (target: < 60s)

## 🆘 Support

### Common Issues
- CI not running? Check [Quick Start](QUICK_START_CI.md#troubleshooting)
- Test failures? See [CI Setup](CI_SETUP.md#troubleshooting)
- Slow builds? Review [Implementation Summary](CI_IMPLEMENTATION_SUMMARY.md#performance-metrics)

### Get Help
1. Check documentation above
2. Review workflow logs in GitHub Actions
3. Open issue with `ci` label

## 🔄 Maintenance

### Weekly
- Review CI performance metrics
- Update dependencies
- Check for security advisories

### Monthly
- Optimize cache strategy
- Review test coverage trends
- Update documentation

---

**Status**: ✅ Production Ready
**Last Updated**: 2025-10-03
**Issue**: #18
**Maintainer**: DevOps Team
