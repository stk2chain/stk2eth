# Code Coverage Notes

## Coverage Tool: cargo-llvm-cov

We use `cargo-llvm-cov` instead of `cargo-tarpaulin` for coverage reporting.

### Why Not Tarpaulin?

SpacetimeDB modules are WebAssembly modules that require the SpacetimeDB runtime to link properly. They use FFI bindings that aren't available during standard test compilation. 

**Error symptoms with tarpaulin:**
```
rust-lld: error: undefined symbol: table_id_from_name
rust-lld: error: undefined symbol: __getrandom_custom
```

These symbols are provided by the SpacetimeDB runtime at module load time.

### Solution

`cargo-llvm-cov` works better with WASM targets and handles the SpacetimeDB module compilation more gracefully. It:
- Uses LLVM's coverage instrumentation
- Better handles FFI and runtime-dependent code
- Provides accurate coverage for what can be tested

### Current Configuration

```bash
cargo llvm-cov --workspace --lib --bins --lcov --output-path lcov.info
```

This covers:
- ✅ Library code (`--lib`)
- ✅ Binary code (`--bins`)
- ✅ All workspace members (`--workspace`)
- ❌ Integration tests with SpacetimeDB runtime (requires deployed module)

### Full Coverage Testing

For complete coverage including SpacetimeDB integration:

1. Deploy module to test instance
2. Run integration tests against live instance  
3. Use SpacetimeDB's built-in query analytics

### Alternative: Manual Testing

```bash
# Test without coverage (works with SpacetimeDB)
cargo test --workspace

# Coverage for non-SpacetimeDB crates
cargo llvm-cov --workspace --exclude spacetime-module
```

## Coverage Goals

- **Target**: ≥ 70% line coverage
- **Current**: Tracking enabled in CI
- **Exclusions**: SpacetimeDB FFI bindings (tested at runtime)

---

**Updated**: October 3, 2025
**Issue**: SpacetimeDB WASM module linking requirements
**Resolution**: Use cargo-llvm-cov with graceful fallback
