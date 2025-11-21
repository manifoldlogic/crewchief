# UNISRCH Project Review Summary

**Date:** 2025-11-21  
**Reviewer:** Antigravity (Technical Architect)  
**Status:** ⚠️ **MAJOR REVISION REQUIRED**

---

## TL;DR

The UNISRCH project planning is **outdated**. The "split brain" problem it aimed to solve for FTS **already doesn't exist** - the MCP server already delegates FTS to the Rust binary using correct patterns.

**What remains:** Complete the pattern by adding vector search delegation (simple, 2-3 hours).

**Recommendation:** Revise tickets and proceed with greatly simplified scope.

---

## Key Discoveries

### ✅ **EXCELLENT NEWS:** Infrastructure Already Built

The MCP codebase at `/packages/maproom-mcp/src/tools/search.ts` already implements:
- ✅ Rust binary subprocess delegation
- ✅ Secure `spawn()` with args array (no command injection risk)
- ✅ Smart binary candidate search with fallbacks
- ✅ JSON output parsing with error handling
- ✅ Database enrichment (fetching chunk IDs)
- ✅ Comprehensive error handling (ProcessError, ValidationError)
- ✅ 466 lines of production-ready code

**Evidence:**
```typescript
// This already exists and works!
export async function handleSearchTool(params, client) {
  const args = ['search', '--repo', repo, '--query', query, '--k', String(limit)]
  result = await trySpawnWithCandidates(candidates, args, { timeout: 30000 })
  rustOutput = JSON.parse(result.stdout)
  const chunkIdMap = await fetchChunkIds(client, repo, rustOutput.hits)
  return bundle
}
```

### 🎯 **ACTUAL GAP:** Vector Mode Not Enabled

Only FTS delegates to Rust. Vector/hybrid modes have placeholder code that throws errors:

```typescript
// index.ts (current state)
async function executeVectorSearch(...) {
  throw new Error('Vector search requires query embedding generation...')
}

async function executeHybridSearch(...) {
  // Fallback to FTS
  return await executeFtsSearch(...)
}
```

With VECSRCH complete, we can now enable vector mode using the **exact same pattern**.

---

## Impact on Original Plan

| Original Ticket | Status | Reality |
|:----------------|:-------|:--------|
| Ticket 1: Clean up legacy code | ❌ Not needed | FTS already delegates, no legacy code exists |
| Ticket 2: Implement Rust CLI wrapper | ❌ Already done | `handleSearchTool` + `trySpawnWithCandidates` exist |
| Ticket 3: Update MCP Tool definition | ❌ Already done | Schema includes all modes, just not implemented |
| Ticket 4: E2E Testing | ✅ Still needed | Tests required for vector mode |

**Wasted Effort if Unchanged:** ~3 hours reinventing existing code  
**Actual Effort Needed:** ~2-3 hours to complete vector delegation

---

## Revised Scope (RECOMMENDED)

### Phase 1: Implementation (2 hours)

**UNISRCH-2001: Enable Vector Mode in Search Tool** (30 min)
- Modify `/src/tools/search.ts` lines 224-230
- Change FTS-only check to allow 'vector' mode
- Add mode-aware command selection: `vector-search` vs `search`

**UNISRCH-2002: Delegate Vector Search** (30 min)
- Replace `/src/index.ts` `executeVectorSearch()` placeholder
- Call `handleSearchTool` with `mode='vector'`
- Transform output to match expected format

**UNISRCH-2003: Update Hybrid Search** (1 hour)
- Implement RRF fusion (call both FTS + Vector, merge by rank)
- OR defer to future MAPDAEMON optimization
- Remove fallback-only behavior

**UNISRCH-2004: Cleanup** (15 min)
- Remove "not implemented" error messages
- Update documentation

### Phase 2: Testing (1 hour)

**UNISRCH-3001: Integration Testing**
- End-to-end vector search via MCP
- Verify JSON schema compatibility
- Test error cases (no embeddings, invalid params)

**Total Effort:** 3 hours (vs original 4-6 hours)

---

## Critical Recommendations

### 1. **DO NOT** Reimplement Existing Code
The original plan risks rebuilding:
- Binary discovery mechanism ❌
- Subprocess spawning utilities ❌
- Error handling framework ❌
- JSON parsing logic ❌

**All of this already exists and works perfectly.**

### 2. **DO** Follow Existing Pattern
Vector mode needs identical implementation to FTS:
```typescript
// Just add this logic:
const command = mode === 'vector' ? 'vector-search' : 'search'
const args = [command, '--repo', repo, '--query', query, '--k', String(limit)]
// Use existing trySpawnWithCandidates()...
```

### 3. **DO** Update Planning Documents
Current analysis/architecture docs describe a non-existent problem. Update before creating tickets to avoid confusion.

### 4. **DO** Verify VECSRCH Output Schema
Run a quick test to ensure `vector-search` command output matches FTS output format:
```json
{
  "hits": [
    {"chunk_id": 123, "score": 0.92, "file_path": "...", ...}
  ]
}
```

### 5. **DEFER** Hybrid RRF Fusion (Optional)
RRF implementation can wait for MAPDAEMON if needed. Hybrid search can continue falling back to FTS as an MVP, or implement simple fusion now.

---

## Risk Assessment

| Risk | Level | Mitigation Status |
|:-----|:------|:------------------|
| Command Injection | ~~High~~ | ✅ Already mitigated (spawn with args array) |
| Process Overhead | Low (accepted) | Deferred to MAPDAEMON |
| Binary Not Found | Low | ✅ Smart candidate search exists |
| Missing Embeddings | Medium (NEW) | Surface clear error from Rust CLI |
| Schema Drift | Low (NEW) | Verify compatibility in tests |

**Overall Risk:** ✅ **LOW** (reusing proven patterns)

---

## Security Status

✅ **EXCELLENT** - All concerns from security-review.md already addressed:
- Uses `spawn()` with args array (no shell interpolation)
- Binary path is fixed/trusted
- Input validated with Zod before spawning
- Timeout protection (30s)
- Error message sanitization

**No new security work needed.**

---

## Dependencies

**Requirements:**
- ✅ VECSRCH complete (Rust `vector-search` CLI exists)
- ✅ Binary discovery infrastructure exists
- ✅ Process spawning utilities exist
- ✅ JSON parsing framework exists
- ✅ Tool schema already includes vector mode

**Blocks:**
- 🔄 MAPDAEMON (will optimize performance)
- 🔄 Full RRF hybrid (optional, can defer)

**All dependencies MET. No blockers.**

---

## Go/No-Go Decision

### ✅ **GO** (with mandatory revisions)

**Decision:** APPROVED to proceed

**Conditions:**
1. ✅ Revise tickets to match "Revised Scope" above
2. ✅ Update planning docs to acknowledge current state
3. ✅ Set realistic effort estimate (3 hours, not 4-6)
4. ✅ Follow existing FTS pattern exactly

**Confidence:** High  
**Complexity:** Low  
**Risk:** Low  
**Value:** High  

**Timeline:** Can start immediately after ticket revision

---

## Comparison: Before vs After

### Before (Original Plan)
- 📋 4 tickets
- ⏱️ 4-6 hours
- 🔨 Build CLI wrapper from scratch
- 🔨 Implement error handling
- 🔨 Create subprocess utilities
- 🔨 Design JSON parsing
- ⚠️ Reinventing existing code

### After (Revised Plan)
- 📋 5 tickets (4 impl + 1 test)
- ⏱️ 3 hours
- ✅ Extend existing `handleSearchTool`
- ✅ Reuse error framework
- ✅ Reuse subprocess utilities
- ✅ Reuse JSON parsing
- ✅ Following proven pattern

**Time Savings:** 2-3 hours  
**Code Reuse:** ~450 lines  
**Risk Reduction:** Significant (using battle-tested code)

---

## Next Actions

### For Project Lead (You)

1. **Read full review:** See `/planning/project-review.md` (comprehensive)
2. **Update planning docs:** Correct analysis.md to reflect current state
3. **Revise tickets:** Use revised scope as template
4. **Verify VECSRCH output:** Quick test to confirm JSON schema matches
5. **Proceed with execution:** Follow `/work-on-project` workflow

### Ready to Create Tickets?

When ready:
- Run: `/create-project-tickets` with revised scope
- OR manually create 5 tickets based on revised scope section
- OR update existing plan.md and regenerate tickets

---

## Conclusion

This project is **simpler and lower-risk** than originally planned because excellent infrastructure already exists. The review caught this early, saving significant time and preventing code duplication.

**Bottom Line:**
- ✅ FTS delegation: Already works perfectly
- 🔄 Vector delegation: Add it using same pattern (2-3 hours)
- ✅ Security: Already excellent
- ✅ Architecture: Already sound
- ✅ Dependencies: All met

**Proceed with confidence, but with revised scope.**

---

**Review Status:** ✅ COMPLETE  
**Full Review:** See `project-review.md` for detailed analysis  
**Recommendation:** **APPROVED WITH REVISIONS**

