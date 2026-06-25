//! `StoreGraph` — Phase-3 deliverable (recursive CTE traversal + scoring, §6.9).
//!
//! Phase-1 stubs: verbatim signatures, empty returns, `// PARITY-TODO`.

use async_trait::async_trait;

use super::PostgresStore;
use crate::config::EdgeQualityWeights;
use crate::db::traits::StoreGraph;
use crate::db::types::{GraphResult, ImportDirection};
use crate::db::SearchHit;

#[allow(unused_variables)]
#[async_trait]
impl StoreGraph for PostgresStore {
    async fn find_callers(
        &self,
        target_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // PARITY-TODO(Phase 3): WITH RECURSIVE over chunk_edges 'calls', depth 3/cap 10.
        Ok(Vec::new())
    }

    async fn find_callees(
        &self,
        source_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // PARITY-TODO(Phase 3).
        Ok(Vec::new())
    }

    async fn find_imports(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // PARITY-TODO(Phase 3): 'imports' edges.
        Ok(Vec::new())
    }

    async fn find_extensions(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // PARITY-TODO(Phase 3): 'extends' edges.
        Ok(Vec::new())
    }

    async fn get_direct_edges(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // PARITY-TODO(Phase 3): depth-1 edges.
        Ok(Vec::new())
    }

    async fn calculate_graph_importance(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        enable_quality: bool,
        weights: &EdgeQualityWeights,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 3).
        Ok(Vec::new())
    }

    async fn calculate_graph_importance_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 3).
        Ok(Vec::new())
    }

    async fn calculate_signal_scores(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 3).
        Ok(Vec::new())
    }

    async fn calculate_signal_scores_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 3).
        Ok(Vec::new())
    }
}
