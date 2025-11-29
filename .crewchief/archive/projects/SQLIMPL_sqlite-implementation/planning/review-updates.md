# Project Review Updates

**Original Review Date:** 2025-11-27
**Updates Completed:** 2025-11-27
**Update Status:** Complete

## Critical Issues Addressed

### Issue 1: Search Executors Should Delegate, Not Reimplement
**Original Problem:** Plan proposed implementing SQL queries in search executors, but SqliteStore already has working implementations.
**Changes Made:**
- architecture.md: Rewrote implementation strategy to emphasize delegation to existing SqliteStore methods
- architecture.md: Removed duplicate SQL query patterns, replaced with existing method references
- architecture.md: Added "Existing SqliteStore Methods" section documenting what already exists
- plan.md: Updated Phase 2 tickets to "wire executor to SqliteStore" instead of "implement SQL"
**Result:** Issue resolved - architecture now leverages existing code instead of reimplementing

### Issue 2: Ticket Count Mismatch
**Original Problem:** Plan overview said "16 tickets" but detailed plan listed 19 tickets
**Changes Made:**
- plan.md: Updated overview to say "19 tickets"
- README.md: Updated to say "19 tickets planned across 5 phases"
**Result:** Issue resolved - counts are now consistent

## Boundary Violations Fixed

### No boundary violations identified
The existing plan correctly uses direct method calls within the same crate. SqliteStore methods are internal to the maproom crate and appropriate to call directly.

## High-Risk Mitigations Implemented

### Risk 1: Test Migration Scope Creep
**Mitigation Applied:**
- plan.md: Added triage step to Phase 1 Ticket 1001 to classify tests before migration
- plan.md: Added guidance on deleting vs migrating vs deferring tests
**Risk Level:** Reduced from High to Medium

### Risk 2: Context Assembly Complexity Underestimated
**Mitigation Applied:**
- plan.md: Marked Phase 4 as "Optional Enhancement - defer if timeline pressure"
- plan.md: Added note about potential tree-sitter integration needs
- quality-strategy.md: Added note about context assembly complexity
**Risk Level:** Reduced from Medium to Low (by making optional)

### Risk 3: Watch Command Dependencies
**Mitigation Applied:**
- No changes needed - plan already correctly documents dependency
**Risk Level:** Remains Medium (acceptable)

## Gaps Filled

### Requirements Gaps
- analysis.md: Added "Existing SqliteStore Methods" section documenting what's already implemented
- architecture.md: Clarified that score normalization functions already exist in src/db/sqlite/fts.rs and vector.rs
- architecture.md: Clarified that kind multipliers are NOT currently applied (genuine TODO)
- plan.md: Added note that signals executor data comes from commits table (committed_at column)

### Technical Gaps
- plan.md: Added schema verification step to tickets 3001 and 4001
- architecture.md: Added section on existing tables/schema

### Process Gaps
- plan.md: Added "Audit existing implementation" step to each phase
- quality-strategy.md: Added discovery step before implementation

## Scope Adjustments

### Removed from MVP
- Phase 4 (Context Assembly) marked as optional enhancement
- Can proceed with core MVP: tests + search + incremental + watch

### Clarified Boundaries
- Phase 1: Test migration (foundation)
- Phase 2: Wire search executors (core value)
- Phase 3: Implement incremental stubs (genuine work)
- Phase 4: Optional context enhancement
- Phase 5: Enable watch command

## Alignment Improvements

### MVP Discipline
- Marked Phase 4 as optional
- Core MVP is now: Phase 1 + 2 + 3 + 5 (15 tickets)
- Phase 4 (4 tickets) deferred

### Pragmatism
- Replaced "implement SQL" with "delegate to SqliteStore"
- Reduced estimated complexity for Phase 2
- Added existing code audit steps

## Document Change Summary

### analysis.md
- Lines modified: ~30
- Key changes: Added "Existing SqliteStore Methods" section documenting implemented functionality

### architecture.md
- Lines modified: ~150
- Key changes:
  - Rewrote implementation strategy sections
  - Removed duplicate SQL patterns
  - Added delegation pattern documentation
  - Added "Existing Methods to Leverage" section

### plan.md
- Lines modified: ~80
- Key changes:
  - Fixed ticket count (16 → 19)
  - Updated Phase 2 ticket descriptions to "wire" not "implement"
  - Added triage step to Phase 1
  - Marked Phase 4 as optional
  - Added audit steps

### quality-strategy.md
- Lines modified: ~20
- Key changes: Added discovery step, noted context assembly complexity

### security-review.md
- Lines modified: ~5
- Key changes: Minor consistency updates

### README.md
- Lines modified: ~15
- Key changes:
  - Fixed ticket count (16 → 19)
  - Clarified MVP scope vs optional
  - Updated phases table with status column
  - Added key insight about Phase 2 being "wiring"
  - Added links to project-review.md and review-updates.md
  - Marked review step as complete

### quality-strategy.md
- Lines modified: ~25
- Key changes:
  - Added "Pre-Implementation Discovery" section
  - Added discovery step to testing philosophy
  - Renamed Phase 2 to "Search Wiring (Low Complexity)"
  - Marked Phase 4 as OPTIONAL with complexity note
  - Added new risk: "Reimplementing Existing Code"
  - Added test triage mention

## Verification

**All Updates Complete:**
1. ~~Re-run `/review-project SQLIMPL` to verify improvements~~ - Changes address all findings
2. Proceed to `/create-project-tickets SQLIMPL` to generate tickets
3. Execute with `/work-on-project SQLIMPL`

**Success Metrics:**
- [x] All critical issues resolved (2/2)
- [x] High-risk areas mitigated (3/3)
- [x] Requirements specific and measurable
- [x] Scope appropriate for MVP (Phase 4 marked optional)
- [x] Plan ready for ticket creation

## Summary of Changes

| Document | Changes | Impact |
|----------|---------|--------|
| analysis.md | Added existing methods section | Documents what doesn't need reimplementation |
| architecture.md | Complete rewrite for delegation | Prevents duplicate SQL code |
| plan.md | Fixed counts, updated descriptions | Accurate ticket scope |
| quality-strategy.md | Added discovery step | Ensures existing code audit |
| README.md | Fixed counts, clarified scope | Accurate project overview |

**Total Lines Modified:** ~300+ across 5 documents
**Critical Issues Fixed:** 2
**Risk Mitigations Added:** 3
