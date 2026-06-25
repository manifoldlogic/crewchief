//! `StoreGraph` — Phase-3 deliverable (recursive CTE traversal + scoring, §6.9).
//!
//! Traversal (`find_callers`/`callees`/`imports`/`extensions`/`get_direct_edges`)
//! is a faithful Postgres port of `db::sqlite::graph` (§6.9): one `WITH RECURSIVE`
//! per direction over `chunk_edges`, default depth 3 / hard cap 10 (R-NFR-2), the
//! same `'/id/id/…'` path-string + `NOT LIKE` cycle detection, and the same
//! `SELECT DISTINCT chunk_id, depth, path ORDER BY depth, chunk_id` shape so rows
//! are byte-for-byte comparable across backends. Integers are cast to text in-SQL
//! (`||` needs an explicit `::text` in Postgres, unlike SQLite's loose typing).
//!
//! The four `calculate_*` scoring methods remain Phase-3b stubs (`// PARITY-TODO`).

use async_trait::async_trait;
use sqlx::Row;

use super::PostgresStore;
use crate::config::EdgeQualityWeights;
use crate::db::traits::StoreGraph;
use crate::db::types::{GraphResult, ImportDirection};
use crate::db::SearchHit;

/// Default traversal depth when the caller passes `None` (mirrors SQLite's
/// `DEFAULT_MAX_DEPTH`).
const DEFAULT_MAX_DEPTH: usize = 3;
/// Hard ceiling on traversal depth regardless of caller request (R-NFR-2;
/// mirrors SQLite's `HARD_MAX_DEPTH`).
const HARD_MAX_DEPTH: usize = 10;

/// Parse a `'/1/2/3'` path string into `[1, 2, 3]` (mirrors SQLite's
/// `parse_path`: split on `/`, drop empties, ignore unparseable segments).
fn parse_path(s: &str) -> Vec<i64> {
    s.split('/')
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect()
}

impl PostgresStore {
    /// Shared recursive-CTE traversal over `chunk_edges` of a single edge `type`.
    ///
    /// `forward = true` follows edges src→dst (callees / outgoing imports / what
    /// this extends); `forward = false` follows dst→src (callers / incoming
    /// imports / subclasses). Identical traversal/cycle/order semantics to the
    /// SQLite backend; only the integer→text casts differ (Postgres `||`).
    async fn traverse_edges(
        &self,
        start: i64,
        edge_type: &str,
        max_depth: Option<usize>,
        forward: bool,
    ) -> anyhow::Result<Vec<GraphResult>> {
        let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).min(HARD_MAX_DEPTH) as i32;
        // `anchor` is the column matched against the current frontier; `neighbor`
        // is the column emitted as the next hop. Forward: anchor=src, emit dst.
        // Backward: anchor=dst, emit src.
        let (anchor, neighbor) = if forward {
            ("src_chunk_id", "dst_chunk_id")
        } else {
            ("dst_chunk_id", "src_chunk_id")
        };
        let sql = format!(
            "WITH RECURSIVE walk(chunk_id, depth, path) AS ( \
                 SELECT {neighbor}, 1, '/' || {neighbor}::text \
                 FROM chunk_edges \
                 WHERE {anchor} = $1 AND type = $2 \
                 UNION ALL \
                 SELECT e.{neighbor}, w.depth + 1, w.path || '/' || e.{neighbor}::text \
                 FROM chunk_edges e \
                 JOIN walk w ON e.{anchor} = w.chunk_id \
                 WHERE w.depth < $3 \
                   AND e.type = $2 \
                   AND w.path NOT LIKE '%/' || e.{neighbor}::text || '/%' \
                   AND w.path NOT LIKE '%/' || e.{neighbor}::text \
             ) \
             SELECT DISTINCT chunk_id, depth, path FROM walk ORDER BY depth, chunk_id"
        );
        let rows = sqlx::query(&sql)
            .bind(start)
            .bind(edge_type)
            .bind(depth)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let path: String = r.get("path");
                GraphResult {
                    chunk_id: r.get("chunk_id"),
                    depth: r.get::<i32, _>("depth") as usize,
                    path: parse_path(&path),
                    edge_type: edge_type.to_string(),
                }
            })
            .collect())
    }
}

#[async_trait]
impl StoreGraph for PostgresStore {
    async fn find_callers(
        &self,
        target_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // Who calls the target → follow 'calls' edges backward (dst→src).
        self.traverse_edges(target_chunk_id, "calls", max_depth, false)
            .await
    }

    async fn find_callees(
        &self,
        source_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // What the source calls → follow 'calls' edges forward (src→dst).
        self.traverse_edges(source_chunk_id, "calls", max_depth, true)
            .await
    }

    async fn find_imports(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // Incoming = who imports this (backward); Outgoing = what this imports (forward).
        let forward = matches!(direction, ImportDirection::Outgoing);
        self.traverse_edges(chunk_id, "imports", max_depth, forward)
            .await
    }

    async fn find_extensions(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // Incoming = subclasses (backward); Outgoing = superclasses (forward).
        let forward = matches!(direction, ImportDirection::Outgoing);
        self.traverse_edges(chunk_id, "extends", max_depth, forward)
            .await
    }

    async fn get_direct_edges(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
    ) -> anyhow::Result<Vec<GraphResult>> {
        // Depth-1 only, ANY edge type; edge_type comes straight from the row.
        let sql = match direction {
            ImportDirection::Incoming => {
                "SELECT src_chunk_id AS related_id, type FROM chunk_edges WHERE dst_chunk_id = $1"
            }
            ImportDirection::Outgoing => {
                "SELECT dst_chunk_id AS related_id, type FROM chunk_edges WHERE src_chunk_id = $1"
            }
        };
        let rows = sqlx::query(sql)
            .bind(chunk_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let related_id: i64 = r.get("related_id");
                GraphResult {
                    chunk_id: related_id,
                    depth: 1,
                    path: vec![related_id],
                    edge_type: r.get("type"),
                }
            })
            .collect())
    }

    #[allow(unused_variables)]
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

    #[allow(unused_variables)]
    async fn calculate_graph_importance_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 3).
        Ok(Vec::new())
    }

    #[allow(unused_variables)]
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

    #[allow(unused_variables)]
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
