# Project Review: UNIWATCH - Unified Watch Command

**Review Date:** 2025-01-28
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

The UNIWATCH project is well-conceived and addresses a real problem: the `maproom watch` command currently sets `worktree_id` at startup and never updates it, causing files to be indexed to the wrong worktree after `git checkout`. The proposed solution (adding `.git/HEAD` file watching to detect branch switches) is architecturally sound and appropriately scoped.

However, several critical findings require attention before ticket creation:

1. **Key components exist but are not exported**: The `setup_head_watcher()`, `DebouncedHandler`, and `BranchSwitchEvent` structures exist in `indexer/mod.rs` but are marked `#[allow(dead_code)]` and not exported publicly. They must be made `pub` before use from `main.rs`.

2. **Tests exist but are disabled**: Multiple UNIWATCH-prefixed tests (UNIWATCH-1002 through UNIWATCH-3002) exist in `indexer/mod.rs` but are marked `#[cfg(disabled_postgresql_test)]` due to the SQLite migration. These tests reference the old `handle_branch_switch` function that was removed.

3. **Architecture document references non-existent function**: The `handle_branch_switch()` function described in architecture.md was removed during the IDXABS SQLite migration. It needs to be reimplemented using `SqliteStore` instead of PostgreSQL's `PgPool`.

4. **E2E test references PostgreSQL**: The `test_unified_watch_workflow.sh` script assumes PostgreSQL database connectivity, but the codebase has migrated to SQLite.

The project is fundamentally sound but the planning documents need updates to reflect the current SQLite-only architecture.

## Critical Issues (Blockers)

### Issue 1: Components Not Exported

**Severity:** Critical
**Category:** Architecture
**Description:** The planning documents reference `setup_head_watcher()`, `DebouncedHandler`, and `BranchSwitchEvent` as "existing components to reuse" at specific line numbers. However, all three are marked `#[allow(dead_code)]` and are private to the `indexer` module:

```rust
#[allow(dead_code)] // Used in UNIWATCH-1004 for branch switch debouncing
struct DebouncedHandler { ... }

#[allow(dead_code)] // Used in tests; will be used when watch_worktree is reimplemented
struct BranchSwitchEvent { ... }

#[allow(dead_code)] // Used in tests; will be used when watch_worktree is reimplemented
fn setup_head_watcher(...) { ... }
```

**Impact:** Implementation will fail immediately when trying to import these components.

**Required Action:**
1. Export components by adding `pub` visibility
2. Update `mod.rs` to re-export: `pub use setup_head_watcher`, `pub use DebouncedHandler`, `pub use BranchSwitchEvent`
3. Remove `#[allow(dead_code)]` annotations

**Documents Affected:** architecture.md (line numbers may shift), README.md

### Issue 2: handle_branch_switch Function Removed

**Severity:** Critical
**Category:** Architecture
**Description:** The architecture.md document (section "Branch Switch Handler", lines 125-167) describes implementing `handle_branch_switch()`. However, the code at `indexer/mod.rs:702-705` states:

```rust
// NOTE: watch_worktree, handle_branch_switch, get_file_id_by_path, and get_file_id_by_worktree_id
// functions have been removed as part of IDXABS-2001 (SQLite-only migration).
// They depended on PostgreSQL's PgPool and will be reimplemented in IDXABS-2006
// (Refactor Incremental Module) with SqliteStore support.
```

**Impact:** The plan assumes this function exists and just needs to be called, but it must be implemented from scratch using `SqliteStore`.

**Required Action:**
1. Update architecture.md to indicate this is NEW code, not reuse
2. Add implementation details for SQLite-based `handle_branch_switch()`
3. Consider moving this to a new file or keeping inline in `main.rs`

**Documents Affected:** architecture.md, plan.md

### Issue 3: Disabled Unit Tests

**Severity:** High
**Category:** Testing
**Description:** Multiple UNIWATCH-prefixed tests exist in `indexer/mod.rs` but are disabled:
- `test_worktree_tracking_state_initialization` (UNIWATCH-1002) - `#[cfg(disabled_postgresql_test)]`
- `test_debounced_handler_prevents_rapid_events` (UNIWATCH-1003) - Active (works without DB)
- `test_handle_branch_switch_updates_state` (UNIWATCH-2001) - `#[cfg(disabled_postgresql_test)]`
- `test_handle_branch_switch_skips_if_same_branch` (UNIWATCH-2001) - `#[cfg(disabled_postgresql_test)]`
- `test_branch_switch_event_serialization` (UNIWATCH-2002) - Active (works without DB)
- `test_dual_watchers_initialize` (UNIWATCH-3001) - Active (works without DB)
- `test_event_loop_handles_both_file_and_head_events` (UNIWATCH-3002) - `#[cfg(disabled_postgresql_test)]`

**Impact:** Test coverage will be incomplete. Some tests need rewriting for SQLite.

**Required Action:**
1. Update quality-strategy.md to acknowledge disabled tests
2. Include enabling/rewriting these tests in Phase 4 (Testing)
3. Add SQLite-compatible test fixtures

**Documents Affected:** quality-strategy.md, plan.md

## High-Risk Areas (Warnings)

### Risk 1: E2E Test Uses PostgreSQL

**Risk Level:** High
**Category:** Testing
**Description:** The `tests/e2e/test_unified_watch_workflow.sh` script contains PostgreSQL-specific code:

```bash
DB_URL="${MAPROOM_DATABASE_URL:-postgresql://maproom:maproom@localhost:5432/maproom}"
psql "$DB_URL" -c "DELETE FROM maproom.repos WHERE name='$REPO_NAME'"
```

**Probability:** High
**Impact:** Medium (E2E tests will fail or produce misleading results)

**Mitigation:**
1. Update E2E script to use SQLite
2. Replace `psql` commands with `sqlite3` or use maproom CLI commands
3. Update cleanup logic for SQLite database location (`~/.maproom/maproom.db`)

### Risk 2: Race Condition Between File and Branch Events

**Risk Level:** Medium
**Category:** Technical
**Description:** The analysis.md correctly identifies this risk but the plan lacks concrete mitigation strategy. When a branch switch and file modification happen simultaneously:
- Which worktree_id is used?
- What happens to queued file events during the branch switch?

**Probability:** Medium
**Impact:** High (data corruption - files indexed to wrong worktree)

**Mitigation:**
1. Add explicit event ordering logic
2. Process queued file events BEFORE updating worktree_id
3. Clear file event queue after branch switch (new branch may have different files)
4. Add integration test for this scenario (currently listed as Test 3 but lacks implementation detail)

### Risk 3: Detached HEAD State Handling

**Risk Level:** Medium
**Category:** Technical
**Description:** The plan mentions handling detached HEAD state ("Use short commit SHA as branch name") but doesn't specify implementation details. Questions:
- What constitutes a "short" SHA? (7 chars? 8 chars?)
- How does this interact with existing worktrees?
- Should a new worktree be created for each checkout in detached state?

**Probability:** Medium
**Impact:** Medium (confusing behavior, potential database bloat)

**Mitigation:**
1. Define exact detached HEAD behavior in architecture.md
2. Use `git rev-parse --short=8 HEAD` for 8-character SHA
3. Consider NOT creating worktree records for detached HEAD (index to parent branch)
4. Add explicit test case for detached HEAD

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds
None identified. The project correctly plans to reuse existing components.

### Boundary Violations
None identified. The plan correctly uses library imports for utility components within the same crate.

### Missed Reuse Opportunities

**Available Component:** `Arc<RwLock<>>` pattern
**Could Solve:** Dynamic state tracking
**Integration Method:** Library import
**Integration Effort:** Low
**Recommendation:** Already identified. The codebase extensively uses this pattern (14+ usages in `config/hot_reload.rs`, `cache/system.rs`, `embedding/cache.rs`). Good architectural alignment.

### Pattern Violations
None identified. The proposed `Arc<RwLock<i64>>` for `worktree_id` matches existing patterns.

### Inappropriate Coupling
None identified. The design appropriately keeps the branch switch handling in `main.rs` where the event loop lives.

## Gaps & Ambiguities

### Requirements Gaps

1. **Undefined: NDJSON Event Destination**
   - BranchSwitchEvent is printed to stdout (`println!`)
   - Is this correct for VSCode extension consumption?
   - Should it be stderr to separate from regular output?
   - Impact: VSCode integration may not work as expected
   - Suggested clarification: Confirm NDJSON should go to stdout

2. **Undefined: Error Recovery Strategy**
   - What happens if `get_or_create_worktree()` fails during branch switch?
   - Should we retry? Skip? Continue with old worktree_id?
   - Impact: Watch command may hang or crash on database errors
   - Suggested clarification: Add explicit error handling flow

### Technical Gaps

1. **Missing: Database Method Verification**
   - Plan assumes `store.get_or_create_worktree()` exists - Verified: EXISTS
   - Plan assumes `store.get_repo_id()` exists - Needs verification
   - Impact: Low (likely exists, but should verify)
   - Required: Grep for `get_repo_id` in SqliteStore

2. **Missing: tokio::sync vs std::sync**
   - Architecture shows `Arc<RwLock<i64>>` but doesn't specify which RwLock
   - `tokio::sync::RwLock` is async-friendly
   - `std::sync::RwLock` works but can block tokio runtime
   - Impact: Potential deadlocks if wrong choice made
   - Required: Specify `std::sync::RwLock` (current pattern in codebase)

### Process Gaps

1. **Undefined: Agent Handoff for Testing**
   - Phase 4 assigns `integration-tester` but disabled tests need `rust-indexer-engineer` to fix first
   - Impact: Testing phase may be blocked
   - Suggested: Add sub-task to Phase 4 for test enabling

## Scope & Feasibility Concerns

### Scope Creep Indicators
None identified. The project is well-scoped:
- Single feature: runtime branch detection
- ~60 lines of code changes (per plan.md)
- No new commands, no schema changes
- Clear out-of-scope items defined

### Feasibility Challenges

1. **Technical: Watcher Cleanup**
   - The `setup_head_watcher()` returns a `notify::RecommendedWatcher` that must be kept alive
   - If dropped, the watcher stops working
   - The current signature doesn't allow returning the watcher to main.rs
   - **Solution**: Modify return type or keep watcher in scope

2. **Technical: Channel Capacity**
   - The existing `setup_head_watcher()` uses a channel with capacity 10
   - This may be insufficient for rapid branch switches
   - The debouncer handles this at the application level (good)
   - **Assessment**: Acceptable, no change needed

## Alignment Assessment

### MVP Discipline
**Rating:** Strong
- Single feature, clearly defined
- Preserves backward compatibility
- No unnecessary additions (e.g., no new CLI flags, no UI changes)
- Phase 1 delivers complete value

### Pragmatism Score
**Rating:** Strong
- Reuses existing components where available
- Leverages existing `tokio::select!` loop
- Simple state management with Arc<RwLock>
- No over-engineering (no event bus, no complex state machines)

### Agent Compatibility
**Rating:** Adequate
- Tasks are appropriately sized (2-4 hours each)
- Clear acceptance criteria
- **Concern**: Disabled tests may confuse verify-ticket agent
- **Concern**: Dependencies between phases may not be clear enough

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [ ] Plan reflects current codebase state (components need export, function removed)
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed (N/A - no new security surface)
- [x] Dependencies on existing systems documented

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [ ] Integration points are well-defined (need to export components)
- [x] Performance requirements are clear (< 2 second detection)
- [ ] Error handling is specified (needs more detail)
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [ ] Rollback plan exists (not specified)
- [x] Integration with existing workflows considered

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified
- [x] Integration points with existing systems mapped
- [x] No reinvention of available functionality
- [x] Proper integration methods chosen (library imports within same crate)
- [x] Component boundaries respected
- [x] Public interfaces used (once exported)
- [x] Appropriate coupling levels maintained

### Tickets (if they exist)
- N/A - Pre-ticket review

### Risk
- [x] Major risks are identified
- [x] Mitigation strategies exist (in analysis.md)
- [ ] Dependencies have fallbacks (database failure not addressed)
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Update architecture.md to reflect current state**
   - Change `setup_head_watcher()` section to note it needs `pub` export
   - Change `handle_branch_switch()` section to note it's NEW implementation (not reuse)
   - Add note that the function was removed in IDXABS-2001 SQLite migration

2. **Update plan.md Phase 1 tasks**
   - Add explicit task: "Export DebouncedHandler, BranchSwitchEvent, setup_head_watcher from indexer module"
   - Add explicit task: "Remove #[allow(dead_code)] annotations from exported items"

3. **Update quality-strategy.md**
   - Acknowledge disabled tests (UNIWATCH-1002, 2001, 3002)
   - Add task to Phase 4: "Enable/rewrite PostgreSQL tests for SQLite"
   - Update E2E script references from PostgreSQL to SQLite

### Phase 1 Adjustments

- Add explicit task for module exports (currently implicit)
- Specify exact RwLock type (`std::sync::RwLock`)
- Add error handling specification for database failures

### Risk Mitigations

1. **Race Condition**: Add integration test that explicitly tests concurrent branch switch + file modification
2. **Watcher Lifetime**: Ensure watcher variable is stored appropriately to prevent early drop
3. **Database Errors**: Add retry logic or graceful degradation when SqliteStore operations fail

### Documentation Updates

- **architecture.md**: Update component availability section (lines 240-248) to reflect current state
- **plan.md**: Add Phase 0.5 for module exports
- **quality-strategy.md**: Add section on disabled test status
- **README.md**: Update "Existing Components to Reuse" table with export requirement

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

The core design is sound and well-aligned with MVP principles. However, the planning documents contain outdated assumptions about the codebase state post-SQLite migration.

**Primary concerns:**
1. Planning docs reference components that exist but aren't exported
2. Key function (`handle_branch_switch`) was removed and needs reimplementation
3. Several unit tests are disabled and need to be re-enabled for SQLite

### Recommended Path Forward

**REVISE THEN PROCEED:** Address the three critical issues (component exports, removed function, disabled tests) in the planning documents before creating tickets. This is ~30 minutes of documentation updates.

### Success Probability
Given current state: 70%
After recommended changes: 90%

### Final Notes

This is a well-conceived project with appropriate scope. The core architecture is correct and the implementation approach (adding a branch to `tokio::select!` for HEAD events) is the right solution. The issues identified are documentation/planning gaps rather than fundamental design problems.

The existing test infrastructure (UNIWATCH-prefixed tests) shows good forethought - these tests just need to be updated for SQLite and re-enabled. The presence of these tests suggests someone already did significant design work on this feature.

Recommendation: Update planning documents to reflect current codebase state, then proceed with ticket creation. Estimated effort for updates: 30 minutes.
