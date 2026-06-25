//! `StoreCore` impl — repos/worktrees/commits/files identity + counts (§6.4).
//!
//! Idempotent get-or-create uses `INSERT … ON CONFLICT … DO UPDATE … RETURNING id`
//! so the id is returned even on conflict. Timestamps bind as RFC3339 strings
//! cast with `$N::timestamptz` (the crate intentionally omits sqlx's chrono
//! feature — see Cargo.toml).

use async_trait::async_trait;
use sqlx::Row;

use super::PostgresStore;
use crate::db::traits::StoreCore;
use crate::db::{FileRecord, RepoInfo, WorktreeInfo};

#[async_trait]
impl StoreCore for PostgresStore {
    fn has_vector_extension(&self) -> bool {
        PostgresStore::has_vector_extension(self)
    }

    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO repos (name, root_path) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET root_path = EXCLUDED.root_path \
             RETURNING id",
        )
        .bind(name)
        .bind(root_path)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn get_or_create_worktree(
        &self,
        repo_id: i64,
        name: &str,
        abs_path: &str,
    ) -> anyhow::Result<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) \
             ON CONFLICT (repo_id, name) DO UPDATE SET abs_path = EXCLUDED.abs_path \
             RETURNING id",
        )
        .bind(repo_id)
        .bind(name)
        .bind(abs_path)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn get_or_create_commit(
        &self,
        repo_id: i64,
        sha: &str,
        committed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64> {
        let committed_at = committed_at.map(|dt| dt.to_rfc3339());
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO commits (repo_id, sha, committed_at) VALUES ($1, $2, $3::timestamptz) \
             ON CONFLICT (repo_id, sha) DO UPDATE SET sha = EXCLUDED.sha \
             RETURNING id",
        )
        .bind(repo_id)
        .bind(sha)
        .bind(committed_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<RepoInfo>> {
        let row = sqlx::query("SELECT id, name, root_path FROM repos WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| RepoInfo {
            id: r.get("id"),
            name: r.get("name"),
            root_path: r.get("root_path"),
        }))
    }

    async fn get_worktree_by_name(
        &self,
        repo_id: i64,
        name: &str,
    ) -> anyhow::Result<Option<WorktreeInfo>> {
        let row = sqlx::query(
            "SELECT id, repo_id, name, abs_path FROM worktrees WHERE repo_id = $1 AND name = $2",
        )
        .bind(repo_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| WorktreeInfo {
            id: r.get("id"),
            repo_id: r.get("repo_id"),
            name: r.get("name"),
            abs_path: r.get("abs_path"),
        }))
    }

    async fn list_repos(&self) -> anyhow::Result<Vec<RepoInfo>> {
        let rows = sqlx::query("SELECT id, name, root_path FROM repos ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| RepoInfo {
                id: r.get("id"),
                name: r.get("name"),
                root_path: r.get("root_path"),
            })
            .collect())
    }

    async fn list_worktrees(&self, repo_id: i64) -> anyhow::Result<Vec<WorktreeInfo>> {
        let rows = sqlx::query(
            "SELECT id, repo_id, name, abs_path FROM worktrees WHERE repo_id = $1 ORDER BY id",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| WorktreeInfo {
                id: r.get("id"),
                repo_id: r.get("repo_id"),
                name: r.get("name"),
                abs_path: r.get("abs_path"),
            })
            .collect())
    }

    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64> {
        let last_modified = file.last_modified.map(|dt| dt.to_rfc3339());
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO files \
                 (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes, last_modified) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8::timestamptz) \
             ON CONFLICT (commit_id, relpath, content_hash) DO UPDATE SET \
                 worktree_id = EXCLUDED.worktree_id, \
                 language = EXCLUDED.language, \
                 size_bytes = EXCLUDED.size_bytes, \
                 last_modified = EXCLUDED.last_modified \
             RETURNING id",
        )
        .bind(file.repo_id)
        .bind(file.worktree_id)
        .bind(file.commit_id)
        .bind(&file.relpath)
        .bind(file.language.as_deref())
        .bind(&file.content_hash)
        .bind(file.size_bytes)
        .bind(last_modified)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn delete_file(&self, file_id: i64) -> anyhow::Result<bool> {
        let res = sqlx::query("DELETE FROM files WHERE id = $1")
            .bind(file_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn get_file_id_by_relpath(
        &self,
        relpath: &str,
        worktree_id: i64,
    ) -> anyhow::Result<Option<i64>> {
        let id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM files WHERE relpath = $1 AND worktree_id = $2 ORDER BY id DESC LIMIT 1",
        )
        .bind(relpath)
        .bind(worktree_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(id)
    }

    async fn get_file_content_hash(&self, file_id: i64) -> anyhow::Result<Option<String>> {
        let h: Option<String> = sqlx::query_scalar("SELECT content_hash FROM files WHERE id = $1")
            .bind(file_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(h)
    }

    async fn update_file_content_hash(
        &self,
        file_id: i64,
        content_hash: &str,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE files SET content_hash = $1 WHERE id = $2")
            .bind(content_hash)
            .bind(file_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_file_content_hashes(
        &self,
        file_ids: &[i64],
    ) -> anyhow::Result<Vec<(i64, String)>> {
        if file_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query("SELECT id, content_hash FROM files WHERE id = ANY($1)")
            .bind(file_ids)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get::<i64, _>("id"), r.get::<String, _>("content_hash")))
            .collect())
    }

    async fn find_file_relpath_by_content_hash(
        &self,
        content_hash: &str,
        exclude_relpath: &str,
    ) -> anyhow::Result<Option<String>> {
        let r: Option<String> = sqlx::query_scalar(
            "SELECT relpath FROM files WHERE content_hash = $1 AND relpath <> $2 LIMIT 1",
        )
        .bind(content_hash)
        .bind(exclude_relpath)
        .fetch_optional(&self.pool)
        .await?;
        Ok(r)
    }

    async fn get_worktree_chunk_count(&self, worktree_id: i64) -> anyhow::Result<i64> {
        let n: i64 =
            sqlx::query_scalar("SELECT count(*) FROM chunk_worktrees WHERE worktree_id = $1")
                .bind(worktree_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(n)
    }

    async fn get_worktree_file_count(&self, worktree_id: i64) -> anyhow::Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM files WHERE worktree_id = $1")
            .bind(worktree_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    async fn get_worktree_embedding_count(&self, worktree_id: i64) -> anyhow::Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT c.blob_sha) FROM chunks c \
             JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
             JOIN code_embeddings e ON e.blob_sha = c.blob_sha \
             WHERE cw.worktree_id = $1",
        )
        .bind(worktree_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    async fn get_worktree_language_breakdown(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<Vec<(String, i64)>> {
        let rows = sqlx::query(
            "SELECT COALESCE(f.language, 'unknown') AS language, count(*) AS n \
             FROM files f WHERE f.worktree_id = $1 \
             GROUP BY f.language ORDER BY n DESC",
        )
        .bind(worktree_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>("language"), r.get::<i64, _>("n")))
            .collect())
    }

    async fn get_worktree_last_scan(&self, worktree_id: i64) -> anyhow::Result<Option<String>> {
        let ts: Option<String> = sqlx::query_scalar(
            "SELECT to_char(last_indexed AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') \
             FROM index_state WHERE worktree_id = $1",
        )
        .bind(worktree_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(ts)
    }

    async fn get_global_chunk_count(&self) -> anyhow::Result<i64> {
        // DISTINCT blob_sha (content-addressed) — parity with SQLite.
        let n: i64 = sqlx::query_scalar("SELECT count(DISTINCT blob_sha) FROM chunks")
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    async fn get_global_embedding_count(&self) -> anyhow::Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM code_embeddings")
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    async fn get_repo_chunk_count(&self, repo_name: &str) -> anyhow::Result<i64> {
        // Resolve repo by exact name or '%/name' suffix (mirrors resolve_repo_id).
        let n: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT c.id) FROM chunks c \
             JOIN files f ON f.id = c.file_id \
             JOIN repos r ON r.id = f.repo_id \
             WHERE r.name = $1 OR r.name LIKE '%/' || $1",
        )
        .bind(repo_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    async fn get_repo_embedding_count(&self, repo_name: &str) -> anyhow::Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT e.blob_sha) FROM code_embeddings e \
             JOIN chunks c ON c.blob_sha = e.blob_sha \
             JOIN files f ON f.id = c.file_id \
             JOIN repos r ON r.id = f.repo_id \
             WHERE r.name = $1 OR r.name LIKE '%/' || $1",
        )
        .bind(repo_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }
}
