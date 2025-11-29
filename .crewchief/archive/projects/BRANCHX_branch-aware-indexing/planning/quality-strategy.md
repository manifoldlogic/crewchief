# Quality Strategy: Branch-Aware Indexing

## Testing Philosophy

**Core principle**: Verify incremental updates produce identical results to full scans

This project adds complexity (worktree tracking, tree SHA comparison, git integration). Testing must ensure:

1. **Correctness**: Incremental updates = full scans (same chunks indexed)
2. **Efficiency**: Tree SHA optimization actually skips work
3. **Git integration**: diff-tree correctly identifies changes
4. **JSONB queries**: Worktree filtering returns correct results

## Test Pyramid

- **60% Unit tests** - Git integration, JSONB operations, pure functions
- **30% Integration tests** - Incremental update logic, database operations
- **10% E2E tests** - Full workflows, performance validation

## Critical Path Tests

**Most important** (run on every commit):

1. ✅ `test_tree_sha_skip_unchanged` - Core optimization
2. ✅ `test_incremental_equals_full_scan` - Correctness guarantee
3. ✅ `test_worktree_filtering` - Query correctness
4. ✅ `test_git_diff_tree_detection` - Change detection

## Unit Tests

### 1. Git Integration Tests

**File**: `crates/maproom/src/git_integration.rs`

```rust
#[test]
fn test_get_git_tree_sha() {
    let repo = create_test_repo();
    let tree_sha = get_git_tree_sha(&repo).unwrap();

    // Should be valid SHA (64 hex chars for SHA-256)
    assert_eq!(tree_sha.len(), 64);
    assert!(tree_sha.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_tree_sha_changes_on_modification() {
    let repo = create_test_repo();
    let tree1 = get_git_tree_sha(&repo).unwrap();

    // Modify a file
    std::fs::write(repo.join("file.ts"), "modified").unwrap();
    git_commit(&repo, "Modify file");

    let tree2 = get_git_tree_sha(&repo).unwrap();

    assert_ne!(tree1, tree2, "Tree SHA must change when content changes");
}

#[test]
fn test_tree_sha_unchanged_for_same_content() {
    let repo = create_test_repo();
    let tree1 = get_git_tree_sha(&repo).unwrap();

    // Checkout same commit
    git_checkout(&repo, "HEAD");

    let tree2 = get_git_tree_sha(&repo).unwrap();

    assert_eq!(tree1, tree2, "Tree SHA must be identical for same content");
}

#[test]
fn test_git_diff_tree_detects_changes() {
    let repo = create_test_repo();
    git_commit(&repo, "Initial");
    let tree1 = get_git_tree_sha(&repo).unwrap();

    // Add file
    std::fs::write(repo.join("new.ts"), "new content").unwrap();
    git_add(&repo, "new.ts");

    // Modify file
    std::fs::write(repo.join("existing.ts"), "modified").unwrap();
    git_add(&repo, "existing.ts");

    // Delete file
    std::fs::remove_file(repo.join("old.ts")).unwrap();
    git_add(&repo, "old.ts");

    git_commit(&repo, "Changes");
    let tree2 = get_git_tree_sha(&repo).unwrap();

    let changes = git_diff_tree(&tree1, &tree2).unwrap();

    assert_eq!(changes.iter().filter(|c| matches!(c.status, FileStatus::Added)).count(), 1);
    assert_eq!(changes.iter().filter(|c| matches!(c.status, FileStatus::Modified)).count(), 1);
    assert_eq!(changes.iter().filter(|c| matches!(c.status, FileStatus::Deleted)).count(), 1);
}
```

### 2. JSONB Operations Tests

**File**: `packages/maproom-mcp/tests/jsonb-queries.test.ts`

```typescript
describe('JSONB worktree_ids queries', () => {
  it('detects chunk in worktree', async () => {
    await createChunk({ worktree_ids: [1, 2, 3] });

    const result = await pool.query(
      "SELECT * FROM chunks WHERE worktree_ids ? '2'"
    );

    expect(result.rows).toHaveLength(1);
  });

  it('detects chunk in any of multiple worktrees', async () => {
    await createChunk({ worktree_ids: [1] });
    await createChunk({ worktree_ids: [2] });
    await createChunk({ worktree_ids: [3] });

    const result = await pool.query(
      "SELECT * FROM chunks WHERE worktree_ids ?| ARRAY['1', '3']"
    );

    expect(result.rows).toHaveLength(2); // Chunks with 1 or 3
  });

  it('appends worktree without duplicates', async () => {
    await createChunk({ chunk_id: 'uuid1', worktree_ids: [1, 2] });

    // Upsert with worktree 2 again
    await upsertChunk({ chunk_id: 'uuid1', worktree_id: 2 });

    const result = await pool.query('SELECT worktree_ids FROM chunks WHERE chunk_id = $1', ['uuid1']);
    expect(result.rows[0].worktree_ids).toEqual([1, 2]); // No duplicate
  });

  it('removes worktree from array', async () => {
    await createChunk({ chunk_id: 'uuid1', worktree_ids: [1, 2, 3] });

    await pool.query(
      "UPDATE chunks SET worktree_ids = worktree_ids - '2' WHERE chunk_id = $1",
      ['uuid1']
    );

    const result = await pool.query('SELECT worktree_ids FROM chunks WHERE chunk_id = $1', ['uuid1']);
    expect(result.rows[0].worktree_ids).toEqual([1, 3]);
  });
});
```

## Integration Tests

### 3. Incremental Update Tests

**File**: `crates/maproom/tests/incremental_update.rs`

```rust
#[tokio::test]
async fn test_incremental_equals_full_scan() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();

    // Full scan (baseline)
    let worktree1 = create_worktree(&pool, "full").await.unwrap();
    full_scan(&pool, worktree1, &repo).await.unwrap();
    let full_chunks = get_all_chunks(&pool, worktree1).await.unwrap();

    // Incremental scan (same content)
    let worktree2 = create_worktree(&pool, "incremental").await.unwrap();
    incremental_update(&pool, worktree2, &repo).await.unwrap();
    let incr_chunks = get_all_chunks(&pool, worktree2).await.unwrap();

    // Should index identical chunks
    assert_eq!(full_chunks.len(), incr_chunks.len());
    assert_eq!(
        full_chunks.into_iter().map(|c| c.blob_sha).collect::<HashSet<_>>(),
        incr_chunks.into_iter().map(|c| c.blob_sha).collect::<HashSet<_>>()
    );
}

#[tokio::test]
async fn test_tree_sha_skip_unchanged() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();
    let worktree_id = create_worktree(&pool, "main").await.unwrap();

    // Initial scan
    let stats1 = incremental_update(&pool, worktree_id, &repo).await.unwrap();
    assert!(stats1.chunks_processed > 0);

    // Second scan (no changes)
    let stats2 = incremental_update(&pool, worktree_id, &repo).await.unwrap();
    assert_eq!(stats2.chunks_processed, 0, "Should skip unchanged tree");
}

#[tokio::test]
async fn test_incremental_only_scans_changed_files() {
    let pool = get_test_pool().await;
    let repo = create_test_repo_with_100_files();
    let worktree_id = create_worktree(&pool, "main").await.unwrap();

    // Initial scan
    incremental_update(&pool, worktree_id, &repo).await.unwrap();

    // Modify 1 file out of 100
    std::fs::write(repo.join("file_01.ts"), "modified").unwrap();
    git_commit(&repo, "Modify 1 file");

    // Incremental scan
    let stats = incremental_update(&pool, worktree_id, &repo).await.unwrap();

    // Should only process changed file
    assert_eq!(stats.files_processed, 1);
    assert!(stats.chunks_processed < 100); // Much less than full scan
}

#[tokio::test]
async fn test_deleted_file_removes_worktree() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();
    let worktree_id = create_worktree(&pool, "main").await.unwrap();

    // Initial scan
    incremental_update(&pool, worktree_id, &repo).await.unwrap();

    let chunk_count_before = count_chunks(&pool, worktree_id).await.unwrap();

    // Delete file
    std::fs::remove_file(repo.join("file.ts")).unwrap();
    git_commit(&repo, "Delete file");

    // Incremental scan
    incremental_update(&pool, worktree_id, &repo).await.unwrap();

    let chunk_count_after = count_chunks(&pool, worktree_id).await.unwrap();

    assert!(chunk_count_after < chunk_count_before, "Deleted file chunks should be removed");
}
```

### 4. Multi-Worktree Tests

```rust
#[tokio::test]
async fn test_same_content_multiple_worktrees() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();

    // Index in worktree 1
    let wt1 = create_worktree(&pool, "main").await.unwrap();
    incremental_update(&pool, wt1, &repo).await.unwrap();

    // Index same content in worktree 2
    let wt2 = create_worktree(&pool, "feature").await.unwrap();
    incremental_update(&pool, wt2, &repo).await.unwrap();

    // Verify chunk has both worktrees
    let chunks = get_chunks_by_content(&pool, "function foo() {}").await.unwrap();
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].worktree_ids.contains(&wt1));
    assert!(chunks[0].worktree_ids.contains(&wt2));
}
```

## E2E Tests

### 5. Branch Switch Workflow

**File**: `packages/maproom-mcp/tests/e2e/branch-workflow.test.ts`

```typescript
describe('Branch switch workflow', () => {
  it('handles branch switch efficiently', async () => {
    const repo = await createTestRepo();

    // Index main branch
    await executeCommand('maproom scan --repo test-repo --worktree main');
    const mainDuration = Date.now();

    // Switch to feature branch (80% same)
    await gitCheckout(repo, 'feature');
    const featureStart = Date.now();
    await executeCommand('maproom scan --repo test-repo --worktree feature');
    const featureDuration = Date.now() - featureStart;

    // Feature scan should be much faster (incremental + cache)
    expect(featureDuration).toBeLessThan(mainDuration * 0.3);
  });

  it('queries return branch-specific results', async () => {
    // Index main and feature
    await indexBranch('main');
    await indexBranch('feature');

    // Query main only
    const mainResults = await search({
      query: 'authentication',
      worktree: 'main',
    });

    // All results should be from main
    mainResults.forEach(result => {
      expect(result.worktree_ids).toContain(1); // main = worktree 1
    });
  });
});
```

## Performance Benchmarks

**File**: `crates/maproom/benches/incremental_update.rs`

```rust
fn benchmark_tree_sha_check(c: &mut Criterion) {
    c.bench_function("tree_sha_check", |b| {
        b.iter(|| {
            get_git_tree_sha(black_box(&repo_path))
        });
    });
}

fn benchmark_full_vs_incremental(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan_comparison");

    group.bench_function("full_scan", |b| {
        b.iter(|| full_scan(black_box(&pool), black_box(&repo)))
    });

    group.bench_function("incremental_update_20pct_changed", |b| {
        b.iter(|| incremental_update(black_box(&pool), black_box(&repo)))
    });

    group.finish();
}
```

**Success criteria**:
- Tree SHA check: <10ms
- Incremental update (20% changed): <5x faster than full scan
- Tree SHA skip (no changes): <100ms

## Test Data

### Fixtures

```
fixtures/
├── simple-branch/
│   ├── main/           # Base branch
│   └── feature/        # 80% same, 20% different
├── multi-branch/
│   ├── main/
│   ├── develop/
│   ├── feature-1/
│   └── feature-2/
└── edge-cases/
    ├── empty-diff/     # Identical branches
    ├── complete-rewrite/ # 0% overlap
    └── file-moved/     # Content same, path changed
```

## Manual Testing Checklist

- [ ] Run full scan on test repository
- [ ] Run incremental update with no changes (should skip)
- [ ] Modify 1 file, run incremental (should process 1 file)
- [ ] Delete file, run incremental (should remove worktree from chunks)
- [ ] Query by worktree_id (should filter correctly)
- [ ] Index same content in two worktrees (should share embedding)
- [ ] Check metrics: cache hit rate, chunks processed
- [ ] Verify git diff-tree output matches actual changes

## Acceptance Criteria

Project is complete when:

1. ✅ All unit tests pass
2. ✅ All integration tests pass
3. ✅ E2E branch workflow test passes
4. ✅ Tree SHA optimization working (skip unchanged)
5. ✅ Incremental updates = full scans (correctness)
6. ✅ Performance benchmarks meet targets
7. ✅ JSONB queries return correct results
8. ✅ Manual testing checklist complete

**Any failure** → Return to implementation
