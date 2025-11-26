# SQLFIX Ticket Index

## Project Overview
**Project**: SQLFIX - SQLite Backend Fixes
**Total Tickets**: 6
**Phases**: 4 (Phase 0-3)

## Execution Order

```
Phase 0: SQLFIX-1000 (prerequisites)
    │
    ▼
Phase 1: SQLFIX-1001 (compile) ─┬─► SQLFIX-1005 (CI) [parallel]
                                │
                                ▼
Phase 2: SQLFIX-1002 (schema) ──► SQLFIX-1003 (CRUD+FTS)
                                      │
                                      ▼
Phase 3:                        SQLFIX-1004 (tests)
```

## Tickets by Phase

### Phase 0: Prerequisites
| ID | Title | Agent | Status |
|----|-------|-------|--------|
| [SQLFIX-1000](SQLFIX-1000_commit-baseline-fixes.md) | Commit Baseline Fixes | rust-indexer-engineer | ✅ Complete |

### Phase 1: Compile Fixes + CI
| ID | Title | Agent | Status |
|----|-------|-------|--------|
| [SQLFIX-1001](SQLFIX-1001_fix-sqlite-compilation.md) | Fix SQLite Compilation | rust-indexer-engineer | ✅ Complete |
| [SQLFIX-1005](SQLFIX-1005_update-ci-sqlite-feature.md) | Update CI for SQLite Feature | github-actions-specialist | ✅ Complete |

### Phase 2: Runtime Functionality
| ID | Title | Agent | Status |
|----|-------|-------|--------|
| [SQLFIX-1002](SQLFIX-1002_fix-schema-initialization.md) | Fix Schema Initialization | rust-indexer-engineer | ✅ Complete |
| [SQLFIX-1003](SQLFIX-1003_fix-crud-operations-fts.md) | Fix CRUD Operations + FTS | rust-indexer-engineer | ✅ Complete |

### Phase 3: Testing
| ID | Title | Agent | Status |
|----|-------|-------|--------|
| [SQLFIX-1004](SQLFIX-1004_add-sqlite-unit-tests.md) | Add SQLite Unit Tests | rust-indexer-engineer | ✅ Complete |

## Dependencies Graph

| Ticket | Depends On | Blocks |
|--------|------------|--------|
| SQLFIX-1000 | None | SQLFIX-1001, SQLFIX-1005 |
| SQLFIX-1001 | SQLFIX-1000 | SQLFIX-1002, SQLFIX-1005 |
| SQLFIX-1002 | SQLFIX-1001 | SQLFIX-1003 |
| SQLFIX-1003 | SQLFIX-1002 | SQLFIX-1004 |
| SQLFIX-1004 | SQLFIX-1003 | None |
| SQLFIX-1005 | SQLFIX-1000 | None (parallel with 1001) |

## Agent Assignments

| Agent | Tickets | Total |
|-------|---------|-------|
| rust-indexer-engineer | 1000, 1001, 1002, 1003, 1004 | 5 |
| github-actions-specialist | 1005 | 1 |

## Success Criteria

### Phase Completion Criteria
- **Phase 0**: Git working tree clean, `cargo check` passes
- **Phase 1**: `cargo check --features sqlite` passes, CI configured
- **Phase 2**: CRUD cycle works, FTS search returns results
- **Phase 3**: `cargo test --features sqlite` passes

### Project Completion Criteria
```bash
cargo check --features sqlite        # Pass
cargo test --features sqlite         # Pass
cargo check                          # No regression
```

## Plan Traceability

| Ticket | Plan Section |
|--------|--------------|
| SQLFIX-1000 | Phase 0: Prerequisites |
| SQLFIX-1001 | Phase 1: Compile Fixes + CI |
| SQLFIX-1002 | Phase 2: Runtime Functionality - Schema |
| SQLFIX-1003 | Phase 2: Runtime Functionality - CRUD+FTS |
| SQLFIX-1004 | Phase 3: Testing |
| SQLFIX-1005 | Phase 1: Compile Fixes + CI |

## Notes

- Tickets SQLFIX-1001 and SQLFIX-1005 can run in parallel after SQLFIX-1000
- SQLFIX-1005 (CI) is numbered higher but executes in Phase 1 for early regression catching
- Vector search is explicitly out of scope (deferred to future SQLVEC2 project)
