# Architecture: Incremental Scan Completion

## Overview

Complete the incremental scanning feature by adding tree SHA checking and state persistence to the scan command. This is a surgical fix to existing infrastructure, not a rewrite.

## Design Principles

1. **Minimal Changes:** Touch only what's necessary
2. **Fail Safe:** Errors default to full scan (never skip incorrectly)
3. **Observable:** Clear logging for debugging
4. **Backward Compatible:** Existing behavior unchanged with `--force`

## System Architecture

### Current Flow (Broken)

```
┌─────────────────────────────────────────────────────────┐
│  CLI: scan command (main.rs)                            │
│  ├─ Parse args (repo, worktree, path, force)           │
│  ├─ Get git info (commit hash)                         │
│  ├─ Log scan mode (incremental vs force)               │
│  └─ Call scan function                                  │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│  indexer::scan_worktree()                               │
│  ├─ Walk files (all files, no filtering)               │
│  ├─ Parse with tree-sitter                             │
│  ├─ Extract chunks                                      │
│  └─ Upsert to database (update worktree_ids)           │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│  auto_generate_embeddings()                             │
│  └─ Generate embeddings for NULL values                │
└─────────────────────────────────────────────────────────┘
                      │
                      ▼
                    DONE
              (no state saved!)
```

### Fixed Flow (This Project)

```
┌─────────────────────────────────────────────────────────┐
│  CLI: scan command (main.rs)                            │
│  ├─ Parse args                                          │
│  ├─ Get git tree SHA (NEW)                             │
│  └─ Call check_and_scan() (NEW)                        │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│  check_and_scan() (NEW FUNCTION)                        │
│  ├─ Get worktree_id from (repo, worktree)              │
│  ├─ Get last_tree_sha from worktree_index_state        │
│  ├─ If current_sha == last_sha && !force:              │
│  │   └─ Skip scan (log + return)                       │
│  ├─ Else:                                               │
│  │   ├─ Call scan_worktree() [existing]                │
│  │   ├─ Collect scan stats                             │
│  │   └─ update_index_state(tree_sha, stats) (NEW)     │
│  └─ Return                                              │
└─────────────────────────────────────────────────────────┘
```

### Integration Points

**1. main.rs:scan command**
- Before calling `scan_worktree()`, add tree SHA check
- After successful scan, call `update_index_state()`

**2. Error Handling**
- Git errors → log warning, proceed with full scan
- Database query errors → log warning, proceed with full scan
- State update errors → log warning, don't fail scan

**3. Metrics Collection**
- Track files/chunks/embeddings from scan
- Report skip vs incremental vs full
- Log tree SHA and comparison result

## Component Design

### New Function: `get_git_tree_sha()`

**Location:** `/crates/maproom/src/git/` (use existing git module)

**Purpose:** Get current git tree SHA (40-char hex)

**Signature:**
```rust
pub fn get_git_tree_sha(repo_path: &Path) -> Result<String>
```

**Implementation:**
```rust
// Execute: git rev-parse HEAD^{tree}
// Returns: "a3d062919d78ebcc9a0684d8e2298e842c18efb7"
```

**Error Cases:**
- Not a git repository → Err
- Detached HEAD → Still works (HEAD is valid)
- Bare repository → Works if in worktree
- Invalid git state → Err

**Note:** This function already exists! Just need to use it.

### New Logic: Tree SHA Check in main.rs

**Location:** `/crates/maproom/src/main.rs` (Commands::Scan handler)

**Insertion Point:** After line 572 (after logging scan mode)

**Pseudocode:**
```rust
// Get git tree SHA
let tree_sha = match get_git_tree_sha(&path) {
    Ok(sha) => sha,
    Err(e) => {
        tracing::warn!("Could not get tree SHA: {}, proceeding with full scan", e);
        None
    }
};

// Query worktree_index_state
if let Some(tree_sha) = tree_sha {
    let worktree_id = db::get_or_create_worktree(&client, &repo, &worktree, &path).await?;
    let last_tree = db::get_last_indexed_tree(&client, worktree_id).await?;

    if last_tree == tree_sha && !force {
        println!("✓ No changes detected (tree SHA match), skipping scan");
        tracing::info!("Scan skipped: tree {} already indexed", tree_sha);
        return Ok(());  // Early return!
    }

    tracing::info!("Tree changed: {} -> {}", last_tree, tree_sha);
}

// [Existing scan code continues here]
```

### New Logic: State Update After Scan

**Location:** `/crates/maproom/src/main.rs` (after scan completes)

**Insertion Point:** After line 635 (after scan_worktree completes)

**Pseudocode:**
```rust
// Collect scan statistics
let scan_stats = db::UpdateStats {
    files_processed: progress.files_processed() as i32,
    chunks_processed: progress.chunks_processed() as i32,
    embeddings_generated: 0,  // Updated after embedding generation
};

// Update index state
if let Some(ref tree_sha) = tree_sha {
    let worktree_id = db::get_or_create_worktree(&client, &repo, &worktree, &path).await?;

    match db::update_index_state(&client, worktree_id, tree_sha, &scan_stats).await {
        Ok(_) => {
            tracing::info!("Updated index state for {} with tree {}", worktree, tree_sha);
        }
        Err(e) => {
            tracing::warn!("Failed to update index state: {}", e);
            // Don't fail the scan, just log the warning
        }
    }
}
```

### Progress Tracker Enhancement

**Location:** `/crates/maproom/src/progress/` (if needed)

**Current Issue:** `ProgressTracker` doesn't expose final counts

**Solution:** Add accessor methods:
```rust
impl ProgressTracker {
    pub fn files_processed(&self) -> usize { ... }
    pub fn chunks_processed(&self) -> usize { ... }
}
```

**Alternative:** Track counts separately in main.rs if ProgressTracker is not easily extensible.

## Data Flow

### Scan Decision Logic

```
Input: (repo, worktree, path, force)
  │
  ├─► Get git tree SHA
  │    Success: tree_sha = "abc123..."
  │    Failure: tree_sha = None → full scan
  │
  ├─► Get worktree_id
  │    Query: SELECT id FROM worktrees WHERE repo_id=? AND name=?
  │    Not found: Create worktree (first time)
  │
  ├─► Get last_tree_sha
  │    Query: SELECT last_tree_sha FROM worktree_index_state WHERE worktree_id=?
  │    Not found: "init" → full scan
  │
  ├─► Compare
  │    │
  │    ├─► if force:
  │    │     Log: "Force flag enabled, performing full scan"
  │    │     → Full scan
  │    │
  │    ├─► if tree_sha == last_tree_sha:
  │    │     Log: "No changes detected, skipping scan"
  │    │     → Skip (early return)
  │    │
  │    └─► else:
  │          Log: "Tree changed from {last} to {current}"
  │          → Full scan (for now; incremental in future)
  │
  └─► After scan:
       Update: worktree_index_state SET last_tree_sha = new_sha
```

### Database Queries

**1. Get or Create Worktree**
```sql
-- Get worktree_id (needed for state queries)
SELECT w.id
FROM maproom.worktrees w
JOIN maproom.repos r ON w.repo_id = r.id
WHERE r.name = $1 AND w.name = $2;

-- If not found, create:
INSERT INTO maproom.worktrees (repo_id, name, abs_path)
VALUES ((SELECT id FROM maproom.repos WHERE name = $1), $2, $3)
RETURNING id;
```

**2. Check Last Indexed State**
```sql
SELECT last_tree_sha
FROM maproom.worktree_index_state
WHERE worktree_id = $1;
```

**3. Update Index State**
```sql
INSERT INTO maproom.worktree_index_state
  (worktree_id, last_tree_sha, last_indexed, chunks_processed, embeddings_generated)
VALUES ($1, $2, NOW(), $3, $4)
ON CONFLICT (worktree_id) DO UPDATE
SET last_tree_sha = EXCLUDED.last_tree_sha,
    last_indexed = NOW(),
    chunks_processed = EXCLUDED.chunks_processed,
    embeddings_generated = EXCLUDED.embeddings_generated;
```

## Error Handling Strategy

### Fail-Safe Defaults

**Principle:** When in doubt, do a full scan. Never skip incorrectly.

| Error Scenario | Behavior | Rationale |
|----------------|----------|-----------|
| Git command fails | Full scan | Can't trust comparison |
| Tree SHA unavailable | Full scan | Unknown state |
| Database query fails | Full scan | Can't determine if changed |
| Worktree not found | Create + full scan | First-time index |
| State table empty | Full scan | First-time index |
| State update fails | Log warning, continue | Scan succeeded, state update is advisory |

### Error Messages

**Git Errors:**
```
⚠️  Could not determine git tree SHA: {error}
   Proceeding with full repository scan
```

**Database Errors:**
```
⚠️  Could not query index state: {error}
   Performing full scan to ensure correctness
```

**State Update Errors (non-fatal):**
```
⚠️  Warning: Could not save index state: {error}
   Scan completed successfully but next scan will be slower
   Consider running: crewchief-maproom db diagnose
```

## Performance Characteristics

### Skip Case (Most Common)

**Operations:**
1. Git tree SHA: 5-10ms
2. Database query (index state): 1-2ms
3. String comparison: <1ms

**Total:** ~10ms (vs 2-3 hours currently)

### Changed Files Case

**Operations:**
1. Tree SHA check: 10ms
2. Full scan: Variable (10s - 5min depending on repo size)
3. State update: 1-2ms

**Total:** Same as current + 10ms overhead (negligible)

### First-Time Index

**Operations:**
1. Tree SHA check: 10ms (no cached state)
2. Full scan: Variable
3. State update (INSERT): 1-2ms

**Total:** Same as current + 10ms overhead

## Testing Strategy

See `quality-strategy.md` for complete testing approach.

**Key Test Cases:**
1. Unchanged worktree → skip (verify early return)
2. Changed worktree → full scan (verify all files processed)
3. Force flag → full scan (verify --force overrides)
4. First-time scan → full scan + state saved
5. Git error → full scan (verify fallback)
6. Database error → full scan (verify fallback)

## Migration Path

**Phase 1 (This Project):**
- Add tree SHA check
- Add state persistence
- Verify with genetic optimizer

**Phase 2 (Future):**
- Integrate `git diff-tree` for true incremental (only changed files)
- Refactor `scan_worktree()` for pluggable file discovery
- Add remote state caching

**Phase 3 (Future):**
- Parallel tree SHA checks
- Predictive indexing (index on branch create)
- Smart embedding cache (reuse across branches)

## Deployment Considerations

**Backward Compatibility:**
- Existing databases work (table exists from migration 0020)
- Scans without tree SHA work (fallback to full)
- `--force` flag continues to work

**No Database Migration Required:**
- Table `worktree_index_state` already exists
- No schema changes needed

**Rollback Safety:**
- If this change causes issues, revert code only
- Database state harmless (just advisor table)
- Can manually clear table: `DELETE FROM maproom.worktree_index_state;`

## Success Metrics

1. **Genetic optimizer setup** < 2 minutes (vs 24+ hours)
2. **Re-scan main branch** < 1 second (vs 30-60 minutes)
3. **State table populated** after every scan
4. **Zero false skips** (correctness over performance)
5. **Graceful degradation** on any error (always safe)

## Open Questions

**Q: Should we parallelize tree SHA checks for multiple worktrees?**
A: No, not in this project. Keep it simple. Premature optimization.

**Q: Should we use `git diff-tree` to process only changed files?**
A: No, not in this project. That's the future refactoring (Phase 2).

**Q: What if two scans run concurrently on the same worktree?**
A: `ON CONFLICT DO UPDATE` handles this safely. Last writer wins.

**Q: Should we track tree SHA per commit or per worktree?**
A: Per worktree. A worktree can switch commits, and we want to detect that.
