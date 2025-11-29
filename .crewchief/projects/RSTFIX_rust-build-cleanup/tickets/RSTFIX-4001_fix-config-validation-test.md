# Ticket: RSTFIX-4001: Fix Config Validation Test Failure

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the failing test `config::hot_reload::tests::test_invalid_config_rejected` which expects negative weight validation to fail during config reload, but currently passes unexpectedly.

## Background
The test `test_invalid_config_rejected` in `crates/maproom/src/config/hot_reload.rs` (line ~410) expects that a configuration with negative fusion weights should be rejected during reload. However, the test is failing because the validation is either not being called or the error is not being propagated correctly. This is Phase 4 of the RSTFIX cleanup project.

Reference: Phase 4 in planning/plan.md - "Test Fix"

## Acceptance Criteria
- [ ] Test `test_invalid_config_rejected` passes
- [ ] Negative weights in `FusionWeights` are properly rejected during config reload
- [ ] Error message clearly indicates the validation failure reason
- [ ] All 906 tests pass (no regressions)

## Technical Requirements
- Root cause the test failure:
  1. Check if `FusionWeights::validate()` is being called during `load_from_file`
  2. Check if YAML deserialization silently accepts negative f32 values
  3. Check if the error is being swallowed somewhere in the reload path
- Fix the validation to ensure invalid configs are rejected
- Ensure fix doesn't break valid config loading

## Implementation Notes

**Test location:** `crates/maproom/src/config/hot_reload.rs:410`

**Investigation path:**
1. **Read the failing test** to understand expectations
2. **Trace `load_from_file()`** to see if it calls `validate()`
3. **Check `FusionWeights::validate()`** implementation
4. **Test YAML parsing** of negative numbers

**Possible issues:**
1. `validate()` not called after YAML parsing
2. `validate()` returns `Ok` when it should return `Err`
3. Error from `validate()` is ignored (`.ok()` or `unwrap_or_default()`)
4. Test setup is incorrect

**Key functions to examine:**
- `SearchConfig::load_from_file()` - Entry point for config loading
- `FusionWeights::validate()` - Should reject negative weights
- `hot_reload::reload_config()` - May have error handling issues

**Solution patterns:**
- If `validate()` not called → Add validation call after parse
- If error swallowed → Propagate error with `?` operator
- If test expectation wrong → Update test to match actual behavior (document why)

**Note:** Test passed in isolation but failed after clean rebuild - investigate if this is due to stale state.

## Dependencies
- Can be done in parallel with Phase 2 and 3 work
- No strict dependencies, but ideally all warning fixes complete first

## Risk Assessment
- **Risk**: Changing validation logic may break valid configs
  - **Mitigation**: Test with variety of valid configs before committing
- **Risk**: Test expectation may be wrong, not the implementation
  - **Mitigation**: Verify business requirement (should negative weights be rejected?)
- **Risk**: Environment-dependent behavior may make fix unstable
  - **Mitigation**: Run test multiple times, check for flakiness

## Files/Packages Affected
- `crates/maproom/src/config/hot_reload.rs` (test location)
- `crates/maproom/src/config/mod.rs` (possibly `SearchConfig`)
- `crates/maproom/src/config/search_config.rs` (possibly `FusionWeights`)
