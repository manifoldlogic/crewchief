# Ticket: SCHMAFIX-5001: Manual Migration Validation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (manual validation ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- verify-ticket (manual validation and checklist completion)
- commit-ticket

## Summary
Manually validate that migrations 0018-0020 apply correctly to both fresh and existing databases, and that the MCP server starts and executes vector search without errors. Complete the manual validation checklist from quality-strategy.md to ensure production readiness.

## Background
Automated tests (Phase 3-4) verify migrations work in isolation, but manual validation ensures they work in the actual development environment with Docker Compose, real MCP server startup, and actual vector search queries. This is the final quality gate before documentation and project completion.

The manual validation checklist in quality-strategy.md (lines 190-217) provides systematic verification steps for:
- Migration application (fresh and incremental scenarios)
- Schema validation (all expected tables/columns exist)
- MCP integration (server starts, vector search works)
- Data safety (no data loss or corruption)

This ticket implements **Phase 5: Manual Validation** from the SCHMAFIX project plan, ensuring production readiness for the integrated migration system.

## Acceptance Criteria
- [ ] Migrations apply cleanly to fresh database (all 20 migrations from 0000-0021)
- [ ] Migrations apply cleanly to v0.17 database (incremental 0018-0020 only)
- [ ] MCP server starts without errors after migrations applied
- [ ] Vector search query executes successfully (mode: vector) without crash
- [ ] Manual validation checklist from quality-strategy.md is 100% complete (all 16 items checked)
- [ ] Schema verification via psql confirms all expected tables/columns exist
- [ ] No data loss or corruption during migration (chunk count preserved)

## Technical Requirements
- **Environment**: Docker Compose with maproom-postgres container
- **Database**: Use actual DATABASE_URL from .env file (not test database)
- **Migration execution**: Run `crewchief-maproom db` (Rust binary from `crates/maproom`)
- **MCP server**: Start via `pnpm run start:mcp` or appropriate command
- **Schema validation**: Use `psql` commands to inspect database
- **Vector search test**: Use MCP tools or direct MCP server query with `mode: "vector"`

## Implementation Notes

### Part 1: Fresh Database Migration (New User Scenario)

Validates that new users can set up the database from scratch.

**Steps:**
1. Stop Docker Compose and remove volumes:
   ```bash
   docker compose -f packages/maproom-mcp/config/docker-compose.yml down -v
   ```

2. Start fresh PostgreSQL:
   ```bash
   docker compose -f packages/maproom-mcp/config/docker-compose.yml up -d maproom-postgres
   ```

3. Wait for PostgreSQL ready:
   ```bash
   docker exec maproom-postgres pg_isready
   ```

4. Run migrations:
   ```bash
   cd crates/maproom && cargo run --bin crewchief-maproom -- db
   ```

5. Verify migration version:
   ```bash
   psql $DATABASE_URL -c "SELECT version, name FROM maproom.schema_migrations ORDER BY version DESC LIMIT 5"
   ```
   **Expected**: Version 21 is latest migration

6. Verify schema:
   ```bash
   psql $DATABASE_URL -c "\d maproom.chunks"
   ```
   **Expected**: Should show `blob_sha` and `worktree_ids` columns

7. Verify code_embeddings table:
   ```bash
   psql $DATABASE_URL -c "\dt maproom.code_embeddings"
   ```
   **Expected**: Table exists

### Part 2: Incremental Migration (Existing User Upgrade Scenario)

Validates that existing v0.17 users can upgrade without data loss.

**Steps:**
1. Create v0.17 database state:
   - Stop PostgreSQL, delete volume, restart
   - Run only migrations 0000-0017 (may need to temporarily comment out 0018-0020 in `crates/maproom/src/db/migrations/queries.rs`)
   - Verify version 17 is latest: `psql $DATABASE_URL -c "SELECT MAX(version) FROM maproom.schema_migrations"`

2. Restore full migration runner:
   - Uncomment migrations 0018-0020 in `queries.rs`
   - Rebuild if necessary

3. Run migrations again:
   ```bash
   cd crates/maproom && cargo run --bin crewchief-maproom -- db
   ```
   **Expected**: Should apply only migrations 0018-0020

4. Verify version 19 is now latest:
   ```bash
   psql $DATABASE_URL -c "SELECT version, name FROM maproom.schema_migrations ORDER BY version DESC LIMIT 5"
   ```

5. Verify new columns/tables exist (same queries as Part 1)

6. Verify no errors in migration output

### Part 3: MCP Server Integration

Validates that the MCP server can start and perform vector searches with the new schema.

**Steps:**
1. Ensure migrations applied (version 19)

2. Start MCP server:
   ```bash
   cd packages/maproom-mcp && pnpm run start
   ```
   (Use appropriate command for your environment)

3. Observe startup logs:
   - **Success**: No "table does not exist" errors
   - **Failure**: Errors mentioning `code_embeddings` or other missing tables

4. Test vector search via MCP:
   - Use `mcp__maproom__search` tool with `mode: "vector"`
   - Query: "test query" (or any search term)
   - **Expected**: No crash, returns results or empty array (empty is OK, crash is NOT OK)

5. Check MCP logs:
   - Confirm `code_embeddings` table was queried successfully
   - No PostgreSQL errors

### Part 4: Schema Validation via psql

Execute these queries and verify output:

```sql
-- Verify blob_sha column
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema='maproom' AND table_name='chunks' AND column_name='blob_sha';
-- Expected: 1 row, data_type='text', is_nullable='YES'

-- Verify code_embeddings table
SELECT table_name FROM information_schema.tables
WHERE table_schema='maproom' AND table_name='code_embeddings';
-- Expected: 1 row

-- Verify worktree_ids JSONB column
SELECT column_name, data_type, column_default
FROM information_schema.columns
WHERE table_schema='maproom' AND table_name='chunks' AND column_name='worktree_ids';
-- Expected: 1 row, data_type='jsonb', column_default="'[]'::jsonb"

-- Verify worktree_index_state table
SELECT table_name FROM information_schema.tables
WHERE table_schema='maproom' AND table_name='worktree_index_state';
-- Expected: 1 row

-- Verify HNSW index exists
SELECT indexname FROM pg_indexes
WHERE schemaname='maproom' AND indexname='idx_code_embeddings_hnsw';
-- Expected: 1 row
```

### Part 5: Manual Validation Checklist

Reference `.agents/projects/SCHMAFIX_schema-migration-integration/planning/quality-strategy.md` (lines 195-217) and complete all checklist items:

**Migration Application:**
- [ ] Fresh database: All migrations apply cleanly
- [ ] Incremental: Only new migrations apply to v0.17 database
- [ ] Idempotency: Can run twice without errors

**Schema Validation:**
- [ ] blob_sha column exists (TEXT type)
- [ ] code_embeddings table exists
- [ ] worktree_ids column exists (JSONB type)
- [ ] worktree_index_state table exists
- [ ] All indexes created successfully

**MCP Integration:**
- [ ] MCP server starts without errors
- [ ] Vector search query executes (no table errors)
- [ ] FTS search still works
- [ ] Status tool returns index stats

**Data Safety:**
- [ ] Chunk count before === after
- [ ] Sample chunks readable after migration
- [ ] No orphaned data in foreign keys

## Dependencies
- **SCHMAFIX-1001** (BLOCKER) - Migration SQL files must exist
- **SCHMAFIX-2001** (BLOCKER) - Rust runner must be updated with new migrations
- **SCHMAFIX-3901** (BLOCKER) - Rust tests must pass (automated validation complete)
- **SCHMAFIX-4001** (BLOCKER) - MCP integration tests must pass

## Risk Assessment
- **Risk**: Incremental migration testing complex (requires v0.17 state recreation)
  - **Mitigation**: Document steps carefully; may skip if too complex and rely on fresh database validation

- **Risk**: MCP server won't start after migrations
  - **Mitigation**: Check logs thoroughly, verify all migrations applied, check PostgreSQL extensions loaded

- **Risk**: Vector search still crashes after code_embeddings table created
  - **Mitigation**: Verify pgvector extension loaded, check HNSW index exists, review MCP code for other missing tables

- **Risk**: Data loss during blob_sha backfill in migration 0018
  - **Mitigation**: Verify chunk count before/after, spot-check sample data

## Files/Packages Affected
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/quality-strategy.md` (reference checklist)
- `packages/maproom-mcp/config/docker-compose.yml` (PostgreSQL container management)
- `.env` (DATABASE_URL configuration)
- `crates/maproom/src/db/migrations/queries.rs` (may need temporary modification for incremental test)
- Database schema: `maproom.chunks`, `maproom.code_embeddings`, `maproom.schema_migrations`, `maproom.worktree_index_state`

## Related Planning Documents
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/plan.md` (Phase 5)
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/quality-strategy.md` (Manual Validation Checklist, lines 190-217)
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/architecture.md` (Expected schema structure)

## Success Definition
- All 16 items in manual validation checklist completed without errors
- MCP vector search works without crashes (primary success metric)
- Schema matches architecture document expectations
- Fresh and incremental migration scenarios both succeed
- No data loss detected (chunk count preserved)
- Ticket verified and ready for commit

## Estimated Effort
1-2 hours

## Notes for verify-ticket Agent
This is a manual validation ticket where you will be the primary executor. You will:
1. Execute all validation steps systematically (Parts 1-5)
2. Check off each item in the manual validation checklist (Part 5)
3. Document any issues or unexpected behavior
4. Confirm success criteria met before marking task completed
5. Self-verify since you are the primary agent

If any validation step fails, document the failure clearly but do not attempt fixes - this ticket is validation only. Fixes would be tracked in follow-up tickets.
