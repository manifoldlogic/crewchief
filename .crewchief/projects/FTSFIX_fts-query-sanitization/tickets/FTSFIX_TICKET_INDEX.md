# FTSFIX Ticket Index

Project: FTS Query Sanitization
Location: `.crewchief/projects/FTSFIX_fts-query-sanitization`

## Overview

This project fixes incomplete FTS5 query sanitization by refactoring to a regex whitelist approach. Single-phase project with one comprehensive ticket.

**Timeline**: 45-60 minutes
**Status**: Ready for implementation

## Phase 1: Fix and Test

**Objective**: Refactor sanitization to use regex whitelist for comprehensive special character handling

### Tickets

| Ticket ID | Title | Agent | Status | Estimated Time |
|-----------|-------|-------|--------|----------------|
| FTSFIX-1001 | Comprehensive FTS Query Sanitization | rust-engineer | Not Started | 45-60 min |

## Phase 1 Details

### FTSFIX-1001: Comprehensive FTS Query Sanitization
- **File**: `crates/maproom/src/db/sqlite/fts.rs`
- **Function**: `build_fts_query()` (line 43)
- **Change**: Replace individual `.replace()` calls with regex whitelist `[^a-zA-Z0-9_\s]`
- **Test**: Add `test_build_fts_query_comprehensive_sanitization` with 8+ test cases
- **Verification**: Unit tests pass, manual queries work, performance within 5% baseline
- **Agents**: rust-engineer, unit-test-runner, verify-ticket, commit-ticket

## Ticket Status Legend

- **Not Started**: No work begun
- **In Progress**: Work actively being performed
- **Testing**: Implementation complete, tests running
- **Verification**: Tests pass, awaiting verify-ticket
- **Complete**: All acceptance criteria met, verified, committed

## Execution Order

1. **FTSFIX-1001** - Can be executed immediately (no dependencies)

## Success Criteria

Project complete when:
- [ ] FTSFIX-1001 verified and committed
- [ ] All existing tests pass
- [ ] Manual verification confirms fix works
- [ ] Performance within 5% of baseline

## Reference Documents

- **Plan**: `planning/plan.md`
- **Architecture**: `planning/architecture.md`
- **Quality Strategy**: `planning/quality-strategy.md`
- **Project Review**: `planning/project-review.md`

## Notes

This is a focused bug fix that can be completed in a single session. The regex whitelist approach provides comprehensive coverage for ALL special characters, making future character-by-character fixes unnecessary.

**Key implementation details**:
- Dependencies already in `Cargo.toml` (regex, once_cell)
- Pattern matches PostgreSQL FTS module (src/search/fts.rs:69)
- 8 comprehensive test cases covering dots, slashes, brackets, braces, at-signs, backslashes, mixed chars, operators
- Performance requirement: <5% overhead vs baseline
- No database migration required (query-time fix only)
