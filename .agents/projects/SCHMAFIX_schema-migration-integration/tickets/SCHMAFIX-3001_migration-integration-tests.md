# Ticket: SCHMAFIX-3001: Migration Integration Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive Rust integration tests in `crates/maproom/tests/migration_integration.rs` to verify migrations 0018-0020 work correctly on fresh databases, existing databases, and when run multiple times (idempotency).

## Background
Migration integration is mission-critical - if migrations fail in production, the database schema won't match code expectations and vector search will crash. After implementing the SQL migration files in SCHMAFIX-1001 and updating the Rust migration runner in SCHMAFIX-2001, we need comprehensive integration tests covering three key scenarios:

1. **Fresh database**: Applying all 20 migrations (0000-0021) from scratch
2. **Incremental migration**: Upgrading from v0.17 to v0.21 (simulating production upgrade)
3. **Idempotency**: Running migrations twice to verify IF NOT EXISTS clauses work

Tests must validate schema correctness (blob_sha column exists, code_embeddings table exists, worktree tracking schema complete) and ensure no data loss during migrations.

This ticket implements **Phase 3: Migration Testing** from the SCHMAFIX project plan.

## Acceptance Criteria
- [ ] File `crates/maproom/tests/migration_integration.rs` exists
- [ ] Test `test_fresh_database_migrations` applies all 20 migrations (0000-0021) to empty database and validates schema
- [ ] Test `test_incremental_migration` applies only migrations 0018-0020 to v0.17 database
- [ ] Test `test_migration_idempotency` runs migrations twice without errors or duplicates
- [ ] Test `test_schema_validation` confirms blob_sha column, code_embeddings table, and BRANCHX schema exist
- [ ] All tests pass locally (`cargo test migration_integration`)
- [ ] Tests use PostgreSQL testcontainers for database isolation (no shared state between tests)

## Technical Requirements

### Test File Location
- Path: `crates/maproom/tests/migration_integration.rs`
- Type: Rust integration test (in `tests/` directory, not `src/`)
- Framework: tokio::test for async testing, standard assert macros

### Database Setup
- Use `testcontainers-rs` for PostgreSQL instances
- Each test creates a fresh, isolated database instance
- Database image: `postgres:15` with `pgvector` extension
- Auto-cleanup via testcontainers (databases dropped after test completion)
- Connection string format: `postgresql://postgres:postgres@localhost:{port}/postgres`

### Schema Validation Queries

**Verify blob_sha column**:
```sql
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema='maproom' AND table_name='chunks' AND column_name='blob_sha'
```

**Verify code_embeddings table**:
```sql
SELECT table_name
FROM information_schema.tables
WHERE table_schema='maproom' AND table_name='code_embeddings'
```

**Verify worktree tracking columns**:
```sql
SELECT column_name
FROM information_schema.columns
WHERE table_schema='maproom' AND table_name='chunks' AND column_name='worktree_ids'
```

**Verify indexes**:
```sql
SELECT indexname
FROM pg_indexes
WHERE schemaname='maproom' AND tablename='chunks' AND indexname='idx_chunks_worktree_ids'
```

### Migration Runner Integration
- Call existing migration logic from `crates/maproom/src/db/queries.rs`
- Use the `run_migrations()` function or equivalent
- Do not duplicate migration logic in tests

### Test Dependencies (Cargo.toml)
Add to `[dev-dependencies]` section:
```toml
testcontainers = "0.15"
tokio = { version = "1", features = ["full"] }
tokio-postgres = "0.7"
```

## Implementation Notes

### Test 1: Fresh Database Migration (`test_fresh_database_migrations`)

**Purpose**: Verify all migrations apply cleanly to an empty database

**Steps**:
1. Spin up PostgreSQL testcontainer
2. Connect to database
3. Run all migrations 0000-0021 via `run_migrations()` function
4. Query `schema_migrations` table to verify latest version is 21
5. Verify key tables exist:
   - `repos`
   - `files`
   - `chunks` (with `blob_sha` and `worktree_ids` columns)
   - `code_embeddings`
   - `worktrees`
   - `worktree_index_state`
6. Verify no errors or panics occurred

**Success Criteria**:
- All migrations execute without error
- `schema_migrations` table shows version 19
- All expected tables and columns exist

### Test 2: Incremental Migration (`test_incremental_migration`)

**Purpose**: Simulate upgrading an existing v0.17 database to v0.21

**Steps**:
1. Spin up PostgreSQL testcontainer
2. Connect to database
3. Run migrations 0000-0017 only (simulate existing production database)
4. Verify `schema_migrations` shows version 17
5. Insert test data into `chunks` table (to verify data preservation)
6. Run migrations 0018-0020 (incremental upgrade)
7. Verify `schema_migrations` now shows version 19
8. Confirm new schema elements exist:
   - `chunks.blob_sha` column (type TEXT, nullable)
   - `chunks.worktree_ids` column (type JSONB, not null, default '[]')
   - `code_embeddings` table
   - `worktree_index_state` table
9. Verify existing test data is preserved and intact
10. Verify backfill succeeded (blob_sha populated for existing rows)

**Success Criteria**:
- Incremental migrations execute successfully
- Schema updated correctly
- No data loss
- Backfill logic executed (blob_sha values populated)

### Test 3: Idempotency (`test_migration_idempotency`)

**Purpose**: Verify migrations can be run multiple times safely (IF NOT EXISTS clauses work)

**Steps**:
1. Spin up PostgreSQL testcontainer
2. Connect to database
3. Run all migrations 0000-0021
4. Verify `schema_migrations` shows version 19
5. Run migrations 0000-0021 again (should be no-ops)
6. Verify `schema_migrations` still shows version 19 (not duplicated)
7. Verify no "already exists" errors occurred
8. Verify schema unchanged (no duplicate tables/columns/indexes)

**Success Criteria**:
- Second migration run completes without errors
- No duplicate schema elements
- `schema_migrations` table has no duplicate entries

### Test 4: Schema Validation (`test_schema_validation`)

**Purpose**: Comprehensive schema validation for all BLOBSHA/BRANCHX elements

**Steps**:
1. Spin up PostgreSQL testcontainer
2. Connect to database
3. Run all migrations 0000-0021
4. Query `information_schema` to validate:
   - **chunks.blob_sha**: column exists, type TEXT, nullable
   - **chunks.worktree_ids**: column exists, type JSONB, not null, default '[]'
   - **code_embeddings table**: exists with columns:
     - `id` (BIGSERIAL PRIMARY KEY)
     - `blob_sha` (TEXT NOT NULL)
     - `embedding` (vector(1536) NOT NULL)
     - `created_at` (TIMESTAMPTZ NOT NULL DEFAULT NOW())
   - **worktree_index_state table**: exists with columns:
     - `worktree_id` (TEXT PRIMARY KEY)
     - `tree_sha` (TEXT NOT NULL)
     - `indexed_at` (TIMESTAMPTZ NOT NULL DEFAULT NOW())
   - **Indexes**:
     - `idx_chunks_worktree_ids` (GIN index on chunks.worktree_ids)
     - `idx_code_embeddings_hnsw` (HNSW index on code_embeddings.embedding)
     - `idx_code_embeddings_blob_sha` (index on code_embeddings.blob_sha)
5. Verify pgvector extension is enabled

**Success Criteria**:
- All expected columns exist with correct types
- All expected tables exist
- All expected indexes exist
- Schema matches BLOBSHA/BRANCHX specifications

### Helper Functions

Create reusable helper functions for common operations:

```rust
async fn setup_postgres_container() -> testcontainers::Container<...> {
    // Spin up postgres:15 container with pgvector
}

async fn connect_to_db(container: &Container) -> tokio_postgres::Client {
    // Create database connection
}

async fn run_migrations_up_to(client: &Client, version: i32) -> Result<()> {
    // Run migrations 0000 through specified version
}

async fn verify_table_exists(client: &Client, table_name: &str) -> bool {
    // Query information_schema
}

async fn verify_column_exists(client: &Client, table_name: &str, column_name: &str) -> bool {
    // Query information_schema
}

async fn verify_migration_version(client: &Client, expected_version: i32) -> bool {
    // Query schema_migrations table
}
```

### Error Handling
- Use `Result<()>` return type for tests
- Use `?` operator for error propagation
- Include descriptive error messages with context
- Log migration SQL on failure for debugging

### Test Isolation
- Each test MUST create its own PostgreSQL container
- No shared state between tests
- Tests can run in parallel without conflicts
- Cleanup handled automatically by testcontainers

## Dependencies

### Blockers
- **SCHMAFIX-1001** (BLOCKER) - Migration SQL files must exist
  - Status: Must be completed first
  - Impact: Tests reference migrations 0018-0020

- **SCHMAFIX-2001** (BLOCKER) - Rust migration runner must be updated
  - Status: Must be completed first
  - Impact: Tests call `run_migrations()` function which must include migrations 0018-0020

### External Dependencies
- PostgreSQL 15+ with pgvector extension
- testcontainers-rs library
- tokio-postgres client library
- Rust toolchain (cargo, rustc)

## Risk Assessment

**Risk**: Testcontainers startup slow or flaky
- **Mitigation**: Acceptable for integration tests, tests can run in parallel
- **Impact**: Test suite takes 30-60 seconds to run
- **Likelihood**: Low (testcontainers is stable)
- **Recovery**: Acceptable tradeoff for database isolation

**Risk**: Migration 0018 backfill fails on large dataset
- **Mitigation**: Test with small dataset first, document performance characteristics
- **Impact**: Backfill query may timeout on very large databases
- **Likelihood**: Low (test dataset is small)
- **Severity**: Medium (production concern, not test concern)
- **Recovery**: Document backfill performance, add timeout handling

**Risk**: pgvector extension not available in test DB
- **Mitigation**: Use official postgres image with pgvector preinstalled (e.g., `ankane/pgvector:latest`)
- **Impact**: Tests fail with "extension not found"
- **Likelihood**: Low (pgvector widely available)
- **Recovery**: Update testcontainer image to one with pgvector support

**Risk**: Migration version conflicts
- **Mitigation**: Each test uses isolated database, no version conflicts possible
- **Impact**: None (isolation prevents conflicts)
- **Likelihood**: Very low

**Risk**: Test failures due to schema drift
- **Mitigation**: Tests validate against known schema, failures indicate real issues
- **Impact**: Test failures reveal migration bugs early
- **Likelihood**: Medium (this is desired behavior - tests should catch bugs)
- **Recovery**: Fix migrations and rerun tests

## Files/Packages Affected

### Files to Create
- `/workspace/crates/maproom/tests/migration_integration.rs` (new integration test file)

### Files to Modify
- `/workspace/crates/maproom/Cargo.toml` (add dev-dependencies for testcontainers)

### Files to Reference
- `/workspace/crates/maproom/src/db/queries.rs` (migration runner logic)
- `/workspace/crates/maproom/migrations/0018_add_blob_sha.sql`
- `/workspace/crates/maproom/migrations/0019_create_code_embeddings.sql`
- `/workspace/crates/maproom/migrations/0020_add_worktree_tracking.sql`
- `/workspace/crates/maproom/migrations/0021_complete_branchx_schema.sql`

### Build Outputs
- Test binary in `target/debug/deps/migration_integration-*`
- Test execution logs and output

## Testing Strategy

### Running Tests Locally

```bash
# Run all migration integration tests
cd /workspace/crates/maproom
cargo test migration_integration

# Run with output visible
cargo test migration_integration -- --nocapture

# Run specific test
cargo test test_fresh_database_migrations -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test migration_integration -- --nocapture
```

### Continuous Integration
- Tests will run in GitHub Actions CI/CD (Phase 4: SCHMAFIX-4001)
- Docker-in-Docker support required for testcontainers
- PostgreSQL service container not needed (testcontainers handles it)

### Test Execution Time
- Expected runtime: 30-60 seconds total
- Per-test: 10-15 seconds (container startup + migration execution)
- Can run in parallel (each test isolated)

## Success Metrics

### Completion Criteria
- All 4 integration tests exist and pass
- Tests cover fresh database, incremental migration, and idempotency scenarios
- Comprehensive schema validation implemented
- Code compiles without errors
- All tests executable via `cargo test migration_integration`

### Quality Criteria
- Tests are isolated (no shared state)
- Clear test names and documentation
- Descriptive assertion messages
- Proper error handling and cleanup
- Reusable helper functions for common operations

## Related Planning Documents

- [SCHMAFIX Plan](../planning/plan.md) - Phase 3: Migration Testing
- [SCHMAFIX Architecture](../planning/architecture.md) - Migration Integration Strategy
- [SCHMAFIX Quality Strategy](../planning/quality-strategy.md) - Critical Path Tests section
- [SCHMAFIX Security Review](../planning/security-review.md) - Test isolation and data safety

## Estimated Effort
2-3 hours

**Breakdown**:
- Setup testcontainers and dependencies: 30 minutes
- Implement test_fresh_database_migrations: 30 minutes
- Implement test_incremental_migration: 45 minutes
- Implement test_migration_idempotency: 20 minutes
- Implement test_schema_validation: 45 minutes
- Documentation and cleanup: 15 minutes

## Next Steps

After this ticket is complete:
- **SCHMAFIX-3002**: Execute integration tests and document results
- **SCHMAFIX-4001**: Update CI/CD to run migration tests in GitHub Actions
- **SCHMAFIX-4002**: Production migration runbook and rollback procedures

## Notes

### Why Integration Tests?

Unit tests verify individual components, but migrations involve:
- Database schema evolution
- Data preservation across versions
- Transaction boundaries
- SQL execution in real PostgreSQL environment
- Idempotency guarantees

Integration tests are the ONLY way to verify these behaviors work correctly together.

### Test Coverage Strategy

These 4 tests provide:
1. **Fresh database coverage**: Ensures complete migration path works
2. **Upgrade coverage**: Validates production upgrade scenario
3. **Idempotency coverage**: Prevents duplicate schema errors
4. **Schema coverage**: Validates all BLOBSHA/BRANCHX elements present

Together, these tests give high confidence that migrations will work in production.

### Testcontainers vs Docker Compose

**Why testcontainers?**
- Each test gets isolated database
- Automatic cleanup (no orphaned containers)
- Parallel test execution
- No manual Docker setup required
- Reproducible across environments

**Why not Docker Compose?**
- Shared database state between tests
- Manual cleanup required
- Tests cannot run in parallel
- Requires external Docker setup

### Schema Validation Importance

Schema validation tests catch:
- Missing columns (typos in migration SQL)
- Wrong data types (TEXT vs VARCHAR)
- Missing indexes (performance regressions)
- Missing constraints (data integrity issues)

These bugs are HARD to catch in production and expensive to fix. Schema validation tests provide safety net.
