# Quality Strategy: Unified Watch Command

## Testing Philosophy

Focus on the critical path: **branch switches correctly update the worktree_id used for file indexing**.

## Test Pyramid

```
        ┌─────────────┐
        │   Manual    │  1 checklist
        └─────────────┘
       ┌───────────────┐
       │  Integration  │  4 tests
       └───────────────┘
      ┌─────────────────┐
      │   Unit Tests    │  (existing)
      └─────────────────┘
```

## Integration Tests

Location: `crates/maproom/tests/unified_watch_test.rs`

### Test 1: Complete Branch Switch Workflow

```rust
#[tokio::test]
async fn test_complete_branch_switch_workflow() {
    // 1. Start watch on main
    // 2. Edit file, verify indexed to main
    // 3. git checkout feature
    // 4. Edit file, verify indexed to feature
}
```

### Test 2: Rapid Branch Switches Debounced

```rust
#[tokio::test]
async fn test_rapid_branch_switches_debounced() {
    // 1. Start watch
    // 2. git checkout b1, b2, b3 rapidly
    // 3. Wait for debounce (2s)
    // 4. Verify only b3 is indexed
}
```

### Test 3: File Changes During Branch Switch

```rust
#[tokio::test]
async fn test_file_changes_during_branch_switch() {
    // 1. Start watch
    // 2. Spawn: git checkout feature
    // 3. Immediately edit file
    // 4. Verify file is indexed (to either branch, but not lost)
}
```

### Test 4: Backward Compatibility

```rust
#[tokio::test]
async fn test_worktree_flag_backward_compatible() {
    // 1. Run watch with --worktree flag
    // 2. Verify deprecation warning
    // 3. Verify auto-detection is used
}
```

## Manual Testing Checklist

Before merging, verify:

- [ ] Start watch on main, edit file → indexed to main
- [ ] Run `git checkout feature` → "branch_switched" NDJSON event
- [ ] Edit file on feature → indexed to feature
- [ ] Run `git checkout main` → state updated
- [ ] Edit file on main → indexed to main
- [ ] Rapid switches (3x in 2s) → only final branch indexed

## Acceptance Criteria

### Functional

- [ ] Branch switches detected within 2 seconds
- [ ] File changes after switch index to correct worktree
- [ ] Rapid switches debounced (2 second window)
- [ ] BranchSwitchEvent NDJSON emitted
- [ ] No file events lost during switch

### Quality Gates

- [ ] All integration tests pass
- [ ] Manual testing checklist complete
- [ ] No clippy warnings in modified code
- [ ] Existing tests still pass

## Error Scenarios

| Scenario | Expected Behavior |
|----------|-------------------|
| .git/HEAD deleted | Log warning, continue file watching |
| Detached HEAD | Use short commit SHA as branch name |
| Database unavailable | Log error, retry on next event |
| Rapid switches | Debounce, process final state only |

## Test Execution

```bash
# Run integration tests
cargo test --test unified_watch_test

# Run all maproom tests
cargo test -p crewchief-maproom

# Check code quality
cargo clippy -p crewchief-maproom
```
