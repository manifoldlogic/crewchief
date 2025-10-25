# Ticket: CODE_QUALITY-6001: Fix Rust Compiler Warnings in Maproom

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (library and binary build cleanly)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Clean up 7 Rust compiler warnings in the maproom crate to achieve a warning-free build. All warnings are related to unused variables, unused assignments, and unused struct fields.

## Background
During recent development work (INC_INDEX-5001, MD_ENHANCE project), the maproom crate accumulated several compiler warnings. While these don't affect functionality, they:
- Create noise in build output, making it harder to spot real issues
- Indicate potential code that can be removed or refactored
- Go against Rust best practices for clean, maintainable code
- May hide actual bugs (unused code often indicates logic errors)

Current warning count: **7 warnings** in `cargo build --bin crewchief-maproom`

## Acceptance Criteria
- [x] Zero warnings when running `cargo build --bin crewchief-maproom`
- [x] Zero warnings when running `cargo test --package crewchief-maproom`
- [x] All existing tests continue to pass (no functionality broken)
- [x] No functionality removed without proper justification
- [x] Code remains readable and maintainable

## Technical Requirements

### Warning 1-2: Unused assignments to `current_section`
**Location**: `crates/maproom/src/indexer/parser.rs:1255` and `1347`
```rust
let mut current_section = String::new();
```
**Issue**: Variable is assigned but never read before being overwritten
**Fix Options**:
- Remove the initial assignment if it's truly unused
- Or prefix with `_` if it's intentionally unused: `let mut _current_section = String::new();`
- Or use the variable if it was meant to be used

### Warning 3: Unused variable `source`
**Location**: `crates/maproom/src/indexer/parser.rs:2409`
```rust
fn extract_rust_function_modifiers(source: &str, node: Node) -> Vec<&'static str>
```
**Issue**: Parameter `source` is never used in function body
**Fix Options**:
- Remove parameter if truly not needed
- Prefix with underscore: `_source: &str`
- Implement the missing functionality if it was intended to use source

### Warning 4: Unused variable `name`
**Location**: `crates/maproom/src/profiling.rs:50`
```rust
pub fn profile_operation<T, F>(name: &str, f: F) -> T
```
**Issue**: Parameter `name` is never used (profiling disabled?)
**Fix Options**:
- Prefix with underscore: `_name: &str`
- Remove if profiling is completely disabled
- Implement actual profiling if intended

### Warning 5: Unused field `db_url`
**Location**: `crates/maproom/src/ab_testing/dashboard.rs:96`
```rust
pub struct Dashboard {
    /// Database connection string
    db_url: String,
}
```
**Issue**: Field is never read
**Fix Options**:
- Remove field if not needed
- Prefix with underscore: `_db_url`
- Implement functionality that uses it

### Warning 6: Unused field `query`
**Location**: `crates/maproom/src/ab_testing/dashboard.rs:465`
```rust
struct QueryResult {
    query: String,
}
```
**Issue**: Field is never read (has derived Debug/Clone but marked dead)
**Fix Options**:
- Remove field if not needed for Debug output
- Use `#[allow(dead_code)]` if intentionally unused for debugging
- Implement functionality that reads it

### Warning 7: Unused field `patterns`
**Location**: `crates/maproom/src/context/detectors/component.rs:46`
```rust
pub struct ComponentDetector {
    patterns: ComponentPatterns,
}
```
**Issue**: Field is never read
**Fix Options**:
- Remove field if not needed
- Use the patterns in detection logic
- Prefix with underscore if intentionally unused

## Implementation Notes

### Approach
1. **Audit each warning individually**: Don't blindly prefix with `_` - understand if code should be removed or functionality implemented
2. **Check git history**: See if functionality was intentionally stubbed out or accidentally left unused
3. **Prefer removal over suppression**: If code is truly unused, remove it rather than using `#[allow(dead_code)]`
4. **Test after each fix**: Ensure no functionality is broken

### Strategy by Warning Type
- **Unused assignments**: Usually indicate logic errors - investigate carefully
- **Unused parameters**: Common in trait implementations or future expansion - prefix with `_`
- **Unused fields**: May indicate incomplete implementations - check if functionality is missing

### Testing Strategy
After all fixes:
```bash
# Verify zero warnings
cargo build --bin crewchief-maproom 2>&1 | grep "warning:" && echo "FAIL: Still has warnings" || echo "PASS: No warnings"

# Run full test suite
cargo test --package crewchief-maproom

# Verify warning count
cargo build --bin crewchief-maproom 2>&1 | grep -c "warning:"
```

## Dependencies
- None (standalone cleanup)

## Risk Assessment
- **Risk**: Removing code that appears unused but is actually needed
  - **Mitigation**: Run full test suite after each change; review git blame for context

- **Risk**: Breaking functionality by removing "dead" code that's used via reflection or macros
  - **Mitigation**: Careful code review; test with `cargo test` and `cargo build`

- **Risk**: Changes may affect ab_testing or profiling modules that aren't well tested
  - **Mitigation**: Check if these modules have tests; add basic smoke tests if missing

## Files/Packages Affected
- `crates/maproom/src/indexer/parser.rs` (3 warnings)
- `crates/maproom/src/profiling.rs` (1 warning)
- `crates/maproom/src/ab_testing/dashboard.rs` (2 warnings)
- `crates/maproom/src/context/detectors/component.rs` (1 warning)

## Success Metrics
- Before: 7 warnings in build output
- After: 0 warnings in build output
- Test pass rate: 100% (no regressions)
