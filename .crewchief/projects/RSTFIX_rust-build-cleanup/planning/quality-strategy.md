# Quality Strategy: Rust Build Cleanup

## Test Approach

This project is about fixing code quality, so the quality bar is simple: zero warnings, all tests pass.

### Verification Commands

```bash
# Build - must show 0 Rust warnings (excluding vendor C)
cargo build --bin crewchief-maproom 2>&1 | grep "warning:" | grep -v "sqlite-vec" | wc -l
# Target: 0

# Tests - all must pass
cargo test -p crewchief-maproom
# Target: 906 passed; 0 failed

# Clippy - must be clean
cargo clippy -p crewchief-maproom 2>&1 | grep "warning:" | grep -v "sqlite-vec" | wc -l
# Target: 0
```

### Risk Mitigation

1. **Regression prevention**: Run full test suite after each file edit
2. **Behavior preservation**: Only remove dead code, don't refactor
3. **Incremental commits**: Commit after each logical batch of fixes

## Acceptance Criteria

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| Rust warnings | Build output | 0 |
| Test pass rate | Test output | 100% (906/906) |
| Clippy warnings | Clippy output | 0 actionable |
| Functional regression | Test suite | None |

## Test Failure Analysis

The one failing test needs root cause analysis:

**Test**: `config::hot_reload::tests::test_invalid_config_rejected`
**Expectation**: Reload with negative weight should error
**Actual**: Reload succeeds

Investigation:
1. Check `SearchConfig::load_from_file()` return type
2. Verify `validate()` is called
3. Check if YAML parsing silently accepts negatives
4. Fix either the test expectation or the validation logic

## Definition of Done

- [ ] `cargo build` shows 0 Rust warnings
- [ ] `cargo test` shows 906 passed, 0 failed
- [ ] `cargo clippy` shows 0 actionable warnings
- [ ] Changes committed with clear commit messages
- [ ] No behavior changes to existing functionality
