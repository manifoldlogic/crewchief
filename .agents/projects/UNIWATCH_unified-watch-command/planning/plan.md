# Implementation Plan (REVISED): Unified Watch Command

## Overview

**Goal**: Unified watch command via minimal modifications to existing `watch_worktree()` function

**Timeline**: 1-2 days (reduced from 2-3 days)
**Approach**: Modify existing infrastructure instead of creating new UnifiedWatcher
**Risk Level**: Low (smaller changes, proven components)

## Key Changes from Original Plan

1. **No UnifiedWatcher struct** - Modify watch_worktree() directly
2. **Fewer tasks** - 12 tickets instead of 17
3. **Simpler integration** - No new module, just function modifications
4. **Faster timeline** - 1-2 days instead of 2-3 days

## Phases

### Phase 1: Foundation (Day 1 Morning - 2 hours)

**Objective**: Add .git/HEAD watching infrastructure

#### Tasks

**1.1: Create setup_head_watcher() Function**
- Location: `crates/maproom/src/indexer/mod.rs` (new function)
- Create function to set up notify::RecommendedWatcher for .git/HEAD
- Bridge std::sync::mpsc to tokio::mpsc channels
- Return watcher handle for cleanup

**Acceptance Criteria**:
- Function compiles and accepts git_head path + tokio channel
- Returns notify watcher that can be dropped for cleanup
- Channel bridging task spawned
- Unit test: `test_setup_head_watcher_creates_bridge()`

**Agent**: rust-indexer-engineer
**Lines**: ~30 new

---

**1.2: Add Dynamic Worktree Tracking**
- Location: `crates/maproom/src/indexer/mod.rs` (in watch_worktree function)
- Add Arc<RwLock<String>> for current_branch
- Add Arc<RwLock<i32>> for current_worktree_id
- Initialize from function parameters

**Acceptance Criteria**:
- State variables created at function start
- Initialized with repo/worktree from parameters
- Arc/RwLock pattern matches existing maproom code
- Unit test: `test_worktree_tracking_initialization()`

**Agent**: rust-indexer-engineer
**Lines**: ~15 modifications

---

**1.3: Copy DebouncedHandler**
- Location: `crates/maproom/src/indexer/mod.rs` or new `debounce.rs` module
- Copy DebouncedHandler from `src/watcher.rs`
- Make it reusable (currently tightly coupled to BranchWatcher)
- 2-second default debounce window

**Acceptance Criteria**:
- DebouncedHandler accessible from watch_worktree()
- Thread-safe (Mutex<Instant>)
- Configurable debounce duration
- Unit test: `test_debouncer_prevents_rapid_events()`

**Agent**: rust-indexer-engineer
**Lines**: ~20 copied/refactored

---

### Phase 2: Branch Switch Logic (Day 1 Afternoon - 3 hours)

**Objective**: Implement branch detection and state updates

#### Tasks

**2.1: Implement handle_branch_switch() Function**
- Location: `crates/maproom/src/indexer/mod.rs` (new function)
- Extract branch name from .git/HEAD (use get_current_branch())
- Check if branch actually changed (early return if same)
- Get/create worktree record in database
- Update current_branch and current_worktree_id (write locks)
- Trigger incremental_update()
- Emit branch_switched NDJSON event

**Acceptance Criteria**:
- Branch switch detected correctly
- Worktree ID updated in shared state
- incremental_update() called
- NDJSON event emitted to stdout
- Unit test: `test_handle_branch_switch_updates_state()`
- Unit test: `test_handle_branch_switch_skips_if_same_branch()`

**Agent**: rust-indexer-engineer
**Lines**: ~40 new

---

**2.2: Create BranchSwitchEvent Struct**
- Location: `crates/maproom/src/indexer/mod.rs` or new `events.rs` module
- Define NDJSON event structure
- Implement Serialize
- Include: old_branch, new_branch, old_worktree_id, new_worktree_id, worktree_created

**Acceptance Criteria**:
- Struct serializes to valid JSON
- All required fields present
- Type field = "branch_switched"
- Unit test: `test_branch_switch_event_serialization()`

**Agent**: rust-indexer-engineer
**Lines**: ~20 new

---

### Phase 3: Event Loop Integration (Day 1 Evening - 3 hours)

**Objective**: Modify event loop to handle both file and branch events

#### Tasks

**3.1: Modify watch_worktree() Initialization**
- Location: `crates/maproom/src/indexer/mod.rs` (watch_worktree function)
- Add .git/HEAD path calculation
- Create head event channel (tokio::mpsc)
- Call setup_head_watcher()
- Store watcher handle for cleanup

**Acceptance Criteria**:
- HEAD watcher created alongside file watcher
- Both watchers started successfully
- Cleanup on function exit
- Integration test: `test_dual_watchers_initialize()`

**Agent**: rust-indexer-engineer
**Lines**: ~20 modifications

---

**3.2: Modify Event Loop to Use tokio::select!**
- Location: `crates/maproom/src/indexer/mod.rs` (processor_task spawn)
- Change `while let Some(event) = event_rx.recv()` to `loop { tokio::select! {} }`
- Add file event branch (existing logic)
- Add head event branch (call handle_branch_switch())
- Preserve all existing error handling

**Acceptance Criteria**:
- Both event sources handled in same loop
- File processing logic unchanged
- Branch switch logic called on HEAD events
- Shutdown still works correctly
- Integration test: `test_event_loop_handles_both_sources()`

**Agent**: rust-indexer-engineer
**Lines**: ~40 modifications

---

**3.3: Update Event Processing to Use Dynamic Worktree ID**
- Location: `crates/maproom/src/indexer/mod.rs` (event processing in processor_task)
- Read current_worktree_id.read().unwrap() instead of using hardcoded
- Use for database queries and logging
- Maintain same processing logic otherwise

**Acceptance Criteria**:
- Worktree ID read dynamically
- File events tagged with correct worktree after branch switch
- No change to processing logic
- Integration test: `test_file_events_use_current_worktree()`

**Agent**: rust-indexer-engineer
**Lines**: ~10 modifications

---

### Phase 4: CLI & Polish (Day 2 Morning - 2 hours)

**Objective**: Update CLI interface and documentation

#### Tasks

**4.1: Update Commands::Watch Handler**
- Location: `crates/maproom/src/main.rs` (Commands::Watch match arm)
- Auto-detect branch if --worktree not provided (use get_current_branch())
- Add deprecation warning if --worktree is provided
- Keep same watch_worktree() call (no API change)

**Acceptance Criteria**:
- `maproom watch` works without --worktree flag
- Auto-detects current branch
- `--worktree` flag still works (with warning)
- Warning logged to stderr
- Integration test: `test_watch_auto_detects_branch()`

**Agent**: rust-indexer-engineer
**Lines**: ~15 modifications

---

**4.2: Update branch-watch Command**
- Location: `crates/maproom/src/main.rs` (Commands::BranchWatch match arm)
- Add deprecation warning
- Log: "branch-watch is deprecated, use 'watch' instead"
- Keep functionality working

**Acceptance Criteria**:
- Warning logged on invocation
- Command still works
- User directed to use `watch` instead

**Agent**: rust-indexer-engineer
**Lines**: ~5 modifications

---

**4.3: Update Documentation**
- Update `crates/maproom/CLAUDE.md` (watch command section)
- Update CLI help text (--help output)
- Add migration examples
- Document NDJSON events

**Acceptance Criteria**:
- Documentation accurate
- Examples show unified watch usage
- NDJSON events documented
- Migration path clear

**Agent**: rust-indexer-engineer
**Lines**: ~50 modifications (docs)

---

### Phase 5: Testing & Verification (Day 2 Afternoon - 3 hours)

**Objective**: Comprehensive testing and quality assurance

#### Tasks

**5.1: Unit Tests**
- All unit tests from acceptance criteria above
- Total: 8 tests
- Coverage: setup_head_watcher, handle_branch_switch, debouncing, state management

**Acceptance Criteria**:
- All 8 unit tests pass
- No clippy warnings
- Code coverage >80% for new code

**Agent**: unit-test-runner

---

**5.2: Integration Tests**
- `test_complete_branch_switch_workflow()` - Full E2E in Rust
- `test_rapid_branch_switches_debounced()` - Debouncing verification
- `test_file_changes_during_branch_switch()` - Race conditions
- `test_worktree_flag_backward_compatible()` - CLI compatibility

**Acceptance Criteria**:
- All 4 integration tests pass
- Real git operations
- Real database state verified

**Agent**: integration-tester

---

**5.3: End-to-End Bash Tests**
- `tests/e2e/test_unified_watch_workflow.sh` - Developer workflow
- Real git repo, real branch switches, real file edits
- Verify database state with psql queries

**Acceptance Criteria**:
- E2E test passes on clean environment
- Can run in CI
- Realistic developer workflow

**Agent**: integration-tester

---

**5.4: Manual Testing**
- Start watch on main branch
- Edit file → verify indexed to main
- Switch to feature branch → verify branch_switched event
- Edit file → verify indexed to feature
- Switch back to main → verify worktree updated
- Stop database → verify watch doesn't crash
- Restart database → verify recovery

**Acceptance Criteria**:
- Manual testing checklist complete
- No regressions found
- NDJSON events correct

**Agent**: verify-ticket

---

## Task Summary

### By Phase
- **Phase 1**: 3 tasks (foundation)
- **Phase 2**: 2 tasks (branch logic)
- **Phase 3**: 3 tasks (event loop)
- **Phase 4**: 3 tasks (CLI polish)
- **Phase 5**: 4 tasks (testing)

**Total: 15 tasks** (vs 17 in original plan)

### By Type
- **Implementation**: 11 tasks
- **Testing**: 4 tasks

### Lines of Code
- **New code**: ~145 lines
- **Modified code**: ~100 lines
- **Documentation**: ~50 lines
- **Total**: ~295 lines (vs 400+ in original)

## Timeline

### Day 1 (6-8 hours)
- **Morning** (2h): Phase 1 - Foundation
- **Afternoon** (3h): Phase 2 - Branch logic
- **Evening** (3h): Phase 3 - Event loop

### Day 2 (5-6 hours)
- **Morning** (2h): Phase 4 - CLI polish
- **Afternoon** (3h): Phase 5 - Testing

**Total: 11-14 hours over 2 days**

## Agent Assignments

**rust-indexer-engineer**:
- All implementation tasks (Phases 1-4)
- 11 tasks total
- ~295 lines of code

**unit-test-runner**:
- Execute unit tests (Phase 5.1)
- Report results only

**integration-tester**:
- Create and run integration tests (Phase 5.2)
- Create and run E2E bash tests (Phase 5.3)

**verify-ticket**:
- Manual testing (Phase 5.4)
- Final acceptance criteria verification

**commit-ticket**:
- Create commits after verification
- Conventional Commit format

## Dependencies

### Modified Files
1. `crates/maproom/src/indexer/mod.rs` - Main modifications (~200 lines changed)
2. `crates/maproom/src/main.rs` - CLI updates (~20 lines changed)
3. `crates/maproom/CLAUDE.md` - Documentation (~50 lines changed)

### No New Files Required
- All changes in existing modules
- No new structs or modules
- Simpler file layout

### External Dependencies
- No new crates needed
- Reuses: notify, tokio, tokio-postgres, anyhow

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Event loop complexity | Low | Medium | Use tokio::select! (proven pattern) |
| Race conditions | Medium | High | RwLock + integration tests |
| Breaking existing usage | Low | High | Backward compatible function signature |
| Channel bridging issues | Low | Medium | Same pattern as existing WorktreeWatcher |

## Success Metrics

### Phase 1
- [x] .git/HEAD watcher created
- [x] Channel bridging works
- [x] Debouncer reusable

### Phase 2
- [x] Branch switch updates state
- [x] NDJSON events emitted
- [x] incremental_update() called

### Phase 3
- [x] Event loop handles both sources
- [x] File events use dynamic worktree
- [x] No events lost

### Phase 4
- [x] CLI auto-detects branch
- [x] Backward compatible
- [x] Documentation updated

### Phase 5
- [x] All tests pass
- [x] Manual testing complete
- [x] No regressions

## Comparison: Original vs Revised

| Metric | Original Plan | Revised Plan | Improvement |
|--------|--------------|--------------|-------------|
| **Total Tasks** | 17 | 15 | 12% fewer |
| **Total Days** | 2-3 | 1-2 | 33% faster |
| **Lines of Code** | 400+ | 295 | 26% less code |
| **New Structs** | 2 (UnifiedWatcher, EventRouter) | 0 | Simpler |
| **New Modules** | 1 (unified_watch.rs) | 0 | Simpler |
| **Integration Complexity** | Replace function | Modify function | Lower risk |

## Rollout Plan

### Development
```bash
# Same as original
git checkout -b feature/unified-watch

# Faster iteration (smaller changes)
cargo test --lib indexer
git commit
```

### Testing
```bash
# Same test commands
cargo test
./tests/e2e/test_unified_watch_workflow.sh
```

### Release
```bash
# Same release process
git merge feature/unified-watch
git tag v0.x.0
./scripts/build-and-package.sh
```

## Rollback Plan

**Easier rollback** than original:
1. Revert changes to watch_worktree() function
2. Revert CLI updates
3. No new modules to remove

## Ticket Generation

After approval, run:
```bash
/create-project-tickets UNIWATCH
```

**Expected tickets** (numbered based on phases):
- UNIWATCH-1001: Create setup_head_watcher function
- UNIWATCH-1002: Add dynamic worktree tracking
- UNIWATCH-1003: Copy DebouncedHandler
- UNIWATCH-2001: Implement handle_branch_switch
- UNIWATCH-2002: Create BranchSwitchEvent struct
- UNIWATCH-3001: Add HEAD watcher to initialization
- UNIWATCH-3002: Modify event loop to use tokio::select
- UNIWATCH-3003: Update event processing for dynamic worktree
- UNIWATCH-4001: Update Commands::Watch handler
- UNIWATCH-4002: Deprecate branch-watch command
- UNIWATCH-4003: Update documentation
- UNIWATCH-5001: Unit tests
- UNIWATCH-5002: Integration tests
- UNIWATCH-5003: E2E bash tests
- UNIWATCH-5004: Manual testing

**15 tickets** (manageable for 1-2 day timeline)

## Acceptance Criteria (Project-Level)

**Functional**:
- [ ] Single `watch` command handles file changes and branch switches
- [ ] Branch switches detected within 1-2 seconds
- [ ] File changes after switch indexed to correct worktree
- [ ] No manual restart needed on branch switch
- [ ] Backward compatible with --worktree flag

**Quality**:
- [ ] All 8 unit tests pass
- [ ] All 4 integration tests pass
- [ ] E2E bash test passes
- [ ] Manual testing checklist complete
- [ ] No clippy warnings

**Documentation**:
- [ ] CLI help text updated
- [ ] CLAUDE.md updated
- [ ] NDJSON events documented
- [ ] Migration examples provided

**Sign-Off**: Ready when all criteria checked ✓

## Why This Plan is Better

1. **Simpler**: Modifying existing vs creating new
2. **Faster**: 1-2 days vs 2-3 days
3. **Lower Risk**: Smaller changes, proven patterns
4. **Easier Testing**: Fewer edge cases
5. **Better Integration**: No API changes
6. **Easier Review**: Changes in unified diff
7. **Same Result**: Identical functionality to original plan
