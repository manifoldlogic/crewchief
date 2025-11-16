# UNIWATCH Ticket Index

**Project**: UNIWATCH (Unified Watch Command)
**Total Tickets**: 15
**Timeline**: 1-2 days
**Status**: All tickets created ✅

## Quick Stats

- **Phase 1** (Foundation): 3 tickets - 2 hours
- **Phase 2** (Branch Logic): 2 tickets - 3 hours
- **Phase 3** (Event Loop): 3 tickets - 3 hours
- **Phase 4** (CLI Polish): 3 tickets - 2 hours
- **Phase 5** (Testing): 4 tickets - 3 hours

**Total Effort**: ~13 hours over 1-2 days

## Phase 1: Foundation (Day 1 Morning - 2 hours)

**Objective**: Add .git/HEAD watching infrastructure

| Ticket ID | Title | Agent | Lines | Status |
|-----------|-------|-------|-------|--------|
| UNIWATCH-1001 | Create setup_head_watcher() Function | rust-indexer-engineer | ~30 new | ⏳ Not Started |
| UNIWATCH-1002 | Add Dynamic Worktree Tracking State | rust-indexer-engineer | ~15 mod | ⏳ Not Started |
| UNIWATCH-1003 | Extract and Reuse DebouncedHandler | rust-indexer-engineer | ~20 copy | ⏳ Not Started |

**Dependencies**: None (all can be done in parallel)

**Deliverables**:
- setup_head_watcher() function for .git/HEAD monitoring
- Arc<RwLock> state tracking for current branch/worktree
- DebouncedHandler for rate limiting events

## Phase 2: Branch Switch Logic (Day 1 Afternoon - 3 hours)

**Objective**: Implement branch detection and state updates

| Ticket ID | Title | Agent | Lines | Status |
|-----------|-------|-------|-------|--------|
| UNIWATCH-2001 | Implement handle_branch_switch() Function | rust-indexer-engineer | ~40 new | ⏳ Not Started |
| UNIWATCH-2002 | Create BranchSwitchEvent NDJSON Struct | rust-indexer-engineer | ~20 new | ⏳ Not Started |

**Dependencies**:
- UNIWATCH-2001 depends on UNIWATCH-1002 (needs Arc<RwLock> state)
- UNIWATCH-2002 depends on UNIWATCH-2001 (event emitted from handler)

**Deliverables**:
- Core branch switch handler function
- NDJSON event structure for VSCode extension integration

## Phase 3: Event Loop Integration (Day 1 Evening - 3 hours)

**Objective**: Wire up event handling and async coordination

| Ticket ID | Title | Agent | Lines | Status |
|-----------|-------|-------|-------|--------|
| UNIWATCH-3001 | Add .git/HEAD Watcher to Initialization | rust-indexer-engineer | ~20 mod | ⏳ Not Started |
| UNIWATCH-3002 | Modify Event Loop to Use tokio::select! | rust-indexer-engineer | ~50 mod | ⏳ Not Started |
| UNIWATCH-3003 | Update Event Processing for Dynamic Worktree ID | rust-indexer-engineer | ~10 mod | ⏳ Not Started |

**Dependencies**:
- UNIWATCH-3001 depends on UNIWATCH-1001, UNIWATCH-1002
- UNIWATCH-3002 depends on UNIWATCH-1001, UNIWATCH-1003, UNIWATCH-2001, UNIWATCH-3001
- UNIWATCH-3003 depends on UNIWATCH-1002, UNIWATCH-3002

**Deliverables**:
- Dual watcher initialization (file + head)
- Unified event loop with tokio::select!
- Dynamic worktree ID tracking in file processing

## Phase 4: CLI Integration & Polish (Day 2 Morning - 2 hours)

**Objective**: Update CLI interface and documentation

| Ticket ID | Title | Agent | Lines | Status |
|-----------|-------|-------|-------|--------|
| UNIWATCH-4001 | Update Commands::Watch to Auto-Detect Branch | rust-indexer-engineer | ~15 mod | ⏳ Not Started |
| UNIWATCH-4002 | Add Deprecation Warning to branch-watch | rust-indexer-engineer | ~5 add | ⏳ Not Started |
| UNIWATCH-4003 | Update Documentation for Unified Watch | rust-indexer-engineer | ~50 mod | ⏳ Not Started |

**Dependencies**:
- UNIWATCH-4003 depends on UNIWATCH-4001, UNIWATCH-2002 (document what was implemented)

**Deliverables**:
- Auto-detection of current branch in CLI
- Deprecation warnings for old usage patterns
- Updated documentation (CLAUDE.md, help text, NDJSON events)

## Phase 5: Testing & Verification (Day 2 Afternoon - 3 hours)

**Objective**: Comprehensive testing and quality assurance

| Ticket ID | Title | Agent | Tests | Status |
|-----------|-------|-------|-------|--------|
| UNIWATCH-5001 | Execute and Verify Unit Tests | unit-test-runner | 8+ tests | ⏳ Not Started |
| UNIWATCH-5002 | Create and Execute Integration Tests | integration-tester | 4 tests | ⏳ Not Started |
| UNIWATCH-5003 | Create End-to-End Bash Test Script | integration-tester | 1 script | ⏳ Not Started |
| UNIWATCH-5004 | Manual Testing and Final Verification | verify-ticket | 24 checks | ⏳ Not Started |

**Dependencies**:
- UNIWATCH-5001 depends on ALL Phase 1-4 tickets
- UNIWATCH-5002 depends on UNIWATCH-5001
- UNIWATCH-5003 depends on UNIWATCH-5001, UNIWATCH-5002
- UNIWATCH-5004 depends on ALL previous tickets

**Deliverables**:
- All unit tests passing (8+ tests)
- Integration test suite (4 tests in Rust)
- E2E bash test script
- Manual testing checklist completed

## File Changes Summary

**Files to Modify**:
- `crates/maproom/src/indexer/mod.rs` - ~200 lines (primary implementation)
- `crates/maproom/src/main.rs` - ~20 lines (CLI updates)
- `crates/maproom/CLAUDE.md` - ~30 lines (documentation)
- `crates/maproom/src/cli/mod.rs` - ~10 lines (help text)

**Files to Create**:
- `crates/maproom/tests/integration/unified_watch_test.rs` - ~200 lines
- `crates/maproom/tests/e2e/test_unified_watch_workflow.sh` - ~100 lines
- `crates/maproom/docs/NDJSON_EVENTS.md` - ~50 lines (optional)

**Total**: ~295 lines of new/modified code + ~350 lines of tests

## Execution Strategy

### Sequential Execution (Recommended)

```bash
# Phase 1 (parallel)
/single-ticket UNIWATCH-1001
/single-ticket UNIWATCH-1002
/single-ticket UNIWATCH-1003

# Phase 2 (sequential)
/single-ticket UNIWATCH-2001
/single-ticket UNIWATCH-2002

# Phase 3 (sequential)
/single-ticket UNIWATCH-3001
/single-ticket UNIWATCH-3002
/single-ticket UNIWATCH-3003

# Phase 4 (mostly sequential)
/single-ticket UNIWATCH-4001
/single-ticket UNIWATCH-4002
/single-ticket UNIWATCH-4003

# Phase 5 (sequential)
/single-ticket UNIWATCH-5001
/single-ticket UNIWATCH-5002
/single-ticket UNIWATCH-5003
/single-ticket UNIWATCH-5004
```

### Automated Execution

```bash
# Execute all tickets in order
/work-on-project UNIWATCH
```

## Test Coverage

**Unit Tests** (8+ tests):
- setup_head_watcher bridge creation
- Worktree tracking initialization
- Debouncer prevents rapid events
- handle_branch_switch updates state
- handle_branch_switch skips unchanged
- BranchSwitchEvent serialization
- Watch command auto-detection
- Additional tests from implementation

**Integration Tests** (4 tests):
- Complete branch switch workflow
- Rapid branch switches debounced
- File changes during branch switch
- Backward compatibility (--worktree flag)

**E2E Tests** (1 script):
- Real developer workflow with actual binary

**Manual Tests** (24 checks):
- Functional testing (8 tests)
- Backward compatibility (4 tests)
- Error scenarios (4 tests)
- NDJSON validation (4 checks)
- Performance (4 metrics)

**Total**: 37+ tests/checks

## Success Criteria

**Ready to merge when**:
- ✅ All 15 tickets completed
- ✅ All unit tests pass (8+)
- ✅ All integration tests pass (4)
- ✅ E2E bash test passes (1)
- ✅ Manual testing checklist complete (24 items)
- ✅ No clippy warnings
- ✅ Documentation updated
- ✅ Backward compatibility verified

## Risk Tracking

**Low Risk**:
- Phase 1-2: Foundation and logic (well-defined, small changes)
- Phase 4: CLI updates (trivial changes)

**Medium Risk**:
- Phase 3: Event loop modifications (complex async coordination)
- UNIWATCH-3002: tokio::select! integration (potential race conditions)

**Mitigation**:
- Comprehensive test coverage
- Integration tests specifically target race conditions
- Manual testing validates real-world scenarios

## Reference Documents

- **Plan**: `.agents/projects/UNIWATCH_unified-watch-command/planning/plan.md`
- **Architecture**: `.agents/projects/UNIWATCH_unified-watch-command/planning/architecture.md`
- **Quality Strategy**: `.agents/projects/UNIWATCH_unified-watch-command/planning/quality-strategy.md`
- **Security Review**: `.agents/projects/UNIWATCH_unified-watch-command/planning/security-review.md`
- **Project Review**: `.agents/projects/UNIWATCH_unified-watch-command/planning/project-review.md`

## Ticket Status Legend

- ⏳ Not Started
- 🚧 In Progress
- ✅ Complete
- ⚠️ Blocked
- ❌ Failed
