# Analysis: Branch-Aware Indexing

## Problem Definition

**BLOBSHA project provides**: Content-addressed storage with deduplication
**BLOBSHA project doesn't provide**: Tracking which branches contain which code

### Current Gap

After BLOBSHA implementation:
- Embeddings are deduplicated ✅
- BUT: No way to query "show me code from branch X"
- BUT: No incremental updates (must rescan entire repository)
- BUT: Can't detect when code changes vs. just checking out existing branch

### The Branch Problem

Developers work across multiple branches:
```bash
git checkout main          # Should query main's code
git checkout feature-auth  # Should query feature-auth's code
```

**Current behavior** (post-BLOBSHA): All chunks mixed together, no branch isolation

**Desired behavior**: Query specific branch(es), filter results by branch

## Root Cause

The database lacks **branch provenance**:

```sql
-- Current (post-BLOBSHA)
SELECT c.chunk_id, c.symbol_name, e.embedding
FROM chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
-- No way to filter by branch!
```

**Missing piece**: Which worktrees/branches contain this chunk?

## Industry Solutions

### Sourcegraph Zoekt
- **Approach**: Bitmask for branch membership
- **Storage**: Each chunk has bitmask where bit N = present in branch N
- **Query**: Bitwise AND to filter branches
- **Scale**: Handles 64 branches per bitmask

### GitHub Code Search
- **Approach**: Branch-specific indexes
- **Trade-off**: Higher storage (separate index per branch) for simpler queries

### Our Approach: JSONB Array

**Why JSONB**:
- Flexible: Unlimited branches (no 64-branch limit)
- Queryable: GIN index for fast contains/overlaps
- Readable: Easy debugging, clear semantics
- PostgreSQL native: Built-in operators

**Schema**:
```sql
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB;
-- Example: [1, 2, 5] = chunk exists in worktrees 1, 2, and 5

-- Query: "Find chunks in worktree 2"
SELECT * FROM chunks WHERE worktree_ids ? '2';

-- Query: "Find chunks in worktree 2 OR 5"
SELECT * FROM chunks WHERE worktree_ids ?| ARRAY['2', '5'];
```

## Incremental Updates via Git Tree SHA

### The Efficiency Problem

**Current**: Full repository scan every time
- 1,000 files, 50,000 chunks
- Time: 5-10 minutes
- Cost: Recompute blob SHA, check cache for 50,000 chunks

**Desired**: Incremental updates
- Only scan changed files
- Time: <1 minute for typical branch switch
- Cost: Only process changed chunks

### Git's Built-In Solution: Tree SHA

Git tracks repository state with **tree SHA** (SHA of entire directory tree):

```bash
git rev-parse HEAD^{tree}
# Output: e3b0c44298fc1c149afbf4c8996fb92427ae41e4
```

**Key property**: Tree SHA changes if and only if content changes

**Usage**:
```rust
let current_tree = get_git_tree_sha(repo_path)?;
let last_indexed_tree = get_last_indexed_tree(worktree_id).await?;

if current_tree == last_indexed_tree {
    // No changes, skip scan entirely
    return Ok(());
}

// Find changed files
let changed_files = git_diff_tree(&last_indexed_tree, &current_tree)?;
// Only scan these files
```

### Git Diff-Tree for Change Detection

```bash
git diff-tree -r --name-status OLD_TREE NEW_TREE
# Output:
# M  src/auth.ts      # Modified
# A  src/new.ts       # Added
# D  src/old.ts       # Deleted
```

**Integration**:
1. Store `last_tree_sha` per worktree
2. On scan, compare current tree to last tree
3. If different, diff-tree to find changes
4. Only reindex changed files
5. Update `last_tree_sha`

## Example: Branch Switch Scenario

```bash
# Developer workflow
git checkout main
# Maproom: Check tree SHA
# - Matches last indexed SHA for main worktree → Skip scan (instant!)

git checkout feature-branch
# Maproom: Check tree SHA
# - Different from last indexed SHA → Scan needed
# - git diff-tree finds 100 changed files (out of 1,000)
# - Parse 100 files into ~5,000 chunks
# - 4,000 chunks: blob SHA cache hit (existing embedding)
# - 1,000 chunks: blob SHA cache miss (generate embedding)
# - Time: 20 seconds
# - Cost: $0.02 (1,000 new embeddings)
```

## Current System Analysis

### Existing Worktrees Table

```sql
CREATE TABLE worktrees (
  id SERIAL PRIMARY KEY,
  repo_id INT REFERENCES repositories(id),
  name TEXT NOT NULL,
  path TEXT NOT NULL,
  branch TEXT,
  created_at TIMESTAMP DEFAULT NOW()
);
```

**Already have**: Worktree tracking
**Missing**:
1. Link between chunks and worktrees (worktree_ids)
2. Indexed state tracking (last_tree_sha)

## What Needs to Change

### 1. Add Worktree Tracking to Chunks

```sql
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB NOT NULL DEFAULT '[]';
CREATE INDEX ON chunks USING gin(worktree_ids);
```

### 2. Create Index State Table

```sql
CREATE TABLE worktree_index_state (
  worktree_id INT PRIMARY KEY REFERENCES worktrees(id),
  last_tree_sha TEXT NOT NULL,
  last_indexed TIMESTAMP DEFAULT NOW()
);
```

### 3. Update Upsert Logic

```rust
async fn upsert_chunk_with_worktree(
    pool: &PgPool,
    chunk: &ParsedChunk,
    worktree_id: i32,
) -> Result<()> {
    let blob_sha = compute_blob_sha(&chunk.content);

    // Ensure embedding exists (from BLOBSHA)
    ensure_embedding_cached(&blob_sha, &chunk.content).await?;

    // Upsert chunk, adding this worktree to worktree_ids
    sqlx::query!(
        r#"
        INSERT INTO chunks (blob_sha, file_path, content, worktree_ids, ...)
        VALUES ($1, $2, $3, jsonb_build_array($4), ...)
        ON CONFLICT (file_path, blob_sha) DO UPDATE
        SET worktree_ids = chunks.worktree_ids || jsonb_build_array($4)
        "#,
        blob_sha,
        chunk.file_path,
        chunk.content,
        worktree_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

## Success Criteria

Project is complete when:

1. **Worktree tracking works**
   - Test: Index same code in two worktrees, chunk has both IDs
   - Verification: Query by worktree_id returns correct chunks

2. **Incremental updates work**
   - Test: Scan branch, modify file, scan again
   - Verification: Only changed file rescanned

3. **Tree SHA optimization works**
   - Test: Scan branch, checkout same branch again
   - Verification: Second scan skipped (<1s)

4. **Branch filtering works**
   - Test: Query "code in main only"
   - Verification: Returns only chunks with worktree_id for main

5. **Changed file detection works**
   - Test: git diff-tree integration
   - Verification: Correctly identifies Added/Modified/Deleted files

## Out of Scope

This project focuses on worktree tracking and incremental updates:

**Not included**:
- Automatic branch switch detection → **BRWATCH project**
- File watching for auto-triggering → **BRWATCH project**
- Search UI branch selector → Future work

**Why**: These features depend on branch-aware indexing working first

## Dependencies

**Requires**: BLOBSHA project complete
- Needs: blob SHA for deduplication
- Needs: code_embeddings table structure
- Needs: Cache-aware upsert logic

**Blocks**: BRWATCH project
- Provides: Incremental update API
- Provides: Worktree-filtered queries

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| JSONB performance | Low | Medium | GIN index, tested at scale |
| Git diff-tree errors | Medium | High | Error handling, fallback to full scan |
| Worktree ID conflicts | Low | Low | Use database primary keys |
| Migration complexity | Medium | Medium | Comprehensive tests, gradual rollout |

## Next Steps

1. Design worktree tracking schema (architecture.md)
2. Plan incremental update algorithm (architecture.md)
3. Define test strategy (quality-strategy.md)
4. Create implementation plan (plan.md)
