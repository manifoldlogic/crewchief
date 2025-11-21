# UNISRCH Ticket Index

**Project:** UNISRCH - Unified Search Client  
**Created:** 2025-11-21  
**Total Tickets:** 5  

---

## Ticket Overview

| Ticket ID | Title | Phase | Effort | Status |
|:----------|:------|:------|:-------|:-------|
| [UNISRCH-2001](./UNISRCH-2001_enable-vector-mode.md) | Enable Vector Mode in Search Tool | Implementation | 30 min | Open |
| [UNISRCH-2002](./UNISRCH-2002_delegate-vector-search.md) | Delegate Vector Search in Index Handler | Implementation | 30 min | Open |
| [UNISRCH-2003](./UNISRCH-2003_update-hybrid-search.md) | Update Hybrid Search Strategy | Implementation | 1 hour | Open |
| [UNISRCH-2004](./UNISRCH-2004_cleanup-placeholders.md) | Remove Placeholder Code and Update Documentation | Implementation | 15 min | Open |
| [UNISRCH-3001](./UNISRCH-3001_integration-testing.md) | Integration Testing for Vector Search | Verification | 1 hour | Open |

**Total Estimated Effort:** 3 hours 15 minutes

---

## Phase Breakdown

### Phase 2: Implementation (4 tickets, 2h 15m)

**Critical Path:**
1. UNISRCH-2001 → Enable vector mode (30 min)
2. UNISRCH-2002 → Delegate vector search (30 min)  
3. UNISRCH-2003 → Update hybrid search (1 hour)
4. UNISRCH-2004 → Cleanup placeholders (15 min)

**Dependencies:**
- 2001 blocks 2002, 3001
- 2002 blocks 2003, 3001
- 2003 blocks 3001
- 2004 can run anytime after 2003

### Phase 3: Verification (1 ticket, 1h)

**Final Validation:**
- UNISRCH-3001 → Integration testing (1 hour)

---

## Execution Strategy

### Sequential Execution (Recommended)

Execute tickets in order to minimize context switching:

```
UNISRCH-2001 (30 min)
  ↓
UNISRCH-2002 (30 min)
  ↓
UNISRCH-2003 (1 hour)
  ↓
UNISRCH-2004 (15 min)
  ↓
UNISRCH-3001 (1 hour)
```

**Total Time:** 3 hours 15 minutes (single session)

### Parallel Execution (If Multiple Agents)

Tickets 2004 (cleanup) can be done in parallel with 2003 (hybrid) by different agents.

---

## Ticket Descriptions

### UNISRCH-2001: Enable Vector Mode in Search Tool
**What:** Modify search tool handler to accept vector mode  
**Why:** Foundation for all vector search functionality  
**How:** Change mode validation, add command selection logic  
**Risk:** Low (small, focused change)

### UNISRCH-2002: Delegate Vector Search in Index Handler
**What:** Replace placeholder executeVectorSearch with delegation  
**Why:** Complete the search delegation pattern  
**How:** Call handleSearchTool(mode='vector'), transform results  
**Risk:** Low (following FTS pattern exactly)

### UNISRCH-2003: Update Hybrid Search Strategy
**What:** Implement RRF fusion or document deferral  
**Why:** True hybrid search needs both FTS + vector  
**How:** Either implement RRF now or defer to MAPDAEMON  
**Risk:** Medium (decision point - 15 min vs 1 hour)  
**Note:** Recommend Option B (defer) - saves time, better implementation later

### UNISRCH-2004: Remove Placeholder Code
**What:** Clean up outdated error messages and docs  
**Why:** Accuracy in error messages and documentation  
**How:** Grep search, update messages, update README  
**Risk:** Low (cosmetic only)

### UNISRCH-3001: Integration Testing
**What:** End-to-end testing of vector search via MCP  
**Why:** Confidence that system works as integrated whole  
**How:** Manual or automated test suite, 7 test cases  
**Risk:** Low (testing only, no production code changes)

---

## Dependencies Graph

```
VECSRCH Project (✅ Complete)
    ↓
UNISRCH-2001 (Enable vector mode)
    ↓
    ├─→ UNISRCH-2002 (Delegate vector)
    │       ↓
    └─→ UNISRCH-2003 (Update hybrid)
            ↓
         UNISRCH-2004 (Cleanup)
            ↓
         UNISRCH-3001 (Testing)
```

**External Dependencies:**
- ✅ VECSRCH-2003 complete (vector-search CLI exists)
- ✅ Database with pgvector extension
- ✅ Embeddings generated (for testing)
- ✅ Rust binary available

---

## Success Criteria

**Project Complete When:**
- [ ] All 5 tickets marked Complete
- [ ] Vector search works via MCP
- [ ] Hybrid search behavior defined (RRF or deferred)
- [ ] Documentation updated
- [ ] Integration tests pass
- [ ] No placeholder error messages remain

**Deliverables:**
1. Vector search delegation working (same pattern as FTS)
2. Hybrid search strategy documented/implemented
3. Clean, accurate error messages and docs
4. Test results confirming end-to-end functionality

---

## Implementation Notes

### Key Insight from Project Review

The original UNISRCH plan estimated 4-6 hours and included tickets to "build CLI wrapper" and "implement subprocess utilities". 

**Review discovered:** All that infrastructure already exists! FTS search already delegates to Rust using perfect patterns (secure spawn, JSON parsing, error handling, database enrichment).

**Result:** Scope reduced to 3 hours by:
- ✅ Reusing existing handleSearchTool (466 lines)
- ✅ Following proven FTS pattern
- ✅ Focusing on what's actually missing (vector mode support)

### Effort Savings

| Original Plan | Revised Scope | Savings |
|:--------------|:---------------|:--------|
| 4-6 hours | 3 hours 15 min | 1-3 hours |
| Build wrapper from scratch | Extend existing | ~400 lines |
| Implement error handling | Reuse framework | ~100 lines |
| Create JSON parser | Reuse existing | ~50 lines |

**Total Code Reuse:** ~550 lines  
**Time Saved:** ~2-3 hours

---

## Risk Summary

| Risk | Mitigation | Status |
|:-----|:-----------|:-------|
| Schema mismatch (FTS vs Vector) | Test early in 3001, fix in 2002 | Monitored |
| Binary not found | Existing discovery handles | Solved |
| Missing embeddings | Clear error from Rust | Handled |
| Performance overhead | Acceptable for MVP, optimize in MAPDAEMON | Accepted |
| Hybrid decision (RRF now or later) | Document decision clearly | Decision needed |

**Overall Risk:** ✅ **LOW**

---

## Alignment with Project Principles

### MVP-Focused Development ✅
- Focused on completing vector delegation (core value)
- Defers RRF optimization to MAPDAEMON (smart trade-off)
- No unnecessary features

### Pragmatic Over Enterprise ✅
- Reuses existing infrastructure
- No reinvention of wheels
- Simple, clear implementation

### AI Agent-Sized Work ✅
- Each ticket: 15 min - 1 hour
- Clear acceptance criteria
- Single responsibility per ticket

### Test for Confidence ✅
- 1 integration test ticket
- Focuses on critical paths
- No excessive coverage

### Complete-Verify-Commit ✅
- Each ticket has definition of done
- Verification steps included
- Clear commit milestones

---

## Reference Documents

- **Project Review:** `../planning/project-review.md` (comprehensive analysis)
- **Review Summary:** `../planning/REVIEW-SUMMARY.md` (executive summary)
- **Plan:** `../planning/plan.md` (original plan)
- **Analysis:** `../planning/analysis.md` (problem definition)
- **Architecture:** `../planning/architecture.md` (solution design)

---

## Progress Tracking

**As you complete tickets, update this section:**

- [ ] UNISRCH-2001: Enable Vector Mode
- [ ] UNISRCH-2002: Delegate Vector Search
- [ ] UNISRCH-2003: Update Hybrid Search
- [ ] UNISRCH-2004: Cleanup Placeholders
- [ ] UNISRCH-3001: Integration Testing

**Completion:** 0/5 (0%)

---

## Next Actions

1. **Review tickets:** Read through all 5 tickets
2. **Verify VECSRCH complete:** Confirm vector-search CLI works
3. **Set up environment:** Database, embeddings, binary path
4. **Start with 2001:** Smallest, foundational change
5. **Follow sequence:** Complete tickets in order for smooth flow

**Ready to start?** Run `/work-on-project` or `/single-ticket UNISRCH-2001`

---

*This ticket index created based on revised project scope from project review (2025-11-21). Original plan significantly simplified by discovering existing FTS delegation infrastructure.*
