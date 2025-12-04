# Ticket: [MRMIGNR-1001]: Pattern Loading Infrastructure

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
Implement foundation for .maproomignore support by creating `load_ignore_patterns()` function and adding `from_repository()` constructor to `IgnorePatternMatcher`.

## Background
The current ignore pattern system only handles `.gitignore` and programmatic patterns. This ticket implements the core infrastructure needed to read and parse `.maproomignore` files from the repository root, enabling user-customizable ignore patterns beyond git's defaults.

This is the foundation upon which scan and watch integrations will be built in subsequent tickets.

Reference: Phase 1 - Foundation (plan.md lines 117-121)

## Acceptance Criteria
- [ ] Function `load_ignore_patterns(root: &Path) -> Result<Vec<String>>` implemented in `crates/maproom/src/incremental/ignore.rs`
- [ ] Function reads `.maproomignore` file from repository root (if exists)
- [ ] Function parses patterns correctly: skips blank lines, skips comments (lines starting with #), returns pattern strings
- [ ] Function gracefully handles missing `.maproomignore` (returns only default patterns, no error)
- [ ] Function returns error if `.maproomignore` exists but contains invalid glob patterns
- [ ] Constructor `IgnorePatternMatcher::from_repository(root: &Path) -> Result<Self>` implemented
- [ ] Constructor calls `load_ignore_patterns()` and builds matcher with combined patterns
- [ ] All existing tests in `incremental/ignore.rs` still pass (no regression)
- [ ] Code passes `cargo clippy -p crewchief-maproom` with no warnings
- [ ] Code formatted with `cargo fmt`

## Technical Requirements

**Location**: `crates/maproom/src/incremental/ignore.rs`

**New function signature**:
```rust
pub fn load_ignore_patterns(root: &Path) -> Result<Vec<String>>
```

**Implementation requirements**:
- Read file at `{root}/.maproomignore` using `std::fs::read_to_string`
- Return `Ok(DEFAULT_IGNORE_PATTERNS.to_vec())` if file doesn't exist
- Parse each line: trim whitespace, skip empty lines, skip lines starting with `#`
- Combine default patterns with `.maproomignore` patterns
- Validate glob patterns using `globset` (fail-fast on invalid patterns)
- Return `Result<Vec<String>>` with clear error messages for I/O or parse failures

**New constructor signature**:
```rust
impl IgnorePatternMatcher {
    pub fn from_repository(root: &Path) -> Result<Self> { ... }
}
```

**Implementation requirements**:
- Call `load_ignore_patterns(root)?` to get pattern list
- Build `GlobSet` from patterns
- Return `IgnorePatternMatcher` instance
- Propagate errors from pattern loading or compilation

**No external dependencies needed** - all functionality uses existing crates (`std::fs`, `globset` via `ignore` crate)

## Implementation Notes

**Implementation approach** (from architecture.md lines 72-100):

```rust
pub fn load_ignore_patterns(root: &Path) -> Result<Vec<String>> {
    let mut patterns = DEFAULT_IGNORE_PATTERNS.iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let maproomignore_path = root.join(".maproomignore");
    if maproomignore_path.exists() {
        let content = std::fs::read_to_string(&maproomignore_path)?;
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                patterns.push(line.to_string());
            }
        }
    }

    Ok(patterns)
}
```

**Constructor implementation**:
```rust
impl IgnorePatternMatcher {
    pub fn from_repository(root: &Path) -> Result<Self> {
        let patterns = load_ignore_patterns(root)?;
        // Build GlobSet from patterns
        // Return IgnorePatternMatcher
    }
}
```

**Order of work**:
1. Implement `load_ignore_patterns()` function
2. Add `from_repository()` constructor to `IgnorePatternMatcher`
3. Run existing tests to verify no regression
4. Run clippy and fix any warnings
5. Format code with `cargo fmt`

## Dependencies
- **Prerequisite**: Existing `IgnorePatternMatcher` infrastructure in `ignore.rs`
- **Blocks**: MRMIGNR-1002 (scan integration), MRMIGNR-1003 (watch integration)
- **External dependencies**: None (uses existing `ignore`, `globset` crates)

## Risk Assessment
- **Risk**: Pattern parsing inconsistent with gitignore semantics
  - **Impact**: Medium - users expect gitignore-like behavior
  - **Mitigation**: Use simple line-based parsing matching gitignore comment/blank line rules. Test with real .gitignore patterns.

- **Risk**: Invalid glob patterns crash or panic
  - **Impact**: High - would break scan/watch startup
  - **Mitigation**: Fail-fast with clear error message during pattern compilation. User sees error immediately.

- **Risk**: File I/O errors not handled gracefully
  - **Impact**: Medium - could confuse users
  - **Mitigation**: Return `Result` with descriptive error messages. Distinguish "file not found" (OK) from "permission denied" (error).

## Files/Packages Affected
- `crates/maproom/src/incremental/ignore.rs` (add `load_ignore_patterns()` function and `from_repository()` constructor)
- No other files modified in this ticket (pure infrastructure work)

## Verification Notes
The verify-ticket agent should confirm:
1. `load_ignore_patterns()` function exists with correct signature
2. Function handles missing file gracefully (no error)
3. Function parses patterns correctly (blank lines, comments)
4. `from_repository()` constructor exists and calls `load_ignore_patterns()`
5. All existing tests in `incremental::ignore` module pass
6. No clippy warnings for the modified code
7. Code is properly formatted

**Testing approach** (from quality-strategy.md lines 34-49):
- Run `cargo test -p crewchief-maproom incremental::ignore` to verify existing tests pass
- Manual verification: create test `.maproomignore` file and run function to confirm parsing
- Next ticket (MRMIGNR-1004) will add comprehensive unit tests for this new functionality
