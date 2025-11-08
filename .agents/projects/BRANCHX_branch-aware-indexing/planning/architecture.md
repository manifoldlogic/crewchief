# Architecture: Branch-Aware Indexing

## Design Principles

1. **Worktree as unit of indexing**: Each git worktree = separate index state
2. **JSONB for flexibility**: Unlimited branches, queryable, readable
3. **Git as source of truth**: Use tree SHA for change detection
4. **Incremental by default**: Only scan what changed
5. **Backward compatible**: Existing queries still work (just add WHERE clause)

## Database Schema Design

### 1. Add Worktree Tracking to Chunks

```sql
-- Add JSONB array tracking which worktrees contain this chunk
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB NOT NULL DEFAULT '[]';

-- GIN index for efficient JSONB queries
CREATE INDEX idx_chunks_worktree_ids ON chunks USING gin(worktree_ids);
```

**Key decisions**:
- JSONB not integer array: Flexible, supports PostgreSQL JSONB operators
- Default `'[]'`: Empty array for backward compatibility
- GIN index: Optimized for contains (`?`) and overlaps (`?|`) queries

### 2. Create Worktree Index State Table

```sql
CREATE TABLE worktree_index_state (
  worktree_id INT PRIMARY KEY REFERENCES worktrees(id),
  last_tree_sha TEXT NOT NULL,
  last_indexed TIMESTAMP DEFAULT NOW(),
  chunks_processed INT DEFAULT 0,
  embeddings_generated INT DEFAULT 0
);

CREATE INDEX ON worktree_index_state(last_tree_sha);
```

**Purpose**: Track which git tree SHA we last indexed for each worktree

**Columns**:
- `last_tree_sha`: Git tree SHA (from `git rev-parse HEAD^{tree}`)
- `chunks_processed`: Metrics for monitoring
- `embeddings_generated`: Cost tracking

### 3. Migration Strategy

**Phase 1**: Add worktree_ids column (backfill with current worktree)
```sql
-- Add column
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB DEFAULT '[]';

-- Backfill existing chunks with their worktree
UPDATE chunks c
SET worktree_ids = jsonb_build_array(
  (SELECT w.id FROM worktrees w
   JOIN files f ON f.worktree_id = w.id
   WHERE f.id = c.file_id)
);

-- Make NOT NULL after backfill
ALTER TABLE chunks ALTER COLUMN worktree_ids SET NOT NULL;
```

**Phase 2**: Create index state table
```sql
CREATE TABLE worktree_index_state (...);

-- Initialize with placeholder tree SHA
INSERT INTO worktree_index_state (worktree_id, last_tree_sha)
SELECT id, 'init' FROM worktrees;
```

## Incremental Update Algorithm

### Core Workflow

```rust
async fn incremental_update(
    pool: &PgPool,
    worktree_id: i32,
    repo_path: &Path,
) -> Result<UpdateStats> {
    // 1. Get current git tree SHA
    let current_tree = get_git_tree_sha(repo_path)?;

    // 2. Get last indexed tree SHA
    let last_tree = get_last_indexed_tree(pool, worktree_id).await?;

    // 3. Quick check: has anything changed?
    if current_tree == last_tree {
        info!("No changes detected (tree SHA match), skipping scan");
        return Ok(UpdateStats::skipped());
    }

    // 4. Find changed files using git diff-tree
    let changed_files = git_diff_tree(&last_tree, &current_tree)?;
    info!("Found {} changed files", changed_files.len());

    // 5. Process changed files
    let mut stats = UpdateStats::new();

    for file in changed_files {
        match file.status {
            FileStatus::Added | FileStatus::Modified => {
                // Parse file and upsert chunks
                let chunks = parse_file_into_chunks(&file.path)?;

                for chunk in chunks {
                    upsert_chunk_with_worktree(pool, &chunk, worktree_id).await?;
                    stats.chunks_processed += 1;
                }
            }
            FileStatus::Deleted => {
                // Remove this worktree from chunks in deleted file
                remove_worktree_from_chunks(pool, worktree_id, &file.path).await?;
            }
        }
    }

    // 6. Update indexed state
    update_index_state(pool, worktree_id, &current_tree, &stats).await?;

    Ok(stats)
}
```

### Git Integration Functions

**Get Git Tree SHA**:
```rust
fn get_git_tree_sha(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!("Failed to get git tree SHA");
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
```

**Find Changed Files**:
```rust
#[derive(Debug)]
struct FileChange {
    status: FileStatus,
    path: PathBuf,
}

#[derive(Debug)]
enum FileStatus {
    Added,
    Modified,
    Deleted,
}

fn git_diff_tree(old_tree: &str, new_tree: &str) -> Result<Vec<FileChange>> {
    let output = Command::new("git")
        .args([
            "diff-tree",
            "-r",              // Recursive
            "--no-commit-id",  // Don't show commit hash
            "--name-status",   // Show status (A/M/D) and filename
            "--diff-filter=AMD", // Only Added, Modified, Deleted
            old_tree,
            new_tree,
        ])
        .output()?;

    if !output.status.success() {
        bail!("git diff-tree failed");
    }

    parse_diff_tree_output(&String::from_utf8(output.stdout)?)
}

fn parse_diff_tree_output(output: &str) -> Result<Vec<FileChange>> {
    let mut changes = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let status = match parts[0] {
            "A" => FileStatus::Added,
            "M" => FileStatus::Modified,
            "D" => FileStatus::Deleted,
            _ => continue,
        };

        changes.push(FileChange {
            status,
            path: PathBuf::from(parts[1]),
        });
    }

    Ok(changes)
}
```

## Worktree Management

### Upsert Chunk with Worktree Tracking

```rust
async fn upsert_chunk_with_worktree(
    pool: &PgPool,
    chunk: &ParsedChunk,
    worktree_id: i32,
) -> Result<Uuid> {
    let blob_sha = compute_blob_sha(&chunk.content);

    // Ensure embedding exists (BLOBSHA responsibility)
    ensure_embedding_cached(pool, &blob_sha, &chunk.content).await?;

    // Upsert chunk, adding this worktree to worktree_ids
    let chunk_id = sqlx::query_scalar!(
        r#"
        INSERT INTO chunks (blob_sha, file_path, symbol_name, content, worktree_ids, ...)
        VALUES ($1, $2, $3, $4, jsonb_build_array($5), ...)
        ON CONFLICT (blob_sha, file_path)
        DO UPDATE SET
          worktree_ids = CASE
            WHEN chunks.worktree_ids ? $5::TEXT THEN chunks.worktree_ids
            ELSE chunks.worktree_ids || jsonb_build_array($5)
          END,
          updated_at = NOW()
        RETURNING chunk_id
        "#,
        blob_sha,
        chunk.file_path,
        chunk.symbol_name,
        chunk.content,
        worktree_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(chunk_id)
}
```

**Key logic**:
- `ON CONFLICT (blob_sha, file_path)`: Same content in same file
- Check if worktree already in array: `worktree_ids ? $5::TEXT`
- If not, append: `worktree_ids || jsonb_build_array($5)`
- If yes, no-op (idempotent)

### Remove Worktree from Chunks

**When file is deleted**:
```rust
async fn remove_worktree_from_chunks(
    pool: &PgPool,
    worktree_id: i32,
    file_path: &Path,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE chunks
        SET worktree_ids = worktree_ids - $1::TEXT
        WHERE file_path = $2
        "#,
        worktree_id.to_string(),
        file_path.to_str().unwrap(),
    )
    .execute(pool)
    .await?;

    // Optional: Clean up chunks with no worktrees
    sqlx::query!(
        "DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0"
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

## Query Patterns

### 1. Query Specific Worktree

```sql
-- Find chunks in worktree 2
SELECT c.chunk_id, c.symbol_name, e.embedding <=> $1 AS distance
FROM chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE c.worktree_ids ? '2'  -- JSONB contains operator
ORDER BY distance
LIMIT 10;
```

**Performance**: GIN index makes this fast (O(log N))

### 2. Query Multiple Worktrees

```sql
-- Find chunks in worktree 2 OR 5
SELECT c.chunk_id, c.symbol_name, e.embedding <=> $1 AS distance
FROM chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE c.worktree_ids ?| ARRAY['2', '5']  -- JSONB overlaps
ORDER BY distance
LIMIT 10;
```

### 3. Deduplicated Cross-Branch Search

```sql
-- Find unique results across multiple branches
SELECT DISTINCT ON (c.blob_sha)
  c.chunk_id,
  c.symbol_name,
  c.file_path,
  c.worktree_ids,
  e.embedding <=> $1 AS distance
FROM chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE c.worktree_ids ?| ARRAY['1', '2', '3']
ORDER BY c.blob_sha, distance
LIMIT 10;
```

**Returns**: One result per unique content, showing which branches have it

## Index State Management

### Update Index State

```rust
async fn update_index_state(
    pool: &PgPool,
    worktree_id: i32,
    tree_sha: &str,
    stats: &UpdateStats,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO worktree_index_state
          (worktree_id, last_tree_sha, last_indexed, chunks_processed, embeddings_generated)
        VALUES ($1, $2, NOW(), $3, $4)
        ON CONFLICT (worktree_id) DO UPDATE
        SET
          last_tree_sha = EXCLUDED.last_tree_sha,
          last_indexed = NOW(),
          chunks_processed = EXCLUDED.chunks_processed,
          embeddings_generated = EXCLUDED.embeddings_generated
        "#,
        worktree_id,
        tree_sha,
        stats.chunks_processed,
        stats.embeddings_generated,
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

### Get Last Indexed Tree

```rust
async fn get_last_indexed_tree(pool: &PgPool, worktree_id: i32) -> Result<String> {
    let result = sqlx::query_scalar!(
        "SELECT last_tree_sha FROM worktree_index_state WHERE worktree_id = $1",
        worktree_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.unwrap_or_else(|| "init".to_string()))
}
```

## CLI Command Updates

### Scan Command (Incremental by Default)

```rust
// maproom scan --repo myproject --worktree main
async fn scan_command(args: ScanArgs) -> Result<()> {
    let pool = get_pool().await?;
    let worktree_id = get_or_create_worktree(&pool, &args.worktree).await?;

    // Incremental update (uses tree SHA comparison)
    let stats = incremental_update(&pool, worktree_id, &args.repo_path).await?;

    info!("Scan complete:");
    info!("  Files processed: {}", stats.files_processed);
    info!("  Chunks processed: {}", stats.chunks_processed);
    info!("  Embeddings generated: {}", stats.embeddings_generated);
    info!("  Estimated cost: ${:.2}", stats.embeddings_generated as f64 * 0.00002);

    Ok(())
}
```

### Force Full Scan

```rust
// maproom scan --repo myproject --worktree main --full
async fn full_scan_command(args: ScanArgs) -> Result<()> {
    // Skip tree SHA comparison, scan everything
    let stats = full_scan(&pool, worktree_id, &args.repo_path).await?;
    // ...
}
```

## Performance Characteristics

### Initial Scan (New Worktree)

| Metric | Value |
|--------|-------|
| Files | 1,000 |
| Chunks | 50,000 |
| Tree SHA check | 0ms (no previous state) |
| Scan time | 5-10 minutes (full scan) |

### Incremental Update (80% overlap)

| Metric | Value |
|--------|-------|
| Tree SHA check | 1ms |
| Changed files | 200 (out of 1,000) |
| Chunks processed | 10,000 |
| Cache hits | 8,000 (80%) |
| Embeddings generated | 2,000 |
| Scan time | 20 seconds |

### Return to Cached Branch

| Metric | Value |
|--------|-------|
| Tree SHA check | 1ms |
| Changed files | 0 |
| Scan time | <1 second (skipped) |

## Technology Choices

### Why JSONB Array?

**Alternatives considered**:

1. **Integer array** (`INT[]`)
   - Pro: Slightly more compact
   - Con: Less PostgreSQL operator support
   - Con: Not standard across other DBs

2. **Separate junction table** (`chunk_worktrees`)
   - Pro: Normalized
   - Con: Requires JOIN for every query
   - Con: More complex upsert logic

3. **Bitmask** (`BIGINT`)
   - Pro: Very compact (64 branches in 8 bytes)
   - Con: Limited to 64 worktrees
   - Con: Less readable

4. **JSONB array** ✅
   - Pro: Unlimited worktrees
   - Pro: GIN index for fast queries
   - Pro: PostgreSQL JSONB operators (`?`, `?|`, `?&`)
   - Pro: Readable, debuggable
   - Con: Slightly larger than bitmask

**Decision**: JSONB for flexibility and PostgreSQL-native support

## Rollback Plan

**Phase 1 rollback** (worktree_ids column):
```sql
ALTER TABLE chunks DROP COLUMN worktree_ids;
```

**Phase 2 rollback** (index state table):
```sql
DROP TABLE worktree_index_state;
```

## Success Metrics

### Functional
- Chunks tracked across multiple worktrees
- Incremental updates only scan changed files
- Tree SHA comparison skips unchanged branches

### Performance
- Tree SHA check: <10ms
- Incremental scan: <1 minute for typical branch switch
- Full scan: Same as current (no regression)

### Cost
- Incremental scan: Proportional to changes (80% overlap = 80% savings)

## Next Steps

1. Implement worktree tracking (Rust + SQL)
2. Add git integration (tree SHA, diff-tree)
3. Update CLI commands
4. Write comprehensive tests (quality-strategy.md)
5. Create implementation plan (plan.md)
