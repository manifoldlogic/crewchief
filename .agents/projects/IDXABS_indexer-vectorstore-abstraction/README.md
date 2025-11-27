# Project: IDXABS_indexer-vectorstore-abstraction

**Status**: In Progress - Phase 6 (Completion) Required
**Note**: Project was incorrectly archived. Core modules stubbed, tests don't compile.
**Tickets**: 18 tickets total (14 original + 4 completion tickets)
**TODOs Found**: 52 stub implementations across 21 files

## Current State

### What's Done ✅
- PostgreSQL files deleted (`db/postgres/`, `pool.rs`, `queries.rs`, `factory.rs`)
- VectorStore trait removed
- Module imports updated to use `SqliteStore` directly
- Feature flags removed
- Main.rs backend switching removed
- Basic commands work: `scan`, `upsert`, `search`, `generate-embeddings`

### What's NOT Done ❌
1. **Watch command stubbed** - Prints "temporarily unavailable" error
2. **Incremental module stubbed** - Functions have TODO comments, no implementation
3. **35 test files still reference PostgreSQL** - `cargo test` fails to compile
4. **No tests validate incremental functionality**
5. **52 TODO stubs across 21 files** - Search, context, strategies all have placeholders

## Remaining Work (Phase 6)

| Ticket | Description | Priority |
|--------|-------------|----------|
| **IDXABS-6001** | Migrate 35 test files from PostgreSQL to SQLite | High |
| **IDXABS-6002** | Implement stubbed incremental module functions (13 TODOs) | High |
| **IDXABS-6003** | Implement watch command for SQLite | High |
| **IDXABS-6004** | Complete all 52 SQLite stub implementations | High |

## Success Criteria (Current Reality)

```bash
# Commands that WORK:
cargo run --bin crewchief-maproom -- scan --path /repo      ✅
cargo run --bin crewchief-maproom -- upsert --paths src/main.rs  ✅
cargo run --bin crewchief-maproom -- generate-embeddings    ✅
cargo run --bin crewchief-maproom -- search "function"      ✅

# Commands that DON'T WORK:
cargo run --bin crewchief-maproom -- watch                  ❌ (prints error)

# Tests:
cargo test                                                   ❌ (35 files fail to compile)
```

## Target Success Criteria

```bash
# All commands work:
cargo run --bin crewchief-maproom -- scan --path /repo      ✅
cargo run --bin crewchief-maproom -- upsert --paths src/main.rs  ✅
cargo run --bin crewchief-maproom -- watch --repo myrepo    ✅  # MUST WORK
cargo run --bin crewchief-maproom -- generate-embeddings    ✅
cargo run --bin crewchief-maproom -- search "function"      ✅

# All tests pass:
cargo test -p crewchief-maproom                             ✅  # MUST PASS
```

## Problem Analysis

### Stubbed Functions (from IDXABS-2006)
The following functions in `crates/maproom/src/incremental/` are stubs:

**processor.rs:**
- `index_new_file()` - Logs warning, returns Ok
- `update_file()` - Logs warning, returns Ok
- `remove_file()` - Logs warning, returns Ok

**detector.rs:**
- `get_hash_from_db()` - Returns `Ok(None)`
- `store_hash_in_db()` - Returns `Ok(())`
- `detect_move()` - Returns `Ok(None)`

**edge_updater.rs:**
- `update_edges()` - Logs debug, returns Ok
- `compute_edges()` - Returns empty Vec
- `find_test_targets()` - Returns empty Vec
- `insert_edges()` - Returns count without inserting

**tree_sha_update.rs:**
- `remove_worktree_from_chunks()` - Returns `Ok(0)`
- `incremental_update()` - Returns `UpdateStats::skipped()`

### Test Files Using PostgreSQL (35 files)
All files in `crates/maproom/tests/` that reference `tokio_postgres`, `PgPool`, or `postgres::` will fail to compile. See IDXABS-6001 for full list.

## Phases

| Phase | Focus | Tickets | Status |
|-------|-------|---------|--------|
| 1 | Delete PostgreSQL Code | 1001-1003 | ✅ Complete |
| 2 | Refactor Core Modules | 2001-2006 | ⚠️ Partial (stubbed) |
| 3 | Main.rs Cleanup | 3001 | ✅ Complete |
| 4 | Testing & Validation | 4001-4002 | ⚠️ Partial |
| 5 | Documentation | 5001 | ✅ Complete |
| **6** | **Completion (NEW)** | **6001-6003** | 🔴 Not Started |

## Execution Order

1. **IDXABS-6001** - Migrate tests to SQLite (enables validation)
2. **IDXABS-6002** - Implement incremental module (core functionality)
3. **IDXABS-6003** - Implement watch command (user-facing feature)

## Relevant Agents

| Agent | Role |
|-------|------|
| **rust-indexer-engineer** | Primary implementer |
| **unit-test-runner** | Execute tests |
| **verify-ticket** | Verify acceptance criteria |
| **commit-ticket** | Create git commits |

## Dependencies

- **UNIWATCH depends on IDXABS-6003** - Watch command must work before UNIWATCH can enhance it

## Quick Reference

```bash
# Current state (won't work):
cargo test -p crewchief-maproom  # ❌ Compilation errors

# After IDXABS-6001:
cargo test -p crewchief-maproom  # ✅ Tests compile and run

# After IDXABS-6002 + IDXABS-6003:
cargo run --bin crewchief-maproom -- watch  # ✅ Watch command works
```
