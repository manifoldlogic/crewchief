# Project Review: UNIWATCH - Unified Watch Command

**Review Date:** 2025-01-16
**Project Status:** Ready with Recommendations
**Overall Risk:** Medium
**Tickets Created:** No - Pre-ticket review

## Executive Summary

UNIWATCH proposes to unify two existing watch commands (`watch` and `branch-watch`) into a single command that handles both file changes and branch switches automatically. The project scope is well-defined, technically sound, and addresses a real usability problem. The planning is comprehensive with detailed architecture, testing strategy, and security considerations.

**Primary Strengths:**
- Clear problem statement with concrete user pain points
- Excellent component reuse strategy (no unnecessary rebuilding)
- Strong architectural separation between components
- Comprehensive test strategy (17 tests across 3 levels)
- Security-conscious (no new attack surfaces)
- Well-scoped MVP (2-3 days, ~17 tickets)

**Primary Concerns:**
- Architecture proposes NEW `UnifiedWatcher` struct when existing infrastructure can be leveraged more directly
- Some architectural complexity that may be unnecessary given existing `MultiWatcher` and `WorktreeWatcher` capabilities
- Missing exploration of simpler alternatives before committing to new struct
- Plan could benefit from clearer integration strategy with existing incremental watching infrastructure

**Recommendation:** **REVISE THEN PROCEED** - Address architectural simplification opportunities and clarify integration approach, then execute.

## Critical Issues (Blockers)

### Issue 1: Potentially Unnecessary New Struct (UnifiedWatcher)

**Severity:** Medium (not blocking, but optimization opportunity)
**Category:** Architecture | Reuse
**Description:**

The plan proposes creating a brand new `UnifiedWatcher` struct in `crates/maproom/src/indexer/unified_watch.rs`. However, examination of the existing codebase reveals sophisticated watching infrastructure that may already solve this problem:

**Existing Infrastructure:**
1. `WorktreeWatcher` (src/incremental/worktree_watcher.rs) - Already wraps FileWatcher and tags events with worktree_id
2. `MultiWatcher` (src/incremental/multi_watcher.rs) - Coordinates multiple WorktreeWatcher instances
3. `IncrementalProcessor` - Already handles file change processing
4. `BranchWatcher` - Already has branch detection logic

**Current Plan:**
```rust
pub struct UnifiedWatcher {
    current_worktree_id: Arc<RwLock<i32>>,
    file_watcher: FileWatcher,
    head_watcher: notify::RecommendedWatcher,
    processor: IncrementalProcessor,
    // ... etc
}
```

**Simpler Alternative (not evaluated in plan):**
Modify `watch_worktree()` to:
1. Create a single `notify::RecommendedWatcher` watching BOTH repo files AND `.git/HEAD`
2. When `.git/HEAD` changes, detect new branch and dynamically update the existing `WorktreeWatcher`'s worktree_id
3. Reuse ALL existing infrastructure without new top-level struct

**Impact:**
- Current approach adds ~300-400 lines of new code
- Alternative might achieve same goal with ~100 lines of modifications to existing code
- More code = more maintenance burden
- New struct introduces additional abstraction layer

**Required Action:**
1. **Evaluate simpler alternatives BEFORE implementing UnifiedWatcher**
2. Consider: Can `watch_worktree()` be modified to watch `.git/HEAD` alongside files?
3. Consider: Can `WorktreeWatcher` be made dynamically updatable (change worktree_id after creation)?
4. Document in architecture.md WHY UnifiedWatcher is necessary vs modifying existing functions
5. If simpler approach works, update architecture and plan accordingly

**Documents Affected:**
- architecture.md (Component Design section)
- plan.md (Phase 1 tasks)

---

### Issue 2: Unclear Integration with Existing `watch_worktree()`

**Severity:** Medium
**Category:** Architecture | Integration
**Description:**

The plan creates `unified_watch.rs` as a new module but doesn't clearly specify how it integrates with the existing `watch_worktree()` function in `src/indexer/mod.rs`.

**Current State:**
- `watch_worktree()` is a 100+ line function that:
  - Creates database pool
  - Initializes `WorktreeWatcher`
  - Starts `ChangeDetector` and `IncrementalProcessor`
  - Spawns async tasks for event processing
  - Runs until shutdown

**Plan States:**
```
Phase 4.1: Update watch Command
- Modify Commands::Watch handler in main.rs
- Call unified_watch::start_watching() instead of watch_worktree()
```

**Missing Clarity:**
1. Does `UnifiedWatcher::start()` duplicate all the pool/processor setup from `watch_worktree()`?
2. Is `watch_worktree()` deprecated entirely?
3. Could `watch_worktree()` be refactored to USE `UnifiedWatcher` internally?
4. What happens to all the event processing logic currently in `watch_worktree()`?

**Impact:**
- Risk of duplicating existing logic (pool creation, processor setup, event handling)
- Unclear if we're replacing or wrapping `watch_worktree()`
- May introduce inconsistencies between old and new code paths

**Required Action:**
1. **Clarify in architecture.md**: Is `UnifiedWatcher` a replacement for or wrapper around existing logic?
2. **Document in plan.md**: Explicitly state what happens to `watch_worktree()` function
3. **Specify integration points**: What code is reused vs reimplemented?
4. **Consider**: Should `watch_worktree()` call `UnifiedWatcher` internally rather than being replaced?

**Documents Affected:**
- architecture.md (Integration section)
- plan.md (Phase 4 tasks)

## High-Risk Areas (Warnings)

### Risk 1: Event Loop Complexity with Two Async Channels

**Risk Level:** Medium
**Category:** Technical
**Description:**

The architecture proposes mixing `tokio::mpsc` (async) and `std::sync::mpsc` (sync) channels:

```rust
file_events: mpsc::Receiver<FileEvent>,                    // tokio async
head_events: std::sync::mpsc::Receiver<notify::Event>,    // std sync
```

This mixing requires careful handling in the `tokio::select!` macro and can lead to:
- Blocking behavior in async context
- Dropped events if channels aren't properly bridged
- Difficult-to-debug race conditions

**Probability:** Medium (common pattern, but tricky to get right)
**Impact:** High (could cause missed events or hangs)

**Mitigation:**
- Convert std::sync::mpsc to tokio::sync::mpsc for uniformity
- OR use `tokio::sync::mpsc::UnboundedReceiver::blocking_recv()` wrapper
- Add integration test: `test_no_events_dropped_during_concurrent_operations()`
- Document channel strategy clearly in code comments

---

### Risk 2: Race Condition During Branch Switch

**Risk Level:** Medium
**Category:** Technical
**Description:**

The architecture acknowledges this risk but the mitigation may be insufficient:

**Scenario:**
1. File change event arrives
2. Branch switch begins (acquires write lock on `current_worktree_id`)
3. File event handler blocks waiting for read lock
4. Branch switch completes, updates database
5. File event handler proceeds with OLD worktree_id cached before acquiring lock

**Current Mitigation:**
```rust
let worktree_id = *self.current_worktree_id.read().unwrap();
// Use worktree_id...
```

**Problem:** The read() gives snapshot at that moment, but by the time processing happens, worktree may have changed.

**Probability:** Low (tight timing window)
**Impact:** High (files indexed to wrong worktree)

**Mitigation:**
- Hold read lock DURING database operation, not just for reading value
- OR re-check worktree_id before database commit
- Add stress test: Rapidly switch branches while editing files
- Document locking strategy in architecture.md

---

### Risk 3: Database Pool Not Shared Between watch_worktree() and BranchWatcher

**Risk Level:** Low
**Category:** Integration | Resource Management
**Description:**

Current code:
- `watch_worktree()` creates its own pool: `crate::db::pool::create_pool()`
- `BranchWatcher` uses a single `Client` passed to constructor
- If UnifiedWatcher replicates `watch_worktree()`, it may create ANOTHER pool

**Impact:**
- Multiple connection pools to same database
- Increased memory usage (each pool has min_connections)
- Potential connection exhaustion

**Probability:** Low (PostgreSQL typically handles this fine)
**Impact:** Low (minor resource waste)

**Mitigation:**
- Share single pool across all components
- Document pool ownership in architecture.md
- Add note in plan.md about pool initialization

## Gaps & Ambiguities

### Requirements Gaps

**Gap 1: Backward Compatibility for `--worktree` Flag**
- Requirement: "Deprecated but working"
- Impact: How much effort to maintain old behavior?
- Suggested Clarification: Specify EXACTLY what "deprecated but working" means:
  - Does it work identically to before?
  - Does it log warning and ignore flag?
  - Does it log warning and use flag value?
- Recommendation: Log warning, ignore flag value, use auto-detection (simplest)

**Gap 2: Multiple Rapid Branch Switches Behavior**
- Requirement: "Debouncing prevents rapid switches"
- Ambiguity: What happens to file events during debounce window?
- Impact: Queued? Dropped? Processed after switch completes?
- Suggested Clarification: Document event queue behavior during branch transitions

**Gap 3: Initial Branch Detection**
- Requirement: "Auto-detect current branch"
- Ambiguity: What if detached HEAD? Empty repo? Corrupted .git/HEAD?
- Impact: Edge cases not covered in error handling
- Suggested Clarification: Specify error handling for edge cases

### Technical Gaps

**Gap 1: NDJSON Event Format**
- Architecture defines new `branch_switched` event type
- Gap: Schema not fully specified
- Impact: VSCode extension won't know how to parse
- Required: Complete JSON schema in architecture.md with example:
```json
{
  "type": "branch_switched",
  "from_branch": "main",
  "to_branch": "feature-auth",
  "worktree_id": 42,
  "timestamp": "2025-01-16T10:30:00Z"
}
```

**Gap 2: Error Recovery Strategy**
- Plan mentions "graceful degradation"
- Gap: No specific recovery procedures
- Impact: Agents won't know how to implement error handling
- Required: Specify for each failure mode:
  - File watcher fails → Continue HEAD watching? Retry? Shutdown?
  - HEAD watcher fails → Continue file watching? Retry? Shutdown?
  - Database fails → Queue events? Drop events? Retry?

**Gap 3: Integration with Existing Event Processing**
- `watch_worktree()` has complex event processing with `ChangeDetector`, `UpdateQueue`, `IncrementalProcessor`
- Gap: Plan doesn't specify if this is reused or reimplemented
- Impact: Risk of duplication or missing functionality
- Required: Explicit statement of which components are reused vs new

### Process Gaps

**Gap 1: Testing Database State Between Tests**
- Quality strategy defines 17 tests
- Gap: No mention of database cleanup between tests
- Impact: Tests may interfere with each other
- Required: Specify test database strategy (transactions? Separate DBs? Cleanup hooks?)

**Gap 2: Manual Testing Checklist Not Specific Enough**
- Checklist says "Start watch on main branch"
- Gap: Doesn't specify HOW to verify events (grep output? Database query? Extension behavior?)
- Impact: Subjective testing, may miss bugs
- Required: More specific verification steps with expected outputs

## Scope & Feasibility Concerns

### Scope Creep Indicators

**None Identified** - Project scope is well-controlled:
- ✅ No new features beyond unification
- ✅ Clear "Out of Scope" section
- ✅ VSCode extension changes deferred to separate project
- ✅ Database schema unchanged
- ✅ Performance improvements explicitly excluded

**Strong MVP Discipline**

### Feasibility Challenges

**Challenge 1: Timeline May Be Optimistic for Full Testing**

**Analysis:**
- Plan: 2-3 days total
- Day 3 entirely devoted to testing
- Testing includes: 10 unit + 5 integration + 2 E2E + manual checklist

**Concern:** Writing 17 tests plus 2 bash scripts plus manual testing in one day is aggressive, especially if earlier phases slip.

**Mitigation:**
- Start writing tests DURING implementation (not after)
- Prioritize critical path tests first
- E2E tests can be post-merge if time-constrained
- Add 0.5 day buffer for testing

**Challenge 2: Async/Sync Channel Mixing**

**Analysis:**
- `notify` crate uses std::sync::mpsc (sync)
- Rest of codebase uses tokio::mpsc (async)
- Bridging requires careful handling

**Mitigation:**
- Prototype channel integration in Phase 1
- If problematic, convert to unified async channels
- Document pattern clearly for future maintainers

## Alignment Assessment

### MVP Discipline
**Rating:** Strong ✅

**Positive Indicators:**
- Single, focused goal: Unify two commands
- Clear problem with measurable success (no manual restart)
- Phase 1 delivers value (UnifiedWatcher compiles and runs)
- Out of Scope section prevents feature creep
- Timeline constrained (2-3 days forces prioritization)

**Areas for Improvement:**
- Could be EVEN simpler if existing structs are leveraged instead of new UnifiedWatcher
- Architecture could explore "minimum viable change" more thoroughly

### Pragmatism Score
**Rating:** Strong ✅

**Positive Indicators:**
- Reuses existing components (FileWatcher, BranchWatcher logic)
- No unnecessary abstraction layers
- Testing strategy focuses on critical paths (17 tests, not 100)
- Security review pragmatic ("no new risks" vs exhaustive analysis)
- Backward compatibility with deprecation path (pragmatic migration)

**Areas for Improvement:**
- EventRouter might be over-engineered (Arc<RwLock> for simple state)
- Could consider simpler state management (see Issue 1)

### Agent Compatibility
**Rating:** Strong ✅

**Positive Indicators:**
- Tasks sized appropriately (2-8 hour chunks)
- Clear acceptance criteria for each task
- Agent assignments specific (rust-indexer-engineer, unit-test-runner)
- Verification criteria explicit and testable
- Sequential phases with clear dependencies

**Potential Issues:**
- Phase 1.3 "Thread safety tests" might be difficult for agents to write correctly
- Integration tests in Phase 5 require agent to create bash scripts (less common)

**Mitigation:**
- Provide example thread safety test in quality-strategy.md
- Template bash test scripts for agents to follow

### Codebase Integration
**Rating:** Strong ✅

**Positive Indicators:**
- Excellent analysis of existing components
- Reuses WorktreeWatcher, FileWatcher, IncrementalProcessor
- No duplication of existing functionality
- Respects existing module structure
- Leverages existing `notify` crate usage

**Areas for Improvement:**
- Could explore deeper integration with existing `watch_worktree()` function
- EventRouter as new component vs extending existing WorktreeWatcher
- See Issue 1 and Issue 2 for specific opportunities

### Separation of Concerns
**Rating:** Adequate ⚠️

**Positive Indicators:**
- UnifiedWatcher clearly separates file watching from branch watching internally
- EventRouter isolates worktree tracking concern
- NDJSON events maintain clean interface with VSCode extension

**Concerns:**
- UnifiedWatcher combines concerns that are currently separate (file + branch)
  - This is INTENTIONAL (the whole point of the project)
  - But means less modularity than current two-command approach
- If file watching or branch watching needs independent changes, now tightly coupled
- Trade-off: Usability vs Modularity (chosen usability, acceptable)

**Mitigation:**
- Keep internal components (FileWatcher, HEAD watcher) decoupled
- Use clear interfaces between sub-components
- Document the coupling trade-off in architecture.md

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds

**None Identified** ✅

The project REUSES existing components rather than rebuilding:
- ✅ WorktreeWatcher (reused)
- ✅ FileWatcher (reused)
- ✅ IncrementalProcessor (reused)
- ✅ BranchWatcher logic (extracted and reused, not duplicated)
- ✅ DebouncedHandler (extracted from BranchWatcher)

**However, see Issue 1**: UnifiedWatcher as new struct may be unnecessary if existing infrastructure can be leveraged differently.

### Boundary Violations

**None Identified** ✅

All integrations respect proper boundaries:
- Uses public APIs of WorktreeWatcher, FileWatcher
- Doesn't reach into internal implementations
- Doesn't bypass module interfaces

### Missed Reuse Opportunities

**Opportunity 1: MultiWatcher Capabilities**

**Available Component:** `MultiWatcher` (src/incremental/multi_watcher.rs)
**Capabilities:**
- Already coordinates multiple WorktreeWatcher instances
- Already has health monitoring and retry logic
- Already aggregates events from multiple watchers

**Could Solve:** Managing file + HEAD watchers
**Integration Effort:** Medium (would need to adapt to single repo, two event sources)
**Recommendation:** Evaluate if MultiWatcher pattern could be adapted for dual-watcher coordination

**Opportunity 2: WorktreeWatcher Mutability**

**Available Component:** `WorktreeWatcher` already exists
**Current Design:** Immutable worktree_id after creation
**Proposed:** Create NEW UnifiedWatcher to manage changing worktree_id
**Alternative:** Add `update_worktree_id()` method to WorktreeWatcher?

**Analysis:**
- WorktreeWatcher is designed for multi-worktree scenarios (multiple instances)
- Making single instance's worktree_id mutable goes against that design
- BUT might be simpler than whole new orchestrator struct

**Recommendation:** Document WHY mutation of WorktreeWatcher was rejected (if it was considered)

### Pattern Violations

**None Identified** ✅

Project follows existing patterns:
- Uses `notify` crate for file watching (established pattern)
- Uses `tokio` for async (established pattern)
- Uses NDJSON for events (established pattern for watch commands)
- Error handling with `anyhow` (established pattern)

### Inappropriate Coupling

**Acceptable Coupling Introduced:**
- UnifiedWatcher couples file watching + branch watching
  - **Rationale:** This is the GOAL of the project
  - **Impact:** Less modularity, but better UX
  - **Verdict:** Acceptable trade-off

**No inappropriate coupling identified.**

## Reuse Opportunities

### Opportunity 1: ChangeDetector and UpdateQueue

**Existing Tool:** `ChangeDetector`, `UpdateQueue` in incremental module
**Current Usage:** Used by `watch_worktree()`
**Plan Mentions:** IncrementalProcessor

**Question:** Are ChangeDetector and UpdateQueue reused by UnifiedWatcher?
**Impact:** If not, may lose functionality like:
- Hash-based change detection
- Priority queue for updates
- Retry logic

**Recommendation:** Explicitly state in architecture.md whether these are reused

### Opportunity 2: Event Processing Tasks

**Existing Pattern:** `watch_worktree()` spawns async tasks for event processing
**Plan:** UnifiedWatcher has `start()` event loop

**Question:** Does UnifiedWatcher replicate the task spawning pattern?
**Recommendation:** Reuse task spawning pattern for consistency

### Opportunity 3: Database Schema Validation

**Existing Code:** `watch_worktree()` validates database schema on startup
**Plan:** UnifiedWatcher has `new()` constructor

**Question:** Does UnifiedWatcher validate schema?
**Recommendation:** Reuse validation logic for consistency

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from (17 tasks specified)
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [x] Dependencies on existing systems documented

**All documentation criteria met** ✅

### Technical
- [x] Technology choices are appropriate (reuse existing, no new dependencies)
- [x] Dependencies are identified and available
- [~] Integration points are well-defined (see Issue 2 - needs clarification)
- [x] Performance requirements are clear (<2% CPU, <20MB memory)
- [~] Error handling is specified (see Gap 2 - needs more detail)
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

**Mostly met, minor gaps on integration and error handling**

### Process
- [x] Agent assignments are appropriate
- [~] Task boundaries are clear (mostly, see Issue 1 on UnifiedWatcher necessity)
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [x] Rollback plan exists
- [x] Integration with existing workflows considered

**Mostly met, architectural decisions could be clearer**

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified
- [~] Integration points with existing systems mapped (see Issue 2)
- [x] No reinvention of available functionality
- [x] Proper integration methods chosen:
  - [x] Not using CLI (this IS the CLI)
  - [x] Using library imports appropriately (WorktreeWatcher, etc.)
  - [x] No cross-boundary violations
- [x] Component boundaries respected
- [x] Public interfaces used (not internals)
- [~] Appropriate coupling levels maintained (acceptable trade-offs, see analysis)

**Strong reuse strategy, minor integration clarity gaps**

### Risk
- [x] Major risks are identified
- [~] Mitigation strategies exist (see Risk 1-3 for enhancements)
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [x] Failure modes are understood

**Risk awareness good, mitigations could be more specific**

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Evaluate Simpler Architectural Alternatives**
   - **Action:** Before implementing UnifiedWatcher, prototype modifying `watch_worktree()` to watch `.git/HEAD` alongside files
   - **Outcome:** If simpler approach works, update architecture.md and plan.md
   - **Timeline:** 2-4 hours of investigation
   - **Benefit:** Could reduce implementation from ~400 to ~100 lines of code

2. **Clarify Integration Strategy**
   - **Action:** Document in architecture.md EXACTLY how UnifiedWatcher relates to `watch_worktree()`
   - **Outcome:** Clear answer to: Replace? Wrap? Call internally?
   - **Documents:** architecture.md (add "Integration with Existing Code" section)

3. **Complete NDJSON Event Schemas**
   - **Action:** Add full JSON schema for `branch_switched` event in architecture.md
   - **Outcome:** VSCode extension can be updated in parallel
   - **Documents:** architecture.md (Component Design - NDJSON Event Format)

4. **Specify Error Recovery Procedures**
   - **Action:** For each failure mode (file watcher, HEAD watcher, database), specify exact recovery behavior
   - **Outcome:** Implementation agents know how to handle errors
   - **Documents:** architecture.md (Error Handling section)

5. **Document Channel Strategy**
   - **Action:** Justify async vs sync channel choices and bridging strategy
   - **Outcome:** Avoids confusion during implementation
   - **Documents:** architecture.md (Component Design - Event Channels)

### Phase 1 Adjustments

**Adjustment 1: Add Architectural Evaluation Step**
- **Before Task 1.1:** Add Task 0.1: "Evaluate simpler alternatives to UnifiedWatcher"
- **Acceptance Criteria:** Document why new struct is necessary OR propose simpler modification to `watch_worktree()`
- **Timeline:** +0.5 day to Phase 1

**Adjustment 2: Clarify Component Reuse**
- **In Task 1.2:** Explicitly state whether ChangeDetector, UpdateQueue are reused
- **Acceptance Criteria:** Document which existing components are integrated

### Risk Mitigations

**Mitigation 1: Channel Integration Prototype**
- **Action:** In Phase 1.2, create simple test of tokio::select! with mixed channels
- **Outcome:** Validate approach early before complex logic
- **Benefit:** Catch integration issues in Phase 1, not Phase 3

**Mitigation 2: Lock Strategy Documentation**
- **Action:** In Phase 2, document locking strategy for race condition prevention
- **Outcome:** Code comments explain why locks are held during operations
- **Benefit:** Maintainers understand thread safety design

**Mitigation 3: Database Pool Sharing**
- **Action:** In Phase 1, clarify pool ownership and sharing
- **Outcome:** Single pool shared across components
- **Benefit:** Avoid resource waste

### Documentation Updates

**architecture.md Updates:**
1. Add "Integration with Existing Code" section explaining relationship to `watch_worktree()`
2. Complete NDJSON event schemas with examples
3. Add "Error Handling Strategy" section with recovery procedures
4. Document channel strategy (async vs sync, bridging approach)
5. Add "Lock Strategy" section explaining thread safety approach
6. Add "Alternative Approaches Considered" section (even if brief)

**plan.md Updates:**
1. Add Phase 0 for architectural evaluation (optional, if simpler approach found)
2. Clarify in Phase 4 what happens to `watch_worktree()` function
3. Add specific error recovery tasks to Phase 3.2
4. Adjust timeline to 3-4 days if architectural evaluation added

**quality-strategy.md Updates:**
1. Add example thread safety test for agents to follow
2. Add template bash script for E2E tests
3. Specify database cleanup strategy between tests
4. Make manual testing checklist more specific (expected outputs, verification commands)

## Review Conclusion

### Readiness Assessment

**Can this project succeed as currently defined?** Yes, with architectural clarifications.

**Primary concerns:**
1. UnifiedWatcher may be more complex than necessary - simpler alternatives not thoroughly evaluated
2. Integration strategy with existing `watch_worktree()` function needs clarification
3. Some technical gaps in error handling and channel integration strategies

### Recommended Path Forward

**REVISE THEN PROCEED:**

**Revisions Needed (1-2 hours):**
1. Evaluate simpler alternatives to UnifiedWatcher (prototype investigation)
2. Clarify integration strategy with existing code (documentation update)
3. Complete NDJSON event schemas (documentation update)
4. Document error recovery procedures (documentation update)
5. Document channel bridging strategy (documentation update)

**After Revisions:**
- Generate tickets with `/create-project-tickets UNIWATCH`
- Execute with confidence

**Rationale for "Revise Then Proceed":**
- Project is fundamentally sound
- Issues are clarifications, not showstoppers
- 1-2 hours of upfront work prevents confusion during execution
- Ensures agents have complete information

### Success Probability

**Given current state:** 75%
- Well-planned, good reuse, clear scope
- But architectural decisions could cause confusion or rework

**After recommended changes:** 90%
- Clarified architecture removes ambiguity
- Simplified approach (if viable) reduces implementation risk
- Complete specifications enable autonomous agent execution

### Final Notes

This is an **excellent example of well-scoped, pragmatic project planning**. The team has:
- Thoroughly analyzed the problem
- Reused existing components
- Defined clear success metrics
- Constrained scope appropriately
- Planned comprehensive testing

The recommendations above are optimizations to an already solid plan, not fundamental rework. The main suggestion is to explore simpler architectural alternatives before committing to the proposed UnifiedWatcher approach.

**Key Insight:** Sometimes the MVP is not building a new thing, but cleverly modifying an existing thing. The plan could benefit from more exploration of the "modify existing" approach before jumping to "create new struct."

**After addressing the recommended clarifications, this project is ready for ticket generation and execution.**
