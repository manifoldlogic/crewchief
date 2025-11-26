# MAPCLI Tickets Review Report

**Review Date**: 2025-11-26
**Reviewer**: Claude Code (Tickets Review Agent)
**Project**: MAPCLI - Maproom CLI Abstraction

## Executive Summary

**Overall Assessment**: ✅ READY FOR EXECUTION

The 6 tickets created for the MAPCLI project are comprehensive, well-structured, and accurately reflect the codebase state. The review identified **0 critical issues**, **2 minor observations**, and **3 recommendations** for optimization during implementation.

### Summary Table

| Ticket | Title | Complexity | Dependencies | Assessment |
|--------|-------|------------|--------------|------------|
| MAPCLI-1000 | Add BackendType Enum and Trait Method | Low | None | ✅ Ready |
| MAPCLI-1001 | Update main.rs to use get_store() Factory | Medium | 1000 | ✅ Ready |
| MAPCLI-1002 | Refactor Daemon to use VectorStore Trait | Medium | 1000 | ✅ Ready |
| MAPCLI-1003 | Add SQLite Backend Detection/Configuration | Medium | 1000 | ✅ Ready |
| MAPCLI-1004 | Update CLI Commands and Refactor status.rs | High | 1001 | ✅ Ready |
| MAPCLI-1005 | E2E Integration Tests with SQLite Backend | Medium | 1004 | ✅ Ready |

---

## Detailed Ticket Review

### MAPCLI-1000: Add BackendType Enum and Trait Method

**Status**: ✅ Ready for implementation

**Codebase Validation**:
- Confirmed `VectorStore` trait in `src/db/mod.rs` has no `backend_type()` method
- Both `PostgresStore` and `SqliteStore` exist and implement `VectorStore`
- Feature gating pattern (`#[cfg(feature = "sqlite")]`) already used throughout codebase

**Accuracy**: Excellent - ticket correctly identifies the exact location and pattern for implementation.

**Technical Feasibility**: Low risk. Adding a sync method to a trait with only internal implementors is straightforward.

**Observations**: None.

---

### MAPCLI-1001: Update main.rs to use get_store() Factory

**Status**: ✅ Ready for implementation

**Codebase Validation**:
- `get_store()` factory already exists in `src/db/factory.rs` and returns `Arc<dyn VectorStore>`
- main.rs currently uses direct `db::connect()` calls for some commands
- Commands listed (search, vector-search, db cleanup-stale, db migrate) are accurately identified

**Accuracy**: Excellent - ticket correctly identifies which commands need updating and which remain PostgreSQL-only.

**Technical Feasibility**: Medium effort. Main changes are mechanical - replacing connection calls with factory calls.

**Observations**:
- Status command is correctly deferred to MAPCLI-1004 (requires status.rs refactor first)

---

### MAPCLI-1002: Refactor Daemon to use VectorStore Trait

**Status**: ✅ Ready for implementation

**Codebase Validation**:
- Confirmed `DaemonState` uses `PgPool` (deadpool-postgres pool)
- Confirmed `execute_search()` contains raw SQL queries:
  - `SELECT id FROM maproom.repos WHERE name = $1`
  - Chunk detail queries with joins
- SearchHit struct exists with all required fields

**Accuracy**: Excellent - ticket precisely identifies the raw SQL patterns that need replacement.

**Technical Feasibility**: Medium effort. The VectorStore trait already has all needed search methods. Main work is routing calls through trait methods.

**Observations**: None.

---

### MAPCLI-1003: Add SQLite Backend Detection/Configuration

**Status**: ✅ Ready for implementation

**Codebase Validation**:
- `get_database_url()` exists in `src/db/connection.rs`
- Current detection is URL prefix-based (`sqlite://` vs `postgresql://`)
- `dirs` crate not currently in dependencies (will need to add)

**Accuracy**: Excellent - detection order logic is well-defined.

**Technical Feasibility**: Medium effort. Straightforward implementation with clear fallback chain.

**Observations**:
- Minor: May need to add `dirs` crate dependency if not already present

---

### MAPCLI-1004: Update CLI Commands and Refactor status.rs

**Status**: ✅ Ready for implementation (CRITICAL TICKET)

**Codebase Validation**:
- **CONFIRMED CRITICAL ISSUE**: `status.rs` lines 28-34 create direct PostgreSQL connection:
  ```rust
  let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;
  ```
- **CONFIRMED**: Uses JSONB operators (`@>`, `jsonb_build_array`) incompatible with SQLite
- This module completely bypasses the factory pattern

**Accuracy**: Excellent - this is the most critical ticket and accurately captures the exact problem.

**Technical Feasibility**: Medium-High effort. Requires:
1. Changing function signatures to accept `Arc<dyn VectorStore>`
2. Replacing SQL queries with trait method calls
3. Updating callers in main.rs

**Observations**:
- This ticket has the highest impact on SQLite compatibility
- Must be completed before E2E tests can pass

---

### MAPCLI-1005: E2E Integration Tests with SQLite Backend

**Status**: ✅ Ready for implementation

**Codebase Validation**:
- Tests directory structure exists at `crates/maproom/tests/`
- No existing SQLite fixture at `tests/fixtures/pre-indexed-maproom.db`
- `jq` and `netcat` tools assumed available (standard on most systems)

**Accuracy**: Excellent - test strategy using pre-indexed fixture is appropriate given scan deferral to Phase 2.

**Technical Feasibility**: Medium effort. Main work is:
1. Creating the pre-indexed fixture programmatically
2. Writing the bash test script
3. CI integration

**Observations**:
- Daemon tests assume specific port (19999) - should ensure no conflicts
- Test creates copy of fixture for isolation - good practice

---

## Critical Issues

**None identified.**

All tickets accurately reflect the codebase state and have feasible implementation paths.

---

## Warnings

### W1: Dependency on `dirs` Crate (MAPCLI-1003)

**Severity**: Low

The `dirs` crate may need to be added to `Cargo.toml` for home directory resolution. The ticket mentions this but doesn't explicitly list it as a task.

**Recommendation**: Check `Cargo.toml` early in MAPCLI-1003 implementation; add `dirs = "5"` if needed.

### W2: Test Fixture Creation Strategy (MAPCLI-1005)

**Severity**: Low

The ticket provides two approaches (bash script vs Rust programmatic) for fixture creation. The programmatic Rust approach is more maintainable.

**Recommendation**: Use the Rust `#[test]` approach with `#[ignore]` to create fixtures, making them reproducible.

---

## Recommendations

### R1: Parallel Execution Opportunity

MAPCLI-1002 (Daemon) and MAPCLI-1003 (Detection) can be developed in parallel after MAPCLI-1000 completes. Consider:

```
                    ┌─> MAPCLI-1002 (Daemon) ─┐
MAPCLI-1000 (Enum) ─┤                          ├─> MAPCLI-1004 (CLI/Status) ─> MAPCLI-1005 (E2E)
                    └─> MAPCLI-1003 (Detection)┘
```

This could reduce overall project time.

### R2: Add Unit Tests for BackendType (MAPCLI-1000)

While compilation tests are sufficient for the enum, consider adding a simple unit test:

```rust
#[test]
fn test_backend_type_values() {
    assert_ne!(BackendType::PostgreSQL, BackendType::SQLite);
}
```

This ensures the enum is usable in test contexts.

### R3: Document Error Messages Consistently (MAPCLI-1001, MAPCLI-1004)

Both tickets show graceful error messages for SQLite limitations. Ensure consistent formatting:

```
The '{{command}}' command requires PostgreSQL backend.
SQLite support for {{feature}} is coming in Phase 2.
Set MAPROOM_DATABASE_URL to a PostgreSQL connection string to use this command.
```

---

## Integration Assessment

### Dependency Graph Validation

```
MAPCLI-1000 ──┬──> MAPCLI-1001 ──> MAPCLI-1004 ──> MAPCLI-1005
              │
              ├──> MAPCLI-1002
              │
              └──> MAPCLI-1003
```

**Assessment**: ✅ Valid

- MAPCLI-1000 is correctly identified as the prerequisite for all other tickets
- MAPCLI-1004 correctly depends on MAPCLI-1001 (needs factory pattern established)
- MAPCLI-1005 correctly depends on MAPCLI-1004 (needs CLI working before E2E)

### Cross-Ticket Consistency

| Aspect | Consistency |
|--------|-------------|
| VectorStore usage | ✅ All tickets use same trait |
| Error message format | ✅ Consistent "Phase 2" messaging |
| Feature flag usage | ✅ All use `--features sqlite` |
| Test commands | ✅ All include both PostgreSQL and SQLite test paths |

### MVP Scope Adherence

All tickets correctly scope to Phase 1 MVP:
- ✅ search, status, cleanup-stale commands with SQLite
- ✅ Daemon serving JSON-RPC with SQLite
- ✅ Graceful errors for scan/upsert/watch
- ✅ E2E tests with pre-indexed fixture

Phase 2 items (scan, upsert, watch indexing) are explicitly deferred.

---

## Files Modified Summary

| File | Tickets Affecting |
|------|-------------------|
| `src/db/mod.rs` | MAPCLI-1000 |
| `src/db/postgres/mod.rs` | MAPCLI-1000 |
| `src/db/sqlite/mod.rs` | MAPCLI-1000 |
| `src/db/factory.rs` | MAPCLI-1003 |
| `src/db/connection.rs` | MAPCLI-1003 |
| `src/main.rs` | MAPCLI-1001, MAPCLI-1004 |
| `src/daemon/mod.rs` | MAPCLI-1002 |
| `src/status.rs` | MAPCLI-1004 |
| `tests/fixtures/*` | MAPCLI-1005 |
| `tests/e2e/*` | MAPCLI-1005 |
| `Cargo.toml` | MAPCLI-1003 (if dirs crate needed) |

No conflicting modifications detected.

---

## Conclusion

The MAPCLI tickets are **ready for execution**. The tickets:

1. ✅ Accurately reflect the current codebase state
2. ✅ Have correct dependency ordering
3. ✅ Include comprehensive acceptance criteria
4. ✅ Provide clear implementation guidance
5. ✅ Adhere to MVP scope
6. ✅ Include testing strategies for both backends

**Recommendation**: Proceed with execution starting from MAPCLI-1000.
