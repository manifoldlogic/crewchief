# Ticket: FTSFIX-1002: Deduplicate FTS Query Sanitization Logic

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
Refactor FTS query sanitization in `crates/maproom/src/db/sqlite/mod.rs` to use the shared regex-based approach from `fts.rs`, eliminating code duplication and ensuring consistent comprehensive special character handling across all FTS query building locations.

## Background
PR #19 (FTSFIX-1001) successfully refactored `build_fts_query()` in `fts.rs` to use regex whitelist pattern `[^a-zA-Z0-9_\s]` for comprehensive special character handling. However, CodeRabbit review identified that `mod.rs` contains **two additional locations** with similar FTS query building logic that still use the old character-by-character `.replace()` approach:

**Locations with duplicated logic:**
1. **Lines 721-740** - `query()` method FTS query building
2. **Lines 845-864** - `query_fts()` method FTS query building

**Current approach in mod.rs** (incomplete):
```rust
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "");
```

This creates:
- **Code duplication** - Same logic in three places requiring synchronized updates
- **Inconsistency** - `fts.rs` handles ALL special characters, `mod.rs` handles only 5
- **Maintenance burden** - Changes to sanitization must be made in three locations
- **Bug potential** - `mod.rs` still vulnerable to same special character bugs FTSFIX-1001 fixed

**Review comment from CodeRabbit:**
> "However, `crates/maproom/src/db/sqlite/mod.rs` contains similar FTS query building logic at lines 722–738 and 846–862 that still uses the old approach with individual `.replace()` calls. This creates code duplication and inconsistency in the codebase."

## Acceptance Criteria
- [ ] Shared `sanitize_fts_term()` function created in `fts.rs` (public visibility)
- [ ] Function signature: `pub fn sanitize_fts_term(term: &str) -> String`
- [ ] Function uses existing `SPECIAL_CHAR_REGEX` from FTSFIX-1001
- [ ] `build_fts_query()` in `fts.rs` refactored to use `sanitize_fts_term()`
- [ ] `query()` method in `mod.rs` (lines 721-740) refactored to use `sanitize_fts_term()`
- [ ] `query_fts()` method in `mod.rs` (lines 845-864) refactored to use `sanitize_fts_term()`
- [ ] All existing FTS tests pass: `cargo test -p crewchief-maproom fts`
- [ ] All existing sqlite tests pass: `cargo test -p crewchief-maproom -- db::sqlite`
- [ ] Manual verification: Search queries in both code paths return results
- [ ] No performance regression (within 5% of baseline)
- [ ] Commit created with conventional commit format: `refactor(maproom): deduplicate FTS query sanitization logic`

## Technical Requirements

### File Locations
- **Primary file**: `/workspace/crates/maproom/src/db/sqlite/fts.rs`
- **Secondary file**: `/workspace/crates/maproom/src/db/sqlite/mod.rs`

### Code Changes

#### 1. Extract Shared Function in `fts.rs`

**Add public function** (after `SPECIAL_CHAR_REGEX` definition, before `build_fts_query()`):

```rust
/// Sanitize a search term for FTS5 queries by replacing special characters with spaces.
/// Uses regex whitelist `[^a-zA-Z0-9_\s]` to handle ALL special characters comprehensively.
///
/// # Examples
/// ```
/// assert_eq!(sanitize_fts_term("package.json"), "package json");
/// assert_eq!(sanitize_fts_term("src/main.rs"), "src main rs");
/// assert_eq!(sanitize_fts_term("array[0]"), "array 0 ");
/// ```
pub fn sanitize_fts_term(term: &str) -> String {
    SPECIAL_CHAR_REGEX.replace_all(term, " ").to_string()
}
```

**Refactor `build_fts_query()`** to use new function (around line 66):

```rust
// BEFORE:
.map(|t| {
    let clean = SPECIAL_CHAR_REGEX.replace_all(t, " ").to_string();
    // ...
})

// AFTER:
.map(|t| {
    let clean = sanitize_fts_term(t);
    // ...
})
```

#### 2. Update `mod.rs` - First Location (lines 721-740)

**Import the new function** (at top of file):
```rust
use super::fts::sanitize_fts_term;
```

**Refactor query building** (around line 726):

```rust
// BEFORE (lines 726-731):
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "");

// AFTER:
let clean = sanitize_fts_term(t);
```

#### 3. Update `mod.rs` - Second Location (lines 845-864)

**Refactor query building** (around line 850):

```rust
// BEFORE (lines 850-855):
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "");

// AFTER:
let clean = sanitize_fts_term(t);
```

### Testing Strategy

**Unit tests:**
```bash
# FTS module tests (should still pass)
cargo test -p crewchief-maproom fts

# SQLite module tests (should still pass)
cargo test -p crewchief-maproom -- db::sqlite

# Full test suite
cargo test -p crewchief-maproom
```

**Manual verification:**
```bash
# Test query via fts.rs path (search command uses this)
crewchief-maproom search --query "package.json" --repo crewchief --mode fts

# Test query via mod.rs paths (internal database methods)
# (This requires integration testing through actual search operations)
```

**Performance baseline:**
```bash
# Before changes
time cargo test -p crewchief-maproom fts
# Note execution time

# After changes
time cargo test -p crewchief-maproom fts
# Verify within 5% of baseline
```

## Implementation Notes

### Why This Matters

**Code Quality:**
- **DRY Principle** - Eliminates duplication of sanitization logic
- **Single Source of Truth** - Changes to sanitization happen in one place
- **Consistency** - All FTS queries use same comprehensive sanitization

**Bug Prevention:**
- **Comprehensive Coverage** - `mod.rs` now handles ALL special characters, not just 5
- **Future-Proof** - Any sanitization improvements benefit all code paths
- **Maintainability** - Developers don't need to remember three locations

**Risk Mitigation:**
- **Low Risk** - Function extraction is mechanical refactoring
- **Test Coverage** - Existing tests verify behavior unchanged
- **Isolated Change** - Only affects FTS query building, not search logic

### Scope Boundaries

**In Scope:**
- Extract shared function from FTSFIX-1001 implementation
- Refactor `mod.rs` to use shared function
- Verify existing tests still pass

**Out of Scope:**
- Changing sanitization behavior (already fixed in FTSFIX-1001)
- Adding new tests (comprehensive tests added in FTSFIX-1001)
- Modifying search ranking or scoring logic

## Dependencies
- **Prerequisite**: FTSFIX-1001 must be merged (contains `SPECIAL_CHAR_REGEX` definition)
- **No new dependencies** - Uses existing `regex` and `once_cell` from FTSFIX-1001

## References
- **Original PR**: #19 (FTSFIX-1001: Comprehensive FTS5 query sanitization)
- **Review Comment**: CodeRabbit review noting code duplication in `mod.rs`
- **Architecture**: `.crewchief/archive/projects/FTSFIX_fts-query-sanitization/planning/architecture.md`
- **Quality Strategy**: `.crewchief/archive/projects/FTSFIX_fts-query-sanitization/planning/quality-strategy.md`

## Estimated Time
- **Implementation**: 20-30 minutes
- **Testing**: 10-15 minutes
- **Verification**: 5-10 minutes
- **Total**: 35-55 minutes
