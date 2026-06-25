//! `StoreIndexState` impl — per-worktree last-indexed tree sha + counters (§6.4).

use async_trait::async_trait;

use super::PostgresStore;
use crate::db::index_state::UpdateStats;
use crate::db::traits::StoreIndexState;

#[async_trait]
impl StoreIndexState for PostgresStore {
    async fn get_last_indexed_tree(&self, worktree_id: i64) -> anyhow::Result<String> {
        // Returns the literal "init" when never indexed (parity with SQLite).
        let tree: Option<String> =
            sqlx::query_scalar("SELECT tree_sha FROM index_state WHERE worktree_id = $1")
                .bind(worktree_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(tree.unwrap_or_else(|| "init".to_string()))
    }

    async fn update_index_state(
        &self,
        worktree_id: i64,
        tree_sha: &str,
        stats: &UpdateStats,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO index_state \
                 (worktree_id, tree_sha, chunks_processed, embeddings_generated, last_indexed) \
             VALUES ($1, $2, $3, $4, now()) \
             ON CONFLICT (worktree_id) DO UPDATE SET \
                 tree_sha = EXCLUDED.tree_sha, \
                 chunks_processed = EXCLUDED.chunks_processed, \
                 embeddings_generated = EXCLUDED.embeddings_generated, \
                 last_indexed = now()",
        )
        .bind(worktree_id)
        .bind(tree_sha)
        .bind(stats.chunks_processed)
        .bind(stats.embeddings_generated)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
