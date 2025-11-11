# Quality Strategy: Schema Migration Integration

## MVP Testing Mindset

**Goal**: Confidence migrations work, not 100% coverage ceremony.

**Focus**: Critical path testing - schema applied correctly, MCP tools work, no data loss.

## Test Strategy

### Critical Path Tests (Must Have)

**1. Fresh Database Migration** ✅
- Start: Empty database
- Action: Run Rust binary
- Verify: All 22 migrations applied (0000-0021)
- Success Criteria: `SELECT * FROM schema_migrations` shows version 21

**2. Incremental Migration** ✅
- Start: Database at version 0017 (current production)
- Action: Run new Rust binary
- Verify: Only new migrations (0018-0021) applied
- Success Criteria: Existing data intact, new schema present

**3. Schema Validation** ✅
- Verify: `blob_sha` column exists in chunks table
- Verify: `code_embeddings` table exists with correct schema
- Verify: `worktree_ids` column already present (from manual SQL)
- Verify: `worktree_index_state` table already present
- Success Criteria: `\d` commands show expected schema

**4. MCP Vector Search** ✅
- Start: MCP server with new schema
- Action: Attempt vector search (mode: vector)
- Verify: No "table does not exist" error
- Success Criteria: Query executes (results may be empty, that's fine)

**5. Data Preservation** ✅
- Start: Database with existing chunks
- Action: Run migrations
- Verify: Chunk count unchanged, content intact
- Success Criteria: `SELECT COUNT(*) FROM chunks` before === after

### Integration Points (Nice to Have)

**6. Idempotency Test** 📋
- Run migrations twice in a row
- Verify: No errors, schema unchanged
- Purpose: Ensure IF NOT EXISTS works

**7. Existing Tests Still Pass** 📋
- Run existing MCP integration tests
- Run existing Rust unit tests
- Verify: No regressions

### Explicitly NOT Testing

❌ **Migration Performance** - One-time operation, seconds acceptable
❌ **Rollback Migrations** - Out of scope for MVP
❌ **Feature Logic** - BLOBSHA/BRANCHX features not implemented yet
❌ **Load Testing** - Schema changes, not query optimization
❌ **Cross-Database** - PostgreSQL only

## Risk Mitigation

### High-Risk Scenarios

**Risk 1**: Migration 0018 fails mid-backfill (blob_sha computation)
- **Impact**: Some chunks have blob_sha, some don't
- **Mitigation**: Wrap backfill in transaction, use batching
- **Test**: Run on database with 100k+ chunks, verify atomic completion

**Risk 2**: code_embeddings table created but index fails
- **Impact**: Table exists but queries slow
- **Mitigation**: Separate index creation into own statement with error handling
- **Test**: Monitor index creation logs

**Risk 3**: Manual worktree_ids schema conflicts with migration 0020
- **Impact**: Migration fails with "column already exists" (but IF NOT EXISTS should handle)
- **Mitigation**: Careful IF NOT EXISTS usage, test on database with partial schema
- **Test**: Apply migration 0020 to database that already has worktree_ids

### Medium-Risk Scenarios

**Risk 4**: MCP references other non-existent tables/columns
- **Impact**: New crashes discovered after fixing code_embeddings
- **Mitigation**: Grep MCP codebase for table references, validate all exist
- **Test**: Run full MCP test suite after migrations

**Risk 5**: Rust code assumes new columns exist in queries
- **Impact**: Old Rust binaries crash against new schema
- **Mitigation**: Ensure additive-only changes, no breaking SELECT patterns
- **Test**: Run old Rust binary (v0.17) against new schema

## Test Implementation Plan

### Phase 1: Migration Execution Tests

**Test File**: `crates/maproom/tests/migration_integration.rs`

```rust
#[tokio::test]
async fn test_fresh_database_migrations() {
    // Create test database
    // Run migrations
    // Query schema_migrations table
    // Assert version 21 exists
}

#[tokio::test]
async fn test_incremental_migrations() {
    // Seed database with migrations 0000-0017
    // Run migration runner
    // Assert only 18-21 applied
    // Verify data integrity
}

#[tokio::test]
async fn test_schema_structure() {
    // Run migrations
    // Query information_schema
    // Assert blob_sha column exists
    // Assert code_embeddings table exists
}
```

### Phase 2: MCP Integration Tests

**Test File**: `packages/maproom-mcp/tests/migrations/schema-integration.test.ts`

```typescript
describe('Schema Migration Integration', () => {
  it('code_embeddings table exists', async () => {
    const result = await client.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'maproom' AND table_name = 'code_embeddings'
    `)
    expect(result.rows).toHaveLength(1)
  })

  it('vector search does not crash', async () => {
    // This used to throw "table does not exist"
    await expect(
      searchVector('test query')
    ).resolves.not.toThrow()
  })
})
```

### Phase 3: Data Integrity Tests

**Test**: Manual verification with production-like database

```bash
# Backup production database
pg_dump maproom > maproom_backup.sql

# Restore to test database
createdb maproom_test
psql maproom_test < maproom_backup.sql

# Count rows before
psql maproom_test -c "SELECT COUNT(*) FROM maproom.chunks"

# Run migrations
cargo run --bin crewchief-maproom -- db --database-url postgresql://localhost/maproom_test

# Count rows after (should be identical)
psql maproom_test -c "SELECT COUNT(*) FROM maproom.chunks"

# Spot check data
psql maproom_test -c "SELECT id, content FROM maproom.chunks LIMIT 10"
```

## Acceptance Criteria for Tests

**Green Light** (Ship It):
- ✅ Fresh database test passes
- ✅ Incremental migration test passes
- ✅ Schema validation passes
- ✅ MCP vector search doesn't crash
- ✅ Data count unchanged after migration

**Blockers** (Don't Ship):
- ❌ Migration fails with non-idempotent error
- ❌ Data loss detected (chunk count decreases)
- ❌ MCP still crashes on code_embeddings reference
- ❌ Migrations can't run twice without error

## Manual Testing Checklist

Before marking complete:

```
Migration Application:
[ ] Fresh database: All migrations apply cleanly
[ ] Incremental: Only new migrations apply to v0.17 database
[ ] Idempotency: Can run twice without errors

Schema Validation:
[ ] blob_sha column exists (TEXT type)
[ ] code_embeddings table exists
[ ] worktree_ids column exists (JSONB type)
[ ] worktree_index_state table exists
[ ] All indexes created successfully

MCP Integration:
[ ] MCP server starts without errors
[ ] Vector search query executes (no table errors)
[ ] FTS search still works
[ ] Status tool returns index stats

Data Safety:
[ ] Chunk count before === after
[ ] Sample chunks readable after migration
[ ] No orphaned data in foreign keys
```

## Continuous Validation

**Post-Deployment Monitoring**:
1. Check `schema_migrations` table in production
2. Verify code_embeddings table exists
3. Monitor logs for migration errors
4. Test vector search in production

**Success Metrics**:
- Zero migration failures in production
- MCP vector search works for new users
- No support tickets about "table does not exist"

## Known Limitations

**What We're NOT Testing**:
1. BLOBSHA feature logic (out of scope)
2. BRANCHX feature logic (out of scope)
3. Migration performance tuning
4. Rollback procedures
5. Multi-version upgrade paths

**Why**: MVP focus on schema correctness, not feature completeness.

## Test-First Development Order

1. ✅ Write migration SQL files
2. ✅ Update Rust migration runner
3. ✅ Write migration tests
4. ✅ Run tests against test database
5. ✅ Fix any failures
6. ✅ Write MCP integration test
7. ✅ Manual validation checklist
8. ✅ Ship

This order ensures we catch issues early, before production deployment.
