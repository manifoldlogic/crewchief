# Project Review Updates

**Original Review Date:** 2025-11-26
**Updates Completed:** 2025-11-26
**Update Status:** ✅ Complete

## Critical Issues Addressed

### Issue 1: Status Module Direct PostgreSQL Coupling
**Original Problem:** `status.rs` creates its own PostgreSQL connection via `tokio_postgres::connect()`, bypassing factory pattern. Uses JSONB operators that won't work with SQLite.

**Changes Made:**
- analysis.md: Added new section documenting status.rs PostgreSQL coupling
- architecture.md: Added status.rs refactoring to Component Changes
- plan.md: Expanded MAPCLI-1004 scope to include status module refactoring

**Result:** Issue resolved - status.rs refactoring now explicitly in scope with concrete approach

---

### Issue 2: Indexer Functions Require PostgreSQL Client
**Original Problem:** `scan_worktree`, `upsert_files`, `watch_worktree` take `tokio_postgres::Client`. No SQLite equivalent exists.

**Changes Made:**
- analysis.md: Added section documenting indexer coupling with function signatures
- architecture.md: Replaced "hybrid approach" placeholder with concrete decision (Option B: defer indexer commands)
- plan.md: Moved scan/upsert/watch to "Phase 2 - Future Work" section, focused MVP on daemon/search/status/cleanup

**Result:** Issue resolved - MVP scope reduced to achievable goals, indexer abstraction deferred

---

### Issue 3: Missing BackendType and backend_type() Method
**Original Problem:** Architecture references `store.backend_type()` and `BackendType` enum but neither exists in codebase.

**Changes Made:**
- plan.md: Created new MAPCLI-1000 ticket as prerequisite for BackendType enum
- architecture.md: Explicitly documented BackendType as first deliverable
- quality-strategy.md: Added test for backend_type() method

**Result:** Issue resolved - BackendType is now explicit first deliverable with own ticket

## High-Risk Mitigations Implemented

### Risk 1: Daemon execute_search Uses Direct Queries
**Mitigation Applied:**
- architecture.md: Added detailed daemon refactoring approach using trait methods
- plan.md: MAPCLI-1002 scope explicitly includes replacing raw queries

**Risk Level:** Reduced from High to Medium

### Risk 2: Search Modes in Daemon
**Mitigation Applied:**
- architecture.md: Documented that daemon will use VectorStore trait search methods exclusively
- plan.md: Added acceptance criteria verifying all search modes work

**Risk Level:** Reduced from Medium to Low

### Risk 3: Auto-Generate Embeddings Uses Raw Queries
**Mitigation Applied:**
- analysis.md: Documented as PostgreSQL-only for MVP
- architecture.md: Added to "Deferred Items" section
- plan.md: Explicitly marked as out of scope for Phase 1

**Risk Level:** Accepted as Low (optional feature)

## Gaps Filled

### Requirements Gaps
- ✅ Status Query Abstraction → Decided to refactor status.rs to use existing list_repos/list_worktrees + iteration
- ✅ Embedding Count Query → Documented as PostgreSQL-only for MVP
- ✅ Default SQLite Path → Updated architecture.md to specify `~/.maproom/maproom.db`

### Technical Gaps
- ✅ SQLite Indexer Implementation → Deferred to Phase 2, not blocking MVP
- ✅ Chunk Detail Fetching → Added `get_search_hit_details()` helper approach to architecture.md

### Process Gaps
- ✅ Testing Strategy for Indexer → Clarified: test with pre-indexed SQLite database
- ✅ Order of Operations → BackendType is now MAPCLI-1000, first ticket

## Scope Adjustments

### Removed from MVP (Moved to Phase 2)
- `scan` command with SQLite - requires indexer abstraction
- `upsert` command with SQLite - requires indexer abstraction
- `watch` command with SQLite - requires indexer abstraction
- `generate-embeddings` with SQLite - optional feature

### Clarified Boundaries
- **Phase 1 (MVP):** daemon, search, status, cleanup with SQLite backend
- **Phase 2 (Future):** scan, upsert, watch with SQLite (indexer abstraction)
- **Out of scope:** Parallel scan for SQLite, embedding generation abstraction

## Alignment Improvements

### MVP Discipline
- Reduced Phase 1 from 5 commands to 4 (search, status, cleanup, daemon)
- Focused on: "Zero-config semantic search for existing indexes"
- Deferred indexing to Phase 2 when indexer can be properly abstracted

### Pragmatism
- Replaced "hybrid approach" placeholder with concrete "defer indexer" decision
- Removed complexity of SQLite indexer functions from MVP
- Accepted that pre-indexed data can be searched (useful for testing)

## Document Change Summary

### analysis.md
- Lines modified: ~60
- Key changes: Added status.rs coupling section, indexer coupling section, updated impact analysis

### architecture.md
- Lines modified: ~80
- Key changes: Replaced hybrid approach, added status.rs refactoring, concrete BackendType spec

### plan.md
- Lines modified: ~100
- Key changes: Added MAPCLI-1000, restructured to 6 tickets, added Phase 2 section

### quality-strategy.md
- Lines modified: ~30
- Key changes: Updated test matrix for reduced scope, added pre-indexed testing approach

### README.md
- Lines modified: ~20
- Key changes: Updated objectives and ticket list to reflect new scope

## Verification

**Completed Updates:**
1. ✅ analysis.md - Added status.rs and indexer coupling sections
2. ✅ architecture.md - Replaced hybrid approach with concrete Option B decision, added status.rs refactoring
3. ✅ plan.md - Added MAPCLI-1000 ticket, restructured to 6 tickets, added Phase 2 section
4. ✅ quality-strategy.md - Updated test matrix, added pre-indexed testing approach
5. ✅ README.md - Updated objectives, tickets, and scope

**Next Steps:**
1. Re-run `/review-project MAPCLI` to verify improvements
2. Proceed to `/create-project-tickets MAPCLI` if review passes

**Success Metrics:**
- [x] All critical issues resolved
- [x] High-risk areas mitigated
- [x] Requirements specific and measurable
- [x] Scope appropriate for MVP
- [x] Plan ready for ticket creation
- [x] BackendType is first deliverable (MAPCLI-1000)
- [x] status.rs refactoring explicitly in MAPCLI-1004
- [x] Indexer commands deferred to Phase 2
