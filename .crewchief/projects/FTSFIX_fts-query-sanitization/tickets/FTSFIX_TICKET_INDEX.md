# FTSFIX Ticket Index

Project: FTS Query Sanitization
Location: `.crewchief/projects/FTSFIX_fts-query-sanitization`

## Overview

This project fixes incomplete FTS5 query sanitization by refactoring to a regex whitelist approach and eliminating code duplication.

**Timeline**: 80-115 minutes (2 phases)
**Status**: Phase 1 complete (PR #19), Phase 2 pending

## Phase 1: Fix and Test ✅ COMPLETE

**Objective**: Refactor sanitization to use regex whitelist for comprehensive special character handling

### Tickets

| Ticket ID | Title | Agent | Status | Estimated Time |
|-----------|-------|-------|--------|----------------|
| FTSFIX-1001 | Comprehensive FTS Query Sanitization | rust-engineer | Complete | 45-60 min |

### FTSFIX-1001: Comprehensive FTS Query Sanitization ✅
- **File**: `crates/maproom/src/db/sqlite/fts.rs`
- **Function**: `build_fts_query()` (line 43)
- **Change**: Replace individual `.replace()` calls with regex whitelist `[^a-zA-Z0-9_\s]`
- **Test**: Add `test_build_fts_query_comprehensive_sanitization` with 8+ test cases
- **Verification**: Unit tests pass, manual queries work, performance within 5% baseline
- **Agents**: rust-engineer, unit-test-runner, verify-ticket, commit-ticket
- **PR**: #19 (audit branch, pending merge)

## Phase 2: Deduplicate and Refactor

**Objective**: Eliminate code duplication by extracting shared sanitization function and refactoring `mod.rs` to use it

**Prerequisite**: Phase 1 (FTSFIX-1001) must be merged to main

### Tickets

| Ticket ID | Title | Agent | Status | Estimated Time |
|-----------|-------|-------|--------|----------------|
| FTSFIX-1002 | Deduplicate FTS Query Sanitization Logic | rust-engineer | Not Started | 35-55 min |

### FTSFIX-1002: Deduplicate FTS Query Sanitization Logic
- **Files**:
  - `crates/maproom/src/db/sqlite/fts.rs` (extract shared function)
  - `crates/maproom/src/db/sqlite/mod.rs` (refactor 2 locations)
- **Issue**: CodeRabbit review identified code duplication in `mod.rs` at lines 722-738 and 846-862
- **Change**: Extract `sanitize_fts_term()` function and use it in all 3 locations
- **Benefit**: DRY principle, single source of truth, consistent behavior across all FTS queries
- **Risk**: Low (mechanical refactoring with existing test coverage)
- **Agents**: rust-engineer, unit-test-runner, verify-ticket, commit-ticket

## Ticket Status Legend

- **Not Started**: No work begun
- **In Progress**: Work actively being performed
- **Testing**: Implementation complete, tests running
- **Verification**: Tests pass, awaiting verify-ticket
- **Complete**: All acceptance criteria met, verified, committed

## Execution Order

1. **FTSFIX-1001** - ✅ Complete (PR #19, audit branch)
2. **FTSFIX-1002** - Pending (requires FTSFIX-1001 merge)

## Success Criteria

**Phase 1 Success** ✅:
- [x] FTSFIX-1001 verified and committed
- [x] All existing tests pass
- [x] Manual verification confirms fix works
- [x] Performance within 5% of baseline

**Phase 2 Success**:
- [ ] FTSFIX-1002 verified and committed
- [ ] Shared function extracted from FTSFIX-1001
- [ ] All 3 locations use shared sanitization
- [ ] All existing tests still pass
- [ ] No performance regression

**Project Complete**:
- [ ] Both phases verified and committed
- [ ] No FTS query building code duplication
- [ ] All FTS queries use comprehensive regex sanitization
- [ ] CodeRabbit review concerns addressed

## Reference Documents

- **Plan**: `planning/plan.md`
- **Architecture**: `planning/architecture.md`
- **Quality Strategy**: `planning/quality-strategy.md`
- **Project Review**: `planning/project-review.md`
- **Security Review**: `planning/security-review.md`

## Review Feedback Integration

**CodeRabbit PR #19 Review**:
> "However, `crates/maproom/src/db/sqlite/mod.rs` contains similar FTS query building logic at lines 722–738 and 846–862 that still uses the old approach with individual `.replace()` calls. This creates code duplication and inconsistency in the codebase."

**Resolution**: Created FTSFIX-1002 to address this feedback by:
1. Extracting shared `sanitize_fts_term()` function
2. Refactoring all 3 locations to use shared implementation
3. Ensuring consistency across entire codebase

## Notes

### Phase 1 (Complete)
Focused bug fix completed in PR #19. The regex whitelist approach provides comprehensive coverage for ALL special characters, making future character-by-character fixes unnecessary.

**Key implementation details**:
- Dependencies already in `Cargo.toml` (regex, once_cell)
- Pattern matches PostgreSQL FTS module (src/search/fts.rs:69)
- 8 comprehensive test cases covering dots, slashes, brackets, braces, at-signs, backslashes, mixed chars, operators
- Performance requirement: <5% overhead vs baseline
- No database migration required (query-time fix only)

### Phase 2 (Pending)
Code quality improvement to eliminate duplication identified in review. Low-risk refactoring that improves maintainability and ensures consistent behavior.

**Key benefits**:
- **DRY Principle**: Single sanitization function used everywhere
- **Maintainability**: Changes to sanitization logic in one place
- **Consistency**: All FTS queries handle special characters identically
- **Bug Prevention**: `mod.rs` now handles ALL special characters (not just 5)
