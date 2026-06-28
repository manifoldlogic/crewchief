//! `StoreCleanup` impl — stale-worktree detection + orphan GC (§6.4, R-WT-4).
//!
//! Deliberate divergence from SQLite (§3.2/§6.4/R-WT-4): `delete_worktree_data`
//! does NOT delete `code_embeddings` — the content-addressed pool is persistent —
//! and orphan-GCs chunks via the `chunk_worktrees` junction so a chunk still
//! mapped to another worktree survives. (SQLite's legacy impl deletes embeddings;
//! the spec's §7 acceptance asserts the keep-embeddings/keep-multi-wt behavior.)

use async_trait::async_trait;
use sqlx::Row;

use super::PostgresStore;
use crate::db::traits::StoreCleanup;
use crate::db::{StaleWorktree, WorktreeCleanupResult};

#[async_trait]
impl StoreCleanup for PostgresStore {
    async fn detect_stale_worktrees(&self) -> anyhow::Result<Vec<StaleWorktree>> {
        let rows = sqlx::query("SELECT id, repo_id, name, abs_path FROM worktrees ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        let mut stale = Vec::new();
        for r in rows {
            let abs_path: String = r.get("abs_path");
            // Disk-existence check; PermissionDenied is treated as "exists"
            // (backend-agnostic detection, §6.4).
            let exists = match tokio::fs::try_exists(&abs_path).await {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => true,
                Err(_) => false,
            };
            if exists {
                continue;
            }
            let id: i64 = r.get("id");
            let chunk_count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM chunk_worktrees WHERE worktree_id = $1")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await?;
            stale.push(StaleWorktree {
                id,
                repo_id: r.get("repo_id"),
                name: r.get("name"),
                abs_path,
                exists: false,
                chunk_count,
            });
        }
        Ok(stale)
    }

    async fn delete_worktree_data(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<WorktreeCleanupResult> {
        // 1. Drop this worktree's chunk<->worktree mappings.
        sqlx::query("DELETE FROM chunk_worktrees WHERE worktree_id = $1")
            .bind(worktree_id)
            .execute(&self.pool)
            .await?;

        // 2. GC chunks no longer referenced by ANY worktree (keeps multi-wt chunks).
        let chunks_deleted = sqlx::query(
            "DELETE FROM chunks WHERE NOT EXISTS \
             (SELECT 1 FROM chunk_worktrees cw WHERE cw.chunk_id = chunks.id)",
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        // 3. GC this worktree's now-chunkless files (shared files keep their chunks).
        let files_deleted = sqlx::query(
            "DELETE FROM files WHERE worktree_id = $1 \
             AND NOT EXISTS (SELECT 1 FROM chunks c WHERE c.file_id = files.id)",
        )
        .bind(worktree_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        // 4. Delete the worktree row (CASCADE drops index_state; files.worktree_id
        //    is SET NULL, so any surviving shared files/chunks are retained).
        sqlx::query("DELETE FROM worktrees WHERE id = $1")
            .bind(worktree_id)
            .execute(&self.pool)
            .await?;

        // code_embeddings are intentionally NOT deleted (persistent pool, R-WT-4).
        Ok(WorktreeCleanupResult {
            chunks_deleted,
            files_deleted,
            embeddings_deleted: 0,
        })
    }
}
