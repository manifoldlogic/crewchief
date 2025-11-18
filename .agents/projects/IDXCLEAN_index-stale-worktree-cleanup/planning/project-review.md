# Project Review: IDXCLEAN - Index Stale Worktree Cleanup

**Review Date**: 2025-11-18
**Reviewer**: Senior Technical Architect
**Review Type**: Pre-Ticket Comprehensive Critical Review
**Project Phase**: Planning Complete → Ready for Ticket Creation

---

## Executive Summary

### Overall Assessment: **APPROVED WITH RECOMMENDATIONS** ✅

The IDXCLEAN project planning is **thorough, well-researched, and execution-ready**. The architecture demonstrates strong MVP thinking with safety-first design. Watch integration strategy is exceptionally well-analyzed with clear trade-offs documented.

**Key Strengths:**
- ✅ MVP-focused scope (manual CLI first, watch integration optional)
- ✅ Comprehensive safety mechanisms (dry-run default, transactions, audit logging)
- ✅ Excellent watch integration analysis with 4 approaches evaluated
- ✅ Clear acceptance criteria and testable deliverables
- ✅ No reinvention detected - genuinely new functionality

**Critical Recommendations:**
- ⚠️ **MUST address**: Integration with existing `Commands::Watch` in main.rs (not documented)
- ⚠️ **MUST clarify**: Relationship to existing `incremental/` module functionality
- ⚠️ **SHOULD enhance**: CLI subcommand structure (`db cleanup-stale` vs. top-level command)

**Risk Assessment**: **Medium** (data deletion) → **Low** (after mitigations applied)

**Confidence Level**: **85%** (high confidence with noted clarifications needed)

---

## Detailed Review

### 1. Codebase Integration Analysis

#### 1.1 Existing Code Patterns ✅

**Database Module** (`crates/maproom/src/db/`):
- ✅ Existing files: `columns.rs`, `connection.rs`, `index_state.rs`, `materialized_views.rs`, `mod.rs`, `pool.rs`, `queries.rs`
- ✅ Proposed `cleanup.rs` does NOT conflict - this is a new module
- ✅ Pattern matches existing structure (feature-based files)
- ✅ `mod.rs` exports functionality via `pub use` - cleanup will follow same pattern

**Incremental Module** (`crates/maproom/src/incremental/`):
- ⚠️ **CRITICAL**: Existing `remove_worktree_from_chunks()` function found in `tree_sha_update.rs:63`
- ⚠️ This function handles worktree removal from chunks' `worktree_ids` JSONB arrays
- ⚠️ Planning docs do NOT mention this existing functionality
- ⚠️ **CONCERN**: Potential overlap between proposed cleanup and existing incremental deletion logic

**Evidence from codebase**:
```rust
// crates/maproom/src/incremental/tree_sha_update.rs:63
pub use tree_sha_update::{incremental_update, remove_worktree_from_chunks};
```

**Search results show**:
- `incremental_deletions.rs` test: "Garbage collection: deletes chunks with empty worktree_ids arrays"
- `tree_sha_update.rs:95`: "2. Deletes chunks that have empty `worktree_ids` arrays (garbage collection)"
- `tree_sha_update.rs:161`: "Deleted orphan chunks with no worktrees"

**Recommendation**:
1. Clarify relationship between `cleanup.rs` and `tree_sha_update.rs::remove_worktree_from_chunks`
2. Ensure no duplication of garbage collection logic
3. Consider reusing existing `remove_worktree_from_chunks` if applicable

#### 1.2 CLI Command Structure ⚠️

**Existing CLI Structure** (`crates/maproom/src/main.rs`):
```rust
enum Commands {
    Db { #[command(subcommand)] command: DbCommand },
    Cache { #[command(subcommand)] command: CacheCommand },
    Scan { ... },
    Upsert { ... },
    Watch { ... },
    Search { ... },
    Status { ... },
    GenerateEmbeddings { ... },
    Migrate { ... },
}
```

**Proposed Command**: `maproom db cleanup-stale`

**Analysis**:
- ✅ Follows existing pattern (Db subcommand with nested commands)
- ✅ Consistent with `Cache` subcommand structure
- ⚠️ Planning docs do NOT show `DbCommand` enum extension
- ⚠️ Planning docs do NOT show implementation in `main.rs` match arms

**Current DbCommand enum**:
```rust
enum DbCommand {
    Migrate,  // Only one command currently!
}
```

**Required changes NOT documented**:
1. Add `CleanupStale` variant to `DbCommand`
2. Add match arm in `main()` to handle `DbCommand::CleanupStale`
3. Wire up to `cleanup::find_stale_worktrees()` and `cleanup::delete_stale_worktrees()`

**Recommendation**: Phase 2 tickets should explicitly include main.rs integration tasks.

#### 1.3 Watch Integration with Existing Watch Command ⚠️ **CRITICAL**

**Existing Watch Implementation** (`main.rs:778`):
```rust
Commands::Watch { repo, worktree, path, throttle } => {
    // Auto-detects branch, watches for file changes
    // Emits NDJSON events (including branch_switched events)
    // Uses incremental/ module for change detection
    // ...existing implementation...
}
```

**Planning Documents Assumption**:
- Architecture.md assumes watch cleanup will be integrated into watch command
- Documents discuss "startup cleanup" and "periodic cleanup" as background tasks
- **BUT**: No analysis of how to integrate with existing Watch command implementation

**Critical Gap**:
- ❌ No analysis of existing Watch command structure
- ❌ No investigation of `incremental/worktree_watcher.rs` or `incremental/multi_watcher.rs`
- ❌ No understanding of current watch event loop and where cleanup would hook in
- ❌ No consideration of how NDJSON event stream affects cleanup logging

**Existing Watch Infrastructure**:
```rust
// From incremental/mod.rs exports:
pub use worktree_watcher::{WatcherStatus, WorktreeWatcher};
pub use multi_watcher::MultiWatcher;
```

**Evidence from CLAUDE.md**:
```
## Watch Command (Unified)

The `watch` command provides unified file and branch watching:
- Auto-detects the current branch
- Watches for file changes (incremental indexing)
- Detects branch switches and automatically re-indexes
- Emits NDJSON events to stdout for integration with tools
```

**Recommendation**:
1. **MUST**: Add analysis of existing Watch command implementation to architecture.md
2. **MUST**: Identify specific integration points in `worktree_watcher.rs` or `multi_watcher.rs`
3. **MUST**: Clarify whether cleanup runs in:
   - Same event loop as file watcher?
   - Separate tokio task?
   - Integrated into `WorktreeWatcher` struct?
4. **SHOULD**: Investigate if existing watch infrastructure already has cleanup hooks

### 2. Architecture Review

#### 2.1 Component Design ✅

**Component 1: Stale Detection** - **EXCELLENT**
- ✅ Clear responsibility: Identify worktrees with non-existent `abs_path`
- ✅ Parallel validation using `tokio::fs::try_exists` (correct async pattern)
- ✅ Rich metadata returned (id, name, path, chunk_count)
- ✅ Error handling: permission denied treated as "exists" (safe assumption)

**Component 2: Safe Deletion** - **EXCELLENT**
- ✅ Transaction-based with ACID guarantees
- ✅ CASCADE leverages database foreign keys (no manual chunk deletion needed)
- ✅ Dry-run mode for inspection
- ✅ Audit logging for every deletion

**Component 3: CLI Interface** - **GOOD** (see CLI structure concerns above)
- ✅ Dry-run default with explicit `--confirm` flag
- ✅ Clear output format
- ✅ User-friendly error messages
- ⚠️ Integration with `main.rs` not fully documented

**Component 4: Watch Integration** - **EXCEPTIONAL ANALYSIS** ✅
- ✅ Four approaches evaluated (startup, periodic, idle-time, post-indexing)
- ✅ Hybrid approach recommended with clear rationale
- ✅ Priority-based async execution clearly designed
- ✅ Performance impact analyzed (<200ms startup, <500ms periodic)
- ⚠️ **BUT**: Assumes watch infrastructure that needs verification (see 1.3)

#### 2.2 Database Schema ✅

**Foreign Key Constraints**:
```sql
-- From migration 0001_init.sql (inferred from queries.rs)
ALTER TABLE worktrees ADD CONSTRAINT worktrees_repo_id_fkey
    FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE;
ALTER TABLE files ADD CONSTRAINT files_worktree_id_fkey
    FOREIGN KEY (worktree_id) REFERENCES worktrees(id) ON DELETE CASCADE;
ALTER TABLE chunks ADD CONSTRAINT chunks_file_id_fkey
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE;
```

**Analysis**:
- ✅ CASCADE properly configured for worktree deletion
- ✅ Single DELETE on worktrees will cascade to files and chunks
- ✅ No orphaned data concerns
- ⚠️ **CONCERN**: Migration 0020 added `worktree_ids JSONB` column to chunks
  - This allows chunks to belong to MULTIPLE worktrees
  - DELETE on worktrees table will CASCADE delete chunks EVEN IF chunk belongs to other worktrees
  - **CRITICAL**: This may cause data loss if chunks are shared across worktrees!

**Evidence**:
```sql
-- Migration 0020_add_worktree_tracking.sql
ALTER TABLE maproom.chunks ADD COLUMN worktree_ids JSONB DEFAULT '[]'::JSONB;
```

**Recommendation**:
1. **MUST**: Review migration 0020 to understand worktree_ids array semantics
2. **MUST**: Clarify if chunks can truly belong to multiple worktrees
3. **IF YES**: Deletion strategy must NOT use CASCADE (must use array removal instead)
4. **IF YES**: Use existing `remove_worktree_from_chunks()` function instead
5. Update architecture.md with correct deletion strategy

#### 2.3 Performance Characteristics ✅

**Validation Performance**:
- ✅ Parallel async checks using `tokio::join_all` (~100ms for 100 worktrees)
- ✅ Realistic benchmarks (SSD ~1ms, HDD ~10ms per file system check)

**Deletion Performance**:
- ✅ Single transaction with CASCADE (~1-2 seconds for 95 worktrees)
- ✅ Batched approach avoids N+1 queries

**Watch Integration Performance**:
- ✅ Startup impact <200ms (background task, non-blocking)
- ✅ Periodic impact <500ms (deferred if busy)
- ✅ <1% overhead claim is believable

### 3. Safety & Security Review

#### 3.1 Defense-in-Depth ✅

**Layer 1: Validation**
- ✅ Disk existence check with `tokio::fs::try_exists`
- ✅ Permission errors treated conservatively (assume exists)

**Layer 2: User Confirmation**
- ✅ Dry-run is default behavior
- ✅ Explicit `--confirm` flag required for deletion

**Layer 3: Transaction Safety**
- ✅ ACID transactions with automatic rollback
- ✅ All-or-nothing deletion

**Layer 4: Audit Logging**
- ✅ Every deletion logged with full context

**Layer 5: Rate Limiting** (watch integration)
- ✅ Minimum 15 minutes between automatic cleanups
- ✅ Safety checks (skip if indexer busy)

#### 3.2 Threat Model Review ✅

**Threat 1: Accidental Deletion of Valid Worktree**
- ✅ Mitigation: Dry-run default, validation accuracy, audit logging
- ✅ Risk: Low

**Threat 2: Database Corruption**
- ✅ Mitigation: ACID transactions, rollback on error
- ✅ Risk: Very Low
- ⚠️ **BUT**: See worktree_ids CASCADE concern (Section 2.2)

**Threat 3: Performance Degradation**
- ✅ Mitigation: Background execution, rate limiting, priority scheduling
- ✅ Risk: Low

**Threat 4: Unauthorized Deletion**
- ✅ Mitigation: OS authentication, no remote execution
- ⚠️ Risk: Low (but no authorization layer beyond OS)

### 4. Testing Strategy Review

#### 4.1 Test Coverage ✅

**Unit Tests (30%)**:
- ✅ Detection accuracy tests
- ✅ Transaction safety tests
- ✅ Validation edge cases

**Integration Tests (60%)**:
- ✅ End-to-end cleanup workflow
- ✅ Database state verification
- ✅ Error handling scenarios

**Manual Testing (10%)**:
- ✅ Staging validation checklist
- ✅ Production-like scenarios

#### 4.2 Critical Test Paths ✅

**Path 1: Detection Accuracy**
- ✅ No false positives (valid worktrees never marked stale)
- ✅ No false negatives (stale worktrees always detected)

**Path 2: Deletion Safety**
- ✅ Transaction rollback on error
- ✅ No partial deletions
- ✅ Audit logs always written

**Path 3: CLI Usability**
- ✅ Dry-run output clear and actionable
- ✅ Error messages help user fix issues

**Path 4: Watch Integration**
- ✅ No interference with file events
- ✅ Performance benchmarks met

### 5. Execution Readiness

#### 5.1 Ticket Structure ✅

**Phase 1: Core Infrastructure** (3 tickets)
- ✅ Clear deliverables
- ✅ Testable acceptance criteria
- ✅ No external dependencies

**Phase 2: CLI Interface** (3 tickets)
- ✅ Well-defined scope
- ⚠️ Missing main.rs integration details (see 1.2)

**Phase 3: Testing** (4 tickets)
- ✅ Comprehensive test coverage
- ✅ Clear success metrics

**Phase 4: Watch Integration** (4 tickets, optional)
- ⚠️ **BLOCKED**: Requires analysis of existing Watch command (see 1.3)
- ⚠️ Cannot execute until integration points identified

**Phase 5: Deployment** (3 tickets)
- ✅ Documentation and rollout plan clear

#### 5.2 Agent Assignments ✅

- ✅ rust-indexer-engineer: Primary implementation (correct choice)
- ✅ integration-tester: Test suite creation (appropriate)
- ✅ verify-ticket + commit-ticket: Workflow automation (standard)

#### 5.3 Timeline ✅

- ✅ 3-4 weeks for MVP (realistic)
- ✅ +1 week for watch integration (reasonable)
- ⚠️ Watch integration may take longer if existing code needs refactoring

### 6. MVP Principles Compliance

#### 6.1 Scope Management ✅

- ✅ **Minimal Viable Product**: Manual cleanup CLI is MVP, watch is optional
- ✅ **Incremental Delivery**: Phases build on each other
- ✅ **User Value**: Each phase delivers working functionality

#### 6.2 Feature Prioritization ✅

**MUST HAVE** (MVP):
- ✅ Stale detection
- ✅ Safe deletion with dry-run
- ✅ CLI command

**SHOULD HAVE** (Optional):
- ✅ Watch integration (clearly marked optional)

**NICE TO HAVE** (Deferred):
- ✅ Git validation (explicitly deferred)
- ✅ Soft delete (future enhancement)

---

## Critical Issues & Blockers

### 🔴 CRITICAL - MUST FIX BEFORE TICKET CREATION

**Issue 1: worktree_ids Array and CASCADE Deletion Conflict**
- **Problem**: Migration 0020 added `worktree_ids JSONB` to chunks allowing multi-worktree chunks
- **Risk**: DELETE CASCADE on worktrees will delete chunks belonging to OTHER worktrees
- **Impact**: Data loss, breaks core assumption of cleanup safety
- **Resolution Required**:
  1. Investigate if chunks can truly belong to multiple worktrees
  2. If yes: Change deletion strategy to use `remove_worktree_from_chunks()` instead of CASCADE
  3. If no: Document why CASCADE is safe
  4. Update architecture.md Section 2.2 with correct approach
  5. Add test to verify no cross-worktree chunk deletion

**Issue 2: Existing Watch Command Integration Not Analyzed**
- **Problem**: Phase 4 assumes watch integration but doesn't analyze existing Watch command
- **Risk**: Cannot execute Phase 4 tickets without understanding integration points
- **Impact**: Phase 4 may be blocked or require significant rework
- **Resolution Required**:
  1. Read `crates/maproom/src/incremental/worktree_watcher.rs`
  2. Read `crates/maproom/src/incremental/multi_watcher.rs`
  3. Analyze existing event loop structure in main.rs Watch command
  4. Identify specific integration points for cleanup hooks
  5. Update architecture.md Section 5 with integration analysis
  6. Add Phase 4 ticket for refactoring if needed

### 🟡 HIGH PRIORITY - SHOULD FIX BEFORE PHASE 2

**Issue 3: main.rs CLI Integration Not Documented**
- **Problem**: Phase 2 tickets don't mention DbCommand enum extension or main.rs match arms
- **Risk**: Incomplete implementation, tickets may be underestimated
- **Resolution Required**:
  1. Add explicit ticket for main.rs integration (new IDXCLEAN-2004?)
  2. Document DbCommand::CleanupStale variant
  3. Document match arm implementation
  4. Update Phase 2 timeline if needed

**Issue 4: Relationship to remove_worktree_from_chunks() Unclear**
- **Problem**: Existing function in tree_sha_update.rs does similar worktree removal
- **Risk**: Duplication of logic, inconsistent behavior
- **Resolution Required**:
  1. Review `incremental/tree_sha_update.rs::remove_worktree_from_chunks()`
  2. Clarify if cleanup should reuse this function
  3. Document relationship in architecture.md
  4. Consider shared module if logic is reusable

---

## Recommendations

### Immediate Actions (Before Ticket Creation)

1. **MUST**: Resolve Critical Issue #1 (worktree_ids CASCADE conflict)
2. **MUST**: Resolve Critical Issue #2 (Watch command integration analysis)
3. **SHOULD**: Resolve High Priority Issue #3 (main.rs integration)
4. **SHOULD**: Resolve High Priority Issue #4 (remove_worktree_from_chunks relationship)

### Phase-Specific Recommendations

**Phase 1 (Core Infrastructure)**:
- Add explicit test for worktree_ids array handling
- Ensure cleanup.rs functions are reusable from both CLI and watch contexts
- Document relationship to incremental/ module functions

**Phase 2 (CLI Interface)**:
- Add IDXCLEAN-2004 ticket for main.rs integration
- Include DbCommand enum extension in acceptance criteria
- Test with both Db::Migrate and Db::CleanupStale to verify subcommand structure

**Phase 3 (Testing)**:
- Add integration test: "Verify chunks belonging to multiple worktrees are not deleted"
- Add integration test: "Verify cleanup doesn't interfere with concurrent watch operations"

**Phase 4 (Watch Integration)**:
- **BLOCKED** until Watch command analysis complete
- Split into two phases if refactoring required:
  - Phase 4a: Refactor watch command to support cleanup hooks
  - Phase 4b: Implement cleanup scheduler

**Phase 5 (Deployment)**:
- Add rollback procedure in case CASCADE deletion causes issues
- Include database backup verification step
- Document upgrade path from single-worktree to multi-worktree chunks

### Documentation Improvements

**architecture.md**:
1. Add section: "5.1 Integration with Existing Watch Command"
2. Add section: "2.2.1 worktree_ids Array and Deletion Strategy"
3. Add section: "2.3.1 Relationship to incremental/tree_sha_update.rs"
4. Update Section 2.2 with correct deletion strategy (CASCADE or array removal)

**plan.md**:
1. Add ticket IDXCLEAN-2004: "Integrate cleanup command with main.rs CLI"
2. Update Phase 4 dependencies: "Requires Watch command integration analysis"
3. Add risk: "worktree_ids CASCADE deletion may require strategy change"

**quality-strategy.md**:
1. Add critical test: "Verify multi-worktree chunk safety"
2. Add integration test: "Verify cleanup + watch concurrency"

---

## Approval Decision

### Status: **CONDITIONALLY APPROVED** ✅⚠️

**Approved For**:
- ✅ Phase 1: Core Infrastructure (tickets IDXCLEAN-1001 to 1003)
- ✅ Phase 2: CLI Interface (tickets IDXCLEAN-2001 to 2003, with main.rs addition)
- ✅ Phase 3: Testing (tickets IDXCLEAN-3001 to 3004)
- ⚠️ Phase 5: Deployment (tickets IDXCLEAN-5001 to 5003, pending Critical Issue #1 resolution)

**BLOCKED Until Resolution**:
- ⛔ Phase 4: Watch Integration (tickets IDXCLEAN-4001 to 4004)
  - **Reason**: Requires analysis of existing Watch command implementation
  - **Action**: Complete Watch command integration analysis before creating Phase 4 tickets

**Conditional Requirements**:
1. **MUST resolve Critical Issue #1** before ANY ticket execution (data loss risk)
2. **MUST resolve Critical Issue #2** before Phase 4 ticket creation
3. **SHOULD resolve High Priority Issues #3 and #4** before Phase 2 execution

**Overall Project Assessment**:
- **Planning Quality**: Excellent (9/10)
- **Execution Readiness**: Good with conditions (7/10)
- **Risk Management**: Strong (8/10)
- **MVP Compliance**: Excellent (10/10)

**Recommendation**:
- Proceed with ticket creation for Phases 1-3 and 5 AFTER resolving Critical Issue #1
- Defer Phase 4 ticket creation until Watch command integration analysis complete
- Update planning documents with missing integration details

---

## Next Steps

### Immediate (Before Running `/create-project-tickets IDXCLEAN`)

1. ✅ **Read and analyze**: `crates/maproom/migrations/0020_add_worktree_tracking.sql`
2. ✅ **Read and analyze**: `crates/maproom/src/incremental/tree_sha_update.rs` (remove_worktree_from_chunks function)
3. ✅ **Determine**: Can chunks belong to multiple worktrees? (Check database schema and existing tests)
4. ✅ **Update**: `architecture.md` Section 2.2 with correct deletion strategy
5. ✅ **Add**: Integration test for multi-worktree chunk safety to `quality-strategy.md`

### Before Phase 2 Execution

6. ✅ **Read and analyze**: Existing Watch command implementation (main.rs:778+)
7. ✅ **Read and analyze**: `crates/maproom/src/incremental/worktree_watcher.rs`
8. ✅ **Read and analyze**: `crates/maproom/src/incremental/multi_watcher.rs`
9. ✅ **Update**: `architecture.md` Section 5 with Watch integration analysis
10. ✅ **Add**: Ticket IDXCLEAN-2004 for main.rs integration to `plan.md`

### Before Phase 4 Ticket Creation

11. ⏸️ **Complete**: Watch command integration analysis (steps 6-9 above)
12. ⏸️ **Decide**: Refactor existing watch command or add hooks?
13. ⏸️ **Update**: `plan.md` Phase 4 tickets with specific integration tasks
14. ⏸️ **Estimate**: Phase 4 timeline adjustment if refactoring required

---

## Review Sign-Off

**Reviewer**: Senior Technical Architect (AI)
**Date**: 2025-11-18
**Verdict**: **APPROVED WITH CRITICAL CONDITIONS** ✅⚠️

**Signature**:
> This project demonstrates excellent planning and MVP thinking. The safety mechanisms are robust and the testing strategy is comprehensive. However, the critical CASCADE deletion concern and missing Watch integration analysis MUST be resolved before full execution. With these issues addressed, this project is execution-ready.

**Confidence Level**: **85%** → Will increase to **95%** after critical issues resolved

**Estimated Impact of Issues**:
- Critical Issue #1: 2-4 hours to investigate + 1-2 days to implement fix (if needed)
- Critical Issue #2: 1 day to analyze + potentially 1 week to refactor (worst case)
- High Priority Issues: 4-8 hours combined

**Recommended Action**: Resolve Critical Issue #1 immediately, then proceed with limited ticket creation (Phases 1-3 only). Defer Phase 4 until Watch integration clarity achieved.
