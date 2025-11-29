# Quality Strategy: Unified Watch Command

## Testing Philosophy

Focus on the critical path: **branch switches correctly update the worktree_id used for file indexing**.

## Disabled Tests Status

Several UNIWATCH-prefixed unit tests exist in `indexer/mod.rs` but are disabled with
`#[cfg(disabled_postgresql_test)]` due to the SQLite migration. These tests reference the
old PostgreSQL-based `handle_branch_switch` function that was removed.

### Tests to Enable/Rewrite for SQLite (3 tests)

| Test | Line | Issue |
|------|------|-------|
| `test_worktree_tracking_initialization` | 780 | Uses `crate::db::pool::create_pool()` |
| `test_handle_branch_switch_updates_state` | 935 | Calls removed `handle_branch_switch` function |
| `test_handle_branch_switch_skips_if_same_branch` | 1103 | Calls removed `handle_branch_switch` function |

### Already Working Tests (No Changes Needed) (4 tests)

| Test | Line | Status |
|------|------|--------|
| `test_debounced_handler_prevents_rapid_events` | N/A | Works (no DB dependency) |
| `test_branch_switch_event_serialization` | N/A | Works (no DB dependency) |
| `test_dual_watchers_initialize` | N/A | Works (no DB dependency) |
| `test_event_loop_handles_both_sources` | 1496 | Works (NOT disabled) |

**Phase 4 Task 1 must enable/rewrite the 3 disabled tests before integration testing.**

## Test Pyramid

```
        ┌─────────────┐
        │   Manual    │  1 checklist
        └─────────────┘
       ┌───────────────┐
       │  Integration  │  5 tests (including detached HEAD)
       └───────────────┘
      ┌─────────────────┐
      │   Unit Tests    │  7 total (4 working, 3 to enable)
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

### Test 5: Detached HEAD State

```rust
#[tokio::test]
async fn test_detached_head_creates_sha_worktree() {
    // 1. Start watch on main
    // 2. git checkout <commit-sha> (detached HEAD)
    // 3. Verify BranchSwitchEvent with 8-char SHA as new_branch
    // 4. Edit file, verify indexed to SHA-named worktree
}
```

## Manual Testing Checklist

Before merging, verify:

- [ ] Start watch on main, edit file → indexed to main
- [ ] Run `git checkout feature` → "branch_switched" NDJSON event to stdout
- [ ] Edit file on feature → indexed to feature
- [ ] Run `git checkout main` → state updated
- [ ] Edit file on main → indexed to main
- [ ] Rapid switches (3x in 2s) → only final branch indexed
- [ ] Detached HEAD checkout → SHA-named worktree created
- [ ] Database error during switch → warning logged, watch continues

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
| Detached HEAD | Use 8-character commit SHA as branch name |
| Database error in get_or_create_worktree | Log warning, continue with old worktree_id |
| Database error in incremental_update | Log warning, continue (non-fatal) |
| Rapid switches | Debounce with 2-second window, process final state only |

## E2E Test Script Migration

The E2E test script at `tests/e2e/test_unified_watch_workflow.sh` currently uses PostgreSQL.
It must be updated for SQLite:

### Current (PostgreSQL)
```bash
DB_URL="${MAPROOM_DATABASE_URL:-postgresql://maproom:maproom@localhost:5432/maproom}"
psql "$DB_URL" -c "DELETE FROM maproom.repos WHERE name='$REPO_NAME'"
```

### Updated (SQLite)
```bash
# SQLite database location
DB_PATH="${MAPROOM_DATABASE_URL:-$HOME/.maproom/maproom.db}"

# Cleanup using sqlite3 or maproom CLI
sqlite3 "$DB_PATH" "DELETE FROM repos WHERE name='$REPO_NAME';"
# OR use maproom CLI if available:
# cargo run --bin crewchief-maproom -- db cleanup-stale --confirm
```

**Note**: Prefer maproom CLI commands for cleanup when possible to ensure proper handling.

## Test Execution

```bash
# Run existing watch auto-detect tests
cargo test --test watch_auto_detect_test -- --nocapture

# Run new integration tests (after implementation)
cargo test --test unified_watch_test -- --nocapture

# Run all maproom tests
cargo test -p crewchief-maproom

# Check code quality
cargo clippy -p crewchief-maproom

# E2E test (after SQLite migration)
./crates/maproom/tests/e2e/test_unified_watch_workflow.sh
```
