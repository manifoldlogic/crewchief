# Project: FTS Query Sanitization

**Slug:** FTSFIX
**Status:** Complete
**Created:** 2025-12-09
**Completed:** 2025-12-13

## Summary

Fix FTS5 query sanitization to comprehensively handle ALL special characters in search queries. The `build_fts_query()` function currently uses individual `.replace()` calls for specific characters, leaving gaps for many special characters. This causes "fts5: syntax error" for common queries containing dots, slashes, brackets, braces, at-signs, and operators.

**Scope:** Refactor to regex whitelist sanitization + comprehensive unit tests
**Impact:** High (enables all special character queries: file paths, extensions, email addresses, arrays, templates, operators)
**Risk:** Low (well-tested approach, used by PostgreSQL FTS module)

## Problem Statement

Users cannot search for queries containing special characters in FTS mode because these characters cause FTS5 syntax errors. According to FTS5 documentation, only alphanumeric characters (A-Z, a-z, 0-9) and underscore (_) are valid in bareword queries. All other characters must be sanitized.

**Failed queries (real-world examples):**
- `package.json` → "fts5: syntax error near '.'"
- `src/main.rs` → "fts5: syntax error near '/'"
- `array[0]` → "fts5: syntax error near '['"
- `user@email.com` → "fts5: syntax error near '@'"
- `template{value}` → "fts5: syntax error near '{'"
- `path\to\file` → "fts5: syntax error near '\'"
- `a+b=c` → "fts5: syntax error near '+'"

**Why this matters:**
- FTS is the fallback mode when embeddings are unavailable
- Special character queries are fundamental to code search (paths, extensions, operators, syntax)
- Complete failure (no results) vs graceful degradation
- Current implementation only handles a subset of special characters

## Proposed Solution

Refactor the sanitization chain in `build_fts_query()` from individual `.replace()` calls to a **regex whitelist approach** that sanitizes ALL non-alphanumeric characters (except underscore) in one operation.

**Technical approach:**
```rust
// Replace lines 49-56 in /crates/maproom/src/db/sqlite/fts.rs
// BEFORE: Multiple .replace() calls
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "")
    .replace('-', " ")
    .replace(':', " ");

// AFTER: Regex whitelist
use once_cell::sync::Lazy;
use regex::Regex;

static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^a-zA-Z0-9_\s]").unwrap()
});

let clean = SPECIAL_CHAR_REGEX.replace_all(t, " ").to_string();
```

**Result:**
- `package.json` → `package* OR json*` → returns files with "package" or "json"
- `src/main.rs` → `src* OR main* OR rs*` → returns files with "src", "main", or "rs"
- `array[0]` → `array* OR 0*` → returns files with "array" or "0"
- `user@email.com` → `user* OR email* OR com*` → returns files with "user", "email", or "com"

**Why this works:**
- Whitelist approach: only allow `[a-zA-Z0-9_\s]`, replace everything else with spaces
- Comprehensive: handles ALL special characters, not just known problematic ones
- Maintainable: single regex pattern vs. dozens of `.replace()` calls
- Future-proof: no updates needed when new special characters are discovered
- Consistent: same pattern used by PostgreSQL FTS module

## Relevant Agents

- **project-planner** - Created this planning (complete)
- **ticket-creator** - Will generate implementation ticket
- **rust-engineer** - Will implement the one-line fix + test
- **unit-test-runner** - Will verify all tests pass
- **verify-ticket** - Will validate fix with real queries
- **commit-ticket** - Will create commit

## Planning Documents

All planning documents are complete:

- [analysis.md](planning/analysis.md) - Problem analysis, FTS5 special characters, user impact
- [architecture.md](planning/architecture.md) - Solution design, one-line fix, edge cases
- [plan.md](planning/plan.md) - Single-phase execution plan (30 min estimated)
- [quality-strategy.md](planning/quality-strategy.md) - Unit test strategy, regression prevention
- [security-review.md](planning/security-review.md) - SQL injection review, input validation analysis

## Key Decisions

1. **Use regex whitelist approach** (not individual `.replace()` calls) - Comprehensive, maintainable, future-proof
2. **Lazy static regex compilation** - One-time compilation overhead, amortized to zero
3. **Single phase execution** - Refactoring is focused enough for one atomic change
4. **Comprehensive test additions** - One test covering all special character types
5. **Performance baseline measurement** - Verify no regression from regex approach
6. **No migration needed** - Query-time fix, instant benefit on deployment

## Timeline

**Estimated time:** 45-60 minutes

| Phase | Deliverables | Duration |
|-------|--------------|----------|
| Phase 1: Fix and Test | Refactored sanitization, comprehensive unit test, performance baseline, verification, commit | 55 min |

## Success Criteria

- [x] Code: Regex whitelist `[^a-zA-Z0-9_\s]` implemented in `build_fts_query()`
- [x] Test: `test_build_fts_query_comprehensive_sanitization` passing (8+ test cases)
- [x] All existing tests pass (155 tests: 41 FTS + 114 SQLite)
- [x] Performance: Baseline measured, no regression >5%
- [x] Manual verification: All special character query types return results (no syntax errors)
  - `package.json` (dots)
  - `src/main.rs` (slashes)
  - `array[0]` (brackets)
  - `user@email.com` (at-signs)
- [x] Commit created with conventional commit format
- [x] Code deduplication: Shared `sanitize_fts_term()` function eliminates duplication across three locations

## Completed Tickets

All tickets completed and verified:

- **FTSFIX-1001:** Comprehensive FTS Query Sanitization - Committed 17d1206c
- **FTSFIX-1002:** Deduplicate FTS Query Sanitization Logic - Committed b9946ae2

## Final Deliverables

**Code Changes:**
- `crates/maproom/src/db/sqlite/fts.rs` - Regex whitelist sanitization + shared function
- `crates/maproom/src/db/sqlite/mod.rs` - Refactored to use shared sanitization

**Test Coverage:**
- 8 comprehensive test cases for special character handling
- 155 total tests passing (41 FTS + 114 SQLite)
- Zero regressions

**Impact:**
- Enables FTS queries with all special characters (dots, slashes, brackets, operators, etc.)
- Eliminates code duplication across three FTS query building locations
- Provides consistent comprehensive sanitization across all code paths

---

**Project Status:** Complete and ready for archive
