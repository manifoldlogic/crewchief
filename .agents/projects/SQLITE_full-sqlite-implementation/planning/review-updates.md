# Project Review Updates

**Original Review Date:** 2025-11-26
**Updates Completed:** 2025-11-26
**Update Status:** Complete

## Critical Issues Addressed

### Issue 1: Schema Migration Strategy Missing
**Original Problem:** No migration versioning system exists. Current `init_schema()` only creates tables if not exists - no versioning, no upgrade path for users with existing databases.

**Changes Made:**
- architecture.md: Added detailed "Migration System" section with versioned migration approach
- architecture.md: Added "Existing Implementation" section acknowledging current sqlite/mod.rs
- plan.md: Added new Phase -1 "Migration Infrastructure" before all other work
- plan.md: Made migration system a blocking prerequisite for schema changes

**Result:** Issue resolved - Migration system is now explicitly planned with schema_migrations table, version checking, and individual migration files.

### Issue 2: Hardcoded 1536-dim Vector Tables
**Original Problem:** Schema hardcodes `float[1536]` for vec_chunks. Architecture proposes 768-dim support but doesn't address routing or migration.

**Changes Made:**
- architecture.md: Clarified MVP uses 1536-dim only, 768-dim deferred to enhancement
- analysis.md: Removed 768-dim from initial goals, added to "Future Enhancements"
- plan.md: Removed 768-dim references from Phase 0-1, added as separate enhancement ticket

**Result:** Issue resolved - MVP scope clarified to 1536-dim only. 768-dim support is explicitly deferred to post-MVP enhancement.

## Reinvention & Duplication Addressed

### Reusable Component: normalize_for_exact_match()
**Original Problem:** Architecture proposed new exact match logic without referencing existing function.

**Changes Made:**
- architecture.md: Added "Reusable Utilities" section explicitly referencing `src/search/fts.rs::normalize_for_exact_match()`
- plan.md: Updated Semantic Ranking ticket to import existing function

**Result:** Existing function will be reused, not reimplemented.

### Reusable Component: spawn_blocking Pattern
**Original Problem:** Architecture didn't acknowledge existing async pattern.

**Changes Made:**
- architecture.md: Added note in "Existing Implementation" section about preserving `spawn_blocking` pattern
- architecture.md: Updated SqliteStore code example to show existing pattern

**Result:** Existing async pattern explicitly preserved.

### FTS Sync Strategy Inconsistency
**Original Problem:** Architecture proposed triggers but current implementation uses manual INSERT.

**Changes Made:**
- architecture.md: Chose manual INSERT approach (consistent with current implementation and PostgreSQL pattern)
- architecture.md: Removed trigger definitions, documented manual sync approach

**Result:** Consistent approach chosen - manual FTS sync via application code.

## High-Risk Mitigations Implemented

### Risk 1: sqlite-vec Extension Loading
**Mitigation Applied:**
- architecture.md: Added "Extension Verification" section with runtime check
- quality-strategy.md: Added test for extension loading verification
- plan.md: Added extension verification test to Phase 0

**Risk Level:** Reduced from High to Medium

### Risk 2: FTS5 External Content Table Synchronization
**Mitigation Applied:**
- architecture.md: Clarified manual sync approach
- quality-strategy.md: Added FTS rebuild test case

**Risk Level:** Reduced from High to Low

### Risk 3: No code_embeddings Table
**Mitigation Applied:**
- architecture.md: Confirmed code_embeddings table design with blob_sha PK
- architecture.md: Added JOIN pattern documentation
- plan.md: Phase 1 explicitly creates code_embeddings with blob_sha deduplication

**Risk Level:** Reduced from High to Medium

### Risk 4: Graph Traversal Performance
**Mitigation Applied:**
- architecture.md: Added max_depth default of 3, hard limit of 10
- quality-strategy.md: Added performance test for 100+ node graphs

**Risk Level:** Remains Medium (acceptable)

## Gaps Filled

### Requirements Gaps
- ✅ FTS column weights → Documented as using FTS5 default BM25, custom weights deferred
- ✅ Graceful degradation → Added fallback behavior: FTS-only if sqlite-vec missing
- ✅ Database file location → Clarified: path is parameter, default `~/.maproom/` is documentation only

### Technical Gaps
- ✅ Blob-to-vector conversion format → Added concrete conversion code example
- ✅ FTS5 rank normalization → Added formula: `normalized = 1 / (1 + abs(rank))`
- ✅ Connection pool and async → Explicitly documented `spawn_blocking` pattern preserved

### Process Gaps
- ✅ Migration system ticket → Added as Phase -1 (blocking prerequisite)
- ✅ Testing sqlite-vec in CI → Added to Phase 6 verification

## Scope Adjustments

### Removed from MVP
- 768-dim embedding support → Moved to "Future Enhancements" section
- Phase 5 (Code Reorganization) → Merged into other phases (do incrementally, not as separate phase)

### Clarified Boundaries
- MVP: 1536-dim only, migration system, junction table, embedding dedup, hybrid search
- Phase 1 now explicitly: schema changes + migration system (combined)
- Out of scope: 768-dim, VSCode integration, PostgreSQL parity

## Alignment Improvements

### MVP Discipline
- Reduced from 7 phases to 5 phases
- 768-dim support deferred (not needed for MVP)
- Phase 5 (reorg) merged into other work

### Pragmatism
- Chose manual FTS sync (simpler, matches PostgreSQL pattern)
- Removed trigger complexity
- Used existing async pattern instead of designing new one

## Document Change Summary

### analysis.md
- Lines modified: ~15
- Key changes: Added reusable utilities reference, clarified 768-dim as future enhancement

### architecture.md
- Lines modified: ~100
- Key changes: Added Existing Implementation section, Migration System, Reusable Utilities, Extension Verification, removed triggers

### plan.md
- Lines modified: ~50
- Key changes: Added Phase -1 for migration, removed Phase 5 (merged), deferred 768-dim, updated phase numbering

### quality-strategy.md
- Lines modified: ~25
- Key changes: Added extension verification test, migration upgrade test, FTS rebuild test

### security-review.md
- Lines modified: ~20
- Key changes: Updated extension loading section to reference bundled sqlite-vec, added graceful degradation

## All Documents Updated Summary

| Document | Lines Changed | Key Updates |
|----------|---------------|-------------|
| architecture.md | ~100 | Migration system, reusable utilities, extension verification |
| analysis.md | ~20 | Reusable utilities section, future enhancements section |
| plan.md | ~60 | Phase -1 added, Phase 5 removed, 768-dim deferred |
| quality-strategy.md | ~30 | Migration tests, extension tests, FTS rebuild tests |
| security-review.md | ~20 | Bundled extension, graceful degradation |

## Verification

**Completed Actions:**
1. ✅ Created review-updates.md tracking document
2. ✅ Fixed Critical Issue 1: Schema Migration Strategy
3. ✅ Fixed Critical Issue 2: Hardcoded 1536-dim (deferred to post-MVP)
4. ✅ Addressed reinvention concerns (reusable utilities documented)
5. ✅ Mitigated high-risk areas (extension verification, graceful degradation)
6. ✅ Filled gaps and ambiguities (FTS sync strategy, blob-vector conversion)
7. ✅ Optimized scope for MVP (removed Phase 5, deferred 768-dim)

**Next Steps:**
1. Re-run `/review-project SQLITE` to verify improvements
2. Address any remaining issues
3. Proceed to `/create-project-tickets SQLITE` if review passes

**Success Metrics:**
- [x] All critical issues resolved
- [x] High-risk areas mitigated
- [x] Requirements specific and measurable
- [x] Scope appropriate for MVP
- [x] Plan ready for ticket creation

---

## Feedback-Based Updates (Round 2)

**Date:** 2025-11-26

### Issues Fixed

1. **Phase Numbering Mismatch** - Renumbered all phases:
   - Old Phase -1 → Phase 0 (Migration Infrastructure)
   - All subsequent phases shifted by 1
   - Summary (README.md) and detailed plan (plan.md) now aligned

2. **Removed Code Organization Phase** - Removed from summary and plan
   - Code reorganization noted as "do incrementally within each phase"
   - No longer appears in phase list

3. **Added Timeline Estimates** - All phases now have estimates:
   - Phase 0: 1-2d, Phase 1: 1-2d, Phase 2: 2-3d, Phase 3: 2-3d
   - Phase 4: 3-4d, Phase 5: 2-3d, Phase 6: 2-3d
   - Total: 14-20 days

4. **Added Known Limitations Section** - Both README.md and plan.md now include:
   - 1536-dim embeddings only (768-dim deferred)
   - No database encryption
   - Single-user only
   - No PostgreSQL migration path

5. **Expanded Success Criteria** - Added:
   - Specific critical test commands
   - Manual verification steps including embedding dedup verification
   - WAL recovery verification

6. **Added File-Based Integration Test** - In quality-strategy.md:
   - Required test using real temp file (not `:memory:`)
   - Tests file permissions, WAL handling, path edge cases (spaces, unicode)
   - Added to critical path coverage table

### Documents Modified

| Document | Changes |
|----------|---------|
| README.md | Phase numbering, timeline estimates, known limitations, expanded success criteria |
| plan.md | Phase renumbering (0-6), time estimates, known limitations, expanded criteria |
| quality-strategy.md | File-based integration test, critical path coverage |
| review-updates.md | This section documenting round 2 updates |
