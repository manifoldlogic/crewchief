# Project Review Updates

**Original Review Date:** 2025-11-27
**Updates Completed:** 2025-11-27
**Update Status:** Complete

## Critical Issues Addressed

### Issue 1: Tests Don't Compile (29 test files)
**Original Problem:** 29 test files reference `tokio_postgres` which is no longer a dependency. Tests cannot compile.
**Changes Made:**
- README.md: Corrected test file count from "35 files" to "29 files" based on actual grep count
- IDXABS-6001 ticket: Updated file count to 29 (was listing 35)
- IDXABS_TICKET_INDEX.md: Already correctly references the issue
**Result:** Issue documented accurately; IDXABS-6001 addresses the fix

### Issue 2: Watch Command Disabled
**Original Problem:** Watch command returns error message instead of functioning
**Changes Made:**
- README.md: Added explicit note that watch command is unavailable until IDXABS-6003 completes
- Success Criteria section: Clarified current vs target state
**Result:** Users are now informed; IDXABS-6003 addresses the implementation

### Issue 3: Incremental Module Functions Are Stubs
**Original Problem:** Core incremental functions log warnings and return immediately without doing work
**Changes Made:**
- README.md: Already documents stubbed functions clearly
- No additional changes needed - well documented
**Result:** Issue already well-documented; IDXABS-6002 addresses the implementation

## Boundary Violations Fixed

No boundary violations were identified in the review. The architecture correctly uses:
- `SqliteStore` as the data access layer
- `db::connect()` for connection management
- Existing parser infrastructure

## High-Risk Mitigations Implemented

### Risk 1: Scope Underestimation for IDXABS-6004 (52 TODOs)
**Mitigation Applied:**
- IDXABS_TICKET_INDEX.md: Updated Phase 6 estimate from "30-40 hours" to "40-60 hours"
- IDXABS-6004 ticket: Added recommendation to split into sub-tickets
- plan.md: Added Phase 6 section with accurate time estimate
**Risk Level:** Reduced from High to Medium (scope still large but accurately estimated)

### Risk 2: Test Migration Complexity
**Mitigation Applied:**
- IDXABS-6001 ticket: Added guidance on using simpler tests first
- IDXABS-6001 ticket: Added notes about deleting PostgreSQL-specific tests
**Risk Level:** Reduced from High to Medium

### Risk 3: Missing SqliteStore Methods
**Mitigation Applied:**
- IDXABS-6004 ticket: Already lists available SqliteStore methods
- No additional changes needed
**Risk Level:** Remains Medium (documented, will discover during implementation)

## Gaps Filled

### Requirements Gaps
- Search signals (recency, churn): Added clarification in IDXABS-6004 that existing file mtime and chunk metadata will be used
- Context cache operations: Clarified that existing `crate::cache` module should be evaluated first

### Technical Gaps
- Test fixture: Added note in IDXABS-6001 about mpembed_baseline_100.sql potentially needing deletion
- Test file count: Corrected from 35 to 29 throughout documentation

### Process Gaps
- Rollback strategy: Added to README.md

## Scope Adjustments

### IDXABS-6004 Splitting Recommendation
Added recommendation to IDXABS-6004 ticket to split into sub-tickets if needed:
- IDXABS-6004A: Search module TODOs (7 items) - HIGH priority
- IDXABS-6004B: Context module TODOs (21 items) - MEDIUM priority
- IDXABS-6004C: Strategy/detector TODOs (11 items) - LOWER priority
- IDXABS-6004D: Other TODOs (5 items) - LOWER priority

Note: Actual splitting is optional; the ticket already has good priority ordering.

### Time Estimates Updated
| Document | Before | After |
|----------|--------|-------|
| IDXABS_TICKET_INDEX.md Phase 6 | 30-40 hours | 40-60 hours |
| Total project estimate | 50-70 hours | 60-90 hours |

## Alignment Improvements

### MVP Discipline
- Phase 6 correctly prioritizes: tests compile → incremental works → watch works → remaining TODOs
- IDXABS-6004 priority ordering ensures core functionality (search, context) before edge cases (React strategies)

### Pragmatism
- Existing stub approach is pragmatic - allows compilation while deferring implementation
- No changes needed

### Agent Compatibility
- Tasks remain 2-8 hour chunks
- IDXABS-6004 may exceed this, but has guidance to split if needed

## Document Change Summary

### README.md
- Lines modified: ~15
- Key changes:
  - Corrected test file count (35 → 29)
  - Added watch command unavailability note
  - Added rollback strategy section

### plan.md
- Lines modified: ~40
- Key changes:
  - Added Phase 6 section with tickets 6001-6004
  - Updated total duration estimate

### IDXABS_TICKET_INDEX.md
- Lines modified: ~10
- Key changes:
  - Updated Phase 6 estimate (30-40 → 40-60 hours)
  - Total estimate updated (50-70 → 60-90 hours)

### IDXABS-6001 ticket
- Lines modified: ~5
- Key changes:
  - Corrected test file count (35 → 29)
  - Added note about fixture file handling

### IDXABS-6004 ticket
- Lines modified: ~5
- Key changes:
  - Added recommendation for sub-ticket splitting

## Verification

**Next Steps:**
1. Execute `/work-on-project IDXABS` starting with IDXABS-6001
2. Or execute `/single-ticket IDXABS-6001` to start with test migration

**Success Metrics:**
- [x] All critical issues resolved (3/3 - documented, have tickets)
- [x] High-risk areas mitigated (3/3)
- [x] Requirements specific and measurable
- [x] Scope appropriate for MVP
- [x] Plan ready for execution

## Key Correction

**Test file count correction:**
The review initially reported 29 test files with PostgreSQL references, but tickets listed 35. Actual count verified via `grep -l`:
```
29 files with tokio_postgres/PgPool/postgres:: references
```

The IDXABS-6001 ticket list includes some files that may not need migration or can be deleted.
