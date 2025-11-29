# IDXABS Ticket Index

**Project**: IDXABS - Indexer SQLite-Only Migration
**Created**: 2025-11-27
**Updated**: 2025-11-27 (post-project-review update)
**Total Tickets**: 18 (14 original + 4 completion)
**Estimated Duration**: 60-90 hours (revised from 50-70 hours)

## Overview

This project removes PostgreSQL support entirely, making SQLite the only database backend for the maproom crate. The migration simplifies the codebase by:
- Deleting PostgreSQL-specific files (~5 in db/)
- Removing the VectorStore trait abstraction
- Eliminating feature flags
- Using SqliteStore directly throughout
- Refactoring ~26 files with PostgreSQL references (~140 references total)

## Phase Summary

| Phase | Tickets | Description | Est. Time |
|-------|---------|-------------|-----------|
| Phase 1 | 1001-1003 | Delete PostgreSQL Code | 4-5 hours |
| Phase 2 | 2001-2007 | Refactor Core Modules | 10-15 hours |
| Phase 3 | 3001 | Main.rs Cleanup | 2-3 hours |
| Phase 4 | 4001-4002 | Testing & Validation | 3-4 hours |
| Phase 5 | 5001 | Documentation | 1-2 hours |
| **Phase 6** | **6001-6004** | **Completion (NEW)** | **40-60 hours** |

## Tickets by Phase

### Phase 1: Delete PostgreSQL Code

| ID | Title | Status | Agent |
|----|-------|--------|-------|
| [IDXABS-1001](IDXABS-1001_delete-postgresql-database-files.md) | Delete PostgreSQL Database Files | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-1002](IDXABS-1002_simplify-db-mod.md) | Simplify db/mod.rs and connection.rs | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-1003](IDXABS-1003_update-cargo-toml.md) | Update Cargo.toml | ⬜ Pending | rust-indexer-engineer |

### Phase 2: Refactor Core Modules

| ID | Title | Status | Agent |
|----|-------|--------|-------|
| [IDXABS-2001](IDXABS-2001_refactor-indexer-module.md) | Refactor Indexer Module | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-2002](IDXABS-2002_refactor-embedding-pipeline.md) | Refactor Embedding Pipeline | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-2003](IDXABS-2003_refactor-search-module.md) | Refactor Search Module | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-2004](IDXABS-2004_refactor-context-module.md) | Refactor Context Module | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-2005](IDXABS-2005_refactor-db-support-files.md) | Refactor db Support Files and Migrate Module | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-2006](IDXABS-2006_refactor-incremental-module.md) | Refactor Incremental Module | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-2007](IDXABS-2007_refactor-upsert-module.md) | Refactor Upsert Module | ⬜ Pending | rust-indexer-engineer |

### Phase 3: Main.rs Cleanup

| ID | Title | Status | Agent |
|----|-------|--------|-------|
| [IDXABS-3001](IDXABS-3001_cleanup-main-rs.md) | Clean Up main.rs | ⬜ Pending | rust-indexer-engineer |

### Phase 4: Testing & Validation

| ID | Title | Status | Agent |
|----|-------|--------|-------|
| [IDXABS-4001](IDXABS-4001_fix-update-tests.md) | Fix and Update Tests | ⬜ Pending | rust-indexer-engineer |
| [IDXABS-4002](IDXABS-4002_e2e-validation-script.md) | E2E Validation Script | ⬜ Pending | rust-indexer-engineer |

### Phase 5: Documentation

| ID | Title | Status | Agent |
|----|-------|--------|-------|
| [IDXABS-5001](IDXABS-5001_update-documentation.md) | Update Documentation | ⬜ Pending | rust-indexer-engineer |

### Phase 6: Completion (NEW - Post-Archive Recovery)

| ID | Title | Status | Agent |
|----|-------|--------|-------|
| [IDXABS-6001](IDXABS-6001_migrate-tests-to-sqlite.md) | Migrate 29 Test Files to SQLite | 🔴 **BLOCKING** | rust-indexer-engineer |
| [IDXABS-6002](IDXABS-6002_implement-incremental-module.md) | Implement Stubbed Incremental Module | 🔴 **BLOCKING** | rust-indexer-engineer |
| [IDXABS-6003](IDXABS-6003_implement-watch-command.md) | Implement Watch Command for SQLite | 🔴 **BLOCKING** | rust-indexer-engineer |
| [IDXABS-6004](IDXABS-6004_complete-all-sqlite-stubs.md) | Complete All 52 SQLite Stub Implementations | 🔴 **BLOCKING** | rust-indexer-engineer |

**Note**: Phase 6 was added after the project was incorrectly archived. These tickets complete the work that was stubbed during the original migration.

**Scope Discovery**: A TODO search revealed **52 stub implementations** across 21 files, not just the incremental module. IDXABS-6004 tracks all of them.

## Dependencies

```
Phase 1: 1001 → 1002 → 1003
             ↓
Phase 2: 2001 → 2002 → 2003 → 2004 → 2005 → 2006 → 2007
             ↓
Phase 3: 3001
             ↓
Phase 4: 4001 → 4002
             ↓
Phase 5: 5001
             ↓
Phase 6: 6001 → 6002 → 6003
```

**Phase 6 Critical Path**: IDXABS-6001 must complete first (tests must compile) before IDXABS-6002 can be validated. IDXABS-6003 depends on IDXABS-6002 for incremental processing.

Note: Phase 2 tickets 2001-2007 can be partially parallelized as they touch different modules, but sequential execution is safer due to potential shared dependencies.

## Key Technical Decisions

1. **Remove VectorStore Trait**: Use SqliteStore directly (no trait objects)
2. **Remove Feature Flags**: SQLite is always enabled, no conditional compilation
3. **Direct Connection Function**: Simple `db::connect()` returns SqliteStore
4. **No Parallel Scanning**: SQLite is single-writer; remove `--parallel` flag

## Success Criteria

```bash
# All commands work without --features sqlite:
cargo run --bin crewchief-maproom -- scan --path /repo
cargo run --bin crewchief-maproom -- upsert --paths src/main.rs
cargo run --bin crewchief-maproom -- watch
cargo run --bin crewchief-maproom -- generate-embeddings
cargo run --bin crewchief-maproom -- search "function"

# All tests pass:
cargo test -p crewchief-maproom
```

## Post-Review Updates (2025-11-27)

Based on `/review-tickets` analysis:
- **Added**: IDXABS-2006 (incremental/ module - 8 PostgreSQL refs across 3 files)
- **Added**: IDXABS-2007 (upsert.rs - 7 PostgreSQL refs)
- **Updated**: IDXABS-1002 to include connection.rs simplification
- **Updated**: IDXABS-1003 with complete feature flag details
- **Updated**: IDXABS-2003 with full search/ file list
- **Updated**: IDXABS-2004 with detector files (hooks.rs, jsx.rs)
- **Updated**: IDXABS-2005 to include migrate/markdown.rs

See `../planning/tickets-review-report.md` for full analysis.

## References

- **Plan**: `../planning/plan.md`
- **Architecture**: `../planning/architecture.md`
- **Analysis**: `../planning/analysis.md`
- **Review**: `../planning/project-review.md`
- **Tickets Review**: `../planning/tickets-review-report.md`
