# UNISRCH-2003: Update Hybrid Search Strategy

**Status:** Open  
**Phase:** 2 (Implementation)  
**Estimated Effort:** 1 hour  
**Priority:** Medium  

---

## Summary

Update `executeHybridSearch()` to properly combine FTS and vector search results using Reciprocal Rank Fusion (RRF) or document a decision to defer full implementation to MAPDAEMON.

---

## Background

**Current State:**
The `/packages/maproom-mcp/src/index.ts` hybrid search implementation is a fallback to FTS only:

```typescript
// index.ts:660-682 (current)
async function executeHybridSearch(...) {
  // For now, fall back to FTS until vector embedding service is integrated
  const result = await executeFtsSearch(client, query, repoId, worktreeId, k, filter, filters, debug)
  
  if (debug && result.debugInfo) {
    result.debugInfo.mode = 'hybrid (fts-only fallback)'
    result.debugInfo.note = 'Hybrid search falls back to FTS until vector embeddings are available...'
  }
  
  return result
}
```

**With Vector Search Available:**
Now that both FTS and vector modes work via delegation, we can implement true hybrid search by:
1. Calling both FTS and vector search
2. Merging results using Reciprocal Rank Fusion (RRF)
3. Returning top-k merged results

**Strategic Decision Point:**
- **Option A:** Implement RRF fusion now (1 hour, delivers full hybrid search)
- **Option B:** Keep fallback, document decision, defer to MAPDAEMON
- **Option C:** Implement simple score averaging (30 min, less optimal than RRF)

---

## Acceptance Criteria

### If Implementing RRF (Option A):
1. ✅ Calls both `executeFtsSearch()` and `executeVectorSearch()`
2. ✅ Merges results using Reciprocal Rank Fusion algorithm
3. ✅ Returns top-k results sorted by fusion score
4. ✅ Handles cases where a chunk appears in both result sets
5. ✅ Debug mode shows FTS score, vector score, and fusion score

### If Deferring (Option B):
1. ✅ Documents decision to defer in code comments
2. ✅ Updates debug message to be more explicit
3. ✅ Links to MAPDAEMON project for future implementation
4. ✅ Keeps fallback behavior unchanged

---

## Technical Requirements

**File to Modify:**
- `/packages/maproom-mcp/src/index.ts`

**Function to Update:**
`executeHybridSearch()` (lines ~660-682)

### Option A: Implement RRF Fusion

**Reciprocal Rank Fusion Algorithm:**
```
For each result in FTS results at rank i:
  rrf_score(chunk) += 1 / (k + i)

For each result in Vector results at rank j:
  rrf_score(chunk) += 1 / (k + j)

Where k is a constant (typically 60)
```

**Implementation:**

```typescript
async function executeHybridSearch(
  client: any,
  query: string,
  repoId: number,
  worktreeId: number | null,
  k: number,
  filter: string,
  filters: any,
  debug: boolean
): Promise<{ rows: any[], debugInfo: any }> {
  // Execute both search modes in parallel
  const [ftsResult, vectorResult] = await Promise.all([
    executeFtsSearch(client, query, repoId, worktreeId, k, filter, filters, debug),
    executeVectorSearch(client, query, repoId, worktreeId, k, filter, filters, debug),
  ])
  
  // RRF constant (typical value from literature)
  const RRF_K = 60
  
  // Build fusion scores map: chunk_id -> { fts_rank?, vector_rank?, rrf_score, chunk_data }
  const fusionMap = new Map<number, any>()
  
  // Process FTS results
  ftsResult.rows.forEach((row, index) => {
    const chunkId = row.id
    const rrfScore = 1 / (RRF_K + index)
    fusionMap.set(chunkId, {
      chunk: row,
      fts_rank: index,
      fts_score: row.fts_score,
      rrf_score: rrfScore,
    })
  })
  
  // Process vector results (add or update)
  vectorResult.rows.forEach((row, index) => {
    const chunkId = row.id
    const rrfScore = 1 / (RRF_K + index)
    
    if (fusionMap.has(chunkId)) {
      // Chunk appears in both - add vector RRF score
      const entry = fusionMap.get(chunkId)!
      entry.vector_rank = index
      entry.vector_score = row.vector_score
      entry.rrf_score += rrfScore
    } else {
      // Chunk only in vector results
      fusionMap.set(chunkId, {
        chunk: row,
        vector_rank: index,
        vector_score: row.vector_score,
        rrf_score: rrfScore,
      })
    }
  })
  
  // Sort by fusion score and take top-k
  const fusedResults = Array.from(fusionMap.values())
    .sort((a, b) => b.rrf_score - a.rrf_score)
    .slice(0, k)
  
  // Transform to row format
  const rows = fusedResults.map(entry => {
    const row = {
      ...entry.chunk,
      hybrid_score: entry.rrf_score,
    }
    
    // Add debug info if requested
    if (debug) {
      row.fts_rank = entry.fts_rank ?? null
      row.vector_rank = entry.vector_rank ?? null
      row.fts_score = entry.fts_score ?? null
      row.vector_score = entry.vector_score ?? null
      row.rrf_fusion_score = entry.rrf_score
    }
    
    return row
  })
  
  const debugInfo = debug ? {
    mode: 'hybrid (RRF fusion)',
    rrf_k: RRF_K,
    fts_results: ftsResult.rows.length,
    vector_results: vectorResult.rows.length,
    unique_chunks: fusionMap.size,
    final_results: rows.length,
  } : null
  
  return { rows, debugInfo }
}
```

### Option B: Document Deferral

**Implementation:**

```typescript
async function executeHybridSearch(
  client: any,
  query: string,
  repoId: number,
  worktreeId: number | null,
  k: number,
  filter: string,
  filters: any,
  debug: boolean
): Promise<{ rows: any[], debugInfo: any }> {
  // UNISRCH-2003 Decision: Defer RRF fusion to MAPDAEMON project
  //
  // Rationale:
  // - RRF fusion requires running both FTS and vector search (2x overhead)
  // - Each search spawns a process (subprocess overhead)
  // - MAPDAEMON will optimize this with persistent connection pools
  // - For MVP, FTS fallback provides acceptable experience
  //
  // Implementation plan:
  // - MAPDAEMON will add persistent Rust daemon with connection pooling
  // - Daemon can run both searches efficiently and merge in-process
  // - Will reduce latency from ~200ms to ~20ms for hybrid search
  //
  // See: .agents/projects/mapdaemon_maproom-daemon-architecture/
  
  const result = await executeFtsSearch(client, query, repoId, worktreeId, k, filter, filters, debug)
  
  if (debug && result.debugInfo) {
    result.debugInfo.mode = 'hybrid (fts-fallback)'
    result.debugInfo.note = [
      'Hybrid search currently falls back to FTS mode for performance.',
      'True hybrid (RRF fusion) deferred to MAPDAEMON project.',
      'Reason: Process overhead from dual subprocess spawning.',
      'See MAPDAEMON for persistent daemon implementation.',
    ].join(' ')
  }
  
  return result
}
```

---

## Implementation Notes

### Recommendation: Option B (Defer to MAPDAEMON)

**Reasoning:**
1. **Performance:** Running both FTS + vector doubles the subprocess overhead (2 x ~100ms = ~200ms)
2. **MAPDAEMON Soon:** Next project in sequence will optimize this with persistent daemon
3. **MVP Principle:** FTS fallback is acceptable for MVP, RRF can wait for optimized version
4. **Time Savings:** Saves 1 hour now, implements better version later

**When to Choose Option A:**
- If hybrid search is critical for immediate use
- If MAPDAEMON timeline is uncertain (>2 weeks away)
- If users explicitly request true hybrid mode

### RRF Algorithm Details

**Why RRF over Score Averaging:**
- FTS scores (0-10) and vector scores (0-1) are on different scales
- Averaging requires normalization, which is lossy
- RRF uses ranks only (scale-independent)
- RRF is proven effective in search retrieval literature

**RRF Constant k=60:**
- Standard value from literature
- Controls how quickly lower ranks decay
- Higher k = more weight to lower ranks
- Lower k = more weight to top ranks

---

## Dependencies

**Requires:**
- ✅ UNISRCH-2001 Complete (vector mode enabled)
- ✅ UNISRCH-2002 Complete (vector delegation works)

**Blocks:**
- UNISRCH-3001 (testing needs final hybrid behavior)

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| RRF performance overhead | High | Medium | Defer to MAPDAEMON with pooling |
| Inconsistent chunk IDs | Low | High | Both searches use same ID resolution |
| Score interpretation confusion | Medium | Low | Clear debug output explaining fusion |

---

## Files to Modify

1. `/packages/maproom-mcp/src/index.ts`
   - Lines ~660-682: Update `executeHybridSearch()` function

**Estimated Lines Changed:**
- Option A: ~80 lines (implementing RRF)
- Option B: ~20 lines (documenting deferral)

---

## Verification Steps

### If Option A (RRF Implementation):

1. **Code Review:**
   - ✅ Both searches called in parallel
   - ✅ RRF algorithm correctly implemented
   - ✅ Duplicate chunks handled (appear in both)
   - ✅ Results sorted by fusion score
   - ✅ Debug mode shows all scores

2. **Manual Test:**
   ```typescript
   const result = await executeHybridSearch(
     client,
     'authentication',
     1, // repo_id
     null,
     10,
     'all',
     {},
     true // debug mode
   )
   
   // Verify:
   // - result.rows has <= 10 items
   // - Each row has hybrid_score
   // - Debug shows fts_rank, vector_rank, fusion_score
   ```

3. **Performance Check:**
   - Hybrid search should take ~2x single mode time
   - Log warnings if >500ms

### If Option B (Deferral):

1. **Code Review:**
   - ✅ Comments clearly explain decision
   - ✅ Links to MAPDAEMON project
   - ✅ Debug message updated
   - ✅ No functional changes to fallback

2. **Documentation Check:**
   - ✅ Decision documented in ticket
   - ✅ Rationale clear for future developers

---

## Definition of Done

### Option A:
- [ ] RRF fusion algorithm implemented
- [ ] Tests pass for hybrid mode
- [ ] Debug output shows fusion details
- [ ] Performance acceptable (<500ms)
- [ ] Code committed with clear message
- [ ] Ticket marked as Complete

### Option B:
- [ ] Deferral decision documented in code
- [ ] Debug message updated
- [ ] Comments link to MAPDAEMON
- [ ] Code committed with clear message
- [ ] Ticket marked as Complete

---

## Decision Required

**Before starting this ticket, decide:**
- [ ] Option A: Implement RRF now (1 hour, immediate value)
- [ ] Option B: Defer to MAPDAEMON (15 min, better long-term)

**Recommendation:** Option B  
**Justification:** MAPDAEMON is next in sequence, will implement RRF more efficiently with connection pooling. Save the 1 hour and do it right in the next project.

**Time Estimate Breakdown:**
- Option A: 60 minutes (implementation + testing)
- Option B: 15 minutes (documentation + commit)
