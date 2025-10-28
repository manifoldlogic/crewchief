# Ticket: MPEMBED-1901: Test database migration on 100-chunk fixture

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Run forward migration, verification, and rollback on 100-chunk test fixture to validate safety before production use.

## Background
Migration must be tested on real data before production deployment. The 100-chunk fixture from MPEMBED-0001 provides fast iteration (< 10 seconds total) while testing both success path (migration works) and failure path (rollback works). This is critical for Phase 1 acceptance criteria requiring zero data loss.

This implements Phase 1: Database Migration testing from the MPEMBED multi-provider embeddings plan.

## Acceptance Criteria
- [ ] Fixture database created from MPEMBED-0001 with 100 chunks
- [ ] Forward migration runs successfully on fixture
- [ ] Verification script passes (all checks ✓)
- [ ] Existing OpenAI embeddings intact after migration (100/100)
- [ ] New Ollama columns exist and are NULL
- [ ] Rollback runs successfully (reverses migration)
- [ ] Verification after rollback confirms columns removed
- [ ] Test documented in `crates/maproom/tests/migration_0015_test.rs`

## Technical Requirements
- Use PostgreSQL test container (ephemeral, isolated)
- Load fixture: `psql < tests/fixtures/mpembed_baseline_100.sql`
- Run migration: `psql < migrations/0015_add_ollama_columns.sql`
- Run verification: `./scripts/verify_migration_0015.sh`
- Run rollback: `psql < migrations/0015_add_ollama_columns_rollback.sql`
- Measure timing: migration should complete in < 5 seconds for 100 chunks
- Use sqlx for database operations in Rust tests

## Implementation Notes
Create a Rust integration test that validates the complete migration lifecycle:

```rust
#[tokio::test]
async fn test_migration_0015_forward_and_rollback() {
    let pool = create_test_pool().await;

    // Load fixture
    load_fixture(&pool, "tests/fixtures/mpembed_baseline_100.sql").await.unwrap();

    // Count existing embeddings
    let before_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before_count, 100);

    // Run forward migration
    run_migration(&pool, "migrations/0015_add_ollama_columns.sql").await.unwrap();

    // Verify columns exist
    let ollama_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding_ollama IS NULL"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ollama_count, 100); // All NULL initially

    // Verify existing embeddings preserved
    let after_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after_count, 100); // No data loss

    // Verify indexes exist
    let index_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM pg_indexes
            WHERE tablename='chunks' AND indexname='idx_chunks_code_vec_ollama'
        )"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(index_exists);

    // Run rollback
    run_migration(&pool, "migrations/0015_add_ollama_columns_rollback.sql").await.unwrap();

    // Verify columns removed
    let column_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_name='chunks' AND column_name='code_embedding_ollama'
        )"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!column_exists);

    // Verify existing embeddings still intact after rollback
    let final_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(final_count, 100); // Still no data loss
}
```

Helper functions needed in `crates/maproom/tests/helpers/migration.rs`:
- `create_test_pool()`: Creates ephemeral PostgreSQL connection
- `load_fixture()`: Executes SQL fixture file
- `run_migration()`: Executes migration SQL file

Key considerations:
- Test both forward and rollback in same test
- Verify data preservation at each step
- Use assertions for critical invariants
- Measure execution time for performance validation

## Dependencies
- MPEMBED-0001 (requires fixture)
- MPEMBED-1001 (forward migration SQL)
- MPEMBED-1002 (rollback migration SQL)
- MPEMBED-1003 (verification script)

## Risk Assessment
- **Risk**: Test passes on fixture but fails on production (size differences)
  - **Mitigation**: Also test on staging with full 23K chunk dataset before production
- **Risk**: Test container database version differs from production
  - **Mitigation**: Use same PostgreSQL version (15+) with pgvector extension in test container

## Files/Packages Affected
- crates/maproom/tests/migration_0015_test.rs (create)
- crates/maproom/tests/helpers/migration.rs (create if needed)
