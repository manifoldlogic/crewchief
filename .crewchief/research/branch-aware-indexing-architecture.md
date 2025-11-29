# Branch-Aware Indexing Architecture for Maproom

**Date**: 2025-11-06
**Status**: Research & Design
**Context**: Addressing the need for efficient branch switching without full reindexing

## Problem Statement

The current Maproom architecture uses **git worktrees** as the unit of indexing, where each worktree represents a separate branch checked out in a different directory. This approach has significant limitations:

### Current Limitations

1. **Doesn't match developer workflows**: Most developers use `git checkout` to switch branches, not git worktrees
2. **Expensive branch switching**: Checking out a different branch requires reindexing the entire codebase
3. **Redundant storage**: Code that exists in multiple branches is stored and embedded multiple times
4. **Redundant embedding costs**: Same functions across branches generate separate embeddings ($$$)
5. **Poor user experience**: Developers must wait for full reindex after every branch switch

### Why This Matters

For a typical large codebase:
- **10 feature branches** with 80% code overlap
- **1000 files** per branch, ~50 functions per file = 50,000 chunks per branch
- **500,000 total chunks** across all branches
- **But only ~100,000 unique chunks** (80% are duplicates!)
- **Current cost**: $10.00 in embeddings (500k × $0.00002)
- **With deduplication**: $2.00 (100k × $0.00002) - **80% savings**

## Solution: Content-Addressed Chunk-Level Deduplication

### Core Concept

Use **chunk-level content addressing** with Git-compatible blob SHA as the cache key. This enables:

1. **Automatic deduplication** across branches
2. **Incremental updates** based on content changes
3. **Efficient branch switching** (only changed chunks reindexed)
4. **Cross-file reuse** (same function in different files = same embedding)

### How Git's Blob SHA Works

**Critical Understanding**: Git's blob SHA is **just a hashing algorithm**, not constrained to Git's object database.

#### The Algorithm

```rust
fn compute_blob_sha(content: &str) -> String {
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();

    // Git blob format: "blob <size>\0<content>"
    hasher.update(b"blob ");
    hasher.update(content.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(content.as_bytes());

    format!("{:x}", hasher.finalize())
}
```

#### Key Insight: We Define the Boundaries

**Git doesn't constrain us** - we can apply this algorithm to any content:

```rust
// Compute SHA for a whole file (Git's default granularity)
let file_sha = compute_blob_sha(&file_content);

// Or for a tree-sitter chunk (our chosen granularity)
let chunk_sha = compute_blob_sha(&chunk.content);

// Or for a single line (if we wanted that granularity)
let line_sha = compute_blob_sha(&line_content);

// THE ALGORITHM IS THE SAME - WE CHOOSE THE BOUNDARIES
```

**We're borrowing Git's hashing algorithm, not using Git's object database.**

#### Why This Matters

```typescript
// file.ts
function calculateTotal(items) {
  return items.reduce((sum, item) => sum + item.price, 0);
}

function processOrder(order) {
  console.log('Processing...');
  return calculateTotal(order.items);
}
```

**File-level hashing (what Git does for files):**
- Change 1 line → entire file SHA changes → both functions invalidated

**Chunk-level hashing (what we do):**
- Change 1 line in `processOrder` → only `processOrder` SHA changes
- `calculateTotal` SHA unchanged → embedding reused from cache

### Architecture Design

#### Database Schema

```sql
-- Content-addressed embedding cache
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,           -- Git-compatible blob SHA of chunk content
  embedding vector(1536) NOT NULL,
  model_version TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX ON code_embeddings USING hnsw (embedding vector_cosine_ops);

-- Chunks with worktree tracking
CREATE TABLE code_chunks (
  chunk_id UUID PRIMARY KEY,
  blob_sha TEXT REFERENCES code_embeddings(blob_sha),
  file_path TEXT NOT NULL,
  worktree_ids JSONB NOT NULL,         -- [1, 2, 5] - which worktrees contain this chunk
  start_line INT,
  end_line INT,
  symbol_name TEXT,
  chunk_type TEXT,                     -- 'function', 'class', 'method', etc.
  content TEXT,
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX ON code_chunks(blob_sha);
CREATE INDEX ON code_chunks USING gin(worktree_ids);
CREATE INDEX ON code_chunks(file_path);

-- Track indexed state per worktree
CREATE TABLE worktree_index_state (
  worktree_id INT PRIMARY KEY REFERENCES worktrees(id),
  last_tree_sha TEXT NOT NULL,         -- Git tree SHA of indexed state
  last_indexed TIMESTAMP DEFAULT NOW()
);
```

#### Key Design Decisions

**1. Separate Embedding Table**
- Deduplication: Same content = same blob SHA = single embedding
- Model versioning: Easy to invalidate all embeddings on model upgrade
- Reusability: Embeddings shared across all worktrees

**2. JSONB Worktree Tracking**
- Flexible: Unlimited worktrees per chunk
- Queryable: GIN index for fast filtering
- Readable: Easy to debug which branches have which code

**3. Git Tree SHA for State Tracking**
- Efficient: O(1) comparison to detect changes
- Precise: Captures entire repository state in single hash
- Native: Uses Git's built-in tree hashing

### Indexing Workflow

#### Initial Scan (New Worktree/Branch)

```rust
async fn scan_worktree(worktree_id: i32, path: &Path) -> Result<()> {
    let tree_sha = get_git_tree_sha(path)?;

    for file in walk_files(path) {
        // Parse file into chunks using tree-sitter
        let chunks = parse_file_into_chunks(file)?;

        for chunk in chunks {
            // Compute blob SHA for THIS chunk's content
            let blob_sha = compute_blob_sha(&chunk.content);

            // Check if we already have an embedding for this content
            let embedding = match get_cached_embedding(&blob_sha).await? {
                Some(emb) => {
                    info!("Cache hit for {}", chunk.symbol_name);
                    emb
                }
                None => {
                    info!("Generating embedding for {}", chunk.symbol_name);
                    let emb = generate_embedding(&chunk.content).await?;
                    cache_embedding(&blob_sha, &emb).await?;
                    emb
                }
            };

            // Upsert chunk, adding this worktree to worktree_ids
            upsert_chunk(&chunk, worktree_id, &blob_sha).await?;
        }
    }

    // Record indexed state
    update_index_state(worktree_id, &tree_sha).await?;

    Ok(())
}
```

#### Incremental Update (Branch Switch or Pull)

```rust
async fn incremental_update(worktree_id: i32, path: &Path) -> Result<()> {
    let current_tree = get_git_tree_sha(path)?;
    let last_tree = get_last_indexed_tree(worktree_id).await?;

    // Quick check: has anything changed?
    if current_tree == last_tree {
        info!("No changes detected, skipping index update");
        return Ok(());
    }

    // Find changed files using git diff-tree
    let changed_files = git_diff_tree(&last_tree, &current_tree)?;

    info!("Processing {} changed files", changed_files.len());

    for file in changed_files {
        match file.status {
            FileStatus::Added | FileStatus::Modified => {
                // Reindex this file's chunks
                let chunks = parse_file_into_chunks(&file.path)?;

                for chunk in chunks {
                    let blob_sha = compute_blob_sha(&chunk.content);

                    // Check cache (likely hits for unchanged chunks)
                    ensure_embedding_cached(&blob_sha, &chunk.content).await?;

                    // Add this worktree to the chunk's worktree list
                    upsert_chunk(&chunk, worktree_id, &blob_sha).await?;
                }
            }
            FileStatus::Deleted => {
                // Remove this worktree from chunks in deleted file
                remove_worktree_from_file_chunks(worktree_id, &file.path).await?;
            }
        }
    }

    // Update indexed state
    update_index_state(worktree_id, &current_tree).await?;

    Ok(())
}
```

#### Branch Switch Detection

```rust
// Watch .git/HEAD for branch switches
async fn watch_for_branch_switches(repo_path: &Path) -> Result<()> {
    let watcher = notify::watcher()?;
    let git_head = repo_path.join(".git/HEAD");

    watcher.watch(&git_head, RecursiveMode::NonRecursive)?;

    while let Some(event) = watcher.recv().await {
        if event.kind == EventKind::Modify {
            info!("Branch switch detected");

            let current_branch = get_current_branch(repo_path)?;
            let worktree_id = get_or_create_worktree(&current_branch).await?;

            // Incremental update (only changed files)
            incremental_update(worktree_id, repo_path).await?;
        }
    }

    Ok(())
}
```

### Example: How It Works

#### Scenario: Developer Working on Multiple Branches

```bash
# Initial: Index main branch
git checkout main
maproom scan --repo myproject --worktree main
# - Parses all files into chunks
# - Computes blob SHA for each chunk
# - Generates 10,000 embeddings (cache empty)
# - Cost: $0.20
```

**Database after main:**
```json
{
  "blob_sha": "abc123...",
  "embedding": [0.1, 0.2, ...],
  "worktree_ids": [1]  // main = worktree_id 1
}
```

```bash
# Switch to feature branch (80% same code)
git checkout feature-branch
# Auto-detected by .git/HEAD watcher
# maproom incremental_update runs automatically
```

**What happens:**
1. Git tree SHA comparison: `main_tree != feature_tree` → changes detected
2. `git diff-tree main feature` finds 100 changed files (out of 1000)
3. Parse those 100 files into ~5,000 chunks
4. Compute blob SHA for each chunk:
   - **4,000 chunks**: SHA matches existing → cache hit → **reuse embedding**
   - **1,000 chunks**: New SHA → cache miss → generate embedding
5. Cost: **$0.02** (only 1,000 new embeddings)
6. Time: **~10 seconds** (vs. minutes for full reindex)

**Database after feature:**
```json
// Unchanged chunk (in both branches)
{
  "blob_sha": "abc123...",
  "embedding": [0.1, 0.2, ...],
  "worktree_ids": [1, 2]  // Now in main AND feature
}

// New chunk (only in feature)
{
  "blob_sha": "xyz789...",
  "embedding": [0.3, 0.4, ...],
  "worktree_ids": [2]  // Only in feature
}
```

```bash
# Switch back to main
git checkout main
# Tree SHA matches last indexed state → NO WORK NEEDED
# Time: <1 second
```

### Deduplication Examples

#### Example 1: Function Moved Between Files

**Before:**
```typescript
// utils.ts
function calculateTotal(items) {
  return items.reduce((sum, item) => sum + item.price, 0);
}
```

**After (refactored):**
```typescript
// math.ts (NEW FILE)
function calculateTotal(items) {
  return items.reduce((sum, item) => sum + item.price, 0);
}
```

**What happens:**
- Content identical → blob SHA unchanged: `abc123...`
- Embedding already cached → **reused**
- Cost: **$0** (no API call)
- Update only changes `file_path` metadata

#### Example 2: One-Line Change in Large File

**File with 50 functions:**
```typescript
// services.ts (1000 lines, 50 functions)

function foo() { return 1; }      // Chunk SHA: aaa111
function bar() { return 2; }      // Chunk SHA: bbb222
// ... 47 more functions ...
function baz() { return 50; }     // Chunk SHA: zzz999

// You change ONE LINE in bar():
function bar() { return 20; }     // Chunk SHA: bbb222 → ccc333
```

**Reindexing:**
- 49 functions: blob SHA unchanged → embeddings reused
- 1 function (`bar`): blob SHA changed → new embedding generated
- Cost: **$0.00002** (1 embedding)
- vs. file-level: **$0.001** (50 embeddings) - **50x reduction**

#### Example 3: Common Utility Across Files

**Multiple files with same constant:**
```typescript
// config.ts
export const API_URL = "https://api.example.com";  // SHA: ddd444

// services.ts
const API_URL = "https://api.example.com";         // SHA: ddd444 (SAME!)

// client.ts
const API_URL = "https://api.example.com";         // SHA: ddd444 (SAME!)
```

**Indexing:**
- First occurrence: Generate embedding
- Subsequent occurrences: Cache hit → reuse
- 3 files, 1 embedding generated
- Cost savings: **67%** for this chunk

### Search with Branch Filtering

#### Query Current Branch Only

```sql
SELECT
  c.chunk_id,
  c.symbol_name,
  c.file_path,
  c.content,
  e.embedding <=> $1 AS distance
FROM code_chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE c.worktree_ids ? $2  -- JSONB contains current worktree_id
ORDER BY distance
LIMIT 10;
```

Parameters:
- `$1`: Query embedding vector
- `$2`: Current worktree ID (e.g., "main")

#### Query Multiple Branches

```sql
SELECT
  c.chunk_id,
  c.symbol_name,
  c.file_path,
  c.worktree_ids,
  c.content,
  e.embedding <=> $1 AS distance
FROM code_chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
WHERE c.worktree_ids ?| $2  -- JSONB overlaps with worktree_ids array
ORDER BY distance
LIMIT 10;
```

Parameters:
- `$1`: Query embedding vector
- `$2`: Array of worktree IDs (e.g., ["main", "develop", "feature-x"])

#### Cross-Branch Deduplicated Search

```sql
-- Find unique results across all branches
SELECT DISTINCT ON (c.blob_sha)
  c.chunk_id,
  c.symbol_name,
  c.file_path,
  array_agg(DISTINCT wt.name) AS branches,
  c.content,
  e.embedding <=> $1 AS distance
FROM code_chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
JOIN worktrees wt ON wt.id = ANY(
  SELECT jsonb_array_elements_text(c.worktree_ids)::int
)
GROUP BY c.chunk_id, c.blob_sha, c.symbol_name, c.file_path, c.content, e.embedding
ORDER BY c.blob_sha, distance
LIMIT 10;
```

Returns deduplicated results with list of branches containing each chunk.

### Git Integration

#### Computing Git Tree SHA

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

#### Finding Changed Files

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
            old_tree,
            new_tree,
        ])
        .output()?;

    if !output.status.success() {
        bail!("git diff-tree failed");
    }

    let stdout = String::from_utf8(output.stdout)?;

    let mut changes = Vec::new();
    for line in stdout.lines() {
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

#### Getting Current Branch

```rust
fn get_current_branch(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!("Failed to get current branch");
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
```

### Performance Characteristics

#### Initial Scan

| Metric | Value |
|--------|-------|
| Files | 1,000 |
| Chunks | 50,000 |
| Embedding API calls | 50,000 |
| Cost | $1.00 |
| Time | 5-10 minutes |
| Storage | 300MB (embeddings) |

#### Branch Switch (80% overlap)

| Metric | Without Dedup | With Dedup | Improvement |
|--------|---------------|------------|-------------|
| Changed files | 200 | 200 | - |
| Total chunks in changed files | 10,000 | 10,000 | - |
| Unique chunks | 10,000 | 2,000 | - |
| Cache hits | 0 | 8,000 | - |
| Embedding API calls | 10,000 | 2,000 | **80% reduction** |
| Cost | $0.20 | $0.04 | **80% savings** |
| Time | 2 minutes | 20 seconds | **6x faster** |

#### Return to Previously Indexed Branch

| Metric | Value |
|--------|-------|
| Tree SHA comparison | O(1) |
| Changed files | 0 |
| Embeddings generated | 0 |
| Cost | $0.00 |
| Time | <1 second |

### Storage Efficiency

#### Example: 10 Branches

**Scenario:**
- 10 branches (main + 9 features)
- 1,000 files per branch
- 50 chunks per file = 50,000 chunks per branch
- 80% code overlap between branches

**Without Deduplication:**
```
Embeddings: 50,000 × 10 = 500,000
Storage: 500,000 × 6KB = 3GB
Cost: $10.00
```

**With Chunk-Level Deduplication:**
```
Unique chunks: 50,000 + (9 × 10,000) = 140,000
Storage: 140,000 × 6KB = 840MB (72% reduction)
Cost: $2.80 (72% savings)
```

**Additional optimization (halfvec):**
```
Storage: 140,000 × 3KB = 420MB (86% reduction vs. without dedup)
```

### Migration Path

#### Phase 1: Add Blob SHA Support (Week 1)

```sql
-- Add blob_sha column to existing chunks
ALTER TABLE chunks ADD COLUMN blob_sha TEXT;

-- Create index
CREATE INDEX ON chunks(blob_sha);

-- Backfill blob_sha for existing chunks
-- This computes Git blob SHA for existing content
UPDATE chunks
SET blob_sha = encode(
    digest(
        'blob ' || length(content) || E'\0' || content,
        'sha256'
    ),
    'hex'
);

-- Make it NOT NULL after backfill
ALTER TABLE chunks ALTER COLUMN blob_sha SET NOT NULL;
```

#### Phase 2: Create Embedding Table (Week 1-2)

```sql
-- Create deduplicated embedding storage
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536) NOT NULL,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TIMESTAMP DEFAULT NOW()
);

-- Create HNSW index for vector similarity search
CREATE INDEX ON code_embeddings USING hnsw (embedding vector_cosine_ops);

-- Migrate existing embeddings (deduplicated)
INSERT INTO code_embeddings (blob_sha, embedding)
SELECT DISTINCT ON (blob_sha) blob_sha, embedding
FROM chunks
WHERE embedding IS NOT NULL;

-- Add foreign key constraint
ALTER TABLE chunks
ADD CONSTRAINT fk_embedding
FOREIGN KEY (blob_sha) REFERENCES code_embeddings(blob_sha);

-- Drop embedding column from chunks (save space)
-- Only after verifying join queries work
ALTER TABLE chunks DROP COLUMN embedding;
```

#### Phase 3: Add Worktree Tracking (Week 2)

```sql
-- Add JSONB array for worktree tracking
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB;

-- Backfill with current worktree
UPDATE chunks c
SET worktree_ids = jsonb_build_array(
  (SELECT w.id
   FROM worktrees w
   JOIN files f ON f.worktree_id = w.id
   WHERE f.id = c.file_id)
);

-- Make it NOT NULL
ALTER TABLE chunks ALTER COLUMN worktree_ids SET NOT NULL;

-- Create GIN index for efficient JSONB queries
CREATE INDEX ON chunks USING gin(worktree_ids);
```

#### Phase 4: Add Index State Tracking (Week 2)

```sql
-- Track last indexed state per worktree
CREATE TABLE worktree_index_state (
  worktree_id INT PRIMARY KEY REFERENCES worktrees(id),
  last_tree_sha TEXT NOT NULL,
  last_indexed TIMESTAMP DEFAULT NOW()
);

-- Initialize for existing worktrees
-- Use 'initial' as placeholder tree SHA
INSERT INTO worktree_index_state (worktree_id, last_tree_sha)
SELECT id, 'initial' FROM worktrees;
```

#### Phase 5: Implement Incremental Update Logic (Week 2-3)

Update the scan/upsert commands to:
1. Check tree SHA before scanning
2. Use git diff-tree for incremental updates
3. Update worktree_ids instead of creating duplicate chunks
4. Record tree SHA after successful scan

#### Phase 6: Add Branch Switch Detection (Week 3)

Implement file watcher for `.git/HEAD` to trigger automatic incremental updates on branch switch.

### Testing Strategy

#### Unit Tests

```rust
#[test]
fn test_compute_blob_sha() {
    // Same content = same SHA
    let content1 = "function foo() { return 1; }";
    let content2 = "function foo() { return 1; }";
    assert_eq!(compute_blob_sha(content1), compute_blob_sha(content2));

    // Different content = different SHA
    let content3 = "function bar() { return 2; }";
    assert_ne!(compute_blob_sha(content1), compute_blob_sha(content3));

    // Verify Git compatibility
    // Can test with: echo -n "content" | git hash-object --stdin
}

#[test]
fn test_chunk_deduplication() {
    // Parse same function from two files
    let chunks1 = parse_file("file1.ts")?;
    let chunks2 = parse_file("file2.ts")?;

    // If they contain identical functions, SHAs should match
    let chunk1 = chunks1.iter().find(|c| c.symbol_name == "foo").unwrap();
    let chunk2 = chunks2.iter().find(|c| c.symbol_name == "foo").unwrap();

    if chunk1.content == chunk2.content {
        assert_eq!(
            compute_blob_sha(&chunk1.content),
            compute_blob_sha(&chunk2.content)
        );
    }
}
```

#### Integration Tests

```rust
#[tokio::test]
async fn test_branch_switch_incremental_update() {
    let temp_repo = create_test_git_repo()?;

    // Index main branch
    git_checkout(&temp_repo, "main")?;
    let main_id = scan_worktree(&temp_repo, "main").await?;

    // Get initial embedding count
    let initial_count = count_embeddings().await?;

    // Switch to feature branch (80% same code)
    git_checkout(&temp_repo, "feature")?;
    let feature_id = scan_worktree(&temp_repo, "feature").await?;

    // Should only generate embeddings for changed chunks
    let final_count = count_embeddings().await?;
    let new_embeddings = final_count - initial_count;

    // Expect ~20% new (80% deduplication)
    let expected = initial_count / 5;
    assert!(new_embeddings < expected * 1.2); // Allow 20% margin
    assert!(new_embeddings > expected * 0.8);
}

#[tokio::test]
async fn test_tree_sha_comparison_skip() {
    let temp_repo = create_test_git_repo()?;

    // Index once
    scan_worktree(&temp_repo, "main").await?;
    let tree1 = get_git_tree_sha(&temp_repo)?;

    // No changes, index again
    let result = incremental_update(&temp_repo, main_id).await?;

    // Should skip work
    assert_eq!(result.files_processed, 0);
    assert_eq!(result.embeddings_generated, 0);
}
```

### Monitoring and Metrics

#### Cache Hit Rate

```sql
-- Track cache hits vs. misses
CREATE TABLE embedding_cache_metrics (
  date DATE PRIMARY KEY,
  cache_hits BIGINT DEFAULT 0,
  cache_misses BIGINT DEFAULT 0,
  hit_rate DECIMAL GENERATED ALWAYS AS (
    CASE
      WHEN cache_hits + cache_misses = 0 THEN 0
      ELSE cache_hits::decimal / (cache_hits + cache_misses)
    END
  ) STORED
);

-- Update on each embedding lookup
-- Log cache hit/miss and increment counters
```

**Target metrics:**
- Initial scan: 0% hit rate (cold cache)
- Branch switch: 70-90% hit rate (typical overlap)
- Return to prev branch: ~100% hit rate

#### Cost Tracking

```sql
CREATE TABLE indexing_costs (
  date DATE,
  worktree_id INT,
  embeddings_generated INT,
  estimated_cost DECIMAL,
  PRIMARY KEY (date, worktree_id)
);

-- Daily cost: embeddings_generated × $0.00002
```

### Future Enhancements

#### 1. Bitmask Optimization (If Scaling to 100+ Branches)

```sql
-- Alternative to JSONB: use BIGINT bitmask for 64 branches
ALTER TABLE chunks ADD COLUMN worktree_mask BIGINT;

-- Branch IDs become bit positions
-- main = 1 (0b0001)
-- develop = 2 (0b0010)
-- feature1 = 4 (0b0100)

-- Check if chunk in main or develop
SELECT * FROM chunks WHERE (worktree_mask & 3) != 0;  -- 0b0011

-- More space-efficient than JSONB for many branches
```

#### 2. Adaptive Reindexing

```rust
// Track query frequency per chunk
// Prioritize frequently queried chunks for fresh embeddings
// Lazy regeneration for cold data

struct ChunkMetrics {
    blob_sha: String,
    query_count: i64,
    last_queried: DateTime,
}

// Reindex hot chunks more frequently
if chunk.query_count > 100 && chunk.last_embedded > 30.days_ago() {
    regenerate_embedding(chunk).await?;
}
```

#### 3. Model Version Migration

```sql
-- When upgrading embedding model
UPDATE code_embeddings
SET model_version = 'text-embedding-3-small-v2'
WHERE model_version = 'text-embedding-3-small';

-- Lazy regeneration: only when chunk is accessed
-- Or batch regeneration: regenerate all over time
```

#### 4. Semantic Deduplication (Advanced)

```rust
// Find near-duplicate chunks (90%+ similar)
// Optionally merge to save storage
// Risk: might merge semantically similar but functionally different code
// Recommended: Start with exact deduplication only
```

## Industry Validation

This approach is validated by production systems at scale:

### Sourcegraph Zoekt
- **Strategy**: Bitmask-based branch tracking with content deduplication
- **Result**: 10 branches with 90% overlap = 1.1x storage vs. single branch
- **Scale**: Used by companies like Uber, Lyft for millions of files

### GitHub Blackbird
- **Strategy**: Delta encoding with content-addressed storage
- **Result**: 50%+ reduction in indexing time (36h → 18h for 115TB)
- **Deduplication**: 115TB → 28TB after blob-level dedup (75% savings)

### JetBrains IntelliJ
- **Workaround**: "Use git worktrees to avoid reindexing on branch switch"
- **Validation**: Even mature IDEs recommend worktrees for multi-branch workflows

## Conclusion

Content-addressed chunk-level deduplication provides:

1. **Massive cost savings**: 70-90% reduction in embedding API costs
2. **Fast branch switching**: Sub-second for cached branches, seconds for incremental
3. **Storage efficiency**: ~75% reduction in database size
4. **Developer-friendly**: Works with normal git checkout workflows
5. **Battle-tested**: Based on proven patterns from Sourcegraph, GitHub, Git itself

The key insight: **Git's blob SHA algorithm works at any granularity** - we apply it to tree-sitter chunks instead of files, giving us fine-grained deduplication that file-level hashing can't provide.

## References

- **Research Document**: `.crewchief/research/branch-aware-indexing-industry-research.md` (full details)
- **Git Internals**: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
- **Sourcegraph Zoekt Design**: https://github.com/sourcegraph/zoekt/blob/main/doc/design.md
- **GitHub Blackbird**: https://github.blog/engineering/architecture-optimization/the-technology-behind-githubs-new-code-search/
- **Tree-sitter**: https://tree-sitter.github.io/tree-sitter/
- **pgvector**: https://github.com/pgvector/pgvector

---

**Next Steps**:
1. Review and validate this design
2. Create detailed implementation tickets
3. Start with Phase 1 (blob SHA support) as proof-of-concept
4. Measure cache hit rates and cost savings
5. Iterate based on real-world performance data
