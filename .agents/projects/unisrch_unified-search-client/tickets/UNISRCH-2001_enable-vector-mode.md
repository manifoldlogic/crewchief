# UNISRCH-2001: Enable Vector Mode in Search Tool

**Status:** Open  
**Phase:** 2 (Implementation)  
**Estimated Effort:** 30 minutes  
**Priority:** High  

---

## Summary

Modify the search tool handler to support vector search mode by removing the FTS-only restriction and adding mode-aware command selection.

---

## Background

**Current State:**
The search tool in `/packages/maproom-mcp/src/tools/search.ts` already implements perfect subprocess delegation for FTS search (secure spawn, JSON parsing, error handling, database enrichment). However, it explicitly rejects vector mode:

```typescript
// Line 224-230 (current)
if (mode !== 'fts') {
  throw new ValidationError(
    `Search mode "${mode}" not supported by Rust binary...`,
    'UNSUPPORTED_MODE'
  )
}
```

**With VECSRCH Complete:**
The Rust binary now exposes a `vector-search` command that follows the same JSON output schema as FTS `search`. We can enable vector mode by:
1. Removing the FTS-only check
2. Selecting the correct command name based on mode
3. Reusing all existing infrastructure (spawn, parse, enrich)

**Design Principle:**
Maximum code reuse - the vector implementation should be identical to FTS except for command name.

---

## Acceptance Criteria

1. ✅ `handleSearchTool()` accepts `mode: 'vector'` without throwing error
2. ✅ Vector mode spawns `crewchief-maproom vector-search` command
3. ✅ FTS mode still spawns `crewchief-maproom search` command  
4. ✅ All other logic remains unchanged (args, parsing, enrichment)
5. ✅ Error messages updated to reflect vector support

---

## Technical Requirements

**File to Modify:**
- `/packages/maproom-mcp/src/tools/search.ts`

**Changes Required:**

### 1. Update Mode Validation (Line ~225)
```typescript
// Before:
if (mode !== 'fts') {
  throw new ValidationError(...)
}

// After:
if (!['fts', 'vector'].includes(mode)) {
  throw new ValidationError(
    `Search mode "${mode}" not supported. Use mode="fts" or mode="vector".`,
    'UNSUPPORTED_MODE'
  )
}
```

### 2. Add Command Selection (Line ~244, before args array)
```typescript
// Determine command based on mode
const command = mode === 'vector' ? 'vector-search' : 'search'

// Build command arguments
const args = [
  command,  // Changed from hardcoded 'search'
  '--repo',
  repo,
  '--query',
  query,
  '--k',
  String(limit),
]
```

### 3. Update Debug Log (Line ~263)
```typescript
log.debug({ args, mode, command }, 'Spawning Rust binary for search')
```

### 4. Update Error Message (Line ~387)
```typescript
hint:
  error.code === 'UNSUPPORTED_MODE'
    ? 'Use mode="fts" for keyword search or mode="vector" for semantic search.'
    : '...'
```

**No Other Changes Needed:**
- ✅ Spawn logic stays identical
- ✅ JSON parsing stays identical
- ✅ Chunk ID enrichment stays identical
- ✅ Error handling stays identical
- ✅ Result transformation stays identical

---

## Implementation Notes

### Key Insight
Vector mode requires **zero new infrastructure**. The entire 466-line search tool implementation works for both modes by changing only the command name.

### Expected VECSRCH Output
The `vector-search` command should output JSON matching this schema:
```json
{
  "hits": [
    {
      "chunk_id": 123,
      "score": 0.92,
      "file_path": "src/auth.rs",
      "symbol_name": "authenticate",
      "kind": "func",
      "start_line": 10,
      "end_line": 20
    }
  ],
  "total": 10,
  "query": "authentication logic",
  "mode": "vector"
}
```

**Note:** If the actual schema differs, this ticket will need to handle transformation. Verify schema in UNISRCH-3001 (testing).

### Threshold Parameter
The vector-search CLI accepts `--threshold` for similarity filtering. Consider adding:
```typescript
if (mode === 'vector' && validatedParams.threshold) {
  args.push('--threshold', String(validatedParams.threshold))
}
```

**Decision:** Add if threshold is in SearchParams type, otherwise defer to future enhancement.

---

## Dependencies

**Requires:**
- ✅ VECSRCH-2003 Complete (vector-search CLI command exists)
- ✅ VECSRCH-3001 Complete (CLI tested and working)

**Blocks:**
- UNISRCH-2002 (vector search delegation in index.ts)
- UNISRCH-3001 (integration testing)

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| Schema mismatch between FTS and vector | Low | Medium | Verify in testing, add transforms if needed |
| Binary not found | Low | Low | Reuses existing candidate search |
| Missing embeddings | Medium | Low | Rust returns clear error, surfaced to user |

---

## Files to Modify

1. `/packages/maproom-mcp/src/tools/search.ts`
   - Lines ~225-230: Mode validation
   - Lines ~244: Command selection
   - Lines ~387: Error messages

**Estimated Lines Changed:** ~15 lines

---

## Verification Steps

1. **Code Review:**
   - ✅ Mode validation allows 'fts' and 'vector'
   - ✅ Command selection logic is correct
   - ✅ No hardcoded 'search' command remains
   - ✅ Error messages updated

2. **Manual Test (if possible):**
   ```bash
   # Call MCP search tool with vector mode
   {
     "repo": "crewchief",
     "query": "authentication",
     "mode": "vector",
     "limit": 5
   }
   ```

3. **Expected Behavior:**
   - ✅ Spawns `crewchief-maproom vector-search ...`
   - ✅ Returns JSON results without error
   - ✅ FTS mode still works unchanged

---

## Definition of Done

- [ ] Code changes completed and reviewed
- [ ] Mode validation accepts both 'fts' and 'vector'
- [ ] Command selection logic implemented
- [ ] Error messages updated
- [ ] No breaking changes to FTS mode
- [ ] Code committed with clear message
- [ ] Ticket marked as Complete

---

## Notes

This ticket is the foundation for UNISRCH. It enables the exact same proven pattern (subprocess delegation, JSON parsing, DB enrichment) to work for vector search. All subsequent tickets build on this change.

**Time Estimate Breakdown:**
- Code changes: 10 minutes
- Testing: 10 minutes  
- Documentation: 5 minutes
- Commit: 5 minutes
- **Total: 30 minutes**
