//! `StoreEncoding` impl — encoding-run lifecycle (§6.4 / §5.3).
//!
//! `EncodingRunRow` string timestamps are formatted byte-identically to SQLite's
//! `datetime('now')` output via `to_char(<col> AT TIME ZONE 'UTC',
//! 'YYYY-MM-DD HH24:MI:SS')` (§5.3), so the rows are interchangeable across
//! backends. `status` is stored verbatim (canonical: running/completed/failed).

use async_trait::async_trait;
use sqlx::Row;

use super::PostgresStore;
use crate::db::traits::StoreEncoding;
use crate::db::types::EncodingRunRow;

#[async_trait]
impl StoreEncoding for PostgresStore {
    async fn create_encoding_run(
        &self,
        total_chunks: i64,
        provider: Option<&str>,
        dimension: Option<i32>,
    ) -> anyhow::Result<i64> {
        // status defaults to 'running', started_at to now() (migrations_pg schema).
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO encoding_runs (total_chunks, provider, dimension) \
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(total_chunks)
        .bind(provider)
        .bind(dimension)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn update_encoding_run_progress(
        &self,
        run_id: i64,
        chunks_completed: i64,
        chunks_per_second: Option<f64>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE encoding_runs \
             SET chunks_completed = $1, chunks_per_second = $2, last_batch_at = now() \
             WHERE id = $3",
        )
        .bind(chunks_completed)
        .bind(chunks_per_second)
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn complete_encoding_run(&self, run_id: i64, status: &str) -> anyhow::Result<()> {
        // status stored verbatim (free-form &str; canonical 'completed'/'failed').
        sqlx::query("UPDATE encoding_runs SET status = $1, finished_at = now() WHERE id = $2")
            .bind(status)
            .bind(run_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_stale_runs_as_failed(&self) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE encoding_runs SET status = 'failed', finished_at = now() WHERE status = 'running'",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_active_encoding_run(&self) -> anyhow::Result<Option<EncodingRunRow>> {
        let row = sqlx::query(
            "SELECT id, \
                to_char(started_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS started_at, \
                to_char(finished_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS finished_at, \
                status, total_chunks, chunks_completed, chunks_per_second, \
                to_char(last_batch_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS last_batch_at, \
                provider, dimension \
             FROM encoding_runs WHERE status = 'running' ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| EncodingRunRow {
            id: r.get("id"),
            started_at: r.get("started_at"),
            finished_at: r.get("finished_at"),
            status: r.get("status"),
            total_chunks: r.get("total_chunks"),
            chunks_completed: r.get("chunks_completed"),
            chunks_per_second: r.get("chunks_per_second"),
            last_batch_at: r.get("last_batch_at"),
            provider: r.get("provider"),
            dimension: r.get("dimension"),
        }))
    }
}
