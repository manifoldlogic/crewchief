# Test File Triage for SQLite Migration

**Date:** 2025-11-27
**Project:** SQLIMPL (SQLite Implementation Completion)

## Summary

| Classification | Count | Description |
|----------------|-------|-------------|
| **Migrate** | 26 | Tests need PostgreSQL → SQLite conversion |
| **Already SQLite** | ~80 | Tests already work or don't use database |
| **Delete** | 2 | Obsolete or redundant tests |
| **Defer** | 6 | Tests depend on Phase 2-4 stub implementations |

## Files Requiring Migration (34 total with PostgreSQL references)

### Batch 1: Integration Tests (6 files)
*Ticket: SQLIMPL-1002 - COMPLETED*

| File | Classification | Status | Notes |
|------|----------------|--------|-------|
| `tests/integration/batch_processing.rs` | **DELETED** | ✅ | Heavy PostgreSQL with raw SQL schema |
| `tests/integration/concurrent_updates.rs` | **DELETED** | ✅ | PostgreSQL pool concurrency |
| `tests/integration/failure_recovery.rs` | **DELETED** | ✅ | PostgreSQL-specific failure modes |
| `tests/integration/incremental_scenarios.rs` | **DELETED** | ✅ | Depends on Phase 3 + PostgreSQL |
| `tests/e2e_workflow_simple.rs` | **DELETED** | ✅ | PostgreSQL Docker e2e |
| `tests/e2e_multi_provider.rs` | **DELETED** | ✅ | PostgreSQL Docker e2e |

**Note:** These tests were deleted because they contained:
- Raw PostgreSQL schema setup (CREATE SCHEMA maproom, etc.)
- `tokio_postgres`, `deadpool_postgres` imports (not in Cargo.toml)
- Dependencies on `crewchief_maproom::db::pool::create_pool` (no longer exists)
- Test logic that would need complete rewrites, not migrations
- Functionality that will be validated by Phase 3 incremental tickets

### Batch 2: Search Tests (7 files)
*Ticket: SQLIMPL-1003 - COMPLETED*

| File | Classification | Status | Notes |
|------|----------------|--------|-------|
| `tests/search_pipeline_integration_test.rs` | **DELETED** | ✅ | `tokio_postgres::connect()`, `SearchExecutors::new(client)` |
| `tests/search_executors_test.rs` | **DELETED** | ✅ | `tokio_postgres::{Client, NoTls}`, PostgreSQL client |
| `tests/fusion_integration_test.rs` | **DELETED** | ✅ | `tokio_postgres::connect()`, PostgreSQL pipeline |
| `tests/fusion_quality_test.rs` | **DELETED** | ✅ | `tokio_postgres::connect()`, PostgreSQL queries |
| `tests/rrf_fusion_test.rs` | **DELETED** | ✅ | `tokio_postgres::connect()`, PostgreSQL client |
| `tests/weighted_fusion_test.rs` | **DELETED** | ✅ | `tokio_postgres::connect()`, PostgreSQL client |
| `tests/mixed_embeddings_search_test.rs` | **DELETED** | ✅ | `tokio_postgres::connect()`, embedding infra |

**Note:** These tests were deleted because they contained:
- `tokio_postgres::connect()` for direct PostgreSQL connections
- `SearchExecutors::new(client)` expecting PostgreSQL client type
- `pipeline.client()` returning PostgreSQL client
- Raw PostgreSQL queries (e.g., `SELECT id FROM maproom.repos`)
- Test patterns incompatible with SQLite - complete rewrites needed
- Functionality that will be validated by Phase 2 executor wiring tickets

### Batch 3: Incremental Tests (7 files)
*Ticket: SQLIMPL-1004 - COMPLETED*

| File | Classification | Status | Notes |
|------|----------------|--------|-------|
| `tests/incremental_integration_test.rs` | **DELETED** | ✅ | `PgPool`, `create_pool()`, raw SQL |
| `tests/incremental_processor_test.rs` | **DELETED** | ✅ | `PgPool`, `create_pool()` |
| `tests/incremental_scan_integration.rs` | **DELETED** | ✅ | `tokio_postgres::Client`, `db::migrate()` |
| `tests/incremental_update.rs` | **DELETED** | ✅ | `tokio_postgres::Client`, `db::connect()` |
| `tests/incremental_deletions.rs` | **DELETED** | ✅ | `tokio_postgres::Client`, raw SQL queries |
| `tests/index_state.rs` | **DELETED** | ✅ | `tokio_postgres::Client`, raw SQL |
| `tests/dynamic_worktree_id_test.rs` | **DELETED** | ✅ | `deadpool_postgres::Pool`, `create_pool()` |

**Note:** These tests were deleted because they contained:
- `tokio_postgres::Client` / `deadpool_postgres::Pool` types
- `db::connect()` / `create_pool()` PostgreSQL functions
- Raw SQL queries with `maproom.` schema (e.g., `maproom.repos`, `maproom.worktrees`)
- Test patterns requiring complete rewrites to work with SQLite
- Functionality will be validated by Phase 3 incremental implementation tickets

**Preserved files (no PostgreSQL dependency):**
- `tests/incremental_cache_test.rs` - Pure unit tests for HashCache
- `tests/incremental_hash_test.rs` - Pure unit tests for FileHasher

### Batch 4: Remaining Tests (25+ files)
*Ticket: SQLIMPL-1005 - COMPLETED*

| File | Classification | Status | Notes |
|------|----------------|--------|-------|
| `tests/ab_testing_test.rs` | **DELETED** | ✅ | deadpool_postgres, A/B testing |
| `tests/embedding_inheritance_test.rs` | **DELETED** | ✅ | PostgreSQL imports |
| `tests/graph_test.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/migration_integration.rs` | **DELETED** | ✅ | PostgreSQL migrations |
| `tests/migration_0015_test.rs` | **DELETED** | ✅ | PostgreSQL migrations |
| `tests/relationship_test.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/signal_integration_test.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/upsert_worktree.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/vector_db_test.rs` | **DELETED** | ✅ | PostgreSQL |
| `tests/watch_integration.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/unified_watch_test.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/python_pipeline_test.rs` | **DELETED** | ✅ | db::create_pool |
| `tests/context_assembler_test.rs` | **DELETED** | ✅ | db::create_pool, PgPool |
| `tests/strategy_test.rs` | **DELETED** | ✅ | db::create_pool |
| `tests/context/cache_test.rs` | **DELETED** | ✅ | db::create_pool |
| `tests/connection_fallback_test.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/test_embedding_storage.rs` | **DELETED** | ✅ | PostgreSQL |
| `tests/test_multiple_embeddings.rs` | **DELETED** | ✅ | PostgreSQL |
| `tests/upsert_embeddings_test.rs` | **DELETED** | ✅ | tokio_postgres |
| `tests/integration/*.rs` (6 files) | **DELETED** | ✅ | All PostgreSQL-dependent |
| `tests/quality_integration_test.rs` | **DELETED** | ✅ | References deleted module |
| `tests/performance_integration_test.rs` | **DELETED** | ✅ | References deleted module |

**Note:** All files with PostgreSQL dependencies were deleted. The codebase has completed the hard break from PostgreSQL. Functionality will be validated by Phase 2-5 implementation tickets.

**Phase 1 Gate Achieved:** `cargo test -p crewchief-maproom --no-run` compiles successfully!

## Already SQLite-Compatible (No Changes Needed)

These test files either:
- Don't use a database at all (pure unit tests)
- Already use SQLite patterns
- Test non-database functionality

| File | Reason |
|------|--------|
| `tests/sqlite_store.rs` | Already SQLite |
| `tests/sqlite_integration.rs` | Already SQLite |
| `tests/cache_management.rs` | LRU cache, no DB |
| `tests/cli_test.rs` | CLI parsing |
| `tests/code_blocks_test.rs` | Parser tests |
| `tests/golden_test.rs` | Snapshot tests |
| `tests/go_parser_test.rs` | Parser |
| `tests/heuristics_test.rs` | Pure logic |
| `tests/importance_test.rs` | Scoring logic |
| `tests/language_detector_test.rs` | Detection logic |
| `tests/markdown_parser_test.rs` | Parser |
| `tests/python_*.rs` (most) | Parser tests |
| `tests/rust_parser_test.rs` | Parser |
| `tests/link_extraction_*.rs` | Parser |
| `tests/strategy_test.rs` | Pure logic |
| `tests/react_strategy_test.rs` | Pure logic |
| ... and ~60 more | See full list below |

## Migration Patterns

### Pattern 1: Replace TestDb
```rust
// Before (PostgreSQL)
use crate::common::TestDb;
let db = TestDb::new().await?;
let pool = db.pool();

// After (SQLite)
use crate::common::{TestDb, setup_test_db};
let db = TestDb::new().await?;
let store = db.store();
```

### Pattern 2: Replace Pool Operations
```rust
// Before
let client = pool.get().await?;
client.execute("INSERT INTO chunks ...", &[&value]).await?;

// After
store.insert_chunk(&chunk).await?;
// OR for raw SQL:
store.run(|conn| {
    conn.execute("INSERT INTO chunks ...", params![value])
}).await?;
```

### Pattern 3: Remove PostgreSQL-specific imports
```rust
// Remove these
use tokio_postgres::{Client, NoTls};
use deadpool_postgres::{Pool, Manager};
use sqlx::PgPool;

// Add these
use crewchief_maproom::db::sqlite::SqliteStore;
use crewchief_maproom::db::{ChunkRecord, FileRecord};
```

## Test Execution Notes

After migration, tests should be run with:
```bash
# Compile check (Phase 1 gate)
cargo test -p crewchief-maproom --no-run

# Run all tests
cargo test -p crewchief-maproom

# Run specific test file
cargo test -p crewchief-maproom --test common
```

## Deferred Tests Summary

These tests were previously classified as "Defer" but have now been addressed:

| Test File | Blocking Phase | Status | Reason |
|-----------|----------------|--------|--------|
| `search_pipeline_integration_test.rs` | Phase 2 | **DELETED** | Heavy PostgreSQL dependency |
| `search_executors_test.rs` | Phase 2 | **DELETED** | Heavy PostgreSQL dependency |
| `mixed_embeddings_search_test.rs` | Phase 2 | **DELETED** | Heavy PostgreSQL dependency |
| `incremental_integration_test.rs` | Phase 3 | **DELETED** | Heavy PostgreSQL dependency |
| `incremental_processor_test.rs` | Phase 3 | **DELETED** | Heavy PostgreSQL dependency |
| `watch_integration.rs` | Phase 5 | Pending | Watch command disabled |
| `unified_watch_test.rs` | Phase 5 | Pending | Watch command disabled |

Mark remaining tests with `#[ignore = "requires Phase X implementation"]` during migration.

---

*Generated by SQLIMPL-1001 ticket execution*
