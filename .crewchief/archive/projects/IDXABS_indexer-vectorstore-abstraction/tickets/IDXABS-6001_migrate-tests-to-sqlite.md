# Ticket: IDXABS-6001: Migrate Integration Tests from PostgreSQL to SQLite

## Status
- [ ] **Task completed** - all tests compile and pass with SQLite
- [ ] **Tests pass** - `cargo test -p crewchief-maproom` succeeds
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Migrate 29 test files from PostgreSQL (`tokio_postgres`) to SQLite (`SqliteStore`). Currently, tests don't compile because they still reference the removed PostgreSQL dependency.

> **Note**: Actual count is 29 files (verified via `grep -l`). The list below includes some files that may not need migration or can be deleted.

## Background
The IDXABS project removed PostgreSQL from the main crate, but the test files were not migrated. This blocks:
- Running any tests (`cargo test` fails to compile)
- Verifying incremental module works
- Validating watch functionality

## Acceptance Criteria
- [ ] All 29 affected test files compile without PostgreSQL references
- [ ] `cargo test -p crewchief-maproom` completes successfully
- [ ] No `tokio_postgres`, `PgPool`, or `postgres::` references in `/tests/`
- [ ] Tests use `SqliteStore::connect(":memory:")` or temp files for isolation
- [ ] At least one test per module validates the incremental functionality

## Test Files to Migrate (29 files confirmed + candidates for deletion)

### High Priority - Incremental Module Tests
1. `tests/incremental_update.rs`
2. `tests/incremental_scan_integration.rs`
3. `tests/incremental_processor_test.rs`
4. `tests/incremental_integration_test.rs`
5. `tests/incremental_deletions.rs`
6. `tests/integration/incremental_scenarios.rs`
7. `tests/integration/failure_recovery.rs`
8. `tests/integration/concurrent_updates.rs`
9. `tests/integration/batch_processing.rs`

### Medium Priority - Watch/Watcher Tests
10. `tests/watch_integration.rs`
11. `tests/unified_watch_test.rs`
12. `tests/dynamic_worktree_id_test.rs`

### Search/Embedding Tests
13. `tests/vector_db_test.rs`
14. `tests/search_pipeline_integration_test.rs`
15. `tests/search_executors_test.rs`
16. `tests/mixed_embeddings_search_test.rs`
17. `tests/embedding_inheritance_test.rs`

### Fusion/Ranking Tests
18. `tests/weighted_fusion_test.rs`
19. `tests/rrf_fusion_test.rs`
20. `tests/fusion_quality_test.rs`
21. `tests/fusion_integration_test.rs`

### Other Tests
22. `tests/upsert_worktree.rs`
23. `tests/store_compat.rs`
24. `tests/signal_integration_test.rs`
25. `tests/relationship_test.rs`
26. `tests/python_pipeline_test.rs`
27. `tests/migration_integration.rs`
28. `tests/migration_0015_test.rs`
29. `tests/index_state.rs`
30. `tests/graph_test.rs`
31. `tests/e2e_workflow_simple.rs`
32. `tests/e2e_multi_provider.rs`
33. `tests/ab_testing_test.rs`
34. `tests/common/mod.rs`
35. `tests/fixtures/mpembed_baseline_100.sql` (SQL fixture - may need conversion)

## Technical Requirements

### Test Database Setup Pattern
```rust
use crewchief_maproom::db::SqliteStore;
use std::sync::Arc;

async fn setup_test_db() -> Arc<SqliteStore> {
    let store = SqliteStore::connect(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}
```

### Migration Pattern for Each Test
```rust
// Before (PostgreSQL)
use tokio_postgres::Client;
let client = connect_pg().await?;

// After (SQLite)
use crewchief_maproom::db::SqliteStore;
let store = setup_test_db().await;
```

## Implementation Notes

### Files That Can Be Deleted
Some test files may be entirely about PostgreSQL-specific behavior:
- `tests/migration_0015_test.rs` - PostgreSQL-specific migration
- Tests that validate PostgreSQL pool behavior

### Common Module Update
`tests/common/mod.rs` needs to provide SQLite helper functions:
```rust
pub async fn setup_test_store() -> Arc<SqliteStore> {
    let store = SqliteStore::connect(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}
```

## Dependencies
- None (this is foundational work)

## Risk Assessment
- **Risk**: Some tests may not be convertible (PostgreSQL-specific features)
  - **Mitigation**: Delete or skip tests that test PostgreSQL-only behavior
- **Risk**: Test isolation issues with SQLite
  - **Mitigation**: Use `:memory:` databases or temp files per test

## Files/Packages Affected
Files to MODIFY: All 35 test files listed above
Files to potentially DELETE: Tests that only validate PostgreSQL behavior

## Estimated Effort
High - 29 files to migrate, each requiring careful analysis and conversion. Some files may be deletable.
**Estimated time**: 8-12 hours

## Migration Strategy
1. **Start with simpler tests** (index_state, cleanup) to establish patterns
2. **Create common test helper** in `tests/common/mod.rs` early
3. **Delete PostgreSQL-specific tests** that don't translate (e.g., pool tests, PostgreSQL migration tests)
4. **Document any missing SqliteStore methods** discovered during migration
