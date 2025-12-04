# Ticket: [MRMIGNR-1004]: Unit Tests for Ignore Patterns

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
Add comprehensive unit tests for .maproomignore pattern loading, parsing, and matching behavior to ensure edge cases are handled correctly.

## Background
The pattern loading infrastructure (MRMIGNR-1001) needs thorough unit test coverage to verify:
- Pattern file parsing (comments, blank lines, whitespace)
- Missing file handling (graceful fallback)
- Invalid pattern handling (fail-fast with clear errors)
- Pattern matching behavior (glob matching, relative paths)
- Constructor behavior (IgnorePatternMatcher::from_repository)

These tests provide confidence that the foundation is solid before integration testing.

Reference: Testing Strategy (plan.md lines 172-183), Quality Strategy (quality-strategy.md lines 34-49)

## Acceptance Criteria
- [ ] Test `test_load_ignore_patterns_missing_file()` passes - gracefully returns defaults when .maproomignore missing
- [ ] Test `test_load_ignore_patterns_with_comments()` passes - skips lines starting with `#`
- [ ] Test `test_load_ignore_patterns_empty_file()` passes - handles empty .maproomignore (returns defaults)
- [ ] Test `test_load_ignore_patterns_invalid_glob()` passes - returns Err for invalid glob patterns
- [ ] Test `test_from_repository_reads_maproomignore()` passes - constructor loads patterns from file
- [ ] Test `test_from_repository_combines_with_defaults()` passes - merges .maproomignore with default patterns
- [ ] Test `test_from_repository_fails_on_invalid()` passes - constructor fails fast on invalid patterns
- [ ] Test `test_should_ignore_matches_pattern()` passes - basic glob matching works
- [ ] Test `test_should_ignore_relative_paths()` passes - paths relative to repo root
- [ ] All new tests pass when run with `cargo test -p crewchief-maproom incremental::ignore`
- [ ] All existing tests in module still pass (no regression)
- [ ] Code passes `cargo clippy -p crewchief-maproom` with no warnings
- [ ] Code formatted with `cargo fmt`

## Technical Requirements

**Location**: `crates/maproom/src/incremental/ignore.rs` in the `#[cfg(test)] mod tests` section

**Test infrastructure**:
- Use `tempfile::TempDir` to create temporary test directories
- Use `std::fs::write()` to create test `.maproomignore` files
- Each test should be self-contained and clean up automatically (TempDir handles this)

**Required test cases** (from quality-strategy.md lines 34-49):

```rust
// Pattern loading tests
#[test]
fn test_load_ignore_patterns_missing_file() {
    // Setup: temp dir without .maproomignore
    // Action: call load_ignore_patterns()
    // Assert: returns Ok with only default patterns
}

#[test]
fn test_load_ignore_patterns_with_comments() {
    // Setup: .maproomignore with comment lines
    // Action: call load_ignore_patterns()
    // Assert: comments skipped, patterns loaded
}

#[test]
fn test_load_ignore_patterns_empty_file() {
    // Setup: empty .maproomignore
    // Action: call load_ignore_patterns()
    // Assert: returns Ok with only default patterns
}

#[test]
fn test_load_ignore_patterns_invalid_glob() {
    // Setup: .maproomignore with "[invalid" pattern
    // Action: call load_ignore_patterns()
    // Assert: returns Err (fail-fast on invalid patterns)
    // Note: This might be validated at GlobSet build time, not file read time
}

// Matcher construction tests
#[test]
fn test_from_repository_reads_maproomignore() {
    // Setup: .maproomignore with "test/**"
    // Action: IgnorePatternMatcher::from_repository()
    // Assert: matcher created successfully, patterns loaded
}

#[test]
fn test_from_repository_combines_with_defaults() {
    // Setup: .maproomignore with custom pattern
    // Action: from_repository()
    // Assert: both default patterns AND custom patterns present
}

#[test]
fn test_from_repository_fails_on_invalid() {
    // Setup: .maproomignore with invalid pattern
    // Action: from_repository()
    // Assert: returns Err with clear message
}

// Matching behavior tests
#[test]
fn test_should_ignore_matches_pattern() {
    // Setup: matcher with "*.tmp" pattern
    // Action: should_ignore("file.tmp")
    // Assert: returns true
    // Action: should_ignore("file.rs")
    // Assert: returns false
}

#[test]
fn test_should_ignore_relative_paths() {
    // Setup: matcher with "test/**" pattern
    // Action: should_ignore("test/file.rs")
    // Assert: returns true
    // Action: should_ignore("src/test/file.rs")
    // Assert: returns false (pattern is relative to root)
}
```

**Test data strategy** (from quality-strategy.md lines 134-177):

Create fixture `.maproomignore` content for tests:
```rust
const BASIC_PATTERNS: &str = "test/**\n*.tmp\nbuild/\n";
const WITH_COMMENTS: &str = "# Skip test fixtures\ntest-fixtures/**\n\n# Build outputs\nbuild/\n";
const INVALID_PATTERN: &str = "[invalid\n*.tmp\n";
```

## Implementation Notes

**Helper function for test setup**:
```rust
fn create_test_repo_with_maproomignore(patterns: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let ignore_file = dir.path().join(".maproomignore");
    std::fs::write(&ignore_file, patterns).unwrap();
    dir
}
```

**Testing philosophy** (from quality-strategy.md lines 3-12):
- Test the contract, not the implementation
- Focus on edge cases and boundary conditions
- Use real file system operations (no mocking)
- Fast unit tests that can run frequently during development

**Order of work**:
1. Add test helper functions for creating test repos
2. Implement pattern loading tests (missing file, comments, empty, invalid)
3. Implement constructor tests (from_repository)
4. Implement matching behavior tests (should_ignore)
5. Run all tests and verify they pass
6. Run full test suite to check for regressions

## Dependencies
- **Prerequisite**: MRMIGNR-1001 (pattern loading infrastructure must be implemented)
- **Blocks**: None (other tickets can proceed independently)
- **External dependencies**: `tempfile` crate (already in dev-dependencies)

## Risk Assessment
- **Risk**: Tests don't catch real-world edge cases
  - **Impact**: Medium - bugs slip through to integration testing
  - **Mitigation**: Review test cases against quality-strategy.md. Add tests for any edge cases discovered during implementation.

- **Risk**: Tests are flaky due to filesystem timing issues
  - **Impact**: Low - annoying but not blocking
  - **Mitigation**: Use TempDir for isolation. Avoid relying on specific timing. Make assertions deterministic.

- **Risk**: Tests pass but integration fails
  - **Impact**: Medium - false confidence
  - **Mitigation**: Integration tests (MRMIGNR-1005) will catch integration issues. Unit tests focus on component behavior.

## Files/Packages Affected
- `crates/maproom/src/incremental/ignore.rs` (add tests in existing `#[cfg(test)] mod tests` block)
- No production code changes in this ticket (test-only)

## Verification Notes
The verify-ticket agent should confirm:
1. All 9 specified test functions exist in ignore.rs
2. Tests use TempDir for filesystem isolation
3. Tests use real file I/O (no mocking)
4. All new tests pass: `cargo test -p crewchief-maproom incremental::ignore`
5. All existing tests still pass (no regression)
6. Test output shows all tests passing with specific test names
7. No clippy warnings in test code
8. Test code formatted properly

**Test execution**:
```bash
# Run just the ignore module tests
cargo test -p crewchief-maproom incremental::ignore -- --nocapture

# Verify specific test names appear in output
# Should see: test_load_ignore_patterns_missing_file ... ok
#            test_load_ignore_patterns_with_comments ... ok
#            etc.
```

**Coverage verification**:
- All public functions in ignore.rs have corresponding tests
- Edge cases documented in quality-strategy.md are covered
- Each test has clear setup/action/assert structure with comments
