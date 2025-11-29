# Implementation Plan: Branch-Aware Indexing

## Project Overview

**Goal**: Enable worktree-specific indexing and incremental updates using git tree SHA
**Dependencies**: BLOBSHA project must be complete
**Timeline**: 5-6 days

## Execution Strategy

Sequential implementation:
1. **Foundation** - Add worktree tracking schema
2. **Git Integration** - Tree SHA and diff-tree
3. **Incremental Logic** - Update algorithm
4. **CLI Updates** - Scan command modifications

## Phase 1: Worktree Tracking Schema (Days 1-2)

### Step 1.1: Add worktree_ids Column

**Agent**: database-engineer

**Migration**: `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql`

```sql
-- Add JSONB column
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB NOT NULL DEFAULT '[]';

-- Backfill with current worktree
UPDATE chunks c
SET worktree_ids = jsonb_build_array(
  (SELECT w.id FROM worktrees w
   JOIN files f ON f.worktree_id = w.id
   WHERE f.id = c.file_id)
);

-- Create GIN index
CREATE INDEX idx_chunks_worktree_ids ON chunks USING gin(worktree_ids);
```

**Acceptance Criteria**:
- Column added successfully
- All existing chunks have worktree_ids populated
- GIN index created

### Step 1.2: Create Index State Table

**Agent**: database-engineer

**Migration**: Same file as 1.1

```sql
CREATE TABLE worktree_index_state (
  worktree_id INT PRIMARY KEY REFERENCES worktrees(id),
  last_tree_sha TEXT NOT NULL,
  last_indexed TIMESTAMP DEFAULT NOW(),
  chunks_processed INT DEFAULT 0,
  embeddings_generated INT DEFAULT 0
);

-- Initialize for existing worktrees
INSERT INTO worktree_index_state (worktree_id, last_tree_sha)
SELECT id, 'init' FROM worktrees;
```

**Acceptance Criteria**:
- Table created
- Existing worktrees initialized

### Step 1.3: Testing

**Agent**: unit-test-runner

**Tests**:
- `test_jsonb_contains_query`
- `test_jsonb_overlaps_query`
- `test_worktree_ids_no_duplicates`
- `test_migration_004_success`

---

## Phase 2: Git Integration (Days 2-3)

### Step 2.1: Implement Git Functions

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/git.rs` (new)

**Functions**:
```rust
pub fn get_git_tree_sha(repo_path: &Path) -> Result<String>;
pub fn git_diff_tree(old_tree: &str, new_tree: &str) -> Result<Vec<FileChange>>;
pub fn get_current_branch(repo_path: &Path) -> Result<String>;
```

**Acceptance Criteria**:
- Functions work with real git repositories
- Unit tests pass (git integration tests)

### Step 2.2: Database Functions

**Agent**: database-engineer

**Functions**:
```rust
pub async fn get_last_indexed_tree(pool: &PgPool, worktree_id: i32) -> Result<String>;
pub async fn update_index_state(pool: &PgPool, worktree_id: i32, tree_sha: &str, stats: &UpdateStats) -> Result<()>;
```

**Acceptance Criteria**:
- Functions query/update worktree_index_state correctly
- Integration tests pass

### Step 2.3: Testing

**Agent**: unit-test-runner

**Tests**:
- `test_get_git_tree_sha`
- `test_tree_sha_changes_on_modification`
- `test_git_diff_tree_detects_changes`
- `test_diff_tree_parses_correctly`

---

## Phase 3: Incremental Update Logic (Days 3-4)

### Step 3.1: Implement Incremental Update

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/incremental.rs` (new)

**Core function**:
```rust
pub async fn incremental_update(
    pool: &PgPool,
    worktree_id: i32,
    repo_path: &Path,
) -> Result<UpdateStats> {
    // 1. Get current tree SHA
    let current_tree = get_git_tree_sha(repo_path)?;

    // 2. Get last indexed tree SHA
    let last_tree = get_last_indexed_tree(pool, worktree_id).await?;

    // 3. Quick check: changed?
    if current_tree == last_tree {
        return Ok(UpdateStats::skipped());
    }

    // 4. Find changed files
    let changed_files = git_diff_tree(&last_tree, &current_tree)?;

    // 5. Process changes
    let stats = process_changed_files(pool, worktree_id, &changed_files).await?;

    // 6. Update index state
    update_index_state(pool, worktree_id, &current_tree, &stats).await?;

    Ok(stats)
}
```

**Acceptance Criteria**:
- Incremental update processes only changed files
- Tree SHA comparison skips unchanged repositories
- Statistics tracked correctly

### Step 3.2: Update Upsert Logic

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/upsert.rs`

**Modify** `upsert_chunk` to accept `worktree_id`:
```rust
pub async fn upsert_chunk_with_worktree(
    pool: &PgPool,
    chunk: &ParsedChunk,
    worktree_id: i32,
) -> Result<Uuid> {
    // Compute blob SHA
    let blob_sha = compute_blob_sha(&chunk.content);

    // Ensure embedding exists (BLOBSHA)
    ensure_embedding_cached(pool, &blob_sha, &chunk.content).await?;

    // Upsert chunk, add worktree to array
    sqlx::query_scalar!(
        r#"
        INSERT INTO chunks (blob_sha, file_path, content, worktree_ids, ...)
        VALUES ($1, $2, $3, jsonb_build_array($4), ...)
        ON CONFLICT (blob_sha, file_path) DO UPDATE
        SET worktree_ids = CASE
          WHEN chunks.worktree_ids ? $4::TEXT THEN chunks.worktree_ids
          ELSE chunks.worktree_ids || jsonb_build_array($4)
        END
        RETURNING chunk_id
        "#,
        blob_sha, chunk.file_path, chunk.content, worktree_id
    )
    .fetch_one(pool)
    .await
}
```

**Acceptance Criteria**:
- Chunk added to worktree_ids array
- No duplicates in array
- Idempotent (running twice doesn't duplicate)

### Step 3.3: Handle File Deletions

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/incremental.rs`

```rust
async fn remove_worktree_from_chunks(
    pool: &PgPool,
    worktree_id: i32,
    file_path: &Path,
) -> Result<()> {
    sqlx::query!(
        "UPDATE chunks SET worktree_ids = worktree_ids - $1::TEXT WHERE file_path = $2",
        worktree_id.to_string(),
        file_path.to_str().unwrap()
    )
    .execute(pool)
    .await?;

    // Optional: Clean up chunks with no worktrees
    sqlx::query!("DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0")
        .execute(pool)
        .await?;

    Ok(())
}
```

**Acceptance Criteria**:
- Deleted files remove worktree from chunks
- Empty chunks cleaned up

### Step 3.4: Testing

**Agent**: unit-test-runner

**Tests**:
- `test_incremental_equals_full_scan` (CRITICAL)
- `test_tree_sha_skip_unchanged` (CRITICAL)
- `test_incremental_only_scans_changed_files`
- `test_deleted_file_removes_worktree`
- `test_same_content_multiple_worktrees`

---

## Phase 4: CLI Updates (Day 5)

### Step 4.1: Update Scan Command

**Agent**: rust-indexer-engineer

**File**: `crates/maproom/src/cli.rs`

**Modify** `scan` command to use incremental by default:
```rust
async fn scan_command(args: ScanArgs) -> Result<()> {
    let pool = get_pool().await?;
    let worktree_id = get_or_create_worktree(&pool, &args.worktree).await?;

    let stats = if args.force {
        full_scan(&pool, worktree_id, &args.repo_path).await?
    } else {
        incremental_update(&pool, worktree_id, &args.repo_path).await?
    };

    info!("Scan complete:");
    info!("  Files processed: {}", stats.files_processed);
    info!("  Chunks processed: {}", stats.chunks_processed);
    info!("  Cache hit rate: {:.1}%", stats.cache_hit_rate() * 100.0);
    info!("  Embeddings generated: {}", stats.embeddings_generated);
    info!("  Estimated cost: ${:.2}", stats.cost());

    Ok(())
}
```

**Acceptance Criteria**:
- `maproom scan` uses incremental by default
- `maproom scan --force` does full scan
- Statistics printed correctly

### Step 4.2: Add Search Filtering (MCP)

**Agent**: general-purpose

**File**: `packages/maproom-mcp/src/search.ts`

**Update** search to accept `worktree` parameter:
```typescript
async function search(args: SearchArgs): Promise<SearchResults> {
  const query = args.query;
  const worktree = args.worktree || 'main'; // Default to main

  const worktreeId = await getWorktreeId(worktree);

  const results = await pool.query(`
    SELECT c.chunk_id, c.symbol_name, c.file_path, c.content,
           e.embedding <=> $1 AS distance
    FROM chunks c
    JOIN code_embeddings e ON c.blob_sha = e.blob_sha
    WHERE c.worktree_ids ? $2::TEXT
      AND e.embedding <=> $1 < 0.5
    ORDER BY distance
    LIMIT 10
  `, [queryEmbedding, worktreeId.toString()]);

  return results.rows;
}
```

**Acceptance Criteria**:
- MCP search accepts worktree filter
- Only returns chunks from specified worktree

### Step 4.3: Testing

**Agent**: unit-test-runner

**Tests**:
- E2E `test_branch_switch_workflow`
- E2E `test_search_filters_by_worktree`
- Integration `test_cli_incremental_default`

---

## Phase 5: Documentation (Day 6)

**Agent**: general-purpose

**Files to create/update**:
- `docs/architecture/branch-aware-indexing.md`
- `packages/maproom-mcp/README.md` (update schema)
- `CHANGELOG.md`

---

## Agent Assignments

1. **database-engineer** - Schema changes, migrations
2. **rust-indexer-engineer** - Git integration, incremental logic, CLI updates
3. **general-purpose** - MCP updates, documentation
4. **unit-test-runner** - Execute tests after each phase
5. **verify-ticket** - Final verification
6. **commit-ticket** - Create commit

## Testing Strategy

### Per-Phase
- Phase 1: Migration tests, JSONB query tests
- Phase 2: Git integration tests
- Phase 3: Incremental update tests (CRITICAL)
- Phase 4: E2E workflow tests

### Critical Tests (Run Always)
1. `test_incremental_equals_full_scan`
2. `test_tree_sha_skip_unchanged`
3. `test_worktree_filtering`
4. `test_git_diff_tree_detection`

## Success Metrics

### Functional
- [x] Worktree tracking works (chunks in multiple worktrees)
- [x] Incremental updates only scan changed files
- [x] Tree SHA optimization skips unchanged branches
- [x] Search filtering by worktree works

### Performance
- [x] Tree SHA check: <10ms
- [x] Incremental update (20% changed): 5-10x faster than full scan
- [x] Tree SHA skip (no changes): <100ms

### Quality
- [x] All tests passing
- [x] Incremental = full scan (correctness)
- [x] Branch isolation verified

## Risk Mitigation

**Backup**: Before Phase 1
**Rollback**: Drop worktree_ids column, drop worktree_index_state table

## Acceptance Checklist

- [ ] All phases complete
- [ ] All tests passing
- [ ] Performance benchmarks met
- [ ] E2E workflow test passes
- [ ] Documentation updated
- [ ] Manual testing complete

**Timeline**: 5-6 days (1 buffer day)

**Next**: BRWATCH project (automatic branch switch detection)
