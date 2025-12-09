# Ticket: FTSFIX-1001: Comprehensive FTS Query Sanitization

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - unit tests and manual verification complete
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Refactor FTS5 query sanitization logic to use regex whitelist pattern `[^a-zA-Z0-9_\s]` for comprehensive special character handling, replacing individual `.replace()` calls. This fixes queries containing dots, slashes, brackets, and other special characters that currently cause FTS5 syntax errors.

## Background
The current `build_fts_query()` implementation uses individual `.replace()` calls for specific special characters (quotes, asterisks, parentheses, hyphens, colons). This approach is fragile and incomplete - it misses many special characters that cause FTS5 syntax errors, including:
- Dots (`.`) in filenames like `package.json`
- Forward slashes (`/`) in paths like `src/main.rs`
- Brackets (`[]`) in array syntax like `array[0]`
- Braces (`{}`) in template syntax like `template{value}`
- At-signs (`@`) in emails like `user@email.com`
- Backslashes (`\`) in Windows paths like `path\to\file`

This fix adopts the same regex whitelist approach used by the PostgreSQL FTS module (src/search/fts.rs:69), providing comprehensive coverage for ALL special characters in a single operation.

**Reference**:
- Plan: `.crewchief/projects/FTSFIX_fts-query-sanitization/planning/plan.md`
- Architecture: `.crewchief/projects/FTSFIX_fts-query-sanitization/planning/architecture.md`
- Quality Strategy: `.crewchief/projects/FTSFIX_fts-query-sanitization/planning/quality-strategy.md`

## Acceptance Criteria
- [x] Code change: Sanitization logic refactored to use regex whitelist `[^a-zA-Z0-9_\s]` in `build_fts_query()`
- [x] Dependencies added: `use once_cell::sync::Lazy;` and `use regex::Regex;` at top of file
- [x] Static regex defined: `static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9_\s]").unwrap());`
- [x] Sanitization chain replaced: `let clean = SPECIAL_CHAR_REGEX.replace_all(t, " ").to_string();`
- [x] Test added: `test_build_fts_query_comprehensive_sanitization` with 8+ test cases
- [x] Test case: Dots (file extensions) - `"package.json"` → `"package* OR json*"`
- [x] Test case: Slashes (file paths) - `"src/main.rs"` → `"src* OR main* OR rs*"`
- [x] Test case: Brackets (array syntax) - `"array[0]"` → `"array* OR 0*"`
- [x] Test case: Braces (template syntax) - `"template{value}"` → `"template* OR value*"`
- [x] Test case: At-signs (email/decorators) - `"user@email.com"` → `"user* OR email* OR com*"`
- [x] Test case: Backslashes (Windows paths) - `"path\\to\\file"` → `"path* OR to* OR file*"`
- [x] Test case: Mixed special characters - `"src/main@v2.rs"` → `"src* OR main* OR v2* OR rs*"`
- [x] Test case: Operators - `"a+b=c"` → `"a* OR b* OR c*"`
- [x] All existing tests pass: `cargo test -p crewchief-maproom`
- [x] Performance baseline measured (before change): Note test execution time
- [x] Performance verified (after change): Within 5% of baseline
- [x] Manual verification: `crewchief-maproom search --query "package.json" --repo crewchief --mode fts` returns results (no syntax error)
- [x] Manual verification: `crewchief-maproom search --query "src/main.rs" --repo crewchief --mode fts` returns results (no syntax error)
- [x] Manual verification: `crewchief-maproom search --query "array[0]" --repo crewchief --mode fts` returns results (no syntax error)
- [x] Manual verification: `crewchief-maproom search --query "user@email.com" --repo crewchief --mode fts` returns results (no syntax error)
- [ ] Commit created with conventional commit format: `fix(maproom): comprehensive FTS5 query sanitization`

## Technical Requirements

### File Location
- **File**: `/home/vscode/.crewchief/worktrees/audit/crates/maproom/src/db/sqlite/fts.rs`
- **Function**: `build_fts_query()` starting at line 43
- **Modification lines**: 49-56 (current sanitization chain)

### Code Changes

**Add imports** (at top of file, after existing imports):
```rust
use once_cell::sync::Lazy;
use regex::Regex;
```

**Add static regex definition** (before `build_fts_query()` function):
```rust
static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^a-zA-Z0-9_\s]").unwrap()
});
```

**Replace lines 49-56** (current sanitization chain):
```rust
// BEFORE (remove this):
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "")
    .replace('-', " ")
    .replace(':', " ");

// AFTER (replace with this):
let clean = SPECIAL_CHAR_REGEX.replace_all(t, " ").to_string();
```

### Test Implementation

**Add test function** (after line 301, after `test_build_fts_query_colon_handling`):
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

### Dependencies
- **regex**: Already in `Cargo.toml` (used throughout maproom)
- **once_cell**: Already in dependency tree via other crates (no explicit add needed)

### Verification Commands

```bash
# 1. Performance baseline (before change)
time cargo test -p crewchief-maproom fts
# Note the execution time (e.g., "0.12s")

# 2. Unit tests (after change)
cargo test -p crewchief-maproom fts

# 3. Full test suite
cargo test -p crewchief-maproom

# 4. Manual verification (requires indexed repo)
crewchief-maproom search --query "package.json" --repo crewchief --mode fts
crewchief-maproom search --query "src/main.rs" --repo crewchief --mode fts
crewchief-maproom search --query "array[0]" --repo crewchief --mode fts
crewchief-maproom search --query "user@email.com" --repo crewchief --mode fts
crewchief-maproom search --query "template{value}" --repo crewchief --mode fts
crewchief-maproom search --query "path\to\file" --repo crewchief --mode fts

# 5. Verify no syntax errors in output
# Expected: Search results (or empty results)
# Not expected: "fts5: syntax error near '.'" or similar
```

## Implementation Notes

### Regex Pattern Explanation
- `[^...]` - Negated character class (match anything NOT in the set)
- `a-zA-Z0-9` - All alphanumeric characters
- `_` - Underscore (valid in FTS5 barewords)
- `\s` - Whitespace (preserve existing spaces)
- **Result**: ALL other characters are replaced with spaces

### Why Regex Whitelist vs Individual Replace Calls

**Advantages**:
1. **Comprehensive** - Handles ALL non-alphanumeric characters, not just known problematic ones
2. **Maintainable** - Single regex pattern vs. dozens of `.replace()` calls
3. **Future-proof** - No updates needed when new special characters are discovered
4. **Consistent** - Same pattern used by PostgreSQL FTS module (src/search/fts.rs:69)
5. **Clear intent** - Whitelist approach makes the rule obvious: "only allow alphanumeric and underscore"

**Performance**:
- Regex compiled once via `Lazy` (amortized to zero overhead)
- Single pass over string vs. multiple `.replace()` allocations
- Expected: Neutral or slight improvement

### Edge Cases (Already Handled by Existing Logic)
- Multiple consecutive special chars: `"package..json"` → `"package  json"` → filtered to `"package json"`
- Leading special chars: `".gitignore"` → `" gitignore"` → filtered to `"gitignore"`
- Trailing special chars: `"file."` → `"file "` → filtered to `"file"`
- Only special chars: `"..."` → `"   "` → filtered to empty (handled by lines 62-64)

### Search Quality Impact
**Important**: Converting special characters to spaces changes query semantics from "exact match" to "contains any term".

**Example**:
- Query: `package.json`
- Sanitized: `package json`
- FTS Query: `package* OR json*`
- Matches: ANY file containing "package" OR "json"

**What this matches**:
- `package.json` (desired - contains both terms)
- `package.ts` (contains "package")
- `config.json` (contains "json")

**Why this is acceptable**:
1. FTS mode is keyword search - not designed for exact filename matching
2. Vector search handles precision - hybrid mode uses vector similarity for ranking
3. User intent - user searching "package.json" likely wants files related to package configuration
4. Better than failure - returning broader results is better than syntax errors

## Dependencies
- **No prerequisite tickets** - Self-contained change to one function
- **No external dependencies** - Uses existing crates (regex, once_cell)
- **No database migration** - Query-time fix only

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaks existing queries | Low | Medium | Comprehensive test suite catches regressions |
| Performance degradation | Very Low | Low | Baseline measurement required, rollback if >5% |
| Edge cases not covered | Low | Low | Existing filter logic handles whitespace/empty |
| Deployment issues | Very Low | Low | No migration required, instant rollback available |

### Risk Details

**Risk 1: Breaks existing queries**
- **Why low probability**: Only adds handling for characters that previously caused errors
- **Detection**: Unit tests would fail
- **Recovery**: Rollback to previous binary

**Risk 2: Performance degradation**
- **Why very low probability**: Regex compilation is one-time cost, string operations are O(n)
- **Detection**: Performance baseline comparison
- **Recovery**: Rollback if regression >5%

**Risk 3: Edge cases not covered**
- **Why low probability**: Existing logic already handles whitespace normalization
- **Detection**: Unit tests cover edge cases (multiple dots, leading dots, etc.)
- **Recovery**: Add additional test cases if discovered

## Files/Packages Affected

### Modified Files
- `crates/maproom/src/db/sqlite/fts.rs`
  - Add imports: `use once_cell::sync::Lazy;` and `use regex::Regex;`
  - Add static regex: `SPECIAL_CHAR_REGEX`
  - Refactor: `build_fts_query()` sanitization logic (lines 49-56)
  - Add test: `test_build_fts_query_comprehensive_sanitization` (after line 301)

### No Changes Required
- `Cargo.toml` - Dependencies already present
- Integration tests - Existing tests validate behavior
- Documentation - User-facing behavior doesn't change (queries that failed now work)

## Timeline
**Estimated time**: 45-60 minutes

| Task | Duration | Cumulative |
|------|----------|------------|
| Refactor sanitization logic | 15 min | 15 min |
| Write comprehensive unit test | 15 min | 30 min |
| Measure performance baseline | 5 min | 35 min |
| Run unit tests | 5 min | 40 min |
| Manual verification | 10 min | 50 min |
| Create commit | 5 min | 55 min |

**Contingency buffer**: +15 min for unexpected test failures or regex debugging (total 60-75 min)

## Commit Message

```
fix(maproom): comprehensive FTS5 query sanitization

Replace individual .replace() calls with regex whitelist pattern
[^a-zA-Z0-9_\s] to sanitize ALL special characters in FTS5 queries.

This fixes queries containing dots (package.json), slashes (src/main.rs),
brackets (array[0]), braces (template{value}), at-signs (user@email.com),
backslashes (path\to\file), and other special characters that caused
FTS5 syntax errors.

Implementation uses regex crate with once_cell::Lazy for one-time
compilation, matching the approach in PostgreSQL FTS module.

Comprehensive test coverage includes 8 test cases covering all
character categories. Performance baseline verified (< 5% overhead).

Project: FTSFIX
```

---

## Notes for Agents

### For rust-engineer
1. Read the current implementation at `crates/maproom/src/db/sqlite/fts.rs`
2. Measure performance baseline BEFORE making changes
3. Follow the exact code changes specified in Technical Requirements
4. Add the comprehensive test as specified
5. Verify all tests pass
6. Compare performance to baseline (must be within 5%)

### For unit-test-runner
1. Run `cargo test -p crewchief-maproom fts` to verify new test passes
2. Run `cargo test -p crewchief-maproom` to verify no regressions
3. Check execution time against baseline (must be within 5%)
4. Report any test failures with full output

### For verify-ticket
1. Confirm all acceptance criteria checkboxes are checked
2. Verify the comprehensive test exists with all 8 test cases
3. Confirm all existing tests still pass
4. Validate manual verification was performed (or document if skipped with reason)
5. Check commit message follows conventional commit format
6. Verify performance is within 5% of baseline

### For commit-ticket
1. Use the exact commit message format specified above
2. Ensure commit body explains the regex whitelist approach
3. Include project slug: FTSFIX
4. Reference this ticket: FTSFIX-1001
