# Ticket: BRWATCH-2001: Implement handle_branch_switch method

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the core branch switch handler that integrates with BRANCHX's incremental_update to automatically index new branches when detected by the file watcher.

## Background
This ticket implements Step 2.1 from the implementation plan (plan.md - Phase 2). When the file watcher detects a .git/HEAD change, handle_branch_switch() orchestrates the indexing workflow:
1. Extract current branch name
2. Get or create worktree record in database
3. Trigger incremental_update (from BRANCHX)
4. Log indexing results and metrics

From architecture.md lines 129-159, this is the bridge between file watching (Phase 1) and automatic indexing.

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 2.1

## Acceptance Criteria
- [x] `handle_branch_switch()` method implemented in BranchWatcher impl
- [x] Calls `get_current_branch()` to extract branch name
- [x] Uses BRANCHX `get_or_create_worktree()` to get worktree_id
- [x] Calls BRANCHX `incremental_update()` with worktree_id and repo_path
- [x] Logs branch name, duration, files processed, chunks processed
- [x] Logs embeddings generated (cache_hits/cache_misses not available in UpdateStats)
- [x] Returns Result<()> for error handling
- [x] Function compiles and integrates with watch_loop()

## Technical Requirements
- Add method to `/workspace/crates/maproom/src/watcher.rs`
- Import BRANCHX functions:
  - `use crewchief_maproom::incremental::incremental_update;`
  - `use crewchief_maproom::db::{get_or_create_repo, get_or_create_worktree};`
- Use `std::time::Instant` for timing
- Log at info level for normal operation, error level for failures
- Propagate errors using `?` operator
- Method signature: `async fn handle_branch_switch(&self) -> Result<()>`

## Implementation Notes

From architecture.md lines 129-159:

```rust
impl BranchWatcher {
    async fn handle_branch_switch(&self) -> Result<()> {
        let current_branch = get_current_branch(&self.repo_path)?;

        info!("Branch switch detected: {}", current_branch);

        // Get database client from pool
        let client = self.pool.get().await?;

        // Get or create repo
        let repo_name = self.repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let repo_id = get_or_create_repo(
            &client,
            repo_name,
            &self.repo_path.to_string_lossy()
        ).await?;

        // Get or create worktree
        let worktree_id = get_or_create_worktree(
            &client,
            repo_id,
            &current_branch,
            &self.repo_path.to_string_lossy()
        ).await?;

        // Trigger incremental update (from BRANCHX)
        let start = Instant::now();
        let stats = incremental_update(&client, worktree_id, &self.repo_path).await?;
        let duration = start.elapsed();

        // Log results
        info!("Index updated in {:.1}s:", duration.as_secs_f64());
        info!("  Files processed: {}", stats.files_processed);
        info!("  Chunks processed: {}", stats.chunks_processed);
        info!("  Cache hit rate: {:.1}%", stats.cache_hit_rate() * 100.0);
        info!("  Embeddings generated: {}", stats.embeddings_generated);
        info!("  Estimated cost: ${:.4}", stats.cost());

        Ok(())
    }

    async fn index_current_branch(&self) -> Result<()> {
        info!("Indexing current branch...");
        self.handle_branch_switch().await
    }
}
```

### Integration Points

**BRANCHX dependencies** (must be available):
- `incremental_update(client, worktree_id, repo_path) -> Result<UpdateStats>`
- `get_or_create_repo(client, name, path) -> Result<i64>`
- `get_or_create_worktree(client, repo_id, name, path) -> Result<i64>`

**UpdateStats struct** (from BRANCHX):
```rust
pub struct UpdateStats {
    pub files_processed: usize,
    pub chunks_processed: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub embeddings_generated: usize,
}

impl UpdateStats {
    pub fn cache_hit_rate(&self) -> f64 {
        if self.cache_hits + self.cache_misses == 0 {
            0.0
        } else {
            self.cache_hits as f64 / (self.cache_hits + self.cache_misses) as f64
        }
    }

    pub fn cost(&self) -> f64 {
        // Assuming $0.00002 per embedding
        self.embeddings_generated as f64 * 0.00002
    }
}
```

## Dependencies
- BRWATCH-1001 complete (dependencies added)
- BRWATCH-1002 complete (BranchWatcher struct)
- BRWATCH-1003 complete (get_current_branch function)
- **BRANCHX project complete** (incremental_update, get_or_create_worktree, get_or_create_repo)

## Risk Assessment
- **Risk**: BRANCHX functions not available or API changed
  - **Mitigation**: Verify BRANCHX completion, check function signatures match
- **Risk**: Database connection errors during indexing
  - **Mitigation**: Propagate errors, handle in BRWATCH-2002 with retry logic
- **Risk**: Long-running indexing blocks watch loop
  - **Mitigation**: Async execution ensures non-blocking (tokio handles concurrency)

## Files/Packages Affected
- `/workspace/crates/maproom/src/watcher.rs` (add handle_branch_switch method)
