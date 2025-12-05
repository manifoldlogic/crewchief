# Quality Strategy: maproom ignore patterns

## Testing Philosophy

**Confidence over coverage.** We test critical behaviors that could cause user-facing bugs or data integrity issues, not every code path for the sake of metrics.

Key principles:
- Test the contract, not the implementation
- Focus on edge cases and integration points
- Use real file system operations for integration tests (tempdir pattern)
- Fast unit tests, thorough integration tests

## Test Types

### Unit Tests

**Scope:**
- Pattern loading from `.maproomignore` file
- Pattern parsing (comments, blank lines, malformed patterns)
- `IgnorePatternMatcher` constructors and methods
- Glob matching behavior with various patterns

**Tools:**
- Rust's built-in `#[test]` framework
- `tempfile` crate for temporary files/directories
- Existing test patterns in `crates/maproom/src/incremental/ignore.rs`

**Coverage Target:**
- All public functions in `ignore.rs` module
- Edge cases: missing file, empty file, comment-only, invalid globs
- NOT targeting 100% - focus on meaningful scenarios

**Key tests:**
```rust
// Pattern loading
test_load_ignore_patterns_missing_file()        // Graceful fallback
test_load_ignore_patterns_with_comments()       // Skip # lines
test_load_ignore_patterns_empty_file()          // Use defaults
test_load_ignore_patterns_invalid_glob()        // Error handling (returns Err)

// Matcher construction
test_from_repository_reads_maproomignore()      // Integration with load function
test_from_repository_combines_with_defaults()   // Pattern merging
test_from_repository_fails_on_invalid()         // Fail-fast on invalid patterns

// Matching behavior
test_should_ignore_matches_pattern()            // Basic glob matching
test_should_ignore_relative_paths()             // Paths are relative to repo root
```

### Integration Tests

**Scope:**
- End-to-end scan with `.maproomignore` file
- Watch operation filtering events based on patterns
- CLI `--exclude` precedence over `.maproomignore`
- Interaction with existing `.gitignore` handling

**Approach:**
- Create temporary git repositories with known structure
- Add `.maproomignore` with specific patterns
- Run scan/watch operations
- Verify files are/aren't indexed as expected
- Use database queries to confirm behavior

**Location:** `crates/maproom/tests/maproomignore_test.rs` (new file)

**Key tests:**
```rust
test_scan_respects_maproomignore()
  1. Create temp repo with test-fixtures/ directory
  2. Add .maproomignore with "test-fixtures/**"
  3. Run scan
  4. Query database - verify test-fixtures/ files NOT present

test_watch_filters_maproomignore_events()
  1. Start watch with .maproomignore excluding "*.tmp"
  2. Create/modify test.tmp file
  3. Assert no indexing event emitted
  4. Modify normal file - verify event processed

test_invalid_patterns_fail_startup()
  1. .maproomignore contains invalid pattern "[invalid"
  2. Attempt to start watcher
  3. Verify watcher fails to start with clear error message
  4. Verify scan fails with clear error message

test_gitignore_still_works()
  1. .gitignore excludes "*.secret"
  2. .maproomignore excludes "test/**"
  3. Both patterns should apply independently
```

### End-to-End Tests

**Scope:** Critical user workflows only

**Approach:** Use existing test infrastructure (`incremental_watcher_test.rs` pattern)

**Critical path:**
1. User creates `.maproomignore` in existing indexed repository
2. Re-scans repository
3. Verifies excluded files removed from index (or never added)

**Not testing:** UI/UX elements (this is a Rust-only change)

## Critical Paths

The following paths MUST be tested before merge:

1. **Pattern loading with file I/O**
   - Read actual `.maproomignore` file from disk
   - Handle missing file without error
   - Parse real glob patterns correctly

2. **Scan exclusion**
   - Files matching `.maproomignore` patterns are NOT indexed
   - Existing `.gitignore` patterns still work
   - Pattern precedence correct (.maproomignore > .gitignore > defaults)

3. **Watch filtering**
   - FileEvents for ignored paths are filtered out
   - Non-ignored events still processed normally
   - Invalid patterns prevent watcher from starting (fail-fast)

4. **Pattern precedence**
   - .maproomignore > .gitignore > defaults
   - Combining patterns works (multiple sources)
   - Path normalization consistent (relative to repo root)

## Test Data Strategy

### Pattern Files

Create fixture `.maproomignore` files for testing:

```
# fixtures/maproomignore-basic
test/**
*.tmp
build/
```

```
# fixtures/maproomignore-complex
# Comments are skipped
test-fixtures/**

# Blank lines are ignored

vendor/**/generated/*
*.sql
```

### Test Repositories

Use `tempfile::TempDir` to create ephemeral test repos:

```rust
fn create_test_repo_with_maproomignore(patterns: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();

    // Initialize git repo
    Command::new("git").args(["init"]).current_dir(&dir).status().unwrap();

    // Write .maproomignore
    let ignore_file = dir.path().join(".maproomignore");
    std::fs::write(&ignore_file, patterns.join("\n")).unwrap();

    // Create test file structure
    std::fs::create_dir_all(dir.path().join("test-fixtures")).unwrap();
    std::fs::write(dir.path().join("test-fixtures/data.sql"), "...").unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "...").unwrap();

    dir
}
```

### No Mocking

We use **real file system operations** for integration tests. This catches real-world issues like:
- Path separator differences (Windows vs Unix)
- File encoding issues
- Permission problems
- Race conditions

Mocking is only acceptable for:
- External API calls (not applicable here)
- Time-dependent behavior (not applicable)

## Quality Gates

Before verification (ticket completion):

- [ ] **Unit tests pass:** All tests in `ignore.rs` module green
- [ ] **Integration tests pass:** New `maproomignore_test.rs` green
- [ ] **No regression:** Existing test suite (all of `crates/maproom/tests/`) passes
- [ ] **Linting clean:** `cargo clippy -p crewchief-maproom` shows no warnings
- [ ] **Format check:** `cargo fmt --check` passes
- [ ] **Documentation builds:** `cargo doc --no-deps` succeeds

## Performance Validation

Not full benchmarks, but sanity checks:

**Scan performance:**
```bash
# Before changes
time cargo run --release -- scan --path /large/repo --repo test

# After changes (with .maproomignore)
time cargo run --release -- scan --path /large/repo --repo test

# Difference should be < 1% (within noise margin)
```

**Watch filtering:**
- Add instrumentation: log time spent in `should_ignore()` check
- Verify < 1ms per event on typical patterns

## Test Execution

### Local Development

```bash
# Run just ignore module tests
cargo test -p crewchief-maproom incremental::ignore

# Run new integration test
cargo test -p crewchief-maproom maproomignore_test

# Run full suite
cargo test -p crewchief-maproom

# With output for debugging
cargo test -p crewchief-maproom -- --nocapture
```

### CI/CD

Tests run automatically on:
- Every commit (GitHub Actions)
- Before merge to main
- Nightly builds

## Debugging Failed Tests

When tests fail:

1. **Check test output:** Read assertion messages carefully
2. **Inspect temp directories:** Tests should log paths for manual inspection
3. **Add tracing:** Use `RUST_LOG=debug cargo test` for detailed logs
4. **Isolate:** Run single test with `cargo test test_name -- --nocapture`
5. **Reproduce manually:** Create `.maproomignore` and run scan command

## Known Test Limitations

**Not testing:**
- Extremely large `.maproomignore` files (1000+ patterns) - assume reasonable use
- Unicode in pattern names - covered by `globset` crate tests
- Concurrent access to `.maproomignore` - single writer model assumed
- Pattern hot-reload during active watch - NOT supported in MVP (restart required)

**Rationale:** These edge cases are either covered by dependencies or rare enough to defer.

## Acceptance Testing

Before marking ticket verified:

1. **Manual smoke test:**
   ```bash
   # Create test repo
   mkdir /tmp/test-repo && cd /tmp/test-repo
   git init
   echo "test-fixtures/**" > .maproomignore
   mkdir test-fixtures
   echo "large data" > test-fixtures/data.sql
   echo "fn main() {}" > main.rs

   # Scan with maproom
   crewchief-maproom scan --path . --repo test --worktree main

   # Search - should NOT find data.sql
   crewchief-maproom search --query "large data" --repo test
   # (should return no results or not find the file)
   ```

2. **Review test coverage:**
   - All critical paths have corresponding tests
   - Edge cases documented in code comments

3. **Code review:**
   - Error handling appropriate
   - No obvious performance issues
   - Follows existing code patterns
