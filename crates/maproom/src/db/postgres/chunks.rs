//! `StoreChunks` impl — chunk upsert, context, and the chunk<->worktree
//! many-to-many junction (§6.4 / §6.7).

use async_trait::async_trait;
use sqlx::Row;

use super::PostgresStore;
use crate::db::traits::StoreChunks;
use crate::db::{ChunkContext, ChunkFull, ChunkRecord, ChunkSummary};

/// Single data-modifying CTE: upsert the chunk (populating `ts_doc` from
/// `ts_doc_text`) AND map it to the worktree, atomically. Shared by `insert_chunk`
/// (executed on the pool) and `insert_chunks_batch` (executed inside one
/// transaction). Bind order: see `bind_chunk`.
const INSERT_CHUNK_CTE: &str = "WITH up AS ( \
         INSERT INTO chunks \
             (file_id, blob_sha, symbol_name, kind, signature, docstring, \
              start_line, end_line, preview, ts_doc, recency_score, churn_score, metadata) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, to_tsvector('simple', $10), $11,$12,$13::jsonb) \
         ON CONFLICT (file_id, start_line, end_line) DO UPDATE SET \
             blob_sha = EXCLUDED.blob_sha, symbol_name = EXCLUDED.symbol_name, \
             kind = EXCLUDED.kind, signature = EXCLUDED.signature, \
             docstring = EXCLUDED.docstring, preview = EXCLUDED.preview, \
             ts_doc = EXCLUDED.ts_doc, recency_score = EXCLUDED.recency_score, \
             churn_score = EXCLUDED.churn_score, metadata = EXCLUDED.metadata \
         RETURNING id \
     ), wt AS ( \
         INSERT INTO chunk_worktrees (chunk_id, worktree_id) \
         SELECT id, $14 FROM up ON CONFLICT DO NOTHING RETURNING chunk_id \
     ) \
     SELECT id FROM up";

/// Bind a [`ChunkRecord`]'s fields to [`INSERT_CHUNK_CTE`] in order.
fn bind_chunk<'q>(
    q: sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments>,
    chunk: &'q ChunkRecord,
    metadata: Option<String>,
) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments> {
    q.bind(chunk.file_id)
        .bind(&chunk.blob_sha)
        .bind(chunk.symbol_name.as_deref())
        .bind(&chunk.kind)
        .bind(chunk.signature.as_deref())
        .bind(chunk.docstring.as_deref())
        .bind(chunk.start_line)
        .bind(chunk.end_line)
        .bind(&chunk.preview)
        .bind(&chunk.ts_doc_text)
        .bind(chunk.recency_score)
        .bind(chunk.churn_score)
        .bind(metadata)
        .bind(chunk.worktree_id)
}

impl PostgresStore {
    /// Map a `chunks JOIN files` row to a [`ChunkSummary`].
    fn row_to_summary(r: &sqlx::postgres::PgRow) -> ChunkSummary {
        ChunkSummary {
            id: r.get("id"),
            symbol_name: r.get("symbol_name"),
            kind: r.get("kind"),
            start_line: r.get("start_line"),
            end_line: r.get("end_line"),
            file_path: r.get("file_path"),
        }
    }
}

#[async_trait]
impl StoreChunks for PostgresStore {
    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64> {
        let metadata = chunk.metadata.as_ref().map(|v| v.to_string());
        let id: i64 = bind_chunk(sqlx::query_scalar(INSERT_CHUNK_CTE), chunk, metadata)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    async fn insert_chunks_batch(&self, chunks: &[ChunkRecord]) -> anyhow::Result<Vec<i64>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }
        // Run the whole batch in ONE transaction so a mid-batch failure rolls back
        // every chunk (no partially-indexed file), matching the SQLite backend.
        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let metadata = chunk.metadata.as_ref().map(|v| v.to_string());
            let id: i64 = bind_chunk(sqlx::query_scalar(INSERT_CHUNK_CTE), chunk, metadata)
                .fetch_one(&mut *tx)
                .await?;
            ids.push(id);
        }
        tx.commit().await?;
        Ok(ids)
    }

    async fn insert_chunk_edge(
        &self,
        src_chunk_id: i64,
        dst_chunk_id: i64,
        edge_type: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO chunk_edges (src_chunk_id, dst_chunk_id, type) VALUES ($1,$2,$3) \
             ON CONFLICT DO NOTHING",
        )
        .bind(src_chunk_id)
        .bind(dst_chunk_id)
        .bind(edge_type)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_edges_for_file(&self, file_id: i64) -> anyhow::Result<u64> {
        let res = sqlx::query(
            "DELETE FROM chunk_edges WHERE src_chunk_id IN ( \
                 SELECT id FROM chunks WHERE file_id = $1 \
             ) OR dst_chunk_id IN ( \
                 SELECT id FROM chunks WHERE file_id = $1 \
             )",
        )
        .bind(file_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    async fn remove_worktree_from_chunks(
        &self,
        worktree_id: i64,
        relpath: &str,
    ) -> anyhow::Result<i64> {
        // Drop this worktree's mapping for the file's chunks ...
        let affected = sqlx::query(
            "DELETE FROM chunk_worktrees WHERE worktree_id = $1 AND chunk_id IN ( \
                 SELECT c.id FROM chunks c JOIN files f ON c.file_id = f.id \
                 WHERE f.relpath = $2 \
             )",
        )
        .bind(worktree_id)
        .bind(relpath)
        .execute(&self.pool)
        .await?
        .rows_affected() as i64;
        // ... then GC chunks no longer referenced by any worktree.
        sqlx::query(
            "DELETE FROM chunks WHERE NOT EXISTS \
             (SELECT 1 FROM chunk_worktrees cw WHERE cw.chunk_id = chunks.id)",
        )
        .execute(&self.pool)
        .await?;
        Ok(affected)
    }

    async fn get_chunk_by_id(&self, chunk_id: i64) -> anyhow::Result<Option<ChunkFull>> {
        let row = sqlx::query(
            "SELECT c.id, c.file_id, c.blob_sha, c.symbol_name, c.kind, c.signature, \
                    c.docstring, c.start_line, c.end_line, c.preview, f.relpath AS file_path \
             FROM chunks c JOIN files f ON f.id = c.file_id WHERE c.id = $1",
        )
        .bind(chunk_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| ChunkFull {
            id: r.get("id"),
            file_id: r.get("file_id"),
            blob_sha: r.get("blob_sha"),
            symbol_name: r.get("symbol_name"),
            kind: r.get("kind"),
            signature: r.get("signature"),
            docstring: r.get("docstring"),
            start_line: r.get("start_line"),
            end_line: r.get("end_line"),
            preview: r.get("preview"),
            file_path: r.get("file_path"),
        }))
    }

    async fn get_file_chunks(&self, file_id: i64) -> anyhow::Result<Vec<ChunkSummary>> {
        let rows = sqlx::query(
            "SELECT c.id, c.symbol_name, c.kind, c.start_line, c.end_line, f.relpath AS file_path \
             FROM chunks c JOIN files f ON f.id = c.file_id \
             WHERE c.file_id = $1 ORDER BY c.start_line",
        )
        .bind(file_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(Self::row_to_summary).collect())
    }

    async fn get_chunk_context(
        &self,
        chunk_id: i64,
        surrounding: usize,
    ) -> anyhow::Result<Option<ChunkContext>> {
        let chunk = match self.get_chunk_by_id(chunk_id).await? {
            Some(c) => c,
            None => return Ok(None),
        };
        let file_path = chunk.file_path.clone();
        let rows = sqlx::query(
            "SELECT c.id, c.symbol_name, c.kind, c.start_line, c.end_line, f.relpath AS file_path \
             FROM chunks c JOIN files f ON f.id = c.file_id \
             WHERE c.file_id = $1 ORDER BY c.start_line, c.id",
        )
        .bind(chunk.file_id)
        .fetch_all(&self.pool)
        .await?;
        let pos = rows.iter().position(|r| r.get::<i64, _>("id") == chunk_id);
        let surrounding_chunks = match pos {
            Some(p) => {
                let start = p.saturating_sub(surrounding);
                let end = (p + surrounding + 1).min(rows.len());
                rows[start..end]
                    .iter()
                    .filter(|r| r.get::<i64, _>("id") != chunk_id)
                    .map(Self::row_to_summary)
                    .collect()
            }
            None => Vec::new(),
        };
        Ok(Some(ChunkContext {
            chunk,
            file_path,
            surrounding_chunks,
        }))
    }

    async fn find_chunk_by_symbol(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        symbol_name: &str,
        relpath: Option<&str>,
    ) -> anyhow::Result<Option<i64>> {
        // Scope the worktree by file ownership (f.worktree_id) and ORDER BY c.id
        // DESC — matching SQLite, so the same symbol resolves to the same chunk on
        // both backends.
        let id: Option<i64> = sqlx::query_scalar(
            "SELECT c.id FROM chunks c \
             JOIN files f ON f.id = c.file_id \
             WHERE f.repo_id = $1 AND c.symbol_name = $2 \
               AND ($3::bigint IS NULL OR f.worktree_id = $3) \
               AND ($4::text IS NULL OR f.relpath = $4) \
             ORDER BY c.id DESC LIMIT 1",
        )
        .bind(repo_id)
        .bind(symbol_name)
        .bind(worktree_id)
        .bind(relpath)
        .fetch_optional(&self.pool)
        .await?;
        Ok(id)
    }

    async fn delete_chunks_by_file(&self, file_id: i64) -> anyhow::Result<u64> {
        let res = sqlx::query("DELETE FROM chunks WHERE file_id = $1")
            .bind(file_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }

    async fn delete_chunks_by_ids(
        &self,
        worktree_id: i64,
        chunk_ids: &[i64],
    ) -> anyhow::Result<usize> {
        // Deliberate divergence from the legacy SqliteStore (same pattern as
        // delete_worktree_data, §3.2 / R-WT-1/R-WT-4): this Postgres path removes
        // only THIS worktree's membership and GCs chunks left orphaned, keeping the
        // content-addressed `code_embeddings` pool (chunk_edges cascade via FK).
        // The legacy SqliteStore deletes the chunk from every worktree and its
        // embedding (to keep its `vec_code` ANN index consistent).
        // One transaction so the junction-delete and the orphan-GC are atomic: a
        // concurrent re-index (add_chunk_to_worktree) or another delete can't
        // interleave between them and make the returned count unreliable.
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM chunk_worktrees WHERE worktree_id = $1 AND chunk_id = ANY($2)")
            .bind(worktree_id)
            .bind(chunk_ids)
            .execute(&mut *tx)
            .await?;
        // Return the count of chunks ACTUALLY deleted (those now orphaned), not the
        // junction-row count — so the caller's "Deleted N chunks" is truthful.
        let gc = sqlx::query(
            "DELETE FROM chunks WHERE id = ANY($1) AND NOT EXISTS \
             (SELECT 1 FROM chunk_worktrees cw WHERE cw.chunk_id = chunks.id)",
        )
        .bind(chunk_ids)
        .execute(&mut *tx)
        .await?;
        let deleted = gc.rows_affected() as usize;
        tx.commit().await?;
        Ok(deleted)
    }

    async fn get_chunks_for_worktree(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<Vec<(i64, String)>> {
        let rows = sqlx::query(
            "SELECT c.id, f.relpath FROM chunks c \
             JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
             JOIN files f ON f.id = c.file_id \
             WHERE cw.worktree_id = $1 ORDER BY c.id",
        )
        .bind(worktree_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get::<i64, _>("id"), r.get::<String, _>("relpath")))
            .collect())
    }

    async fn get_chunks_by_blob_sha(&self, blob_sha: &str) -> anyhow::Result<Vec<ChunkSummary>> {
        let rows = sqlx::query(
            "SELECT c.id, c.symbol_name, c.kind, c.start_line, c.end_line, f.relpath AS file_path \
             FROM chunks c JOIN files f ON f.id = c.file_id \
             WHERE c.blob_sha = $1 ORDER BY c.start_line",
        )
        .bind(blob_sha)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(Self::row_to_summary).collect())
    }

    async fn add_chunk_to_worktree(&self, chunk_id: i64, worktree_id: i64) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO chunk_worktrees (chunk_id, worktree_id) VALUES ($1,$2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(chunk_id)
        .bind(worktree_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_chunk_worktrees(&self, chunk_id: i64) -> anyhow::Result<Vec<i64>> {
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT worktree_id FROM chunk_worktrees WHERE chunk_id = $1 ORDER BY worktree_id",
        )
        .bind(chunk_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(ids)
    }
}
