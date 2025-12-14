# Ticket: [SRCHCONF-1002]: Make Exact Match Multiplier Always Available

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
- rust-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify the FTS semantic ranking module to compute exact_match_multiplier unconditionally (not just in debug mode) and make it accessible to confidence computation.

## Background
Currently, the exact_match_multiplier (3.0x boost for exact symbol name matches) is only computed when debug mode is enabled. For confidence scoring, we need this information to be always available so we can set the is_exact_match field in ConfidenceSignals.

This is a critical dependency for Phase 1 confidence scoring - without this change, is_exact_match will always be false unless debug=true.

## Acceptance Criteria
- [ ] exact_match_multiplier is computed unconditionally in FTS semantic ranking (not debug-mode-only)
- [ ] exact_match_multiplier is accessible to confidence computation (either stored in FusedResult or retrievable via function)
- [ ] Debug mode still shows exact_match_multiplier in score breakdown (backward compatibility)
- [ ] All existing tests pass (no regressions)
- [ ] New test verifies exact_match_multiplier available without debug mode
- [ ] Zero clippy warnings

## Technical Requirements
Two possible implementation approaches:

**Option A: Add to FusedResult struct**
```rust
pub struct FusedResult {
    // ... existing fields ...
    pub exact_match_multiplier: Option<f32>, // NEW: always computed, not debug-only
}
```

**Option B: Recompute in confidence module**
- Add helper function to re-check if query matches symbol name
- Keep FusedResult unchanged

**Recommendation**: Option A if FusedResult is accessible during result assembly, Option B if modifying FusedResult is too invasive. Rust engineer should evaluate both and choose the cleanest approach.

## Implementation Notes
Current state (from architecture.md):
- exact_match_multiplier only computed/exposed in debug mode
- Multiplier is 3.0 for exact matches, 1.0 otherwise
- Used in FTS semantic ranking to boost exact symbol name matches

Required change:
- Compute exact_match_multiplier always (remove debug-only condition)
- Store or make accessible for confidence computation
- is_exact_match logic: `multiplier >= 2.9` → true

Location likely in:
- `crates/maproom/src/search/fts.rs` or equivalent semantic ranking code
- Wherever FTS scoring applies kind and exact match multipliers

## Dependencies
- **Prerequisite**: SRCHCONF-1001 (defines ConfidenceSignals struct that needs this data)

## Risk Assessment
- **Risk**: Modifying FusedResult struct may require updates in multiple places
  - **Mitigation**: Search for all FusedResult construction sites, update systematically. Run full test suite to catch issues.
- **Risk**: Performance impact from always computing exact match check
  - **Mitigation**: Already computed in scoring path, just removing debug-only gate. No new computation added.

## Files/Packages Affected
- `crates/maproom/src/search/fts.rs` (or equivalent semantic ranking module) - Compute exact_match_multiplier unconditionally
- `crates/maproom/src/search/fusion.rs` or similar - Possibly add field to FusedResult
- `crates/maproom/src/search/executors.rs` - May need updates to access multiplier
- Test files for FTS and fusion modules

## Verification Notes
The verify-ticket agent should check:
- exact_match_multiplier is computed without requiring debug=true
- Confidence module can access the multiplier value
- All existing tests pass (run full test suite)
- New test demonstrates exact match detection works without debug mode
- No performance degradation (should be neutral since computation already existed)
