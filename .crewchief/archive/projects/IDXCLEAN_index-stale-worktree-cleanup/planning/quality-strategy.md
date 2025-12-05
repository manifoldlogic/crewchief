# Quality Strategy: Index Stale Worktree Cleanup

## Testing Philosophy

**Core Principle:** Prevent data loss through comprehensive testing.

This is a **data deletion** system. The highest risk is accidentally deleting valid worktrees. Our testing strategy prioritizes **safety validation** over code coverage metrics. Every test should answer: "How does this prevent data loss?"

**MVP Mindset:**
- Tests must prevent rework, not ceremonially reach coverage targets
- Focus on critical paths: detection accuracy, deletion safety, transaction integrity
- Integration tests more valuable than unit tests (database interactions)
- Manual verification complements automated tests (inspect before production use)

---

## Test Strategy Overview

### Test Pyramid

```
                 ┌─────────────────┐
                 │  Manual Testing │  ← Verify on real database
                 │   (Production)  │
                 └─────────────────┘
                        ▲
                 ┌──────────────────┐
                 │ Integration Tests│  ← Database + disk validation
                 │   (Core Safety)  │
                 └──────────────────┘
                        ▲
                 ┌──────────────────┐
                 │   Unit Tests     │  ← Detection logic, report formatting
                 │  (Fast Feedback) │
                 └──────────────────┘
```

**Test Distribution:**
- Unit tests: 30% (fast feedback, logic validation)
- Integration tests: 60% (database interactions, safety verification)
- Manual tests: 10% (production validation, edge cases)

---

## Critical Test Paths

### Path 1: Detection Accuracy

**Goal:** Ensure we correctly identify stale vs. valid worktrees.

**Test Coverage:**

1. **Stale worktree detection:**
   - Worktree with non-existent abs_path → Detected as stale
   - Multiple stale worktrees → All detected
   - Empty database → No stale worktrees found

2. **Valid worktree preservation:**
   - Worktree with valid abs_path → Not detected as stale
   - Worktree with symlink abs_path (target exists) → Not detected as stale
   - Recently created worktree → Not detected as stale

3. **Edge cases:**
   - Worktree path exists but is not a directory → Detected as stale
   - Worktree path is a file (not directory) → Detected as stale
   - Worktree path has special characters → Correctly validated
   - Worktree path is relative (should be absolute) → Handle gracefully

**Critical Tests:**

```rust
#[tokio::test]
async fn test_detects_stale_worktree() {
    let db = setup_test_db().await;

    // Create worktree with non-existent path
    let worktree_id = db.insert_worktree(Worktree {
        name: "deleted-branch".into(),
        abs_path: "/tmp/nonexistent/path".into(),
        // ...
    }).await.unwrap();

    let detector = StaleWorktreeDetector::new(db);
    let stale = detector.detect_stale_worktrees().await.unwrap();

    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].id, worktree_id);
    assert!(!stale[0].exists);
}

#[tokio::test]
async fn test_preserves_valid_worktree() {
    let db = setup_test_db().await;
    let temp_dir = tempfile::tempdir().unwrap();

    // Create worktree with valid path
    let worktree_id = db.insert_worktree(Worktree {
        name: "main".into(),
        abs_path: temp_dir.path().to_string_lossy().into(),
        // ...
    }).await.unwrap();

    let detector = StaleWorktreeDetector::new(db);
    let stale = detector.detect_stale_worktrees().await.unwrap();

    // Valid worktree should NOT be in stale list
    assert!(stale.iter().all(|s| s.id != worktree_id));
}
```

### Path 2: Deletion Safety

**Goal:** Ensure deletion only removes stale worktrees, never valid ones.

**Test Coverage:**

1. **Safe deletion:**
   - Delete single stale worktree → Successfully removed from database
   - Delete multiple stale worktrees → All removed
   - Multi-worktree chunks preserved → Chunk in 2 worktrees, delete 1, verify chunk preserved with updated worktree_ids
   - Single-worktree chunks garbage collected → Chunk in 1 worktree, delete it, verify chunk deleted

2. **Transaction integrity:**
   - Deletion fails mid-transaction → All changes rolled back
   - Database constraint violation → Transaction aborted
   - Network error during commit → No partial state

3. **Audit trail:**
   - Every deletion logged → Tracing captures all deletions
   - Failed deletions logged → Errors recorded with context
   - Dry-run logged → No actual changes made

**Critical Tests:**

```rust
#[tokio::test]
async fn test_deletes_only_stale_worktrees() {
    let db = setup_test_db().await;

    // Create 1 valid, 2 stale worktrees
    let valid_id = create_valid_worktree(&db).await;
    let stale1_id = create_stale_worktree(&db, "/tmp/stale1").await;
    let stale2_id = create_stale_worktree(&db, "/tmp/stale2").await;

    let stale = vec![
        StaleWorktree { id: stale1_id, exists: false, /* ... */ },
        StaleWorktree { id: stale2_id, exists: false, /* ... */ },
    ];

    let cleaner = WorktreeCleaner::new(db.clone(), false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    assert_eq!(report.deleted_count, 2);

    // Verify: stale worktrees gone
    assert!(db.get_worktree(stale1_id).await.is_err());
    assert!(db.get_worktree(stale2_id).await.is_err());

    // Verify: valid worktree still exists
    assert!(db.get_worktree(valid_id).await.is_ok());
}

#[tokio::test]
async fn test_transaction_rollback_on_error() {
    let db = setup_test_db().await;

    // Create 2 stale worktrees, but second has invalid ID
    let stale1_id = create_stale_worktree(&db, "/tmp/stale1").await;

    let stale = vec![
        StaleWorktree { id: stale1_id, exists: false, /* ... */ },
        StaleWorktree { id: 999999, exists: false, /* ... */ }, // Invalid ID
    ];

    let cleaner = WorktreeCleaner::new(db.clone(), false);
    let result = cleaner.cleanup_stale_worktrees(stale).await;

    // Transaction should fail
    assert!(result.is_err());

    // Verify: first worktree NOT deleted (rollback worked)
    assert!(db.get_worktree(stale1_id).await.is_ok());
}
```

### Path 3: CLI Usability

**Goal:** Ensure CLI provides clear feedback and prevents accidents.

**Test Coverage:**

1. **Dry-run behavior:**
   - Default execution is dry-run → No changes made
   - Dry-run shows what would be deleted → Report matches reality
   - Dry-run repeatable → No side effects

2. **Confirmation requirement:**
   - `--confirm` flag required for deletion → Without flag, dry-run only
   - Clear warning messages → User understands impact
   - Confirmation text matches actual operation → No surprises

3. **Error handling:**
   - Database connection failure → Clear error message
   - No stale worktrees found → Success message (not error)
   - Partial failure → Report shows succeeded and failed

**Critical Tests:**

```rust
#[tokio::test]
async fn test_cli_default_is_dry_run() {
    let db = setup_test_db().await;
    let stale_id = create_stale_worktree(&db, "/tmp/stale").await;

    // Run command without --confirm
    let cmd = CleanupStaleCommand { confirm: false, verbose: false };
    cmd.execute(&test_config()).await.unwrap();

    // Verify: worktree still exists (dry-run)
    assert!(db.get_worktree(stale_id).await.is_ok());
}

#[tokio::test]
async fn test_cli_confirm_actually_deletes() {
    let db = setup_test_db().await;
    let stale_id = create_stale_worktree(&db, "/tmp/stale").await;

    // Run command with --confirm
    let cmd = CleanupStaleCommand { confirm: true, verbose: false };
    cmd.execute(&test_config()).await.unwrap();

    // Verify: worktree deleted
    assert!(db.get_worktree(stale_id).await.is_err());
}
```

### Path 4: Watch Integration Safety

**Goal:** Ensure automatic cleanup doesn't interfere with indexing.

**Test Coverage:**

1. **Non-interference:**
   - Cleanup runs while indexer idle → No conflicts
   - Cleanup defers when indexer busy → Indexing completes first
   - Cleanup fails gracefully → Watch continues normally

2. **Rate limiting:**
   - Cleanup skips if ran recently → No redundant operations
   - Cleanup respects configured interval → Doesn't run too often
   - Cleanup timestamps tracked correctly → Rate limit enforced

3. **Background execution:**
   - Cleanup doesn't block watch startup → Watch starts immediately
   - Cleanup runs asynchronously → File events processed concurrently
   - Cleanup can be cancelled → Watch shutdown cleans up tasks

**Critical Tests:**

```rust
#[tokio::test]
async fn test_watch_startup_cleanup_runs_in_background() {
    let watch = WatchManager::new(test_config()).await;
    let start = Instant::now();

    // Start watch (should not block on cleanup)
    let watch_handle = tokio::spawn(async move {
        watch.start_watch(PathBuf::from("/tmp/test")).await
    });

    // Verify: watch started quickly (< 200ms)
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(start.elapsed() < Duration::from_millis(300));

    watch_handle.abort();
}

#[tokio::test]
async fn test_periodic_cleanup_defers_when_indexer_busy() {
    let db = setup_test_db().await;
    let indexer = Arc::new(MockIndexer::new());
    let scheduler = CleanupScheduler::new(db, indexer.clone());

    // Simulate busy indexer
    indexer.set_busy(true);

    // Try to run cleanup
    let result = scheduler.run_cleanup_if_safe().await;

    // Verify: cleanup deferred (no error, but nothing deleted)
    assert!(result.is_ok());
    // Check logs: "Deferring cleanup: indexer is busy"
}
```

---

## Integration Test Suite

### Test Fixtures

**Database fixtures (SQLite):**
```rust
async fn setup_test_db() -> SqliteStore {
    // Create temporary in-memory SQLite database
    let store = SqliteStore::new_test().await.expect("create test store");
    // Migrations run automatically on store creation
    store
}

async fn create_stale_worktree(store: &SqliteStore, path: &str) -> i64 {
    store.insert_worktree(Worktree {
        name: format!("stale-{}", uuid::Uuid::new_v4()),
        abs_path: path.into(),
        repo_id: 1,
        commit_sha: "abc123".into(),
    }).await.unwrap()
}

async fn create_valid_worktree(store: &SqliteStore) -> i64 {
    let temp_dir = tempfile::tempdir().unwrap();
    store.insert_worktree(Worktree {
        name: "main".into(),
        abs_path: temp_dir.path().to_string_lossy().into(),
        repo_id: 1,
        commit_sha: "abc123".into(),
    }).await.unwrap()
}
```

**Disk fixtures:**
```rust
fn create_test_worktree_on_disk() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();
    dir
}
```

### Integration Test Scenarios

**Scenario 1: End-to-end cleanup**
```rust
#[tokio::test]
async fn test_e2e_cleanup_workflow() {
    // Setup: 5 valid worktrees, 95 stale worktrees
    let db = setup_test_db().await;
    for i in 0..5 {
        create_valid_worktree(&db).await;
    }
    for i in 0..95 {
        create_stale_worktree(&db, &format!("/tmp/stale-{}", i)).await;
    }

    // Detect stale
    let detector = StaleWorktreeDetector::new(db.clone());
    let stale = detector.detect_stale_worktrees().await.unwrap();
    assert_eq!(stale.len(), 95);

    // Delete stale
    let cleaner = WorktreeCleaner::new(db.clone(), false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();
    assert_eq!(report.deleted_count, 95);

    // Verify final state
    let remaining = db.count_worktrees().await.unwrap();
    assert_eq!(remaining, 5);
}
```

**Scenario 2: Cleanup with chunks**
```rust
#[tokio::test]
async fn test_cleanup_cascades_to_chunks() {
    let db = setup_test_db().await;

    // Create stale worktree with 100 chunks
    let stale_id = create_stale_worktree(&db, "/tmp/stale").await;
    for i in 0..100 {
        db.insert_chunk(Chunk {
            worktree_id: stale_id,
            relpath: format!("file{}.rs", i),
            // ...
        }).await.unwrap();
    }

    // Verify chunks exist
    let chunk_count = db.count_chunks_for_worktree(stale_id).await.unwrap();
    assert_eq!(chunk_count, 100);

    // Delete worktree
    let stale = vec![StaleWorktree { id: stale_id, exists: false, /* ... */ }];
    let cleaner = WorktreeCleaner::new(db.clone(), false);
    cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify chunks also deleted (CASCADE)
    let chunk_count = db.count_chunks_for_worktree(stale_id).await.unwrap();
    assert_eq!(chunk_count, 0);
}
```

**Scenario 3: Parallel detection**
```rust
#[tokio::test]
async fn test_parallel_detection_performance() {
    let db = setup_test_db().await;

    // Create 100 stale worktrees
    for i in 0..100 {
        create_stale_worktree(&db, &format!("/tmp/stale-{}", i)).await;
    }

    let start = Instant::now();
    let detector = StaleWorktreeDetector::new(db);
    let stale = detector.detect_stale_worktrees().await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(stale.len(), 100);
    // Should complete in < 1 second (parallel validation)
    assert!(elapsed < Duration::from_secs(1));
}
```

**Scenario 4: Multi-Worktree Chunk Safety (SQLite)**
```rust
#[tokio::test]
async fn test_multi_worktree_chunk_preserved_on_partial_deletion() {
    let store = setup_test_db().await;

    // Create 2 worktrees sharing a chunk via chunk_worktrees junction table
    let worktree_a_id = create_valid_worktree(&store).await;
    let worktree_b_id = create_stale_worktree(&store, "/tmp/stale-b").await;

    // Create a file and chunk
    let file_id = store.insert_file(File {
        relpath: "shared.rs".into(),
        worktree_id: worktree_a_id,
        // ...
    }).await.unwrap();

    let chunk_id = store.insert_chunk(Chunk {
        file_id,
        // ...
    }).await.unwrap();

    // Associate chunk with BOTH worktrees via junction table
    store.add_chunk_worktree(chunk_id, worktree_a_id).await.unwrap();
    store.add_chunk_worktree(chunk_id, worktree_b_id).await.unwrap();

    // Verify chunk exists in both worktrees
    let worktree_ids = store.get_chunk_worktrees(chunk_id).await.unwrap();
    assert_eq!(worktree_ids.len(), 2);

    // Delete stale worktree B (A is still valid)
    let stale = vec![StaleWorktree { id: worktree_b_id, exists: false, /* ... */ }];
    let cleaner = WorktreeCleaner::new(store.clone(), false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();
    assert_eq!(report.deleted_count, 1);

    // Critical assertion: chunk MUST still exist
    let chunk = store.get_chunk(chunk_id).await.unwrap();
    assert!(chunk.is_some(), "Multi-worktree chunk should not be deleted");

    // Critical assertion: chunk_worktrees junction table updated correctly
    let remaining_worktrees = store.get_chunk_worktrees(chunk_id).await.unwrap();
    assert_eq!(remaining_worktrees.len(), 1, "Chunk should have 1 worktree after removal");
    assert_eq!(remaining_worktrees[0], worktree_a_id);

    // Verify worktree A (valid) still exists
    assert!(store.get_worktree(worktree_a_id).await.is_ok());
}
```

**Scenario 5: Garbage Collection Accuracy (SQLite)**
```rust
#[tokio::test]
async fn test_single_worktree_chunk_garbage_collected() {
    let store = setup_test_db().await;

    // Create a stale worktree with a chunk that ONLY belongs to this worktree
    let stale_id = create_stale_worktree(&store, "/tmp/stale").await;

    let file_id = store.insert_file(File {
        relpath: "orphan.rs".into(),
        worktree_id: stale_id,
        // ...
    }).await.unwrap();

    let chunk_id = store.insert_chunk(Chunk {
        file_id,
        // ...
    }).await.unwrap();

    // Associate chunk with ONLY this worktree via junction table
    store.add_chunk_worktree(chunk_id, stale_id).await.unwrap();

    // Verify chunk exists before deletion
    assert!(store.get_chunk(chunk_id).await.is_ok());

    // Delete the stale worktree
    let stale = vec![StaleWorktree { id: stale_id, exists: false, /* ... */ }];
    let cleaner = WorktreeCleaner::new(store.clone(), false);
    let report = cleaner.cleanup_stale_worktrees(stale).await.unwrap();

    // Verify report shows chunk was cleaned
    assert_eq!(report.deleted_count, 1);
    assert_eq!(report.chunks_cleaned, 1);

    // Critical assertion: chunk MUST be deleted (garbage collection)
    // Cleanup removes chunk_worktrees entry, then garbage collects orphaned chunks
    let chunk = store.get_chunk(chunk_id).await;
    assert!(chunk.is_err() || chunk.unwrap().is_none(),
            "Single-worktree chunk should be garbage collected");

    // Verify worktree also deleted
    assert!(store.get_worktree(stale_id).await.is_err());
}
```

---

## Unit Test Suite

### Detection Logic Tests

```rust
#[test]
fn test_stale_worktree_struct() {
    let stale = StaleWorktree {
        id: 1,
        repo_id: 1,
        name: "test".into(),
        abs_path: "/tmp/test".into(),
        exists: false,
        chunk_count: 42,
    };

    assert!(!stale.exists);
    assert_eq!(stale.chunk_count, 42);
}
```

### Report Formatting Tests

```rust
#[test]
fn test_cleanup_report_formatting() {
    let report = CleanupReport {
        total_stale: 100,
        deleted_count: 95,
        failed_count: 5,
        deleted_ids: vec![1, 2, 3],
        failed_deletions: vec![(4, anyhow!("error"))],
    };

    assert_eq!(report.success_rate(), 0.95);
    assert_eq!(report.total_chunks_deleted(), /* ... */);
}
```

### Configuration Tests

```rust
#[test]
fn test_cleanup_config_defaults() {
    let config = CleanupConfig::default();

    assert_eq!(config.cleanup_interval, Duration::from_secs(1800));
    assert_eq!(config.cleanup_cooldown, Duration::from_secs(900));
    assert_eq!(config.batch_size, 50);
    assert!(config.auto_cleanup);
}
```

---

## Manual Testing Checklist

### Pre-Production Validation

**Test on staging database:**
1. [ ] Run dry-run on real database with 100+ stale worktrees
2. [ ] Verify reported stale worktrees are actually stale (inspect paths)
3. [ ] Check chunk counts are reasonable
4. [ ] Run with `--confirm` and verify deletion
5. [ ] Verify search quality improves after cleanup

**Test watch integration:**
1. [ ] Start watch with auto-cleanup enabled
2. [ ] Verify watch starts without delay
3. [ ] Monitor background cleanup in logs
4. [ ] Verify indexing continues during cleanup
5. [ ] Test cleanup rate limiting (check logs for deferral)

**Edge case validation:**
1. [ ] Test with empty database (no stale worktrees)
2. [ ] Test with all-valid database (no deletions)
3. [ ] Test with network-mounted paths (if applicable)
4. [ ] Test with special characters in paths
5. [ ] Test with very long paths (> 255 characters)

### Production Rollout

**Phase 1: Manual cleanup only**
1. [ ] Deploy CLI command to production
2. [ ] Run dry-run on production database
3. [ ] Review results with team
4. [ ] Execute cleanup with confirmation
5. [ ] Monitor search quality improvement

**Phase 2: Watch integration**
1. [ ] Enable auto-cleanup in watch (staged rollout)
2. [ ] Monitor performance metrics
3. [ ] Watch for error logs
4. [ ] Verify no interference with indexing
5. [ ] Gradually increase cleanup frequency if stable

---

## Risk Mitigation

### Risk: Accidentally delete valid worktree

**Mitigation strategies:**
1. **Validation:** Disk existence check before marking stale
2. **Dry-run default:** User must explicitly confirm deletion
3. **Audit logging:** All deletions logged with full context
4. **Incremental rollout:** Test on staging first, monitor production
5. **Recovery path:** Database backups allow restoration if needed

### Risk: Cleanup interferes with indexing

**Mitigation strategies:**
1. **Priority scheduling:** Indexing takes precedence over cleanup
2. **Safety checks:** Defer cleanup if indexer busy
3. **Rate limiting:** Cleanup runs max once per 15 minutes
4. **Background execution:** Cleanup runs in separate tokio task
5. **Graceful failure:** Cleanup errors don't break watch

### Risk: Performance degradation

**Mitigation strategies:**
1. **Parallel validation:** Use tokio for async disk checks
2. **Batch processing:** Limit worktrees checked per cycle
3. **Caching:** Cache validation results (15 min TTL)
4. **Profiling:** Measure actual impact before shipping
5. **Configuration:** Allow users to tune parameters

### Risk: Database corruption

**Mitigation strategies:**
1. **Transaction safety:** All deletes in single transaction
2. **Foreign key CASCADE:** Database handles referential integrity
3. **Constraint validation:** Database enforces data consistency
4. **Backup verification:** Test restore from backup before cleanup
5. **Monitoring:** Alert on unexpected database errors

---

## Success Metrics

### Functional Metrics

**Detection accuracy:**
- [ ] 100% of stale worktrees detected (path doesn't exist)
- [ ] 0% false positives (valid worktrees marked stale)
- [ ] Detection completes in < 1 second for 100 worktrees

**Deletion safety:**
- [ ] 0 valid worktrees accidentally deleted
- [ ] 100% of stale worktrees successfully deleted
- [ ] 100% of orphaned chunks removed via CASCADE

**CLI usability:**
- [ ] Dry-run shows accurate preview of changes
- [ ] Confirmation required for actual deletion
- [ ] Error messages are clear and actionable

### Performance Metrics

**Startup impact:**
- [ ] Watch startup delay < 200ms (background cleanup)
- [ ] Manual cleanup completes < 2 seconds (100 worktrees)
- [ ] Periodic cleanup completes < 500ms

**Resource usage:**
- [ ] CPU usage < 5% during cleanup
- [ ] Memory usage < 50 MB additional
- [ ] Database load < 10 queries/second

### Quality Metrics

**Search improvement:**
- [ ] Result duplication reduced by > 90%
- [ ] Search result count drops from 15x to 1-2x
- [ ] Users report improved search relevance

**Database health:**
- [ ] Database size reduced by > 90% (stale chunks removed)
- [ ] Query performance improved (fewer worktrees to join)
- [ ] Worktree count stable over time (< 10 active)

---

## Test Execution Plan

### Development Phase

```bash
# Run unit tests (fast feedback)
cargo test --lib cleanup

# Run integration tests (database validation)
cargo test --test cleanup_integration

# Run watch integration tests
cargo test --test watch_cleanup
```

**Coverage targets:**
- Line coverage: > 85% overall, > 90% for cleanup module (critical safety code)
- Branch coverage: > 70% (test error cases)
- Integration coverage: 100% (test all database interactions, especially multi-worktree scenarios)

### CI/CD Phase

```yaml
# .github/workflows/test.yml
- name: Run cleanup tests
  run: |
    cargo test --lib cleanup
    cargo test --test cleanup_integration
    cargo test --test watch_cleanup
```

### Pre-Release Phase

1. Run full test suite on staging database
2. Perform manual validation checklist
3. Monitor logs for unexpected errors
4. Verify search quality improvement
5. Get team approval before production deployment

---

## Confidence Level

**MVP (Manual cleanup command):**
- Confidence: **High** (95%)
- Rationale: Well-tested, explicit user confirmation, clear rollback path
- Blocker: None identified

**Watch integration:**
- Confidence: **Medium** (75%)
- Rationale: More complex interactions, needs monitoring in production
- Blocker: Need performance metrics from staging

**Production readiness:**
- Confidence: **High** (90%)
- Rationale: Comprehensive test suite, incremental rollout, safety mechanisms
- Blocker: Manual validation on staging database required

---

## Summary

**Testing priorities:**
1. **Safety:** Prevent data loss through validation and transactions
2. **Accuracy:** Correctly identify stale vs. valid worktrees
3. **Performance:** Ensure cleanup doesn't impact normal operations
4. **Usability:** Clear CLI feedback and error messages

**Key test suites:**
- Integration tests: Database interactions, transaction safety, CASCADE behavior
- Unit tests: Detection logic, report formatting, configuration
- Manual tests: Production validation, edge cases, rollout verification

**Confidence builders:**
- Dry-run default prevents accidents
- Transaction rollback prevents corruption
- Audit logging enables recovery
- Incremental rollout limits blast radius
