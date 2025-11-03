# Ticket: PROVFIX-2001: Add Missing updated_at Column to chunks Table

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
Add missing `updated_at TIMESTAMPTZ` column to `maproom.chunks` table to fix "column updated_at does not exist" errors during embedding updates. Embeddings currently generate successfully via API but fail to persist to database.

## Background
During OpenAI provider testing, discovered that embedding generation works but database updates fail:
```
ERROR: column "updated_at" of relation "chunks" does not exist
Failed to update embeddings for chunk 451
```

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md` section "3. Database Schema Issue":
- Embeddings generate successfully via OpenAI API
- Database UPDATE query expects `updated_at` column
- Column missing from schema, causing silent data loss
- 854/854 chunks failed to update in test run
- Schema mismatch between table definition and query expectations

This is a straightforward schema fix with no complex logic - just add the missing column.

This is Phase 2, Ticket 1 of the PROVFIX implementation plan. Phase 2 is independent and can run in parallel with Phase 1.

## Acceptance Criteria
- [ ] New migration file created in `/workspace/crates/maproom/migrations/`
- [ ] Migration adds `updated_at TIMESTAMPTZ` column with `DEFAULT NOW()`
- [ ] Migration includes `IF NOT EXISTS` for safety (idempotent)
- [ ] Trigger created to auto-update `updated_at` on row UPDATE
- [ ] Migration tested on fresh database (column created)
- [ ] Migration tested on existing database (column added, no data loss)
- [ ] Embedding updates succeed without "column does not exist" errors
- [ ] `cargo run --bin crewchief-maproom` applies migration automatically

## Technical Requirements

### 1. Create New Migration File

**Path**: `/workspace/crates/maproom/migrations/00XX_add_updated_at_to_chunks.sql`
- Number sequentially after last migration (check existing migrations)
- Follow existing migration file naming conventions

### 2. Migration SQL

```sql
-- Add updated_at column to chunks table
ALTER TABLE maproom.chunks
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

-- Create trigger function for auto-update
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE 'plpgsql';

-- Create trigger on chunks table
DROP TRIGGER IF EXISTS update_chunks_updated_at ON maproom.chunks;
CREATE TRIGGER update_chunks_updated_at
    BEFORE UPDATE ON maproom.chunks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

### 3. Verify Migration in Codebase

- Check that Maproom runs migrations on startup
- Verify migration numbering is correct
- Test migration applies cleanly

## Implementation Notes

### Reference Documentation
See `.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md` section "3. Fix Database Schema" for complete SQL.

### Key Considerations
- Use `IF NOT EXISTS` for idempotency (safe to run multiple times)
- Use `TIMESTAMPTZ` not `TIMESTAMP` for timezone awareness
- Default to `NOW()` so existing rows get timestamp
- Trigger only updates on UPDATE, not INSERT (INSERT gets default)
- Use `DROP TRIGGER IF EXISTS` before CREATE for idempotency

### Migration Best Practices
1. Always check existing migration numbers to avoid conflicts
2. Test on fresh database first (clean slate)
3. Test on database with existing data (realistic scenario)
4. Verify trigger function is created successfully
5. Confirm trigger is attached to chunks table

## Dependencies
- None (independent of Phase 1 Rust fixes)
- Can be developed and tested in parallel with PROVFIX-1001/1002

## Risk Assessment
- **Risk**: Migration fails on existing database with data
  - **Mitigation**: Use `IF NOT EXISTS` for idempotency; test on copy of production database first

- **Risk**: Trigger has performance impact on large updates
  - **Mitigation**: Simple trigger with minimal overhead; standard PostgreSQL pattern

- **Risk**: Migration numbering conflicts with concurrent work
  - **Mitigation**: Check existing migration numbers; coordinate with team if needed

## Files/Packages Affected
- `/workspace/crates/maproom/migrations/00XX_add_updated_at_to_chunks.sql` (new file)

## Testing Notes

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md` section "2. Database Migration (Manual Test)":

### Test Plan

```bash
# Test 1: Fresh database
docker compose down -v
docker compose up -d postgres
# Run migration (automatic on maproom startup)
# Verify: setup succeeds, no errors

# Test 2: Existing database with chunks
# Insert some chunks without embeddings
node bin/cli.cjs scan /workspace/packages/maproom-mcp --no-embeddings
# Verify: chunks exist without updated_at

# Test 3: Run migration
# Migration should add column to existing chunks with current timestamp

# Test 4: Verify column exists
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "SELECT chunk_id, updated_at FROM maproom.chunks LIMIT 5;"
# Verify: updated_at column exists with timestamps

# Test 5: Verify embedding updates work
node bin/cli.cjs scan /workspace/packages/maproom-mcp --generate-embeddings
# Verify: No "column updated_at does not exist" errors
```

### Success Criteria

**Before**: Embeddings generate but fail to persist (database errors)
**After**: Embeddings generate and persist successfully (no errors)

### Database Verification

```sql
-- Should show updated_at column with timestamps
\d maproom.chunks;

-- Should show trigger exists
\dy update_chunks_updated_at;
```

## Planning References
- Analysis: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md`
  - Section: "3. Database Schema Issue"
- Architecture: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md`
  - Section: "3. Fix Database Schema"
- Quality Strategy: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/quality-strategy.md`
  - Section: "2. Database Migration (Manual Test)"
- Plan: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md`
  - Phase 2, Ticket 1
