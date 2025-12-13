# Quality Strategy: FTS Query Sanitization

## Testing Philosophy

**Confidence over coverage** - This bug fix requires targeted testing that proves the fix works without over-testing. The existing comprehensive test suite for `build_fts_query()` already covers edge cases; we only need to add coverage for dot handling specifically.

**Key principle:** Test the change, leverage existing tests for regression detection.

## Test Types

### Unit Tests

**Scope:** Test `build_fts_query()` comprehensive special character sanitization via regex whitelist

**Tools:**
- Rust built-in test framework (`#[test]`)
- `cargo test` for execution

**Coverage Target:** 100% of refactored code (the regex sanitization line)

**New test to add:**
```rust
#[test]
fn test_build_fts_query_comprehensive_sanitization() {
    // Dots (file extensions)
    let query = build_fts_query("package.json");
    assert_eq!(query, "package* OR json*");

    // Slashes (file paths)
    let query = build_fts_query("src/main.rs");
    assert_eq!(query, "src* OR main* OR rs*");

    // Brackets (array syntax)
    let query = build_fts_query("array[0]");
    assert_eq!(query, "array* OR 0*");

    // Braces (template syntax)
    let query = build_fts_query("template{value}");
    assert_eq!(query, "template* OR value*");

    // At sign (email/decorators)
    let query = build_fts_query("user@email.com");
    assert_eq!(query, "user* OR email* OR com*");

    // Backslash (Windows paths)
    let query = build_fts_query("path\\to\\file");
    assert_eq!(query, "path* OR to* OR file*");

    // Mixed special characters
    let query = build_fts_query("src/main@v2.rs");
    assert_eq!(query, "src* OR main* OR v2* OR rs*");

    // Operators
    let query = build_fts_query("a+b=c");
    assert_eq!(query, "a* OR b* OR c*");
}
```

**Rationale for test cases:**
1. **Dots** - File extensions (most commonly reported issue)
2. **Slashes** - File paths (very common in code search)
3. **Brackets** - Array syntax, FTS5 column filters
4. **Braces** - Template syntax, code blocks
5. **At signs** - Email addresses, decorators
6. **Backslashes** - Windows paths
7. **Mixed chars** - Real-world complex queries
8. **Operators** - Math/comparison operators in code

**Why these test cases?**
- Each represents a real-world query pattern
- Covers diverse character categories
- Validates regex whitelist approach
- Existing tests cover empty strings, whitespace, etc.

### Integration Tests

**Scope:** Not needed for this fix

**Rationale:**
- Integration tests exist at `/scripts/test_sqlite_e2e.sh`
- They already test search functionality end-to-end
- No new integration points introduced
- If existing integration tests fail, it indicates regression

### End-to-End Tests

**Scope:** Manual verification only

**Approach:**
```bash
# Test with real indexed repository
crewchief-maproom search --query "package.json" --repo crewchief --mode fts
crewchief-maproom search --query "config.yaml" --repo crewchief --mode fts
crewchief-maproom search --query ".gitignore" --repo crewchief --mode fts
```

**Expected results:**
- No FTS5 syntax errors
- Search results returned (or empty if no matches)
- Results should include files containing ANY of the extracted terms

**Real-world test cases to verify:**
- `package.json` → finds files with "package" or "json"
- `src/main.rs` → finds files with "src", "main", or "rs"
- `array[0]` → finds files with "array" or "0"
- `user@email.com` → finds files with "user", "email", or "com"

**Why manual only?**
- E2E tests require indexed repository (not in unit test environment)
- Automated E2E tests already exist for search functionality
- Manual verification takes <10 minutes and provides immediate confidence

## Critical Paths

The following paths MUST be tested:

1. **Comprehensive sanitization unit test** - Proves the fix works for ALL special characters
   - Test cases: Dots, slashes, brackets, braces, at-signs, backslashes, operators
   - Verification: Unit test passes

2. **Existing sanitization tests** - Proves no regression
   - All 10 existing `test_build_fts_query_*` tests must pass
   - Verification: `cargo test -p crewchief-maproom fts`

3. **Manual search with special characters** - Proves real-world usage works
   - Test cases: `package.json`, `src/main.rs`, `array[0]`, `user@email.com`
   - Verification: No syntax errors, results returned for all

## Test Data Strategy

**Unit tests:** Use hardcoded query strings (no external data needed)

**Manual verification:** Use any indexed repository containing common files like:
- package.json
- tsconfig.json
- .gitignore
- index.ts

**Why no test fixtures?**
- Query sanitization is pure function (string in, string out)
- No database or file system interaction at this layer
- Hardcoded strings are sufficient and more readable

## Regression Prevention

### Existing Test Suite Coverage

The `build_fts_query()` function already has excellent test coverage:

| Test | Purpose | Status |
|------|---------|--------|
| `test_build_fts_query_simple` | Single word | Existing |
| `test_build_fts_query_multiple_words` | Multi-word queries | Existing |
| `test_build_fts_query_sanitize_quotes` | Quote handling | Existing |
| `test_build_fts_query_sanitize_wildcards` | Wildcard handling | Existing |
| `test_build_fts_query_sanitize_parens` | Parenthesis handling | Existing |
| `test_build_fts_query_empty` | Empty query | Existing |
| `test_build_fts_query_only_special_chars` | All special chars | Existing |
| `test_build_fts_query_hyphen_handling` | Hyphen as separator | Existing |
| `test_build_fts_query_colon_handling` | Colon as separator | Existing |
| `test_build_fts_query_whitespace` | Whitespace normalization | Existing |
| **`test_build_fts_query_comprehensive_sanitization`** | **All special chars** | **NEW** |

Running all 11 tests ensures:
- New feature works (dot handling)
- No regressions (all existing tests pass)

### CI/CD Verification

The fix will be verified by existing CI pipelines:
- `cargo test` runs on every PR
- Test failures block merge
- No special CI configuration needed

## Quality Gates

Before verification (ticket can be marked complete):

- [ ] Unit test added: `test_build_fts_query_comprehensive_sanitization`
- [ ] All unit tests pass: `cargo test -p crewchief-maproom fts`
- [ ] Full test suite passes: `cargo test -p crewchief-maproom`
- [ ] Performance baseline measured and within 5% threshold
- [ ] No linting errors: `cargo clippy -p crewchief-maproom`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Manual verification: Test queries with all special char types return results

## Performance Testing

**Required:** Baseline measurement before and after

**Rationale:**
- Switching from multiple `.replace()` calls to regex could have performance implications
- Need to verify regex compilation overhead is amortized via `Lazy`
- Ensure no regression in query throughput

**Performance test approach:**

```bash
# Before change
time cargo test -p crewchief-maproom fts
# Note: execution time (e.g., "0.12s")

# After change
time cargo test -p crewchief-maproom fts
# Compare: should be within 5% of baseline
```

**Acceptance criteria:**
- Test execution time within 5% of baseline
- If regression >5%, investigate regex pattern or initialization

**Expected outcome:**
- Likely neutral or slight improvement (fewer allocations)
- Regex compilation is one-time cost via `Lazy`

## Test Maintenance

### When to update tests

Update tests if:
- Dot handling behavior changes (unlikely)
- New separator characters added (follow same pattern)
- FTS5 query syntax changes (rare, upstream SQLite change)

### Test documentation

Each test includes inline comments explaining:
- What case is being tested
- Why it matters
- Expected behavior

Example:
```rust
// File extensions (common case)
let query = build_fts_query("package.json");
assert_eq!(query, "package* OR json*");
```

## Edge Case Testing

Edge cases are already covered by existing tests:

| Edge Case | Covered By | Verification |
|-----------|------------|--------------|
| Empty string | `test_build_fts_query_empty` | Existing test |
| Only special chars | `test_build_fts_query_only_special_chars` | Existing test |
| Multiple spaces | `test_build_fts_query_whitespace` | Existing test |
| Mixed separators | **NEW test** | `"src/main@v2.rs"` → multiple special chars |
| Multiple consecutive special chars | Regex behavior | `"package..json"`, `"path//file"` work correctly |
| Leading/trailing special chars | Regex behavior | `".gitignore"`, `/path/` work correctly |
| Windows paths | **NEW test** | `"path\\to\\file"` → `"path* OR to* OR file*"` |
| Email addresses | **NEW test** | `"user@email.com"` → `"user* OR email* OR com*"` |
| Array notation | **NEW test** | `"array[0]"` → `"array* OR 0*"` |
| Template syntax | **NEW test** | `"template{value}"` → `"template* OR value*"` |

**Why we don't need more edge case tests:**

The existing `flat_map` and `filter` logic in `build_fts_query()` (lines 69-70) automatically handles:
- Multiple consecutive spaces (from multiple dots)
- Leading/trailing spaces (from leading/trailing dots)
- Empty terms (filtered out)

## Test Execution

### Local development
```bash
# Run only FTS tests (fast)
cargo test -p crewchief-maproom fts

# Run all maproom tests (comprehensive)
cargo test -p crewchief-maproom

# Run with output (for debugging)
cargo test -p crewchief-maproom fts -- --nocapture
```

### CI/CD
- Tests run automatically on PR
- Must pass before merge
- No manual intervention needed

### Manual verification
```bash
# Index a repository (if not already indexed)
crewchief-maproom scan --path /path/to/repo --repo myrepo --worktree main

# Test dot queries
crewchief-maproom search --query "package.json" --repo myrepo --mode fts
crewchief-maproom search --query "config.yaml" --repo myrepo --mode fts
crewchief-maproom search --query ".eslintrc" --repo myrepo --mode fts
```

## Test Isolation

All tests are isolated:
- No shared state
- No database interaction (pure function)
- No file system interaction
- Can run in parallel
- Deterministic results

## Mutation Testing

**Not needed for this fix**

The change is so simple (one line) that mutation testing would not provide additional value:
- Remove the line → test fails (obvious)
- Change space to empty string → test fails (covered by assertion)
- Change character → test fails (covered by assertion)

## Test Coverage Report

After implementation, verify coverage:

```bash
# Generate coverage report (if needed)
cargo tarpaulin -p crewchief-maproom --out Stdout -- --test-threads 1
```

**Expected coverage:** 100% of new code (the one added line)

**Note:** Coverage metrics are not a goal (confidence is), but can be useful for verification.

## Success Criteria Summary

Testing is complete when:
1. New unit test passes
2. All existing tests pass
3. Manual verification confirms fix works
4. No linting/formatting errors
5. Code reviewed (if applicable)

**Total test execution time:** <2 minutes
