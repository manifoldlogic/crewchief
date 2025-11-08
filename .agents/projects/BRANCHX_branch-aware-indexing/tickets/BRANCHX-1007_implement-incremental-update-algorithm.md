# Ticket: BRANCHX-1007: Implement incremental update algorithm

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create the core incremental update function that compares tree SHA, finds changed files, and processes only changes.

## Background
This is Phase 3, Step 3.1 of BRANCHX—the heart of the project. After establishing schema (Phase 1) and git integration (Phase 2), we now implement the algorithm that makes incremental updates work: compare current tree SHA to last indexed SHA, skip if identical, otherwise use git diff-tree to find changed files and process only those.

This is the 5-10x performance improvement that makes branch switching fast.

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 3.1

## Acceptance Criteria
- [ ] New file `crates/maproom/src/incremental.rs` created
- [ ] `incremental_update(pool, worktree_id, repo_path)` function implemented
- [ ] Tree SHA comparison: if unchanged, return stats with 0 chunks processed
- [ ] If changed, git diff-tree finds changed files
- [ ] Only changed files are processed (not full scan)
- [ ] UpdateStats tracks files_processed, chunks_processed, embeddings_generated
- [ ] Tree SHA updated in database after successful processing
- [ ] Unit tests pass

## Technical Requirements
- Return `Result<UpdateStats>` with comprehensive metrics
- Handle empty diff (no changes) efficiently
- Handle git errors gracefully (fallback to full scan or error?)
- Process Added/Modified files: parse chunks, upsert with worktree_id
- Process Deleted files: remove worktree from chunks (next ticket)
- Use existing parse_file and upsert_chunk functions from BLOBSHA
- Log progress: "Processing 100 changed files (out of 1,000 total)"

## Implementation Notes

Create `crates/maproom/src/incremental.rs`:

```rust
use crate::git::{get_git_tree_sha, git_diff_tree, FileStatus};
use crate::index_state::{get_last_indexed_tree, update_index_state, UpdateStats};
use crate::parse::parse_file_into_chunks;
use crate::upsert::upsert_chunk_with_worktree;
use anyhow::Result;
use sqlx::PgPool;
use std::path::Path;
use tracing::info;

pub async fn incremental_update(
    pool: &PgPool,
    worktree_id: i32,
    repo_path: &Path,
) -> Result<UpdateStats> {
    // 1. Get current git tree SHA
    let current_tree = get_git_tree_sha(repo_path)?;
    info!("Current tree SHA: {}", current_tree);

    // 2. Get last indexed tree SHA
    let last_tree = get_last_indexed_tree(pool, worktree_id).await?;
    info!("Last indexed tree SHA: {}", last_tree);

    // 3. Quick check: changed?
    if current_tree == last_tree {
        info!("No changes detected (tree SHA match), skipping scan");
        return Ok(UpdateStats::skipped());
    }

    // 4. Find changed files
    let changed_files = git_diff_tree(&last_tree, &current_tree, repo_path)?;
    info!("Found {} changed files", changed_files.len());

    // 5. Process changes
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

                stats.files_processed += 1;
            }
            FileStatus::Deleted => {
                // Handle in next ticket (BRANCHX-1008)
                info!("Skipping deleted file (not yet implemented): {:?}", file.path);
            }
        }
    }

    // 6. Update index state
    update_index_state(pool, worktree_id, &current_tree, &stats).await?;
    info!("Index state updated");

    Ok(stats)
}
```

UpdateStats implementation:
```rust
impl UpdateStats {
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            chunks_processed: 0,
            embeddings_generated: 0,
        }
    }

    pub fn skipped() -> Self {
        Self::new()
    }

    pub fn cache_hit_rate(&self) -> f64 {
        if self.chunks_processed == 0 { return 1.0; }
        1.0 - (self.embeddings_generated as f64 / self.chunks_processed as f64)
    }

    pub fn cost(&self) -> f64 {
        self.embeddings_generated as f64 * 0.00002 // $0.00002 per embedding
    }
}
```

See `architecture.md` section "Incremental Update Algorithm" for complete design.

## Dependencies
- BRANCHX-1004 complete (git functions)
- BRANCHX-1005 complete (index state functions)
- BRANCHX-1006 tests pass (git integration validated)
- Existing BLOBSHA functions: parse_file_into_chunks, compute_blob_sha

## Risk Assessment
- **Risk**: Incremental scan produces different results than full scan
  - **Mitigation**: Comprehensive test in BRANCHX-1901 (test_incremental_equals_full_scan)
- **Risk**: Tree SHA comparison misses changes
  - **Mitigation**: Git tree SHA is cryptographically guaranteed (SHA-256)
- **Risk**: Processing fails midway, database in inconsistent state
  - **Mitigation**: Consider transactions or atomic updates (future enhancement)

## Files/Packages Affected
- `crates/maproom/src/incremental.rs` (new)
- `crates/maproom/src/lib.rs` (add `pub mod incremental;`)

## Planning References
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 3.1 (lines 127-163)
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Incremental Update Algorithm (lines 76-129)
