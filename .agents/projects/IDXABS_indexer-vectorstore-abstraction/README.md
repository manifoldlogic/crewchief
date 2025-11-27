# Project: IDXABS_indexer-vectorstore-abstraction

## Project Summary

**Remove PostgreSQL entirely** and make SQLite the only database backend for maproom. This dramatically simplifies the codebase by:
- Deleting ~5 PostgreSQL-specific files
- Removing the VectorStore trait abstraction
- Eliminating feature flags
- Using SqliteStore directly throughout

## Problem Statement

The maproom crate currently maintains two database backends (PostgreSQL and SQLite), creating:
1. **Maintenance burden** - 183 PostgreSQL references across 33 files
2. **Complexity** - VectorStore trait, factory pattern, backend switching
3. **Blocked commands** - Indexing commands explicitly reject SQLite

## Proposed Solution

**Delete PostgreSQL, keep only SQLite:**

1. Delete `db/postgres/`, `db/pool.rs`, `db/queries.rs`, `db/factory.rs`
2. Remove VectorStore trait, use `SqliteStore` directly
3. Refactor all modules to use `&SqliteStore`
4. Remove feature flags (SQLite always enabled)
5. Remove backend switching in main.rs

## Benefits

- **Zero-configuration** - No Docker/PostgreSQL setup needed
- **Simpler code** - One implementation, no trait objects
- **Faster compilation** - Fewer dependencies
- **Portable** - Single-file database

## Relevant Agents

| Agent | Role |
|-------|------|
| **rust-indexer-engineer** | Primary implementer for all refactoring |
| **verify-ticket** | Verifies acceptance criteria |
| **commit-ticket** | Creates git commits |

## Planning Documents

- [Analysis](./planning/analysis.md) - What to delete, what to keep
- [Architecture](./planning/architecture.md) - Design decisions and module changes
- [Plan](./planning/plan.md) - Phased execution plan with tickets

## Phases

| Phase | Focus | Tickets | Est. Time |
|-------|-------|---------|-----------|
| 1 | Delete PostgreSQL Code | 1001-1003 | 4-5 hours |
| 2 | Refactor Core Modules | 2001-2005 | 8-12 hours |
| 3 | Main.rs Cleanup | 3001 | 2-3 hours |
| 4 | Testing & Validation | 4001-4002 | 3-4 hours |
| 5 | Documentation | 5001 | 1-2 hours |

**Total: 18-26 hours**

## Success Criteria

```bash
# All commands work WITHOUT --features sqlite:
cargo run --bin crewchief-maproom -- scan --path /repo
cargo run --bin crewchief-maproom -- upsert --paths src/main.rs
cargo run --bin crewchief-maproom -- watch
cargo run --bin crewchief-maproom -- generate-embeddings
cargo run --bin crewchief-maproom -- search "function"

# All tests pass:
cargo test

# Default database location:
~/.maproom/maproom.db
```

## Files to Delete

- `crates/maproom/src/db/postgres/` - PostgreSQL implementation
- `crates/maproom/src/db/pool.rs` - Connection pooling
- `crates/maproom/src/db/queries.rs` - 28 PostgreSQL queries
- `crates/maproom/src/db/factory.rs` - Backend switching
- `crates/maproom/src/db/materialized_views.rs` - Materialized views

## Dependencies

None - this is a simplification project. All required SQLite functionality already exists in `SqliteStore`.

## Quick Reference

```bash
# After completion:
cargo build --bin crewchief-maproom  # No --features sqlite needed
cargo test                           # All tests pass
```
