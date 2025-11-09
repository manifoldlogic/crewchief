# Ticket: SCHMAFIX-4001: Write MCP Integration Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create TypeScript integration tests in `packages/maproom-mcp/tests/migrations/schema-integration.test.ts` to verify the MCP server works correctly with the new database schema, specifically confirming the code_embeddings table exists and vector search doesn't crash.

## Background
The original problem that triggered SCHMAFIX was that MCP TypeScript code (line 511 of src/index.ts) references a `code_embeddings` table that doesn't exist, causing vector search to crash with "relation does not exist" error. After integrating migrations 0018-0020, the schema should be complete. These tests verify the MCP layer can successfully query the new schema without errors. This is end-to-end validation that the migration integration actually fixed the original problem.

This ticket implements Phase 4 - MCP Integration Verification from the SCHMAFIX project plan.

## Acceptance Criteria
- [ ] File `packages/maproom-mcp/tests/migrations/schema-integration.test.ts` exists
- [ ] Test `code_embeddings table exists` confirms table is queryable
- [ ] Test `vector search doesnt crash` confirms MCP vector search mode executes without error
- [ ] Test `blob_sha column accessible` confirms chunks table has blob_sha column
- [ ] Test `worktree_ids JSONB column accessible` confirms BRANCHX schema integrated
- [ ] Test `worktree_index_state table exists` confirms BRANCHX tracking schema present
- [ ] All tests pass locally (`pnpm test schema-integration`)
- [ ] Tests use the existing test database setup (matching other MCP tests)

## Technical Requirements
- Test file location: `packages/maproom-mcp/tests/migrations/schema-integration.test.ts`
- Test framework: Vitest (matching existing MCP tests)
- Database: Use `testClient` pattern from existing tests (shared test database)
- Test structure: Follow patterns from `jsonb-queries.test.ts` and `004-worktree-tracking.test.ts`
- Lifecycle: Use `beforeAll` for setup, `afterAll` for cleanup
- Database queries: Direct PostgreSQL queries via `testClient.query()`
- Vector search test: Mock or minimal search query to verify code_embeddings table accessible

## Implementation Notes

### Test Setup
```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import pg from 'pg'
const { Client } = pg

let testClient: Client | null = null

beforeAll(async () => {
  testClient = new Client({ connectionString: process.env.DATABASE_URL })
  await testClient.connect()
})

afterAll(async () => {
  await testClient?.end()
})
```

### Test 1: code_embeddings Table Exists
```typescript
it('code_embeddings table exists and is queryable', async () => {
  const result = await testClient.query(`
    SELECT table_name FROM information_schema.tables
    WHERE table_schema = 'maproom' AND table_name = 'code_embeddings'
  `)
  expect(result.rows).toHaveLength(1)
  expect(result.rows[0].table_name).toBe('code_embeddings')
})
```

### Test 2: Vector Search Doesn't Crash
```typescript
it('vector search query executes without crashing', async () => {
  // This is the query that was failing before (line 511 of src/index.ts)
  const result = await testClient.query(
    'SELECT COUNT(*) as count FROM maproom.code_embeddings LIMIT 1'
  )
  expect(result.rows).toHaveLength(1)
  expect(result.rows[0].count).toBeDefined()
  // Count might be '0' (empty table) - that's fine, we just care it doesn't crash
})
```

### Test 3: blob_sha Column Accessible
```typescript
it('chunks table has blob_sha column', async () => {
  const result = await testClient.query(`
    SELECT column_name, data_type FROM information_schema.columns
    WHERE table_schema = 'maproom' AND table_name = 'chunks' AND column_name = 'blob_sha'
  `)
  expect(result.rows).toHaveLength(1)
  expect(result.rows[0].data_type).toBe('text')
})
```

### Test 4: worktree_ids JSONB Column Accessible
```typescript
it('chunks table has worktree_ids JSONB column', async () => {
  const result = await testClient.query(`
    SELECT column_name, data_type FROM information_schema.columns
    WHERE table_schema = 'maproom' AND table_name = 'chunks' AND column_name = 'worktree_ids'
  `)
  expect(result.rows).toHaveLength(1)
  expect(result.rows[0].data_type).toBe('jsonb')
})
```

### Test 5: worktree_index_state Table Exists
```typescript
it('worktree_index_state table exists for BRANCHX tracking', async () => {
  const result = await testClient.query(`
    SELECT table_name FROM information_schema.tables
    WHERE table_schema = 'maproom' AND table_name = 'worktree_index_state'
  `)
  expect(result.rows).toHaveLength(1)
})
```

## Dependencies
- **SCHMAFIX-1001** (BLOCKER) - Migration SQL files must exist
- **SCHMAFIX-2001** (BLOCKER) - Rust runner updated to include migrations
- **SCHMAFIX-3901** (BLOCKER) - Rust migration tests must pass before MCP tests

## Risk Assessment
- **Risk**: Test database doesn't have migrations applied
  - **Mitigation**: Document that migrations must be run first; follow existing test patterns
- **Risk**: testClient pattern doesn't work with current setup
  - **Mitigation**: Use existing test patterns from jsonb-queries.test.ts as reference
- **Risk**: Vector search test too complex to implement simply
  - **Mitigation**: Keep it simple - just query table existence, not full vector search functionality

## Files/Packages Affected
- `packages/maproom-mcp/tests/migrations/schema-integration.test.ts` (CREATE)

## Files to Reference
- `packages/maproom-mcp/tests/jsonb-queries.test.ts` (test setup patterns)
- `packages/maproom-mcp/tests/migrations/004-worktree-tracking.test.ts` (schema validation patterns)
- `packages/maproom-mcp/src/index.ts` (line 511 - the code_embeddings query that was failing)

## Related Planning Documents
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/plan.md` (Phase 4)
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/analysis.md` (Original problem: code_embeddings table missing)
- `.agents/projects/SCHMAFIX_schema-migration-integration/planning/quality-strategy.md` (MCP integration validation)

## Estimated Effort
1-2 hours
