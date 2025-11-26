# Implementation Plan: SQLite Backend Fixes

## Overview

This project fixes the broken SQLite backend implementation from the SQLVEC project, making it compile and pass basic tests. Vector search is deferred to a future project.

**Estimated Tickets**: 5 (plus 1 prerequisite)
**Priority**: High (blocks zero-dependency distribution)

---

## Phase 0: Prerequisites (Ticket 1000)

### Goal
Commit existing fixes made during investigation that are prerequisites for this project.

### Ticket 1000: Commit Baseline Fixes
**Status**: Pre-existing uncommitted changes

The following fixes were made during the initial investigation and must be committed first:
- `crates/maproom/src/db/mod.rs`: Added `#[cfg(feature = "sqlite")]` gate on sqlite module
- `crates/maproom/src/db/factory.rs`: Feature-gated SQLite imports and usage
- `crates/maproom/src/db/postgres/mod.rs`: Refactored to use connection pool
- `crates/maproom/src/db/queries.rs`: Fixed SearchHit type import
- `packages/vscode-maproom/src/extension.ts`: Restored working version

**Test**: `cargo check` passes (postgres feature, default)
**Agent**: rust-indexer-engineer

---

## Phase 1: Compile Fixes + CI (Tickets 1001, 1005)

### Goal
Make `cargo check --features sqlite` pass and establish CI safety net.

### Ticket 1001: Fix SQLite Compilation (Merged from 1001+1002)
This ticket combines all compile-time fixes into one coherent change:

1. **Cargo.toml Dependencies**
   - Add `chrono` feature to `rusqlite`: `features = ["bundled", "chrono"]`
   - Verify `r2d2` and `r2d2_sqlite` version compatibility

2. **Module Export**
   - Add `pub mod schema;` at top of `sqlite/mod.rs`

3. **Move Semantics Fix**
   - Fix `find_chunk_by_symbol` (lines 523-577):
     - Clone `relpath` before first use: `let relpath_owned = relpath.map(|s| s.to_string());`
     - Use `relpath_owned.as_deref()` in subsequent branches
     - Consolidate SQL query logic to fix parameter mismatches

4. **Connection Initialization**
   - Add `PRAGMA busy_timeout = 5000;` to `with_init` callback

5. **Security Hardening**
   - Add file permissions (0600) after pool creation in `connect()`

**Test**: `cargo check --features sqlite` passes
**Agent**: rust-indexer-engineer

### Ticket 1005: Update CI for SQLite Feature (Moved Earlier)
CI must be established early to catch regressions:

- Add `--features sqlite` to GitHub Actions workflow
- Ensure both postgres and sqlite features tested
- Add feature matrix to test job

**Test**: CI passes with green check on both features
**Agent**: github-actions-specialist

---

## Phase 2: Runtime Functionality (Tickets 1002, 1003)

### Goal
SQLite backend can perform basic CRUD operations with correct FTS search.

### Ticket 1002: Fix Schema Initialization
1. **Schema Alignment**
   - Add missing `ts_doc_text TEXT` column to chunks table
   - Fix FTS5 table definition (use standalone table, not external content)

2. **Schema Initialization**
   - Ensure `migrate()` creates all tables
   - Add indices for common queries
   - Handle schema already exists case (idempotency)

**Test**: In-memory SQLite creates all tables with correct columns
**Agent**: rust-indexer-engineer

### Ticket 1003: Fix CRUD Operations + FTS
1. **CRUD Operations**
   - Verify all `VectorStore` trait methods work
   - Fix any runtime errors discovered during testing
   - Stub vector operations with log warning (deferred)

2. **FTS5 Query Syntax Fix**
   - Fix invalid `"term"*` syntax (lines 454-459)
   - Change to valid prefix syntax: `term*` without quotes
   - Use `OR` instead of implicit AND for broader matching

**Test**: Create repo → worktree → file → chunk cycle completes; FTS search returns results
**Agent**: rust-indexer-engineer

---

## Phase 3: Testing (Ticket 1004)

### Goal
SQLite backend has test coverage validating all functionality.

### Ticket 1004: Add SQLite Unit Tests
- Create `tests/sqlite_store.rs`
- Test CRUD operations
- Test FTS search with valid syntax
- Test concurrent access (basic)
- Test idempotent operations

**Test**: `cargo test --features sqlite` passes
**Agent**: rust-indexer-engineer

---

## Phase Summary

| Phase | Tickets | Focus | Exit Criteria |
|-------|---------|-------|---------------|
| 0 | 1000 | Prerequisites | Uncommitted fixes committed |
| 1 | 1001, 1005 | Compile + CI | `cargo check --features sqlite` passes, CI green |
| 2 | 1002, 1003 | Runtime | Schema correct, CRUD + FTS work |
| 3 | 1004 | Quality | Tests pass |

---

## Agent Assignments

| Agent | Tickets | Responsibility |
|-------|---------|----------------|
| rust-indexer-engineer | 1000, 1001, 1002, 1003, 1004 | Rust implementation and tests |
| github-actions-specialist | 1005 | CI workflow updates |

---

## Dependencies

```
1000 (prerequisites) ──► 1001 (compile) ──┬──► 1002 (schema) ──► 1003 (CRUD+FTS) ──► 1004 (tests)
                                          │
                                          └──► 1005 (CI)
```

Phase 0 must complete first. In Phase 1, tickets 1001 and 1005 can run in parallel after 1000.

---

## Success Criteria

### Must Pass
```bash
cargo check --features sqlite        # Phase 1 complete
cargo test --features sqlite         # Phase 3 complete
cargo check                          # No regression (postgres)
```

### Should Pass
```bash
# Manual verification
MAPROOM_DATABASE_URL=sqlite://test.db cargo run -- status
```

---

## Out of Scope (Future Projects)

1. **Vector Search**: sqlite-vec integration for similarity search
2. **VSCode Extension**: SQLite mode in extension UI
3. **Benchmarks**: Performance comparison with Postgres
4. **Migration Tooling**: Convert between backends
5. **768-dim Support**: Multiple embedding dimensions

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| sqlite-vec API changes | Pinned to vendored commit |
| Postgres regression | CI tests both features (established in Phase 1) |
| Test flakiness | Use in-memory SQLite for tests |
| Lock contention | WAL mode + busy_timeout |
| Uncommitted work lost | Phase 0 commits prerequisites first |

---

## Rollback Strategy

The feature flag provides a natural rollback mechanism:
- If SQLite feature causes issues, compile with `--features postgres` only
- No changes to Postgres code path required
- Can disable sqlite feature in CI without code changes

---

## Definition of Done

- [ ] Prerequisite fixes committed (Phase 0)
- [ ] `cargo check --features sqlite` passes
- [ ] `cargo test --features sqlite` passes
- [ ] CI workflow tests both backends
- [ ] No regressions in default (postgres) feature
- [ ] Original SQLVEC project archived with reference to this fix
