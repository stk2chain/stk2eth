# Linting & Formatting Guide

This guide shows all available commands for code quality checks in the STK2ETH project.

## Quick Reference

### Using Makefile (Recommended)

```bash
# Run all quality checks (formatting + linting)
make quality

# Check code formatting
make fmt-check

# Format code automatically
make fmt

# Run clippy linting
make lint
```

### Using Cargo Directly

```bash
# Check code formatting
cargo fmt --all -- --check

# Format code automatically
cargo fmt --all

# Run clippy with warnings as errors
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Alternative: Find and format all Rust files
find . -name "*.rs" -print0 | xargs -0 rustfmt --edition 2021 --check
find . -name "*.rs" -print0 | xargs -0 rustfmt --edition 2021
```

## CI/CD Pipeline Checks

The CI/CD pipeline automatically runs these checks on every push and pull request:

### 1. Formatting Check
```yaml
- name: Check formatting
  run: cargo fmt --all -- --check
```

### 2. Clippy Linting
```yaml
- name: Run clippy
  run: cargo clippy --all-targets --all-features -- -D warnings
```

## Pre-commit Workflow

Before committing code, run:

```bash
# Option 1: Use the combined quality check
make quality

# Option 2: Run checks individually
make fmt-check
make lint

# Option 3: Auto-fix formatting issues
make fmt
```

## Common Issues & Fixes

### Issue: Unused imports
**Error:**
```
warning: unused import: `super::*`
```
**Fix:** Remove the unused import

### Issue: Length comparison to zero
**Error:**
```
warning: length comparison to zero
help: using `!is_empty` is clearer and more explicit
```
**Fix:** Replace `.len() > 0` with `!is_empty()`

### Issue: Too many function arguments
**Error:**
```
warning: this function has too many arguments (15/7)
```
**Fix:** Add `#[allow(clippy::too_many_arguments)]` attribute if necessary

### Issue: Dead code
**Error:**
```
warning: method `display` is never used
```
**Fix:** Add `#[allow(dead_code)]` attribute if the code is intentionally unused (e.g., public API)

### Issue: Trailing whitespace
**Error:**
```
Diff in file.rs:
-    let x = 1;
+    let x = 1;
```
**Fix:** Run `make fmt` to auto-fix

## Integration with Git Hooks

You can set up a pre-commit hook to automatically check code quality:

```bash
# Create .git/hooks/pre-commit
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
make quality
if [ $? -ne 0 ]; then
    echo "❌ Code quality checks failed. Please fix the issues before committing."
    exit 1
fi
EOF

chmod +x .git/hooks/pre-commit
```

## Editor Integration

### VSCode
Install the `rust-analyzer` extension and add to `settings.json`:
```json
{
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    },
    "rust-analyzer.check.command": "clippy"
}
```

### Vim/Neovim
Add to your config:
```vim
" Format on save
autocmd BufWritePre *.rs lua vim.lsp.buf.format()

" Run clippy
nnoremap <leader>c :!cargo clippy<CR>
```

## Required Checks for Merge

All pull requests must pass:
1. ✅ `cargo fmt --all -- --check` (no formatting errors)
2. ✅ `cargo clippy --workspace --all-targets --all-features -- -D warnings` (no lint warnings)
3. ✅ All unit tests
4. ✅ All integration tests
5. ✅ All stress tests

The CI pipeline enforces these checks automatically via the `lint-and-format` job.

## Helpful Make Commands

```bash
# View all available commands
make help

# Run full verification (fmt-check + lint + check + build + test + DB connection)
make verify

# Run quality checks only (fmt-check + lint)
make quality

# Run quality checks + tests
make quality && make test

# Clean and rebuild with quality checks
make clean && make quality && make build

# Before committing (recommended workflow)
make fmt        # Auto-format code
make verify     # Run full verification
```

## Verification Workflow

The `make verify` command runs a comprehensive check in this order:

1. **Format Check** (`fmt-check`) - Ensures code is properly formatted
2. **Linting** (`lint`) - Runs clippy to catch code issues
3. **Compilation** (`check`) - Verifies all components compile
4. **Build** (`build`) - Builds the workspace in debug and release mode
5. **Tests** (`test`) - Runs all unit and integration tests
6. **DB Connection** - Checks SpacetimeDB connection and reducer response

If any step fails, the verification stops immediately.

## Troubleshooting

### Command not found: rustfmt
```bash
rustup component add rustfmt
```

### Command not found: clippy
```bash
rustup component add clippy
```

### CI fails but local checks pass
Ensure you're running the same commands as CI:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Performance issues with clippy
Use release mode for faster checks:
```bash
cargo clippy --release --workspace -- -D warnings
```
