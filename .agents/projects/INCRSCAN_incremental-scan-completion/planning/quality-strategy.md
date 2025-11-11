# Quality Strategy: Incremental Scan Completion

## Testing Philosophy

**Goal:** Ship with confidence, not ceremony.

**Priorities:**
1. **Correctness:** Never skip a scan incorrectly
2. **Regression Prevention:** Don't break existing functionality
3. **Observable Failures:** Easy to debug when things go wrong

**Anti-Patterns to Avoid:**
- Testing implementation details
- Brittle mocks that break on refactors
- Coverage-driven testing (targeting %)
- Tests that don't prevent real bugs

## Risk-Based Testing

### Critical Paths (Must Test)

**1. Skip Decision Logic**
- ✅ Unchanged tree SHA → scan skipped
- ✅ Force flag → always scans
- ✅ Changed tree SHA → full scan
- ✅ No cached state → full scan
- ✅ Git error → fallback to full scan

**2. State Persistence**
- ✅ State saved after successful scan
- ✅ State includes correct tree SHA
- ✅ Stats (files/chunks) tracked correctly
- ✅ Update works on subsequent scans (upsert)

**3. Error Handling**
- ✅ Git failures don't crash scan
- ✅ Database errors don't crash scan
- ✅ State update failures are non-fatal
- ✅ All errors log clearly

### Medium Risk (Should Test)

**1. Worktree Management**
- Get or create worktree ID
- Handle new repositories
- Handle new worktrees

**2. Metrics Collection**
- Progress tracker integration
- Stats accuracy after scan

**3. Parallel vs Sequential**
- Both modes update state
- Both modes respect tree SHA check

### Low Risk (Spot Check)

**1. Logging**
- Correct messages for each decision
- Debug information available

**2. Help Text**
- Incremental mode documented
- Force flag explained

## Test Coverage Matrix

| Scenario | Type | Location | Priority |
|----------|------|----------|----------|
| Scan unchanged worktree → skip | Integration | `tests/incremental_scan.rs` | P0 |
| Scan changed worktree → full | Integration | `tests/incremental_scan.rs` | P0 |
| Scan with --force → full | Integration | `tests/incremental_scan.rs` | P0 |
| First scan saves state | Integration | `tests/incremental_scan.rs` | P0 |
| Second scan reads state | Integration | `tests/incremental_scan.rs` | P0 |
| Git error → fallback | Integration | `tests/error_handling.rs` | P1 |
| DB error → fallback | Integration | `tests/error_handling.rs` | P1 |
| State update error → warn | Integration | `tests/error_handling.rs` | P1 |
| Parallel scan updates state | Integration | `tests/parallel_scan.rs` | P2 |
| Stats tracking accuracy | Unit | `src/main.rs` | P2 |
| Logging output | Manual | Genetic optimizer | P2 |

## Integration Tests

### Test Setup

**Database:**
- Use test database (`maproom_test`)
- Apply migrations before each test
- Clean state between tests

**Git Repository:**
- Create temp git repo in `/tmp`
- Add test files, commit
- Manipulate tree for different scenarios

**Approach:**
```rust
#[tokio::test]
async fn test_scan_skip_unchanged() {
    // Setup: temp repo + database
    let temp_repo = create_test_repo().await;
    let db = setup_test_db().await;

    // First scan: should process all files
    let result1 = scan_worktree(&db, "test", "main", &temp_repo, "HEAD", ...).await?;
    assert!(result1.files_processed > 0);

    // Check state saved
    let state = get_index_state(&db, "test", "main").await?;
    assert!(state.last_tree_sha.is_some());

    // Second scan: should skip (no changes)
    let result2 = scan_worktree(&db, "test", "main", &temp_repo, "HEAD", ...).await?;
    assert_eq!(result2.files_processed, 0); // Skipped!
}
```

### Key Test Scenarios

**Test 1: Unchanged Tree Skip**
```
Setup:
  - Create repo with 10 files
  - Commit (tree SHA: abc123)
  - Scan once

Action:
  - Scan again (same tree)

Expect:
  - Early return before file walk
  - Log: "No changes detected"
  - Duration: < 100ms
```

**Test 2: Changed Tree Full Scan**
```
Setup:
  - Scan repo (tree SHA: abc123)
  - Modify a file
  - Commit (tree SHA: def456)

Action:
  - Scan again

Expect:
  - Full scan executes
  - All files processed
  - State updated to def456
```

**Test 3: Force Flag Override**
```
Setup:
  - Scan repo
  - No changes

Action:
  - Scan with --force flag

Expect:
  - Full scan despite no changes
  - Log: "Force flag enabled"
  - State updated with same tree SHA
```

**Test 4: First-Time Scan**
```
Setup:
  - Clean database (no worktree_index_state)

Action:
  - Scan repo

Expect:
  - Full scan executes
  - State record created
  - last_tree_sha populated
```

**Test 5: Git Error Fallback**
```
Setup:
  - Not a git repo (no .git directory)

Action:
  - Attempt scan

Expect:
  - Warning logged
  - Full scan executes
  - No state saved (no tree SHA)
```

**Test 6: Database Error Fallback**
```
Setup:
  - Mock database to fail on state query

Action:
  - Scan repo

Expect:
  - Warning logged
  - Full scan executes
  - Continues despite query failure
```

**Test 7: State Update Failure**
```
Setup:
  - Scan completes successfully
  - Mock database to fail on UPDATE

Action:
  - Observe behavior

Expect:
  - Scan completes
  - Warning logged about state update
  - Returns success (non-fatal)
```

## Manual Testing

### Genetic Optimizer Validation

**Purpose:** Real-world integration test with 12 worktrees

**Steps:**
1. Clear existing state: `DELETE FROM maproom.worktree_index_state;`
2. Run: `pnpm tsx packages/cli/scripts/run-genetic-optimizer-ultra.ts`
3. Observe:
   - First worktree: Full scan (~30 seconds)
   - Remaining 11 worktrees: Skipped (~1 second each)
   - Total time: < 2 minutes (vs 24+ hours)

**Success Criteria:**
- ✅ All worktrees show "No changes detected"
- ✅ State table populated for each worktree
- ✅ Tree SHAs match across all worktrees
- ✅ Competition setup completes successfully

### Developer Workflow Test

**Scenario:** Normal development flow

**Steps:**
1. Edit a file in main worktree
2. Run: `crewchief-maproom scan`
3. Observe: Full scan (changed file)
4. Run scan again (no changes)
5. Observe: Skip (< 1 second)
6. Checkout different branch
7. Run scan
8. Observe: Full scan (different tree)

**Success Criteria:**
- ✅ Scans are fast when no changes
- ✅ Changes are always detected
- ✅ Logging makes behavior clear

### Error Condition Tests

**Test: Corrupt Git Repo**
```bash
# Break git repo
rm -rf /workspace/.git/objects/pack/*

# Try to scan
crewchief-maproom scan

# Expect: Warning + fallback to full scan
```

**Test: Read-Only Database**
```bash
# Make database read-only
chmod 444 maproom.db

# Try to scan
crewchief-maproom scan

# Expect: Scan succeeds, warning about state update
```

## Regression Testing

### Existing Behavior Must Not Change

**Test Matrix:**
| Command | Expected Behavior | Test Method |
|---------|------------------|-------------|
| `scan` | Full scan first time | Integration test |
| `scan --force` | Always full scan | Integration test |
| `scan --parallel` | Parallel processing | Integration test |
| `scan --concurrency 8` | Respects concurrency | Manual |
| `scan --verbose` | Detailed output | Manual |

### Backward Compatibility

**Empty State Table:**
- Existing databases may have empty `worktree_index_state`
- Must handle gracefully (treat as first-time)
- Must not break on NULL values

**Missing Tree SHA:**
- Git commands might fail
- Must fallback to full scan
- Must not crash or skip incorrectly

## Performance Benchmarks

### Before/After Comparison

**Scenario: Unchanged Worktree (typical case)**
| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Duration | 120-180 minutes | 5-10ms | 720,000x |
| Chunks updated | 474,419 | 0 | N/A |
| API calls | ~474K | 0 | 100% reduction |
| DB writes | ~474K UPDATEs | 0 | 100% reduction |

**Scenario: Changed Files**
| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Duration | Same | Same + 10ms | Negligible |
| Correctness | Same | Same | No regression |

**Scenario: First-Time Scan**
| Metric | Before | After | Difference |
|--------|--------|-------|------------|
| Duration | Baseline | +10ms | <0.1% overhead |
| Correctness | Same | Same | No regression |

### Performance Regression Prevention

**Red Flags:**
- ⚠️ Scans taking longer than before
- ⚠️ Incremental mode slower than full mode
- ⚠️ Database query time increasing

**Monitoring:**
```rust
// Add timing to main.rs
let start = Instant::now();
let result = check_and_scan(...).await?;
let duration = start.elapsed();

if duration > threshold {
    tracing::warn!("Scan took {:.2}s (expected < 1s)", duration.as_secs_f64());
}
```

## Test Implementation Plan

### Phase 1: Core Integration Tests (P0)
**File:** `crates/maproom/tests/incremental_scan_integration.rs`

**Tests:**
1. `test_unchanged_tree_skip` - Verify skip logic
2. `test_changed_tree_scan` - Verify change detection
3. `test_force_flag_override` - Verify force behavior
4. `test_first_scan_state` - Verify state creation
5. `test_concurrent_scans` - Verify race handling

**Estimated Time:** 2-4 hours to write + debug

### Phase 2: Error Handling Tests (P1)
**File:** `crates/maproom/tests/scan_error_handling.rs`

**Tests:**
1. `test_git_failure_fallback` - Verify safe fallback
2. `test_db_query_failure` - Verify resilience
3. `test_state_update_failure` - Verify non-fatal

**Estimated Time:** 1-2 hours

### Phase 3: Manual Validation (P0)
**Activity:** Run genetic optimizer

**Time:** 30 minutes

**Verification:**
- All worktrees indexed
- Correct skip behavior
- Performance improvement confirmed

## Definition of Done

### Code Complete
- ✅ Tree SHA check implemented
- ✅ State persistence added
- ✅ Error handling verified
- ✅ Logging added

### Tests Pass
- ✅ All P0 integration tests pass
- ✅ All P1 error tests pass
- ✅ No regression in existing tests
- ✅ Manual validation successful

### Documentation
- ✅ Code comments explain logic
- ✅ CHANGELOG updated
- ✅ README reflects new behavior

### Performance
- ✅ Unchanged scans < 1 second
- ✅ No regression in full scan speed
- ✅ Genetic optimizer completes in < 2 minutes

## Acceptance Criteria

**Must Have:**
1. Unchanged worktrees skip scanning (verified)
2. Changed worktrees process correctly (verified)
3. Force flag works as before (verified)
4. Error cases handled safely (verified)
5. State persists after every scan (verified)

**Nice to Have:**
- Metrics dashboard for skip rate
- CLI flag to force state rebuild
- Diagnostic command for state inspection

**Explicitly Out of Scope:**
- True incremental scanning (git diff-tree)
- Remote state caching
- Predictive indexing
- Parallel tree SHA checks
