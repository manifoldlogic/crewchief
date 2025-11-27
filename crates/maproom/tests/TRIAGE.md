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
*Ticket: SQLIMPL-1002*

| File | Classification | Notes |
|------|----------------|-------|
| `tests/integration/batch_processing.rs` | Migrate | Uses `TestDb` from common |
| `tests/integration/concurrent_updates.rs` | Migrate | Pool-based concurrency tests |
| `tests/integration/failure_recovery.rs` | Migrate | Error handling scenarios |
| `tests/integration/incremental_scenarios.rs` | Defer | Depends on Phase 3 stubs |
| `tests/e2e_workflow_simple.rs` | Migrate | Full scan/search workflow |
| `tests/e2e_multi_provider.rs` | Migrate | Multi-provider embedding tests |

### Batch 2: Search Tests (6 files)
*Ticket: SQLIMPL-1003*

| File | Classification | Notes |
|------|----------------|-------|
| `tests/search_pipeline_integration_test.rs` | Defer | Depends on Phase 2 executor stubs |
| `tests/search_executors_test.rs` | Defer | Depends on Phase 2 executor stubs |
| `tests/fusion_integration_test.rs` | Migrate | RRF fusion logic (already SQLite-compatible) |
| `tests/fusion_quality_test.rs` | Migrate | Quality assertions |
| `tests/rrf_fusion_test.rs` | Migrate | RRF algorithm unit tests |
| `tests/mixed_embeddings_search_test.rs` | Defer | Depends on embedding infra |

### Batch 3: Incremental Tests (7 files)
*Ticket: SQLIMPL-1004*

| File | Classification | Notes |
|------|----------------|-------|
| `tests/incremental_integration_test.rs` | Defer | Depends on Phase 3 stubs |
| `tests/incremental_processor_test.rs` | Defer | Depends on Phase 3 stubs |
| `tests/incremental_scan_integration.rs` | Migrate | Basic scan flow |
| `tests/incremental_update.rs` | Migrate | Update detection |
| `tests/incremental_deletions.rs` | Migrate | Delete tracking |
| `tests/index_state.rs` | Migrate | State management |
| `tests/dynamic_worktree_id_test.rs` | Migrate | Worktree ID resolution |

### Batch 4: Remaining Tests (15 files)
*Ticket: SQLIMPL-1005*

| File | Classification | Notes |
|------|----------------|-------|
| `tests/ab_testing_test.rs` | Delete | A/B testing for deprecated feature |
| `tests/embedding_inheritance_test.rs` | Migrate | Embedding dedup logic |
| `tests/graph_test.rs` | Migrate | Graph traversal |
| `tests/migration_integration.rs` | Delete | PostgreSQL-specific migrations |
| `tests/migration_0015_test.rs` | Delete | PostgreSQL migration |
| `tests/relationship_test.rs` | Migrate | Chunk relationships |
| `tests/signal_integration_test.rs` | Migrate | Signal scoring |
| `tests/upsert_worktree.rs` | Migrate | Worktree operations |
| `tests/vector_db_test.rs` | Migrate | Vector store ops |
| `tests/watch_integration.rs` | Defer | Depends on Phase 5 |
| `tests/unified_watch_test.rs` | Defer | Depends on Phase 5 |
| `tests/weighted_fusion_test.rs` | Migrate | Fusion weighting |
| `tests/python_pipeline_test.rs` | Migrate | Python parsing |

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

These tests require stub implementations from later phases:

| Test File | Blocking Phase | Reason |
|-----------|----------------|--------|
| `search_pipeline_integration_test.rs` | Phase 2 | FTS/Vector executor stubs |
| `search_executors_test.rs` | Phase 2 | Executor stubs |
| `mixed_embeddings_search_test.rs` | Phase 2 | Search pipeline |
| `incremental_integration_test.rs` | Phase 3 | Processor stubs |
| `incremental_processor_test.rs` | Phase 3 | Processor stubs |
| `watch_integration.rs` | Phase 5 | Watch command disabled |
| `unified_watch_test.rs` | Phase 5 | Watch command disabled |
| `integration/incremental_scenarios.rs` | Phase 3 | Incremental stubs |

Mark these with `#[ignore = "requires Phase X implementation"]` during migration.

---

*Generated by SQLIMPL-1001 ticket execution*
