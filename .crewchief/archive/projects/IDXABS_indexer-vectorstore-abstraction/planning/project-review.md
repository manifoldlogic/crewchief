# Project Review: IDXABS - SQLite-Only Migration (Completion Phase)

**Review Date:** 2025-11-27
**Project Status:** Needs Work
**Overall Risk:** Medium

## Executive Summary

The IDXABS project has made significant progress in removing PostgreSQL and simplifying to SQLite-only. The main crate now **compiles successfully** (87 warnings, no errors). However, the project was prematurely archived with substantial work remaining:

1. **29 test files** still reference PostgreSQL dependencies and **cannot compile**
2. **52 TODO stubs** across 21 source files return placeholder/empty values
3. **Watch command** is explicitly disabled with an error message
4. **Incremental indexing** is stubbed (core functions log warnings and return immediately)

The Phase 6 tickets (6001-6004) correctly identify this remaining work. The project planning is sound, but the estimated effort (30-40 hours for Phase 6) may be optimistic given the scope of the 52 TODO stubs spanning search, context, and incremental modules.

## Critical Issues (Blockers)

### Issue 1: Tests Don't Compile
**Severity:** Critical
**Category:** Execution
**Description:** 29 test files in `crates/maproom/tests/` reference `tokio_postgres`, `PgPool`, or `postgres::` which are no longer available as dependencies. The tests fail to compile:
```
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `tokio_postgres`
```
**Impact:** Cannot run `cargo test -p crewchief-maproom` - no automated validation of changes
**Required Action:** Complete IDXABS-6001 to migrate all 29 test files to SQLite
**Documents Affected:** None - tickets already address this

### Issue 2: Watch Command Disabled
**Severity:** Critical
**Category:** Requirements
**Description:** The watch command in `main.rs:981-989` explicitly returns an error:
```rust
anyhow::bail!(
    "Watch command is temporarily unavailable.\n\
    The watch_worktree function was removed during SQLite-only migration..."
);
```
**Impact:** Users cannot use file watching for incremental indexing - a documented feature
**Required Action:** Complete IDXABS-6003 after IDXABS-6002 (incremental module)
**Documents Affected:** README.md currently shows watch as working, which is incorrect

### Issue 3: Incremental Module Functions Are Stubs
**Severity:** Critical
**Category:** Requirements
**Description:** Core incremental functions in `processor.rs`, `detector.rs`, `edge_updater.rs`, and `tree_sha_update.rs` are stubbed with TODO comments and either:
- Log warnings and return `Ok(())` without doing work
- Return `Ok(None)` or empty collections
- Return hardcoded values (e.g., `UpdateStats::skipped()`)

**Impact:** Even if watch command is implemented, incremental updates won't actually index files
**Required Action:** Complete IDXABS-6002 to implement all incremental functions
**Documents Affected:** None - tickets already address this

## Reinvention & Duplication Analysis

### No Unnecessary Rebuilds Detected
The project correctly uses existing SqliteStore methods. The TODO stubs reference methods that need implementation, not duplication of existing functionality.

### Boundary Violations: None
The architecture correctly uses:
- `SqliteStore` as the data access layer
- `db::connect()` for connection management
- Existing parser infrastructure in `indexer/parser.rs`

### Pattern Consistency
The existing code follows established patterns:
- All modules use `&SqliteStore` or `Arc<SqliteStore>`
- Error handling via `anyhow::Result`
- Async functions with `tokio`
- Tracing for logging

## High-Risk Areas (Warnings)

### Risk 1: Scope Underestimation for TODO Resolution
**Risk Level:** High
**Category:** Execution
**Description:** The 52 TODO stubs span multiple functional areas:
- **Search module (7 TODOs)**: signals, graph, fts, vector, pipeline - affects search result quality
- **Context module (21 TODOs)**: cache, graph, assembler, detectors - affects context assembly
- **Incremental module (13 TODOs)**: processor, detector, edge_updater - covered by IDXABS-6002
- **Strategies (6 TODOs)**: rust, python, react - affects language-specific context
- **Other (5 TODOs)**: migrate, embedding, db/sqlite

**Probability:** High - 52 stubs will take significant time
**Impact:** High - many TODOs affect core functionality
**Mitigation:**
1. Prioritize by user impact (search > context > strategies)
2. Consider splitting IDXABS-6004 into sub-tickets by module
3. Update estimate from 30-40 hours to 40-60 hours

### Risk 2: Test Migration Complexity
**Risk Level:** High
**Category:** Execution
**Description:** 29 test files need migration from PostgreSQL to SQLite. Some tests may:
- Test PostgreSQL-specific behavior that doesn't apply
- Need significant rewriting (not just s/&Client/&SqliteStore/)
- Reveal missing SqliteStore methods

**Probability:** Medium
**Impact:** Medium
**Mitigation:**
1. Start with simpler tests (index_state, cleanup) to establish patterns
2. Delete tests that only validated PostgreSQL-specific behavior
3. Document any tests that expose missing SqliteStore methods

### Risk 3: Missing SqliteStore Methods for Search/Context
**Risk Level:** Medium
**Category:** Technical
**Description:** Several TODO comments indicate they need SqliteStore methods that may not exist:
```rust
// TODO: Implement using SqliteStore methods in IDXABS-4001
```
The existing SqliteStore has graph methods (`find_callers`, `find_callees`, `find_imports`) but some search/context operations may need additional methods.

**Probability:** Medium
**Impact:** Medium - may extend scope
**Mitigation:** Audit SqliteStore methods against TODO requirements before starting

## Gaps & Ambiguities

### Requirements Gaps
1. **Search signals (recency, churn)** - TODOs don't specify data source
   - Where does recency data come from? Git history? File mtime?
   - Where does churn data come from? Git blame?
   - Suggested: Document data sources in IDXABS-6004

2. **Context cache operations** - 8 TODOs in cache.rs
   - What caching strategy should be used? LRU? TTL?
   - Should this use the existing `crate::cache` module?
   - Suggested: Clarify if new caching is needed or existing works

### Technical Gaps
1. **Test fixture location** - Ticket 6001 lists `tests/fixtures/mpembed_baseline_100.sql` but this appears to be a PostgreSQL SQL file that may need conversion or deletion

2. **Batch operations in SQLite** - Several incremental operations need batch processing but SQLite has different transaction semantics than PostgreSQL

### Process Gaps
None - the ticket workflow is well-defined

## Scope & Feasibility Concerns

### Scope Assessment
The scope is appropriate:
- Phase 6 correctly identifies the remaining work
- Tickets are well-structured with clear acceptance criteria
- No scope creep beyond completing the original migration

### Feasibility Challenges
1. **Effort estimation** - 30-40 hours may be optimistic for 52 TODOs + 29 test migrations
2. **Test validation** - Without working tests, it's hard to verify implementations

## Alignment Assessment

### MVP Discipline
**Rating:** Adequate
- Focus on completing the migration (good)
- No new features being added (good)
- Phase 6 correctly prioritizes: tests compile → incremental works → watch works
- Some TODOs (context strategies) could potentially be deferred if not critical

### Pragmatism Score
**Rating:** Strong
- Deleting PostgreSQL rather than maintaining two backends (excellent)
- Simple `db::connect()` API (excellent)
- TODO stubs allow compilation while deferring implementation (pragmatic)

### Agent Compatibility
**Rating:** Strong
- Tasks are 2-8 hour chunks (appropriate for IDXABS-6001, 6002, 6003)
- IDXABS-6004 may be too large (52 TODOs) - consider splitting
- Clear acceptance criteria exist
- Single agent assignment (rust-indexer-engineer) is appropriate

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed (existing review valid)
- [x] Dependencies on existing systems documented

### Technical
- [x] Technology choices are appropriate (SQLite + existing Rust ecosystem)
- [x] Dependencies are identified and available
- [x] Integration points are well-defined
- [ ] **Performance requirements are clear** - some TODOs affect search ranking
- [x] Error handling is specified
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [ ] **Rollback plan exists** - add git revert strategy to README
- [x] Integration with existing workflows considered

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified (SqliteStore methods)
- [x] Integration points with existing systems mapped
- [x] No reinvention of available functionality
- [x] Proper integration methods chosen (library imports for same crate)
- [x] Component boundaries respected
- [x] Public interfaces used

### Tickets
- [x] Tickets align with plan objectives
- [x] All plan deliverables have corresponding tickets
- [x] Dependencies are properly sequenced (6001 → 6002 → 6003)
- [ ] **Scope per ticket is appropriate** - IDXABS-6004 may be too large
- [x] Acceptance criteria are measurable

### Risk
- [x] Major risks are identified
- [ ] **Mitigation strategies exist** - need strategy for large scope
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Resuming Work)
1. **Split IDXABS-6004** into module-specific tickets:
   - IDXABS-6004A: Search module TODOs (7 items)
   - IDXABS-6004B: Context module TODOs (21 items)
   - IDXABS-6004C: Strategy/detector TODOs (11 items)
   - IDXABS-6004D: Other TODOs (5 items)

2. **Update README.md** to indicate watch command is unavailable until 6003 completes

3. **Audit SqliteStore methods** against TODO requirements to identify any missing methods upfront

### Phase 6 Execution Order (Validated)
The proposed order is correct:
1. **IDXABS-6001** - Tests compile (enables validation)
2. **IDXABS-6002** - Incremental module implemented (core functionality)
3. **IDXABS-6003** - Watch command works (user-facing feature)
4. **IDXABS-6004** - All TODOs resolved (complete functionality)

### Risk Mitigations
1. **For large scope**: Prioritize search module TODOs first (directly impacts users)
2. **For test migration**: Create reusable test helpers early in 6001
3. **For missing methods**: Add methods to SqliteStore as discovered, not all upfront

### Documentation Updates
- **README.md**: Note watch command unavailable
- **review-updates.md**: Record this review's findings
- **plan.md**: Consider increasing Phase 6 estimate to 40-60 hours

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

**Primary concerns:**
1. IDXABS-6004 scope is very large (52 TODOs) - recommend splitting
2. Effort estimate (30-40 hours) may be 50% low
3. Tests cannot run until IDXABS-6001 completes, limiting early validation

### Recommended Path Forward

**PROCEED:** Project is well-planned and the remaining work is clearly identified. The Phase 6 tickets correctly capture the work that was left incomplete.

Recommended modifications:
1. Split IDXABS-6004 into smaller tickets (optional but helpful)
2. Update time estimate to 40-60 hours for Phase 6
3. Update README to reflect current state (watch unavailable)

### Success Probability
Given current state: 75%
After recommended changes: 85%

### Final Notes
The project demonstrates good engineering decisions (deleting PostgreSQL rather than maintaining abstraction, using stubs to enable compilation while deferring work). The main issue is that it was archived prematurely. The Phase 6 tickets provide a solid path to completion.

The 52 TODO stubs are well-documented with ticket references (IDXABS-2003, IDXABS-4001) making them easy to find and track. The critical path (6001 → 6002 → 6003) is correctly identified.

Key metric to track: **TODO count reduction** - each ticket should reduce the `grep -r "TODO" crates/maproom/src | wc -l` count measurably.
