# Pull Request

## Description
<!-- Provide a brief description of the changes in this PR -->

## Type of Change
<!-- Mark the relevant option with an "x" -->
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactoring

## Related Issue
<!-- Link to the issue this PR addresses -->
Closes #

## Changes Made
<!-- List the main changes made in this PR -->
-
-
-

## Testing
<!-- Describe the tests you ran to verify your changes -->

### Local Testing
- [ ] Unit tests pass (`cargo test --workspace --lib --bins`)
- [ ] Integration tests pass (`cargo test --workspace --test '*'`)
- [ ] Linting passes (`cargo clippy --all-targets --all-features`)
- [ ] Formatting passes (`cargo fmt --all -- --check`)
- [ ] Make verify passes (`make verify`)

### CI/CD Pipeline
<!-- The CI pipeline will automatically run these checks -->
- [ ] All CI checks pass
- [ ] Code coverage is maintained or improved
- [ ] E2E test completes in < 60s
- [ ] Stress tests pass (if applicable)

## USSD → SpacetimeDB → Ethereum Pipeline
<!-- If this PR affects the core pipeline, test these components -->
- [ ] USSD rendering works correctly
- [ ] SpacetimeDB reducers function as expected
- [ ] Ethereum transactions execute successfully
- [ ] Swap state transitions work (Pending → InProgress → Executed → Confirmed)

## Performance Impact
<!-- Describe any performance implications -->
- [ ] No performance degradation
- [ ] Performance improved
- [ ] Performance impact acceptable (explain below)

**Details:**

## Security Considerations
<!-- Describe any security implications or concerns -->
- [ ] No new security vulnerabilities introduced
- [ ] Security audit passed (`cargo audit`)
- [ ] Sensitive data properly handled

## Documentation
<!-- Documentation updates -->
- [ ] Code comments added/updated
- [ ] README updated (if needed)
- [ ] API documentation updated (if needed)
- [ ] CI/CD documentation updated (if needed)

## Deployment Notes
<!-- Any special deployment considerations -->
- [ ] No special deployment steps required
- [ ] Database migration required (describe below)
- [ ] Configuration changes required (describe below)

**Details:**

## Screenshots / Logs
<!-- If applicable, add screenshots or relevant logs -->

## Checklist
<!-- Final checks before requesting review -->
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] Any dependent changes have been merged and published

## Additional Context
<!-- Add any other context about the PR here -->

---

## CI Status
The CI pipeline will run automatically. Check the status here:

[![CI Status](https://github.com/stk2chain/stk2eth/actions/workflows/ci.yml/badge.svg?branch=your-branch)](https://github.com/stk2chain/stk2eth/actions/workflows/ci.yml)
