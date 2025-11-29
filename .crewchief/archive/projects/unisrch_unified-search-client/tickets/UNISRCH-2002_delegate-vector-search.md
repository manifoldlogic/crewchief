# UNISRCH-2002: Delegate Vector Search in Index Handler

**Status:** ✅ Completed  
**Phase:** 2 (Implementation)  
**Estimated Effort:** 30 minutes  
**Priority:** High  

---

## Summary

Replace the placeholder `executeVectorSearch()` function in index.ts with proper delegation to the search tool handler, matching the FTS pattern.

---

## Background

**Current State:**
The `/packages/maproom-mcp/src/index.ts` file contains a placeholder for vector search that throws an error:

```typescript
// index.ts:620-658 (current)
async function executeVectorSearch(...) {
  // Check if embeddings exist
  const { rows: embeddingCheck } = await client.query(...)
  if (embeddingCheck[0].count === '0') {
    throw new Error('Vector search requires embeddings...')
  }
  
  // TODO: Integrate with embedding service
  throw new Error(
    'Vector search requires query embedding generation...' +
    'Use mode:"fts" or mode:"hybrid" as alternatives.'
  )
}
```

**Desired State:**
With UNISRCH-2001 complete, the search tool now supports vector mode. We can delegate to it exactly like FTS does:

```typescript
// index.ts:698-711 (FTS pattern to follow)
if (mode === 'fts') {
  const { handleSearchTool } = await import('./tools/search.js')
  const result = await handleSearchTool(
    { query, repo, worktree, limit: k, mode, debug },
    client
  )
  return { hits: result.hits, error: result.error, ... }
}
```

---

## Acceptance Criteria

1. ✅ `executeVectorSearch()` calls `handleSearchTool` with mode='vector'
2. ✅ Removes placeholder error-throwing code
3. ✅ Returns results in format expected by caller
4. ✅ Handles errors gracefully (propagates from handleSearchTool)
5. ✅ Matches FTS delegation pattern exactly

---

## Technical Requirements

**File to Modify:**
- `/packages/maproom-mcp/src/index.ts`

**Function to Replace:**
`executeVectorSearch()` (lines ~620-658)

### Implementation

**Replace entire function body:**

```typescript
async function executeVectorSearch(
  client: any,
  query: string,
  repoId: number,
  worktreeId: number | null,
  k: number,
  filter: string,
  filters: any,
  debug: boolean
): Promise<{ rows: any[], debugInfo: any }> {
  // Delegate to search tool handler (same pattern as FTS)
  const { handleSearchTool } = await import('./tools/search.js')
  
  // We need repo name, not just ID - query it
  const { rows: repoRows } = await client.query(
    'SELECT name FROM maproom.repos WHERE id = $1',
    [repoId]
  )
  
  if (repoRows.length === 0) {
    throw new Error(`Repository with ID ${repoId} not found`)
  }
  
  const repo = repoRows[0].name
  
  // Get worktree name if worktreeId is provided
  let worktree: string | undefined
  if (worktreeId) {
    const { rows: wtRows } = await client.query(
      'SELECT name FROM maproom.worktrees WHERE id = $1',
      [worktreeId]
    )
    if (wtRows.length > 0) {
      worktree = wtRows[0].name
    }
  }
  
  // Call search tool with vector mode
  const result = await handleSearchTool(
    {
      query,
      repo,
      worktree,
      limit: k,
      mode: 'vector',
      debug,
    },
    client
  )
  
  // Transform SearchBundle format to old row format
  // handleSearchTool returns: { hits: SearchResult[], total, query, mode, ... }
  // Caller expects: { rows: any[], debugInfo: any }
  
  const rows = result.hits.map(hit => ({
    id: hit.chunk_id,
    relpath: hit.relpath,
    symbol_name: hit.symbol_name,
    kind: hit.kind,
    start_line: hit.start_line,
    end_line: hit.end_line,
    vector_score: hit.score,
    // Add score_breakdown if present (debug mode)
    ...(hit.score_breakdown && {
      base_score: hit.score_breakdown.base_fts,
      kind_mult: hit.score_breakdown.kind_multiplier,
      exact_mult: hit.score_breakdown.exact_match_multiplier,
    }),
  }))
  
  const debugInfo = debug ? {
    mode: 'vector',
    total: result.total,
    query: result.query,
  } : null
  
  return { rows, debugInfo }
}
```

---

## Implementation Notes

### Pattern Consistency
This implementation follows the **exact same delegation pattern** as FTS mode (lines 698-711 in index.ts). The only differences are:
1. Mode parameter is 'vector' instead of 'fts'
2. Result transformation maps to `vector_score` field

### ID to Name Resolution
The caller passes `repoId` and `worktreeId`, but handleSearchTool expects `repo` (name) and `worktree` (name). We query the database to resolve these.

**Alternative:** Modify signature of executeVectorSearch to accept names instead of IDs. However, this would break the pattern established by executeFtsSearch, so we maintain consistency.

### Error Handling
All errors from handleSearchTool (ProcessError, ValidationError) will propagate naturally. The caller in handleSearch() already has try/catch logic to format these properly.

### Debug Mode
If `debug=true`, the search tool includes `score_breakdown` in results. We preserve this in the row transformation for debugging purposes.

---

## Dependencies

**Requires:**
- ✅ UNISRCH-2001 Complete (handleSearchTool supports vector mode)

**Blocks:**
- UNISRCH-2003 (hybrid search needs this function working)
- UNISRCH-3001 (integration testing)

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| ID→name query overhead | Low | Low | Minimal (2 simple queries), can optimize later |
| Schema transformation bugs | Low | Medium | Test thoroughly, verify field mapping |
| Error propagation issues | Low | Low | Existing try/catch handles all error types |

---

## Files to Modify

1. `/packages/maproom-mcp/src/index.ts`
   - Lines ~620-658: Replace `executeVectorSearch()` function body

**Estimated Lines Changed:** ~60 lines (replacing ~38)

---

## Verification Steps

1. **Code Review:**
   - ✅ Function delegates to `handleSearchTool`
   - ✅ ID→name resolution queries are correct
   - ✅ Result transformation maps all fields
   - ✅ Error handling relies on propagation
   - ✅ Debug mode preserved

2. **Type Check:**
   ```bash
   cd packages/maproom-mcp
   npm run typecheck
   ```

3. **Manual Test (if DB available):**
   ```typescript
   const result = await executeVectorSearch(
     client,
     'authentication logic',
     1, // repo_id
     null, // worktree_id
     10, // k
     'all', // filter
     {}, // filters
     false // debug
   )
   console.log(result.rows) // Should have vector search results
   ```

---

## Definition of Done

- [ ] Placeholder code removed
- [ ] Delegation to handleSearchTool implemented
- [ ] ID→name resolution queries added
- [ ] Result transformation correct
- [ ] TypeScript compilation passes
- [ ] No errors in linting
- [ ] Code committed with clear message
- [ ] Ticket marked as Complete

---

## Notes

### Why Not Remove This Function?

You might wonder: why not just call `handleSearchTool` directly from the switch statement in `handleSearch()`?

**Answer:** We keep `executeVectorSearch()` to maintain parallel structure with `executeFtsSearch()` and `executeHybridSearch()`. This allows each mode to have mode-specific logic if needed in the future (e.g., different caching strategies, different error messages, mode-specific optimizations).

For now, it's a thin delegation wrapper, which is fine. MAPDAEMON will likely refactor this entire flow anyway.

**Time Estimate Breakdown:**
- Code changes: 15 minutes
- Testing: 10 minutes
- Commit: 5 minutes
- **Total: 30 minutes**
