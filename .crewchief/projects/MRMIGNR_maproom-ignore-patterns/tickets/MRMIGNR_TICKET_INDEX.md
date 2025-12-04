# MRMIGNR Ticket Index

Project: Maproom Ignore Patterns (.maproomignore support)

## Overview

This project implements `.maproomignore` support to allow users to exclude files from maproom indexing without modifying `.gitignore`. All work is completed in a single phase with clear dependencies between tickets.

## Phase 1: Unified Ignore Pattern Implementation

**Objective:** Implement complete `.maproomignore` support with unified pattern handling across both scan and watch operations.

### Tickets

| ID | Title | Status | Agent | Dependencies |
|----|-------|--------|-------|--------------|
| MRMIGNR-1001 | Pattern Loading Infrastructure | Not Started | rust-indexer-engineer | None |
| MRMIGNR-1002 | Scan Integration | Not Started | rust-indexer-engineer | MRMIGNR-1001 |
| MRMIGNR-1003 | Watch Integration | Not Started | rust-indexer-engineer | MRMIGNR-1001 |
| MRMIGNR-1004 | Unit Tests | Not Started | rust-indexer-engineer | MRMIGNR-1001 |
| MRMIGNR-1005 | Integration Tests | Not Started | rust-indexer-engineer | MRMIGNR-1002, MRMIGNR-1003 |
| MRMIGNR-1006 | Documentation Update | Not Started | documentation-writer | MRMIGNR-1001, MRMIGNR-1002, MRMIGNR-1003, MRMIGNR-1005 |

## Execution Order

**Critical path:**
1. MRMIGNR-1001 (foundation - blocks all other work)
2. MRMIGNR-1002 + MRMIGNR-1003 + MRMIGNR-1004 (can run in parallel after 1001)
3. MRMIGNR-1005 (requires 1002 and 1003 complete)
4. MRMIGNR-1006 (documentation, requires all others complete)

**Recommended order:**
1. MRMIGNR-1001 - Pattern loading infrastructure
2. MRMIGNR-1004 - Unit tests (verify foundation works)
3. MRMIGNR-1002 - Scan integration
4. MRMIGNR-1003 - Watch integration
5. MRMIGNR-1005 - Integration tests (verify end-to-end)
6. MRMIGNR-1006 - Documentation (capture complete system)

## Ticket Details

### MRMIGNR-1001: Pattern Loading Infrastructure
- **Scope**: ~2-3 hours
- **Files**: `crates/maproom/src/incremental/ignore.rs`
- **Deliverables**: `load_ignore_patterns()` function, `from_repository()` constructor
- **Key risk**: Pattern parsing edge cases
- **Success criteria**: Existing tests pass, no regression

### MRMIGNR-1002: Scan Integration
- **Scope**: ~3-4 hours
- **Files**: `crates/maproom/src/indexer/mod.rs`
- **Deliverables**: OverrideBuilder integration, scan respects .maproomignore
- **Key risk**: OverrideBuilder API usage
- **Success criteria**: Manual smoke test passes, files excluded correctly

### MRMIGNR-1003: Watch Integration
- **Scope**: ~3-4 hours
- **Files**: `crates/maproom/src/incremental/worktree_watcher.rs`
- **Deliverables**: Event filtering in `event_conversion_task()`
- **Key risk**: Breaking incremental updates
- **Success criteria**: Events filtered correctly, no regression in watch behavior

### MRMIGNR-1004: Unit Tests
- **Scope**: ~2-3 hours
- **Files**: `crates/maproom/src/incremental/ignore.rs` (tests)
- **Deliverables**: 9 unit tests covering pattern loading, parsing, matching
- **Key risk**: Tests don't catch edge cases
- **Success criteria**: All tests pass, edge cases covered

### MRMIGNR-1005: Integration Tests
- **Scope**: ~4-5 hours
- **Files**: `crates/maproom/tests/maproomignore_test.rs` (new file)
- **Deliverables**: 4 integration tests for scan, watch, errors, gitignore independence
- **Key risk**: Flaky async tests
- **Success criteria**: All integration tests pass reliably

### MRMIGNR-1006: Documentation Update
- **Scope**: ~2 hours
- **Files**: `crates/maproom/CLAUDE.md`
- **Deliverables**: Usage examples, pattern syntax, precedence rules, limitations
- **Key risk**: Documentation becomes outdated
- **Success criteria**: Clear, comprehensive documentation with examples

## Total Estimated Effort

**Phase 1**: 16-21 hours total
- Implementation: 8-11 hours (tickets 1001-1003)
- Testing: 6-8 hours (tickets 1004-1005)
- Documentation: 2 hours (ticket 1006)

## Success Metrics

From plan.md:

**Functional:**
- [ ] Scan operation ignores files matching `.maproomignore` patterns
- [ ] Watch operation ignores file changes matching `.maproomignore` patterns
- [ ] `.gitignore` patterns continue to work unchanged
- [ ] Invalid patterns cause fail-fast errors at startup

**Quality:**
- [ ] All unit tests pass (including new pattern tests)
- [ ] Integration tests demonstrate scan/watch parity
- [ ] No regression in existing test suite
- [ ] Code passes `cargo clippy` with no warnings

**Documentation:**
- [ ] CLAUDE.md updated with `.maproomignore` section
- [ ] Example `.maproomignore` file provided
- [ ] Pattern precedence clearly documented

**Performance:**
- [ ] Scan benchmark shows <1% overhead with .maproomignore
- [ ] Watch event filtering adds <1ms per event

## References

- **Planning**: `.crewchief/projects/MRMIGNR_maproom-ignore-patterns/planning/plan.md`
- **Architecture**: `.crewchief/projects/MRMIGNR_maproom-ignore-patterns/planning/architecture.md`
- **Quality Strategy**: `.crewchief/projects/MRMIGNR_maproom-ignore-patterns/planning/quality-strategy.md`

## Notes

- Single phase implementation (no cross-phase dependencies)
- All tickets scope 2-8 hours as required
- Clear dependency chain ensures logical progression
- Pattern validation is fail-fast (errors at startup, not during indexing)
- Hot-reload NOT supported in MVP (watcher restart required for pattern changes)
