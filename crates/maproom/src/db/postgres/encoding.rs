//! `StoreEncoding` — Phase-3 deliverable (encoding-run lifecycle, §6.4).
//!
//! Phase-1 stubs: verbatim signatures, empty/default returns, `// PARITY-TODO`.
//! Phase-3 will format the `String` timestamp fields via
//! `to_char(... AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')` (§5.3).

use async_trait::async_trait;

use super::PostgresStore;
use crate::db::traits::StoreEncoding;
use crate::db::types::EncodingRunRow;

#[allow(unused_variables)]
#[async_trait]
impl StoreEncoding for PostgresStore {
    async fn create_encoding_run(
        &self,
        total_chunks: i64,
        provider: Option<&str>,
        dimension: Option<i32>,
    ) -> anyhow::Result<i64> {
        // PARITY-TODO(Phase 3): INSERT (status 'running') RETURNING id.
        Ok(0)
    }

    async fn update_encoding_run_progress(
        &self,
        run_id: i64,
        chunks_completed: i64,
        chunks_per_second: Option<f64>,
    ) -> anyhow::Result<()> {
        // PARITY-TODO(Phase 3).
        Ok(())
    }

    async fn complete_encoding_run(&self, run_id: i64, status: &str) -> anyhow::Result<()> {
        // PARITY-TODO(Phase 3): set terminal status + finished_at = now().
        Ok(())
    }

    async fn mark_stale_runs_as_failed(&self) -> anyhow::Result<()> {
        // PARITY-TODO(Phase 3): UPDATE running -> failed.
        Ok(())
    }

    async fn get_active_encoding_run(&self) -> anyhow::Result<Option<EncodingRunRow>> {
        // PARITY-TODO(Phase 3).
        Ok(None)
    }
}
