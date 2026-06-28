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
//! Scoring (`calculate_graph_importance` legacy + quality, `*_for_chunks`, and
//! `calculate_signal_scores`/`*_for_chunks`) ports the SQLite log/weighted edge
//! formulas verbatim; the only dialect changes are `IN (…)` → `= ANY($1)` over a
//! bound bigint array and an explicit `::double precision` cast on every score
//! expression so it decodes as `f64` (Postgres `numeric` arithmetic would not).

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

    /// Legacy edge-count graph importance (unweighted log formula). Mirrors
    /// SQLite's `calculate_graph_importance_legacy`: log-scaled, weighted blend
    /// of inbound call/import/test edges. The score expression is cast to
    /// `double precision` so it decodes as `f64` (Postgres `numeric` would not).
    async fn graph_importance_legacy(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchHit>> {
        const SCORE: &str = "COALESCE( \
                ln(2 + COALESCE(e.callers, 0)::double precision) * 0.3 \
              + ln(2 + COALESCE(e.importers, 0)::double precision) * 0.2 \
              + ln(2 + COALESCE(e.tests, 0)::double precision) * 0.1, \
            0)::double precision AS graph_score";
        const EDGE_COUNTS: &str = "WITH edge_counts AS ( \
                SELECT dst_chunk_id AS chunk_id, \
                    SUM(CASE WHEN type = 'calls' THEN 1 ELSE 0 END) AS callers, \
                    SUM(CASE WHEN type = 'imports' THEN 1 ELSE 0 END) AS importers, \
                    SUM(CASE WHEN type = 'test_of' THEN 1 ELSE 0 END) AS tests \
                FROM chunk_edges GROUP BY dst_chunk_id \
            )";
        let rows = match worktree_id {
            Some(wid) => {
                let sql = format!(
                    "{EDGE_COUNTS} \
                     SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SCORE} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
                     LEFT JOIN edge_counts e ON e.chunk_id = c.id \
                     WHERE f.repo_id = $1 AND cw.worktree_id = $2 \
                     ORDER BY graph_score DESC LIMIT $3"
                );
                sqlx::query(&sql)
                    .bind(repo_id)
                    .bind(wid)
                    .bind(limit as i64)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                let sql = format!(
                    "{EDGE_COUNTS} \
                     SELECT DISTINCT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SCORE} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     LEFT JOIN edge_counts e ON e.chunk_id = c.id \
                     WHERE f.repo_id = $1 \
                     ORDER BY graph_score DESC LIMIT $2"
                );
                sqlx::query(&sql)
                    .bind(repo_id)
                    .bind(limit as i64)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(rows
            .iter()
            .map(|r| row_to_score_hit(r, "graph_score"))
            .collect())
    }

    /// Quality-weighted graph importance (SRCHREL-2002). Mirrors SQLite's
    /// `calculate_graph_importance_quality`: each inbound edge is weighted by
    /// edge type (calls weight) and source-code class (test vs production, by
    /// path/kind heuristics), summed, then log-scaled.
    async fn graph_importance_quality(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        weights: &EdgeQualityWeights,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let calls_weight = weights.calls as f64;
        let test_code_weight = weights.test_code as f64;
        let production_code_weight = weights.production_code as f64;
        // Test-source heuristic (identical patterns to SQLite).
        const TEST_PRED: &str = "src_file.relpath LIKE '%/test/%' \
                OR src_file.relpath LIKE '%/tests/%' \
                OR src_file.relpath LIKE '%/__tests__/%' \
                OR src_file.relpath LIKE '%.test.ts%' \
                OR src_file.relpath LIKE '%.test.js%' \
                OR src_file.relpath LIKE '%.test.tsx%' \
                OR src_file.relpath LIKE '%.test.jsx%' \
                OR src_file.relpath LIKE '%.spec.ts%' \
                OR src_file.relpath LIKE '%.spec.js%' \
                OR src_file.relpath LIKE '%_test.rs%' \
                OR src_file.relpath LIKE '%_test.py%' \
                OR src_chunk.kind LIKE '%test%'";
        const SCORE: &str =
            "COALESCE(ln(2.0 + COALESCE(i.quality_weighted_sum, 0.0)), 0.0)::double precision \
             AS graph_score";
        let rows = match worktree_id {
            Some(wid) => {
                // $1 repo, $2 worktree, $3 limit, $4 calls, $5 test, $6 prod.
                let sql = format!(
                    "WITH quality_edges AS ( \
                         SELECT ce.dst_chunk_id AS chunk_id, \
                             (CASE ce.type WHEN 'calls' THEN $4 ELSE 1.0 END) \
                           * (CASE WHEN {TEST_PRED} THEN $5 ELSE $6 END) AS edge_quality \
                         FROM chunk_edges ce \
                         JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id \
                         JOIN files src_file ON src_file.id = src_chunk.file_id \
                         WHERE ce.dst_chunk_id IN ( \
                             SELECT c.id FROM chunks c \
                             JOIN files f ON f.id = c.file_id \
                             JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
                             WHERE f.repo_id = $1 AND cw.worktree_id = $2 \
                         ) \
                     ), \
                     importance_scores AS ( \
                         SELECT chunk_id, SUM(edge_quality) AS quality_weighted_sum \
                         FROM quality_edges GROUP BY chunk_id \
                     ) \
                     SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SCORE} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
                     LEFT JOIN importance_scores i ON i.chunk_id = c.id \
                     WHERE f.repo_id = $1 AND cw.worktree_id = $2 \
                     ORDER BY graph_score DESC LIMIT $3"
                );
                sqlx::query(&sql)
                    .bind(repo_id)
                    .bind(wid)
                    .bind(limit as i64)
                    .bind(calls_weight)
                    .bind(test_code_weight)
                    .bind(production_code_weight)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                // $1 repo, $2 limit, $3 calls, $4 test, $5 prod.
                let sql = format!(
                    "WITH quality_edges AS ( \
                         SELECT ce.dst_chunk_id AS chunk_id, \
                             (CASE ce.type WHEN 'calls' THEN $3 ELSE 1.0 END) \
                           * (CASE WHEN {TEST_PRED} THEN $4 ELSE $5 END) AS edge_quality \
                         FROM chunk_edges ce \
                         JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id \
                         JOIN files src_file ON src_file.id = src_chunk.file_id \
                         WHERE ce.dst_chunk_id IN ( \
                             SELECT c.id FROM chunks c \
                             JOIN files f ON f.id = c.file_id \
                             WHERE f.repo_id = $1 \
                         ) \
                     ), \
                     importance_scores AS ( \
                         SELECT chunk_id, SUM(edge_quality) AS quality_weighted_sum \
                         FROM quality_edges GROUP BY chunk_id \
                     ) \
                     SELECT DISTINCT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SCORE} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     LEFT JOIN importance_scores i ON i.chunk_id = c.id \
                     WHERE f.repo_id = $1 \
                     ORDER BY graph_score DESC LIMIT $2"
                );
                sqlx::query(&sql)
                    .bind(repo_id)
                    .bind(limit as i64)
                    .bind(calls_weight)
                    .bind(test_code_weight)
                    .bind(production_code_weight)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(rows
            .iter()
            .map(|r| row_to_score_hit(r, "graph_score"))
            .collect())
    }
}

/// Map a score-query row (id/lines/symbol/kind/relpath + a `double precision`
/// score column) into a `SearchHit`. Shared by the importance/signal queries.
fn row_to_score_hit(r: &sqlx::postgres::PgRow, score_col: &str) -> SearchHit {
    SearchHit {
        chunk_id: r.get("id"),
        start_line: r.get("start_line"),
        end_line: r.get("end_line"),
        symbol_name: r.get("symbol_name"),
        kind: r.get("kind"),
        file_relpath: r.get("relpath"),
        score: r.get::<f64, _>(score_col),
        base_score: None,
        kind_mult: None,
        exact_mult: None,
        preview: None,
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

    async fn calculate_graph_importance(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        enable_quality: bool,
        weights: &EdgeQualityWeights,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // Dispatch mirrors SQLite: quality path when enabled, else legacy log.
        if enable_quality {
            self.graph_importance_quality(repo_id, worktree_id, limit, weights)
                .await
        } else {
            self.graph_importance_legacy(repo_id, worktree_id, limit)
                .await
        }
    }

    async fn calculate_graph_importance_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        if chunk_ids.is_empty() {
            return Ok(Vec::new());
        }
        // Legacy log formula restricted to the given chunk ids (no LIMIT). The
        // SQLite IN-list becomes a single `= ANY($1)` bigint-array bind.
        const SCORE: &str = "COALESCE( \
                ln(2 + COALESCE(e.callers, 0)::double precision) * 0.3 \
              + ln(2 + COALESCE(e.importers, 0)::double precision) * 0.2 \
              + ln(2 + COALESCE(e.tests, 0)::double precision) * 0.1, \
            0)::double precision AS graph_score";
        const EDGE_COUNTS: &str = "WITH edge_counts AS ( \
                SELECT dst_chunk_id AS chunk_id, \
                    SUM(CASE WHEN type = 'calls' THEN 1 ELSE 0 END) AS callers, \
                    SUM(CASE WHEN type = 'imports' THEN 1 ELSE 0 END) AS importers, \
                    SUM(CASE WHEN type = 'test_of' THEN 1 ELSE 0 END) AS tests \
                FROM chunk_edges WHERE dst_chunk_id = ANY($1) GROUP BY dst_chunk_id \
            )";
        let rows = match worktree_id {
            Some(wid) => {
                let sql = format!(
                    "{EDGE_COUNTS} \
                     SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SCORE} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
                     LEFT JOIN edge_counts e ON e.chunk_id = c.id \
                     WHERE c.id = ANY($1) AND f.repo_id = $2 AND cw.worktree_id = $3 \
                     ORDER BY graph_score DESC"
                );
                sqlx::query(&sql)
                    .bind(chunk_ids)
                    .bind(repo_id)
                    .bind(wid)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                let sql = format!(
                    "{EDGE_COUNTS} \
                     SELECT DISTINCT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SCORE} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     LEFT JOIN edge_counts e ON e.chunk_id = c.id \
                     WHERE c.id = ANY($1) AND f.repo_id = $2 \
                     ORDER BY graph_score DESC"
                );
                sqlx::query(&sql)
                    .bind(chunk_ids)
                    .bind(repo_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(rows
            .iter()
            .map(|r| row_to_score_hit(r, "graph_score"))
            .collect())
    }

    async fn calculate_signal_scores(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // combined_signal = recency*wr + churn*wc, cast to double precision.
        const SIG: &str =
            "(c.recency_score * $%R + c.churn_score * $%C)::double precision AS combined_signal";
        let rows = match worktree_id {
            Some(wid) => {
                // $1 repo, $2 worktree, $3 recency, $4 churn, $5 limit.
                let sig = SIG.replace("$%R", "$3").replace("$%C", "$4");
                let sql = format!(
                    "SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {sig} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
                     WHERE f.repo_id = $1 AND cw.worktree_id = $2 \
                     ORDER BY combined_signal DESC LIMIT $5"
                );
                sqlx::query(&sql)
                    .bind(repo_id)
                    .bind(wid)
                    .bind(recency_weight as f64)
                    .bind(churn_weight as f64)
                    .bind(limit as i64)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                // $1 repo, $2 recency, $3 churn, $4 limit.
                let sig = SIG.replace("$%R", "$2").replace("$%C", "$3");
                let sql = format!(
                    "SELECT DISTINCT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {sig} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     WHERE f.repo_id = $1 \
                     ORDER BY combined_signal DESC LIMIT $4"
                );
                sqlx::query(&sql)
                    .bind(repo_id)
                    .bind(recency_weight as f64)
                    .bind(churn_weight as f64)
                    .bind(limit as i64)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(rows
            .iter()
            .map(|r| row_to_score_hit(r, "combined_signal"))
            .collect())
    }

    async fn calculate_signal_scores_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
    ) -> anyhow::Result<Vec<SearchHit>> {
        if chunk_ids.is_empty() {
            return Ok(Vec::new());
        }
        // $1 recency, $2 churn, $3 chunk_ids, $4 repo, [$5 worktree]. No LIMIT.
        const SIG: &str =
            "(c.recency_score * $1 + c.churn_score * $2)::double precision AS combined_signal";
        let rows = match worktree_id {
            Some(wid) => {
                let sql = format!(
                    "SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SIG} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
                     WHERE c.id = ANY($3) AND f.repo_id = $4 AND cw.worktree_id = $5 \
                     ORDER BY combined_signal DESC"
                );
                sqlx::query(&sql)
                    .bind(recency_weight as f64)
                    .bind(churn_weight as f64)
                    .bind(chunk_ids)
                    .bind(repo_id)
                    .bind(wid)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                let sql = format!(
                    "SELECT DISTINCT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, {SIG} \
                     FROM chunks c \
                     JOIN files f ON f.id = c.file_id \
                     WHERE c.id = ANY($3) AND f.repo_id = $4 \
                     ORDER BY combined_signal DESC"
                );
                sqlx::query(&sql)
                    .bind(recency_weight as f64)
                    .bind(churn_weight as f64)
                    .bind(chunk_ids)
                    .bind(repo_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(rows
            .iter()
            .map(|r| row_to_score_hit(r, "combined_signal"))
            .collect())
    }
}
