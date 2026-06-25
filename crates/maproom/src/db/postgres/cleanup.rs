//! `StoreCleanup` — Phase-3 deliverable (stale-worktree detection + orphan GC, §6.4).
//!
//! Phase-1 stubs: verbatim signatures, empty/default returns, `// PARITY-TODO`.

use async_trait::async_trait;

use super::PostgresStore;
use crate::db::traits::StoreCleanup;
use crate::db::{StaleWorktree, WorktreeCleanupResult};

#[allow(unused_variables)]
#[async_trait]
impl StoreCleanup for PostgresStore {
    async fn detect_stale_worktrees(&self) -> anyhow::Result<Vec<StaleWorktree>> {
        // PARITY-TODO(Phase 3): disk-existence check over worktrees + per-wt chunk_count.
        Ok(Vec::new())
    }

    async fn delete_worktree_data(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<WorktreeCleanupResult> {
        // PARITY-TODO(Phase 3): drop chunk_worktrees rows, GC orphan chunks (keep
        // code_embeddings), delete worktree (CASCADE index_state), return summary.
        Ok(WorktreeCleanupResult::default())
    }
}
