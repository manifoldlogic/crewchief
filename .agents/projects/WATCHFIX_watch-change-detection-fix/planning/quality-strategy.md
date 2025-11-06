# Quality Strategy: Watch Change Detection Fix

## Testing Philosophy

**Pragmatic MVP approach**: Tests should provide confidence and prevent rework, not be exhaustive or ceremonial. Focus on high-value tests that catch real bugs, not edge cases that will never happen.

**Risk-based prioritization**: Test the risky parts (path normalization, async coordination) more thoroughly than the safe parts (existing ChangeDetector logic).

**Fast feedback loops**: Unit tests run in <1s, integration tests in <10s. No slow, flaky tests.

## Test Pyramid

```
        ╱╲
       ╱  ╲
      ╱ E2E ╲          1-2 tests  (watch full workflow)
     ╱────────╲
    ╱          ╲
   ╱Integration╲       3-5 tests  (processor_task, database)
  ╱──────────────╲
 ╱                ╲
╱   Unit Tests     ╲   10-15 tests (path normalization, helpers)
╲──────────────────╱
```

**Ratio**: 70% unit, 25% integration, 5% E2E

## Critical Test Coverage

### 1. Path Normalization (HIGH RISK)

**Why critical**: This is the bug's root cause. Must work correctly on all platforms.

**Tests needed**:

```rust
#[cfg(test)]
mod path_normalization_tests {
    use super::*;

    #[test]
    fn test_normalize_to_relpath_simple() {
        let abs = Path::new("/workspace/packages/cli/src/main.ts");
        let root = Path::new("/workspace");
        let rel = normalize_to_relpath(abs, root).unwrap();
        assert_eq!(rel.to_str().unwrap(), "packages/cli/src/main.ts");
    }

    #[test]
    fn test_normalize_to_relpath_nested() {
        let abs = Path::new("/workspace/packages/cli/src/agents/runner.ts");
        let root = Path::new("/workspace");
        let rel = normalize_to_relpath(abs, root).unwrap();
        assert_eq!(rel.to_str().unwrap(), "packages/cli/src/agents/runner.ts");
    }

    #[test]
    fn test_normalize_to_relpath_outside_repo() {
        let abs = Path::new("/etc/hosts");
        let root = Path::new("/workspace");
        assert!(normalize_to_relpath(abs, root).is_err());
    }

    #[test]
    fn test_normalize_to_relpath_same_as_root() {
        let abs = Path::new("/workspace");
        let root = Path::new("/workspace");
        let rel = normalize_to_relpath(abs, root).unwrap();
        assert_eq!(rel.to_str().unwrap(), "");
    }

    #[test]
    fn test_normalize_trailing_slashes() {
        let abs = Path::new("/workspace/packages/cli/");
        let root = Path::new("/workspace/");
        let rel = normalize_to_relpath(abs, root).unwrap();
        assert_eq!(rel.to_str().unwrap(), "packages/cli");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_normalize_windows_paths() {
        let abs = Path::new("C:\\workspace\\packages\\cli\\src\\main.ts");
        let root = Path::new("C:\\workspace");
        let rel = normalize_to_relpath(abs, root).unwrap();
        assert_eq!(rel.to_str().unwrap(), "packages\\cli\\src\\main.ts");
    }
}
```

**Coverage target**: 100% of normalize_to_relpath() function

### 2. Change Type Classification (HIGH RISK)

**Why critical**: This is where the bug manifests. Must return correct ChangeType.

**Test strategy**: Mock get_file_id_by_path() and ChangeDetector, verify logic flow.

```rust
#[cfg(test)]
mod change_classification_tests {
    // Test that Modified file with existing file_id calls ChangeDetector
    #[tokio::test]
    async fn test_modified_file_calls_detector() {
        // Mock get_file_id_by_path to return Some(123)
        // Mock ChangeDetector.detect_change to return Modified
        // Assert: UpdateTask enqueued with ChangeType::Modified
    }

    // Test that Modified file without file_id creates New
    #[tokio::test]
    async fn test_new_file_creates_new_changetype() {
        // Mock get_file_id_by_path to return None
        // Assert: ChangeType::New created
        // Assert: ChangeDetector NOT called
    }

    // Test that database error doesn't crash
    #[tokio::test]
    async fn test_database_error_handling() {
        // Mock get_file_id_by_path to return Err(...)
        // Assert: Event skipped gracefully
        // Assert: Warning logged
    }
}
```

**Coverage target**: 90% of processor_task change detection logic

**Note**: Full integration test (below) is more valuable than extensive mocking here.

### 3. Multi-File Processing (MEDIUM RISK)

**Why test**: This was the reported bug scenario. Must work for simultaneous changes.

**Test strategy**: Integration test with real database.

```rust
#[tokio::test]
async fn test_watch_multiple_files() {
    // Setup
    let pool = setup_test_db().await;
    let temp_dir = create_test_repo(&pool).await;

    // Create 3 test files in database
    insert_file(&pool, "src/a.rs", "content a").await;
    insert_file(&pool, "src/b.rs", "content b").await;
    insert_file(&pool, "src/c.rs", "content c").await;

    // Start watch (background task)
    let watch_handle = start_watch(&pool, &temp_dir).await;

    // Modify all 3 files
    modify_file(&temp_dir, "src/a.rs", "// new comment a");
    modify_file(&temp_dir, "src/b.rs", "// new comment b");
    modify_file(&temp_dir, "src/c.rs", "// new comment c");

    // Wait for processing (debounce + processing time)
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Assert: All 3 files re-indexed
    assert_chunks_updated(&pool, "src/a.rs").await;
    assert_chunks_updated(&pool, "src/b.rs").await;
    assert_chunks_updated(&pool, "src/c.rs").await;

    // Cleanup
    watch_handle.abort();
    cleanup_test_db(&pool).await;
}
```

**Coverage target**: Full happy path for 3-file scenario

### 4. IncrementalProcessor Path Handling (MEDIUM RISK)

**Why test**: We're changing how paths are used in index_new_file() and update_file().

**Test strategy**: Integration tests with database.

```rust
#[tokio::test]
async fn test_index_new_file_with_relpath() {
    let pool = setup_test_db().await;
    let processor = IncrementalProcessor::new(pool.clone());

    // Create file in filesystem
    let abs_path = Path::new("/tmp/test_repo/src/new.rs");
    fs::write(abs_path, "fn main() {}").unwrap();

    // Insert file record with relpath
    insert_file_record(&pool, "src/new.rs", abs_path).await;

    // Index the file
    let hash = FileHasher::hash_file(abs_path).unwrap();
    processor.index_new_file(abs_path, &hash).await.unwrap();

    // Assert: Chunks inserted
    let chunks = query_chunks(&pool, "src/new.rs").await;
    assert!(!chunks.is_empty());
}

#[tokio::test]
async fn test_update_file_with_relpath() {
    let pool = setup_test_db().await;
    let processor = IncrementalProcessor::new(pool.clone());

    // Setup: existing file with chunks
    let abs_path = Path::new("/tmp/test_repo/src/main.rs");
    let file_id = insert_file_with_chunks(&pool, "src/main.rs", "old content").await;

    // Modify file
    fs::write(abs_path, "new content").unwrap();
    let new_hash = FileHasher::hash_file(abs_path).unwrap();

    // Update
    processor.update_file(abs_path, &new_hash).await.unwrap();

    // Assert: Chunks replaced
    let chunks = query_chunks(&pool, "src/main.rs").await;
    assert_eq!(get_chunk_content(&chunks[0]), "new content");
}
```

**Coverage target**: 80% of index_new_file() and update_file() success paths

## Lower Priority Tests

### 5. Edge Cases (LOW RISK)

These are nice-to-have but not critical for MVP:

- ✅ Temp file → rename sequence (already handled by existing code)
- ✅ Rapid successive saves (debouncer handles this)
- ❌ File deleted during processing (acceptable failure mode)
- ❌ Symlinks (out of scope)
- ❌ Non-UTF8 filenames (out of scope)
- ❌ Very large files (>10MB) (out of scope)

**Test strategy**: Document expected behavior, add tests only if bugs occur.

### 6. Performance Tests (LOW PRIORITY)

**Why skip**: Performance hasn't regressed. Existing code is fast enough.

**If needed later**:
- Benchmark: 100 files modified simultaneously
- Assert: < 1min total processing time
- Profile memory usage

### 7. Concurrency Tests (SKIP)

**Why skip**: Async architecture is proven, not changing it.

**Existing coverage**: Tokio's async runtime has its own tests.

## Test Data Management

### Fixture Strategy

**Use real database**: Spin up PostgreSQL in Docker for tests.

```rust
async fn setup_test_db() -> PgPool {
    // Create test database
    let pool = create_pool().await.unwrap();

    // Run migrations
    run_migrations(&pool).await;

    // Seed minimal data (1 repo, 1 worktree)
    seed_test_data(&pool).await;

    pool
}

async fn cleanup_test_db(pool: &PgPool) {
    // Truncate tables
    let client = pool.get().await.unwrap();
    client.execute("TRUNCATE maproom.chunks, maproom.files, maproom.worktrees, maproom.repos CASCADE", &[]).await.unwrap();
}
```

**Fixture files**: Store in `crates/maproom/tests/fixtures/`:
- `sample.rs` - Simple Rust file
- `sample.ts` - Simple TypeScript file
- `sample.md` - Simple markdown file

**Avoid**: Complex fixture generation. Keep it simple.

## CI/CD Integration

### Pre-commit

**Run**:
- Unit tests only (< 1s)
- `cargo fmt --check`
- `cargo clippy`

**Skip**:
- Integration tests (too slow)
- E2E tests (require database)

### CI Pipeline (GitHub Actions)

**On PR**:
1. Run all tests (unit + integration + E2E)
2. Require database (postgres service container)
3. Test on multiple platforms (Linux, macOS)
4. Code coverage report (optional, not blocking)

**On merge to main**:
1. Run all tests again
2. Build release binaries
3. Package for all platforms

### Test Environment

```yaml
# .github/workflows/test.yml
services:
  postgres:
    image: pgvector/pgvector:pg16
    env:
      POSTGRES_USER: maproom
      POSTGRES_PASSWORD: maproom
      POSTGRES_DB: maproom
    options: >-
      --health-cmd pg_isready
      --health-interval 10s
      --health-timeout 5s
      --health-retries 5
```

## Manual Testing Checklist

Before merging:

- [ ] Start watch command
- [ ] Modify single file, verify indexed
- [ ] Modify 3 files simultaneously, verify all indexed
- [ ] Check database: `SELECT relpath, updated_at FROM chunks WHERE ... `
- [ ] Check logs: No WARN messages about path mismatches
- [ ] Rapid saves: Save file 5 times in 2s, verify single update
- [ ] Temp files: Modify in VS Code (creates .tmp), verify works
- [ ] Stop watch cleanly (Ctrl+C)

## Test Maintenance

**Keep tests fast**:
- Unit tests: < 100ms each
- Integration tests: < 2s each
- E2E tests: < 10s each

**Keep tests independent**:
- No shared state between tests
- Clean up test data after each test
- Use unique file paths per test

**Keep tests readable**:
- Clear test names: `test_<what>_<condition>_<expected>`
- Minimal setup code
- Obvious assertions

## Coverage Targets

| Component | Target | Rationale |
|-----------|--------|-----------|
| normalize_to_relpath | 100% | Bug root cause |
| processor_task change detection | 90% | High risk area |
| IncrementalProcessor | 80% | Changed logic |
| UpdateTask | 60% | Simple struct |
| End-to-end | 1 test | Confidence check |

**Overall target**: 85% line coverage (pragmatic, not exhaustive)

## What We're NOT Testing

**Explicitly out of scope**:
1. ChangeDetector internals (already has tests)
2. FileWatcher internals (notify crate has tests)
3. Database migrations (separate concern)
4. Embedding generation (unchanged)
5. Search functionality (unchanged)
6. Edge relationship updates (unchanged)

**Rationale**: Don't test what we didn't change. Focus on the fix.

## Regression Testing

**Manual regression tests** (run once before release):

1. **Scan command still works**:
   ```bash
   maproom scan --repo test --worktree main --path /tmp/test
   ```

2. **Upsert command still works**:
   ```bash
   maproom upsert --repo test --worktree main --commit HEAD --paths src/main.rs
   ```

3. **Search still works**:
   ```bash
   maproom search "function definition"
   ```

**Rationale**: Ensure we didn't break existing commands.

## Success Criteria

Fix is ready to merge when:

1. ✅ All unit tests pass (100%)
2. ✅ All integration tests pass (100%)
3. ✅ E2E test passes (multi-file watch scenario)
4. ✅ Manual checklist completed
5. ✅ No regression in scan/upsert/search
6. ✅ Code review approved
7. ✅ CI pipeline green

## Test Timeline

**Day 1**: Write unit tests for path normalization
**Day 2**: Write integration tests for change detection
**Day 3**: Write E2E test for multi-file watch
**Day 4**: Manual testing, regression checks
**Day 5**: CI integration, polish

**Total**: 5 days of test development (alongside implementation)

## Conclusion

This testing strategy prioritizes high-value tests that prevent rework. We test the risky parts (path normalization, change classification) thoroughly, and skip ceremonial tests for unchanged code. The result: confidence in the fix without excessive test burden.
