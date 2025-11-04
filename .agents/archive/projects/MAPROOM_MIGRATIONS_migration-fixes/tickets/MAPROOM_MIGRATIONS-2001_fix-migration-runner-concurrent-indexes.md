# Ticket: MAPROOM_MIGRATIONS-2001: Fix Migration Runner to Support CONCURRENT Indexes

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the fixes identified in MAPROOM_MIGRATIONS-1001 to resolve transaction issues with CREATE INDEX CONCURRENTLY statements. Ensure all 16 migrations can be applied cleanly from a fresh database.

## Background
This ticket implements the fixes identified during investigation (MAPROOM_MIGRATIONS-1001). The migration runner currently cannot apply migrations with CONCURRENT indexes due to transaction conflicts.

During PROVFIX work, we discovered multiple issues:
1. CREATE INDEX CONCURRENTLY statements fail with "cannot run inside a transaction block" errors
2. Migrations 0004, 0008, 0010, 0012, and 0015 use CONCURRENT indexes
3. Even with `simple_query()` approach, transaction errors persist
4. Manual SQL workarounds were needed for migration 0016 (updated_at column)
5. No migration tracking means migrations aren't idempotent

This is blocking clean database setup and preventing automated deployments.

**Related Planning Documents:**
- Project location: `.agents/projects/MAPROOM_MIGRATIONS_migration-runner-fixes`
- Investigation ticket: `MAPROOM_MIGRATIONS-1001`

## Acceptance Criteria
- [ ] All migrations (0001-0016) apply cleanly from fresh database (devcontainer postgres)
- [ ] All migrations (0001-0016) apply cleanly from fresh database (maproom-postgres)
- [ ] CREATE INDEX CONCURRENTLY statements execute successfully
- [ ] Migration tracking implemented (track which migrations have been applied)
- [ ] Migrations are idempotent (can run multiple times safely without errors)
- [ ] No manual SQL workarounds required
- [ ] Migration 0016 (updated_at column) applies correctly
- [ ] Test with both database instances:
  - Devcontainer PostgreSQL: `postgresql://postgres:postgres@postgres:5432/crewchief`
  - Maproom MCP PostgreSQL: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`

## Technical Requirements

### Database Connection Management
- Ensure CONCURRENT index operations run outside transaction blocks
- Use separate connections for CONCURRENT operations if needed
- Properly handle transaction isolation for regular vs CONCURRENT operations

### Migration Tracking
- Implement `schema_migrations` or similar tracking table
- Store: migration number, filename, applied_at timestamp, checksum (optional)
- Query tracking table before applying migrations
- Skip already-applied migrations

### Migration Runner Updates
- Fix `migrate()` function in `/workspace/crates/maproom/src/db/queries.rs`
- Update `execute_with_concurrent_indexes()` if needed
- Ensure proper error handling and rollback for failed migrations
- Add detailed logging for debugging

### Files to Modify
- `/workspace/crates/maproom/src/db/queries.rs` - Migration runner implementation
- `/workspace/crates/maproom/migrations/0000_schema_migrations.sql` - New migration tracking table (create as first migration)

### Migration Files Affected
- `0004_create_indexes.sql` - First CONCURRENT index migration
- `0008_add_graph_indexes.sql` - Uses execute_with_concurrent_indexes()
- `0010_add_symbol_metadata_index.sql` - CONCURRENT index
- `0012_create_gin_indexes.sql` - Multiple CONCURRENT indexes
- `0015_additional_gin_indexes.sql` - Additional CONCURRENT indexes
- `0016_add_updated_at_column.sql` - Latest migration (must apply cleanly)

## Implementation Notes

### Likely Implementation Tasks
Based on investigation findings, implement the following:

1. **Create Migration Tracking Table** (0000_schema_migrations.sql)
   ```sql
   CREATE TABLE IF NOT EXISTS maproom.schema_migrations (
       version INTEGER PRIMARY KEY,
       filename TEXT NOT NULL,
       applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       checksum TEXT  -- Optional: for validation
   );
   ```

2. **Update Migration Runner** (queries.rs)
   - Load and check schema_migrations table
   - Filter out already-applied migrations
   - For each migration:
     - Check if it contains CONCURRENT operations
     - Use appropriate execution method (transaction vs non-transaction)
     - Record successful application in schema_migrations
   - Handle errors gracefully with detailed messages

3. **Fix Transaction Handling**
   - Regular migrations: Run in transaction (can rollback)
   - CONCURRENT migrations: Run outside transaction (cannot rollback)
   - Consider using `batch_execute()` for regular migrations
   - Use `simple_query()` or direct connection for CONCURRENT operations

4. **Connection Isolation for CONCURRENT Operations**
   - Option A: Use separate connection pool for CONCURRENT operations
   - Option B: Close and reopen connection between regular and CONCURRENT migrations
   - Option C: Use autocommit mode for CONCURRENT operations

5. **Idempotency Improvements**
   - Check schema_migrations before applying
   - Use IF NOT EXISTS in migration SQL where appropriate
   - Add migration checksums for validation (optional enhancement)

6. **Testing Strategy**
   - Test fresh database (no existing tables)
   - Test partially-migrated database (some migrations applied)
   - Test fully-migrated database (all migrations applied, should skip)
   - Test both database instances (devcontainer and maproom-postgres)

### Code Structure Suggestions

```rust
// Pseudo-code for migration runner logic
pub async fn migrate(pool: &PgPool) -> Result<()> {
    // 1. Ensure schema_migrations table exists
    create_schema_migrations_table_if_not_exists(pool).await?;

    // 2. Get list of applied migrations
    let applied_migrations = get_applied_migrations(pool).await?;

    // 3. Get list of available migration files
    let migration_files = discover_migration_files()?;

    // 4. Filter to unapplied migrations
    let pending_migrations = filter_pending(migration_files, applied_migrations);

    // 5. Apply each pending migration
    for migration in pending_migrations {
        if migration.has_concurrent_indexes() {
            apply_concurrent_migration(pool, &migration).await?;
        } else {
            apply_regular_migration(pool, &migration).await?;
        }

        // 6. Record successful application
        record_migration(pool, &migration).await?;
    }

    Ok(())
}
```

### Testing Commands

```bash
# Test 1: Fresh database (devcontainer postgres)
cd /workspace
docker compose down -v
docker compose up -d postgres
sleep 5  # Wait for postgres to be ready
DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief" \
    cargo run --bin crewchief-maproom -- db migrate
# Expected: All 16 migrations apply successfully

# Test 2: Idempotency (devcontainer postgres)
DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief" \
    cargo run --bin crewchief-maproom -- db migrate
# Expected: No errors, already-applied migrations skipped

# Test 3: Verify schema (devcontainer postgres)
docker exec -it devcontainer-postgres psql -U postgres -d crewchief \
    -c "SELECT version, filename, applied_at FROM maproom.schema_migrations ORDER BY version;"
docker exec -it devcontainer-postgres psql -U postgres -d crewchief \
    -c "\d maproom.chunks" | grep updated_at
# Expected: schema_migrations table shows all 16 migrations, updated_at column exists

# Test 4: Fresh database (maproom-postgres)
cd /workspace/packages/maproom-mcp
docker compose down -v
docker compose up -d maproom-postgres
sleep 5
DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom" \
    cargo run --bin crewchief-maproom -- db migrate
# Expected: All 16 migrations apply successfully

# Test 5: Verify schema (maproom-postgres)
docker exec -it maproom-postgres psql -U maproom -d maproom \
    -c "SELECT version, filename, applied_at FROM maproom.schema_migrations ORDER BY version;"
docker exec -it maproom-postgres psql -U maproom -d maproom \
    -c "\d maproom.chunks" | grep updated_at
# Expected: schema_migrations table shows all 16 migrations, updated_at column exists
```

## Dependencies
- **Requires**: MAPROOM_MIGRATIONS-1001 (investigation findings)
- **Blocks**: Clean database setup for new environments
- **Blocks**: Automated deployment pipelines

## Risk Assessment

- **Risk**: Changes to migration runner could break existing databases
  - **Mitigation**: Test with fresh databases first; add schema_migrations table as first migration; ensure backward compatibility

- **Risk**: CONCURRENT indexes may still fail in certain PostgreSQL configurations
  - **Mitigation**: Test with both database instances; add detailed error messages; document workarounds if needed

- **Risk**: Migration tracking implementation may have edge cases
  - **Mitigation**: Implement comprehensive tests; handle partial failures gracefully; add rollback logic where possible

- **Risk**: Connection handling changes may introduce new issues
  - **Mitigation**: Test connection pooling behavior; add logging for debugging; follow tokio-postgres best practices

## Files/Packages Affected

### Files to Create
- `/workspace/crates/maproom/migrations/0000_schema_migrations.sql` - Migration tracking table

### Files to Modify
- `/workspace/crates/maproom/src/db/queries.rs` - Migration runner implementation
  - `migrate()` function - Add tracking logic
  - `execute_with_concurrent_indexes()` - Fix transaction handling
  - Add helper functions: `create_schema_migrations_table_if_not_exists()`, `get_applied_migrations()`, `record_migration()`

### Files to Reference
- `/workspace/crates/maproom/migrations/0004_create_indexes.sql`
- `/workspace/crates/maproom/migrations/0008_add_graph_indexes.sql`
- `/workspace/crates/maproom/migrations/0010_add_symbol_metadata_index.sql`
- `/workspace/crates/maproom/migrations/0012_create_gin_indexes.sql`
- `/workspace/crates/maproom/migrations/0015_additional_gin_indexes.sql`
- `/workspace/crates/maproom/migrations/0016_add_updated_at_column.sql`

### Configuration Files
- `/workspace/docker-compose.yml` - Devcontainer postgres configuration
- `/workspace/packages/maproom-mcp/docker-compose.yml` - Maproom MCP postgres configuration

## Estimated Effort
2-3 hours implementation + 1 hour testing = 3-4 hours total

## Priority
**High** - Blocking clean database setup and automated deployments
