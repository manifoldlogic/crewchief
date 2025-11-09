# Implementation Plan: Automatic Branch Switch Detection

## Project Overview

**Goal**: Automatically detect branch switches and trigger incremental indexing
**Dependencies**: BRANCHX must be complete
**Timeline**: 3-4 days

## Phase 1: File Watcher Implementation (Day 1)

### Step 1.1: Add notify Dependency

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/Cargo.toml`

```toml
[dependencies]
notify = "5.0"
ctrlc = "3.2"  # For graceful shutdown
```

**Acceptance Criteria**: Dependencies added successfully

### Step 1.2: Implement BranchWatcher

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/watcher.rs` (new)

**Core struct and methods**:
- `BranchWatcher::new()`
- `BranchWatcher::start()`
- `BranchWatcher::watch_loop()`
- `get_current_branch()`

**Acceptance Criteria**:
- Watcher detects .git/HEAD changes
- Current branch extracted correctly
- Unit tests pass

### Step 1.3: Testing

**Agent**: unit-test-runner

**Tests**:
- `test_parse_branch_ref`
- `test_parse_detached_head`
- `test_watcher_detects_head_change`

**Deliverable**: File watcher working

---

## Phase 2: Branch Switch Handler (Day 2)

### Step 2.1: Implement Handler

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/watcher.rs`

**Method**: `BranchWatcher::handle_branch_switch()`

**Logic**:
1. Get current branch name
2. Get/create worktree
3. Call `incremental_update()` (from BRANCHX)
4. Log results

**Acceptance Criteria**:
- Handler calls incremental_update
- Errors logged, watcher continues
- Integration test passes

### Step 2.2: Error Handling

**Agent**: rust-indexer-engineer

**Add**:
- Retry logic (3 attempts)
- Graceful degradation (continue watching on error)
- Logging at appropriate levels

**Acceptance Criteria**:
- Errors don't crash watcher
- Retries work correctly
- Logging comprehensive

### Step 2.3: Testing

**Agent**: unit-test-runner

**Tests**:
- `test_auto_update_on_switch`
- `test_handler_retries_on_error`
- `test_handler_continues_after_error`

**Deliverable**: Auto-update working

---

## Phase 3: CLI Command (Day 3)

### Step 3.1: Add Watch Command

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/cli.rs`

**Add**:
- `WatchArgs` struct
- `watch_command()` async fn
- Graceful shutdown (Ctrl+C)

**Usage**:
```bash
maproom watch --repo /path/to/repo [--verbose]
```

**Acceptance Criteria**:
- Command starts watcher
- Ctrl+C shuts down gracefully
- Logging shows activity

### Step 3.2: Integration Testing

**Agent**: unit-test-runner

**E2E tests**:
- `test_cli_watch_command`
- `test_graceful_shutdown`
- `test_multiple_branch_switches`

### Step 3.3: Performance Testing

**Agent**: unit-test-runner

**Verify**:
- CPU usage <5% while idle
- Memory usage <20MB
- Detection latency <1s

**Deliverable**: CLI command working

---

## Phase 4: Documentation (Day 4)

### Step 4.1: User Documentation

**Agent**: general-purpose

**Files**:
- `docs/features/automatic-indexing.md` (new)
- `packages/maproom-mcp/README.md` (update)
- `CHANGELOG.md` (add entry)

**Content**:
- Usage guide
- Troubleshooting
- Performance tuning

### Step 4.2: Code Documentation

**Agent**: general-purpose

**Add**:
- Rustdoc comments to `watcher.rs`
- Examples in doc comments
- Module-level documentation

**Deliverable**: Documentation complete

---

## Agent Assignments

1. **rust-indexer-engineer** - Watcher implementation, CLI command
2. **general-purpose** - Documentation
3. **unit-test-runner** - Test execution
4. **verify-ticket** - Final verification
5. **commit-ticket** - Create commit

## Testing Strategy

### Unit Tests
- Branch name parsing
- File watcher event handling
- Error scenarios

### Integration Tests
- Full workflow (switch → detect → update)
- Error handling (DB errors, git errors)
- Concurrent switches

### E2E Tests
- CLI command usage
- Multiple rapid switches
- Long-running stability

### Performance Tests
- CPU usage while idle
- Memory usage
- Detection latency

## Success Criteria

- [ ] File watcher detects branch switches
- [ ] Auto-update triggers incremental_update
- [ ] CLI command works
- [ ] Graceful shutdown (Ctrl+C)
- [ ] Error handling robust
- [ ] CPU <5% idle, Memory <20MB
- [ ] 100% detection reliability
- [ ] Documentation complete

## Risk Mitigation

**Backup**: Not needed (no schema changes)
**Rollback**: Simply don't use `maproom watch` command

## Acceptance Checklist

- [ ] All phases complete
- [ ] All tests passing
- [ ] Performance benchmarks met
- [ ] CLI command documented
- [ ] Manual testing successful
- [ ] Resource usage verified

**Timeline**: 3-4 days (1 buffer day)

**Completes**: Branch-aware indexing sequence (BLOBSHA → BRANCHX → BRWATCH)
