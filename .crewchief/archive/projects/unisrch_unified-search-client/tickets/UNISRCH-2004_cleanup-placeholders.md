# UNISRCH-2004: Remove Placeholder Code and Update Documentation

**Status:** ✅ Completed  
**Phase:** 2 (Implementation)  
**Estimated Effort:** 15 minutes  
**Priority:** Low  

---

## Summary

Clean up remaining placeholder error messages and update tool documentation to reflect full vector search support.

---

## Background

**Current State:**
Various error messages and comments throughout the codebase reference vector search as "not implemented" or "in progress". With UNISRCH-2001 and UNISRCH-2002 complete, these messages are now outdated and confusing.

**Examples:**
- Error messages saying "vector mode not supported"
- Comments referencing "HYBRID_SEARCH-2001" project
- Tool descriptions that don't mention vector support
- Debug notes about "vector embeddings not available"

**Desired State:**
- All error messages accurate
- Documentation reflects vector + FTS support
- No references to placeholder implementations
- Clear guidance on when to use each mode

---

## Acceptance Criteria

1. ✅ No error messages claim vector search is unavailable
2. ✅ Tool schema descriptions mention vector mode
3. ✅ Comments updated to reflect current functionality
4. ✅ README or tool docs explain mode selection
5. ✅ References to old project codes removed

---

## Technical Requirements

**Files to Review and Update:**

### 1. `/packages/maproom-mcp/src/index.ts`

**Line ~196:** Tool description
```typescript
// Before:
description: 'Search mode: "fts" for full-text keyword search, "vector" for semantic similarity, "hybrid" (default) for combined approach'

// After:
description: 'Search mode: "fts" for keyword search (fast), "vector" for semantic similarity (requires embeddings), "hybrid" for combined ranking (slower but comprehensive)'
```

**Line ~656:** Remove old TODO comment
```typescript
// Before:
// TODO: Integrate with embedding service to generate query embedding
// Vector search implementation is in progress (HYBRID_SEARCH-2001).

// After:
// (Remove entirely - vector search now works)
```

### 2. `/packages/maproom-mcp/src/tools/search.ts`

**Line ~227:** Already updated in UNISRCH-2001, verify wording
```typescript
// Should say:
`Search mode "${mode}" not supported. Use mode="fts" or mode="vector".`

// Not:
`Search mode "${mode}" not supported by Rust binary. Only "fts" mode is available...`
```

**Line ~437:** Update hint in error formatting
```typescript
// Before:
hint: '... mode must be "fts", "vector", or "hybrid".'

// After:  
hint: '... mode must be "fts", "vector", or "hybrid". FTS for keywords, vector for semantics (requires embeddings), hybrid for best results.'
```

### 3. `/packages/maproom-mcp/README.md` (if exists)

Add mode selection guidance:

```markdown
## Search Modes

The search tool supports three modes:

- **FTS (Full-Text Search)**: Fast keyword-based search using PostgreSQL FTS
  - Best for: Finding specific function names, error messages, exact terms
  - Latency: ~50-100ms
  - Requires: Indexed repository

- **Vector (Semantic Search)**: AI-powered similarity search using embeddings
  - Best for: Conceptual queries, "code that does X", finding similar patterns
  - Latency: ~100-200ms
  - Requires: Indexed repository + generated embeddings (run `generate-embeddings`)

- **Hybrid (Combined)**: Merges FTS and vector results with reciprocal rank fusion
  - Best for: Most searches - combines precision of FTS with recall of vector
  - Latency: ~200-300ms (runs both searches)
  - Requires: Same as vector mode

### Examples

```typescript
// Fast keyword search
{ mode: "fts", query: "handleSearch", repo: "crewchief" }

// Semantic similarity search
{ mode: "vector", query: "authentication logic", repo: "crewchief" }

// Best of both worlds
{ mode: "hybrid", query: "error handling patterns", repo: "crewchief" }
```

### Mode Selection Guide

- Use **FTS** when: Looking for specific identifiers, known terms
- Use **Vector** when: Exploring concepts, finding related code
- Use **Hybrid** when: Unsure which mode fits, or want comprehensive results
```

### 4. Check for Other References

Search codebase for outdated references:
```bash
cd packages/maproom-mcp
grep -r "not implemented" src/
grep -r "in progress" src/
grep -r "HYBRID_SEARCH" src/
grep -r "TODO.*vector" src/
```

Remove or update any found.

---

## Implementation Notes

### Why This Matters

Outdated error messages and documentation create confusion:
- Users think vector search doesn't work when it does
- Developers see TODO comments and think code is incomplete
- Error messages provide wrong guidance ("use FTS instead")

### Accuracy in Error Messages

Keep error messages that are still valid:
- ✅ "No embeddings found - run generate-embeddings" (valid)
- ✅ "Repository not indexed - run scan" (valid)
- ❌ "Vector search not implemented" (outdated - remove)

---

## Dependencies

**Requires:**
- ✅ UNISRCH-2001 Complete (vector mode enabled)
- ✅ UNISRCH-2002 Complete (delegation works)
- ✅ UNISRCH-2003 Complete (hybrid behavior defined)

**Blocks:**
- None (cosmetic cleanup)

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| Breaking error message parsing | Low | Low | Review MCP clients, ensure no dependency on exact error text |
| Missing a reference | Medium | Low | Thorough grep search before commit |

---

## Files to Modify

1. `/packages/maproom-mcp/src/index.ts` (~5 locations)
2. `/packages/maproom-mcp/src/tools/search.ts` (~2 locations)
3. `/packages/maproom-mcp/README.md` (if exists, or create)
4. Any other files found via grep

**Estimated Lines Changed:** ~20-30 lines

---

## Verification Steps

1. **Grep Search:**
   ```bash
   # Should return zero results:
   grep -r "not implemented" src/
   grep -r "in progress.*vector" src/
   grep -r "HYBRID_SEARCH-2001" src/
   ```

2. **Tool Schema Check:**
   - ✅ Description accurately describes all modes
   - ✅ No references to placeholder implementation

3. **Error Message Review:**
   - ✅ All error hints provide actionable guidance
   - ✅ No contradictory messages (saying both "works" and "not available")

4. **Documentation Quality:**
   - ✅ README explains mode selection clearly
   - ✅ Examples show realistic usage
   - ✅ Performance expectations set

---

## Definition of Done

- [ ] Grep search for outdated references completes with no results
- [ ] Tool descriptions updated in schema
- [ ] Error messages provide accurate guidance
- [ ] README documentation added/updated
- [ ] All comments reviewed and updated
- [ ] Code committed with clear message
- [ ] Ticket marked as Complete

---

## Notes

This is a "polish" ticket - not strictly necessary for functionality, but important for:
- User experience (accurate error messages)
- Developer experience (no confusing TODOs)
- Project professionalism (documentation matches reality)

**Can be combined with final commit of UNISRCH-2003** if time is short.

**Time Estimate Breakdown:**
- Grep search: 3 minutes
- Update messages: 5 minutes
- Update docs: 5 minutes
- Commit: 2 minutes
- **Total: 15 minutes**
