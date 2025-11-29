# Ticket: MCPDB-1006: PostgreSQL Dependency Handling

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose (TypeScript implementation)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add conditional logic to `search.ts` and `index.ts` to handle PostgreSQL-dependent code paths when using SQLite backend, implementing graceful degradation with appropriate warnings.

## Background
The MCP server has two code paths that bypass the daemon and use direct PostgreSQL queries:
1. `search.ts:fetchChunkIds()` - enriches search results with chunk IDs
2. `index.ts:handleStatus()` - queries database directly for statistics

For SQLite mode, these need graceful degradation:
- Search: Return `chunk_id: 0` with warning log
- Status: Return degraded response with hint

**Plan Reference:** Phase 2 - PostgreSQL Dependency Handling (plan.md, architecture.md Decision 4)

## Acceptance Criteria
- [x] `search.ts` skips `fetchChunkIds()` call when backend is SQLite
- [x] Search results return `chunk_id: 0` for SQLite with warning logged
- [x] `handleStatus()` detects SQLite and returns degraded response
- [x] Degraded status response includes `hint` guiding users to search tool
- [x] Degraded status response includes `backendType: 'sqlite'` and `sqlitePath`
- [x] PostgreSQL code paths unchanged (no regressions)
- [x] `resolveDatabaseConfig()` exported from resolve-database.ts for use in handlers

## Technical Requirements

### search.ts Changes

**Location:** `src/tools/search.ts` around line 322

**Current Code:**
```typescript
const chunkIdMap = await fetchChunkIds(client, repo, rustOutput.hits)
```

**Updated Code:**
```typescript
import { resolveDatabaseConfig } from '../utils/resolve-database.js'

// In handleSearchTool, before fetchChunkIds call:
const dbConfig = resolveDatabaseConfig()
let chunkIdMap: Map<string, number>

if (dbConfig.type === 'sqlite') {
  // SQLite: daemon doesn't return chunk IDs (Phase 2 enhancement)
  log.warn(
    { hits: rustOutput.hits.length },
    'SQLite mode: chunk IDs unavailable, using 0'
  )
  chunkIdMap = new Map() // Empty map = all chunk_id will be 0
} else {
  // PostgreSQL: use existing fetchChunkIds
  chunkIdMap = await fetchChunkIds(client, repo, rustOutput.hits)
}
```

### index.ts Changes

**Location:** `src/index.ts:handleStatus()` around line 353

**Updated Code:**
```typescript
import { resolveDatabaseConfig } from './utils/resolve-database.js'

async function handleStatus(params: any): Promise<any> {
  const dbConfig = resolveDatabaseConfig()

  if (dbConfig.type === 'sqlite') {
    // SQLite: return degraded response (no direct SQL stats)
    return {
      repos: [],
      totalRepos: 0,
      totalFiles: 0,
      totalChunks: 0,
      hint: 'SQLite mode: detailed statistics not available. Use search tool for indexed content.',
      backendType: 'sqlite',
      sqlitePath: dbConfig.path,
      searchTips: [
        'Use simple terms: "auth" instead of "authentication_handler"',
        'Search concepts: "message bus" or "event handling"',
        'Filter by type: use filter:"code" or filter:"docs"'
      ]
    }
  }

  // PostgreSQL: existing implementation unchanged
  const client = await getPg()
  // ... rest of existing code
}
```

### Export Addition

**Location:** `src/utils/resolve-database.ts`

Ensure `resolveDatabaseConfig` is exported (should be done in MCPDB-1001, verify here).

## Implementation Notes

### Why Graceful Degradation
The alternative (implementing dual SQL for both backends) would:
- Duplicate complex queries
- Require SQLite schema knowledge in TypeScript
- Add maintenance burden

Graceful degradation is the MVP approach:
- Core functionality works (search via daemon)
- Limitations are documented and logged
- Full feature access available via PostgreSQL

### Log Levels
- `log.warn` for SQLite limitations (user should know)
- `log.debug` for normal SQLite operation (not noisy)

### Testing Approach
- Mock `resolveDatabaseConfig()` to return SQLite config
- Verify `fetchChunkIds` is not called
- Verify warning is logged
- Verify `chunk_id: 0` in results
- Verify status returns degraded response

## Dependencies
- **MCPDB-1001**: Provides `resolveDatabaseConfig()` function
- **MCPDB-1002**: Daemon integration (for search to work at all)

## Risk Assessment
- **Risk**: Accidentally breaking PostgreSQL search path
  - **Mitigation**: Conditional check is explicit; PostgreSQL path is the `else` branch unchanged
- **Risk**: Users confused by `chunk_id: 0`
  - **Mitigation**: Warning logged; documented in README.md Known Limitations
- **Risk**: Status response structure change breaks clients
  - **Mitigation**: SQLite-specific fields are additions, not changes to existing fields

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/search.ts` (modify)
- `packages/maproom-mcp/src/index.ts` (modify)
- `packages/maproom-mcp/src/utils/resolve-database.ts` (verify export)
