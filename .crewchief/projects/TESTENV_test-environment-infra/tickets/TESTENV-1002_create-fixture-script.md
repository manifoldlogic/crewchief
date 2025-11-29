# Ticket: TESTENV-1002: Create fixture generation script

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (script validation occurs in TESTENV-1003)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: N/A - Script validation occurs when fixtures are generated in TESTENV-1003.

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Create a shell script that generates SQL fixtures from the test corpus by indexing files through the maproom daemon and exporting the resulting data.

## Background
The existing `crates/maproom/scripts/create_fixture.sh` provides a proven pattern for fixture generation. This ticket adapts that pattern for the MCP test corpus, creating a script that:
1. Indexes the test corpus files using the maproom daemon
2. Exports the indexed data as SQL INSERT statements
3. Includes version headers for schema compatibility tracking

Reference: [plan.md](../planning/plan.md) - Phase 1, Deliverable 2: "Fixture Generation Script"

## Acceptance Criteria
- [ ] Script created at `packages/maproom-mcp/scripts/create-test-fixtures.sh`
- [ ] Script indexes corpus files from `tests/corpus/` directory
- [ ] Script exports data to `tests/setup/test-fixtures.sql`
- [ ] Output includes version header with schema compatibility info
- [ ] Script is executable and includes usage instructions
- [ ] Script uses COPY format for proper escaping (matches existing pattern)

## Technical Requirements

### Script Location
`packages/maproom-mcp/scripts/create-test-fixtures.sh`

### Script Features
1. **Index corpus files**: Use `crewchief-maproom scan` to index test corpus
2. **Export data**: Use `pg_dump` or COPY to export as SQL
3. **Version header**: Include fixture version and schema compatibility
4. **Idempotency**: Output can be loaded multiple times safely

### Output Format
```sql
-- Fixture Version: 1.0.0
-- Compatible Schema: migrations 0000-0020
-- Generated: 2025-XX-XX
-- Generator: packages/maproom-mcp/scripts/create-test-fixtures.sh
--
-- Test corpus: packages/maproom-mcp/tests/corpus/
-- Chunks: ~100 (TypeScript, Python, Rust, Markdown)

BEGIN;

-- Temporarily disable triggers for faster loading
SET session_replication_role = replica;

-- Repository
\COPY maproom.repos FROM stdin;
1000	test-corpus	/workspace/packages/maproom-mcp/tests/corpus
\.

-- Worktree
\COPY maproom.worktrees FROM stdin;
...
\.

-- [Additional tables: commits, files, chunks]

-- Re-enable triggers
SET session_replication_role = DEFAULT;

-- Update sequences
SELECT setval('maproom.repos_id_seq', 1100);
-- ...

COMMIT;

-- Verification
\echo 'Fixture Statistics:'
SELECT COUNT(*) as chunks FROM maproom.chunks;
```

### Reference Implementation
Study `crates/maproom/scripts/create_fixture.sh` for:
- Stratified sampling approach
- COPY format usage
- Sequence handling
- Verification queries

## Implementation Notes

1. **Adapt existing script** - Don't reinvent; modify the proven pattern from `create_fixture.sh`

2. **Use fixed IDs** - Start repository/worktree IDs at 1000 to avoid conflicts with production data

3. **Handle embeddings** - If embedding generation is slow, consider skipping embeddings for MVP (FTS-only search works for fixture testing)

4. **Environment variables**:
   ```bash
   MAPROOM_DATABASE_URL=postgresql://maproom:maproom@postgres-test:5432/maproom_test
   TEST_CORPUS_PATH=packages/maproom-mcp/tests/corpus
   OUTPUT_FILE=packages/maproom-mcp/tests/setup/test-fixtures.sql
   ```

5. **Error handling** - Script should fail fast with clear error messages if:
   - Database is not accessible
   - Corpus directory doesn't exist
   - Daemon is not running

## Dependencies
- TESTENV-1001 (test corpus must exist before indexing)

## Risk Assessment
- **Risk**: Daemon not available for indexing
  - **Mitigation**: Document that daemon must be running; add pre-flight check
- **Risk**: Large fixture file if too many chunks
  - **Mitigation**: Target ~100 chunks; corpus is intentionally small
- **Risk**: Embeddings slow down generation
  - **Mitigation**: Can skip embeddings for MVP; FTS works without them

## Files/Packages Affected
- `packages/maproom-mcp/scripts/create-test-fixtures.sh` (NEW)
- `packages/maproom-mcp/scripts/` directory (NEW if doesn't exist)
