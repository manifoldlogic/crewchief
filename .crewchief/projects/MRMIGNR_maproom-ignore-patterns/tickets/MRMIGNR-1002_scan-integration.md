# Ticket: [MRMIGNR-1002]: Scan Integration with .maproomignore

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
Integrate .maproomignore patterns into the scan operation using `OverrideBuilder` API to exclude files matching patterns during filesystem walk.

## Background
Currently, the scan operation only respects `.gitignore` patterns. This ticket adds support for `.maproomignore` patterns by loading them during scan initialization and applying them as negative overrides via the `ignore` crate's `OverrideBuilder`.

This maintains existing `.gitignore` behavior while adding an additional layer of user-defined exclusions specific to maproom indexing.

Reference: Phase 1 - Scan Integration (plan.md lines 122-128), Architecture Component 2 (architecture.md lines 107-125)

## Acceptance Criteria
- [ ] Scan operation loads `.maproomignore` patterns using `load_ignore_patterns()`
- [ ] Patterns applied to `WalkBuilder` via `OverrideBuilder` as negative overrides
- [ ] Files matching `.maproomignore` patterns are NOT indexed during scan
- [ ] Existing `.gitignore` behavior unchanged (regression test)
- [ ] Programmatic `exclude` parameter continues to work (merges with .maproomignore patterns)
- [ ] Invalid patterns in `.maproomignore` cause scan to fail with clear error message
- [ ] Manual smoke test passes: create .maproomignore with pattern, verify files excluded
- [ ] All existing indexer tests pass (no regression)
- [ ] Code passes `cargo clippy -p crewchief-maproom` with no warnings
- [ ] Code formatted with `cargo fmt`

## Technical Requirements

**Location**: `crates/maproom/src/indexer/mod.rs`

**Integration point**: In the scan setup logic where `WalkBuilder` is configured

**Implementation approach** (from architecture.md lines 109-124):

```rust
// Existing gitignore handling
walk.git_ignore(true);  // Keep this unchanged

// NEW: Add .maproomignore patterns
let maproomignore_patterns = load_ignore_patterns(&root_abs)?;
let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);
for pattern in maproomignore_patterns {
    ob.add(&format!("!{}", pattern))?;  // Negative override = exclude
}
walk.overrides(ob.build()?);

// If programmatic exclude patterns provided, merge them
if let Some(exclude_patterns) = exclude {
    for pattern in exclude_patterns {
        ob.add(&format!("!{}", pattern))?;
    }
}
```

**Error handling**:
- Invalid glob patterns in `.maproomignore` should propagate as `Result::Err`
- Error message should clearly indicate which pattern failed and why
- Scan should fail-fast (not index partial results)

**Pattern precedence** (from architecture.md line 222):
- .maproomignore patterns > .gitignore patterns > default patterns
- Both .maproomignore AND .gitignore apply (not mutually exclusive)

## Implementation Notes

**Changes required**:
1. Import `load_ignore_patterns` from `crate::incremental::ignore`
2. Call `load_ignore_patterns(&root_abs)?` before WalkBuilder setup
3. Create `OverrideBuilder` instance
4. Add each pattern as negative override using `!{pattern}` syntax
5. Apply overrides to WalkBuilder with `.overrides(ob.build()?)`
6. Merge programmatic exclude patterns if provided

**Testing approach**:
- Run existing `cargo test -p crewchief-maproom indexer` to verify no regression
- Manual test: create repo with `.maproomignore` containing `test-fixtures/**`, verify exclusion
- Next ticket (MRMIGNR-1005) adds integration tests for comprehensive validation

**Pattern format** (from architecture.md lines 286-291):
- Patterns are gitignore-style globs relative to repository root
- Example: `test-fixtures/**` excludes all files under test-fixtures/
- Example: `*.tmp` excludes all .tmp files recursively
- Patterns with leading `/` are relative to root (git semantics)

## Dependencies
- **Prerequisite**: MRMIGNR-1001 (pattern loading infrastructure must exist)
- **Blocks**: MRMIGNR-1005 (integration tests need working implementation)
- **External dependencies**: `ignore::overrides::OverrideBuilder` (already in Cargo.toml)

## Risk Assessment
- **Risk**: OverrideBuilder syntax incorrect, patterns don't exclude files
  - **Impact**: High - core functionality broken
  - **Mitigation**: Test with multiple pattern types. Verify negative override syntax `!pattern` in ignore crate docs. Manual smoke test required.

- **Risk**: Pattern precedence wrong (gitignore overrides maproomignore or vice versa)
  - **Impact**: Medium - confusing behavior
  - **Mitigation**: Test with overlapping .gitignore and .maproomignore patterns. Document precedence clearly.

- **Risk**: Performance regression from additional pattern matching
  - **Impact**: Low - globset is highly optimized
  - **Mitigation**: Pattern compilation happens once per scan. Benchmark large repos (from quality-strategy.md lines 206-215).

- **Risk**: Breaking existing exclude parameter behavior
  - **Impact**: Medium - daemon integration affected
  - **Mitigation**: Test programmatic exclude parameter continues to work. Merge with .maproomignore patterns, don't replace.

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (modify scan setup logic)
- Potentially `crates/maproom/src/indexer/error.rs` if new error types needed for pattern validation

## Verification Notes
The verify-ticket agent should confirm:
1. `load_ignore_patterns()` is called during scan initialization
2. `OverrideBuilder` is created and patterns added with `!` prefix
3. Overrides applied to WalkBuilder
4. Manual smoke test passes:
   - Create temp repo with `.maproomignore` containing `test/**`
   - Add files under `test/` directory
   - Run scan
   - Verify files under `test/` NOT in index
5. Existing indexer tests pass (`cargo test -p crewchief-maproom indexer`)
6. No clippy warnings
7. Code formatted properly

**Smoke test procedure** (from quality-strategy.md lines 270-286):
```bash
# Create test repo
mkdir /tmp/test-repo && cd /tmp/test-repo
git init
echo "test-fixtures/**" > .maproomignore
mkdir test-fixtures
echo "large data" > test-fixtures/data.sql
echo "fn main() {}" > main.rs

# Run scan
crewchief-maproom scan --path . --repo test --worktree main

# Verify test-fixtures/data.sql NOT indexed
# Query database or check file list output
```
