//! Graph-based importance scoring using chunk_edges.
//!
//! This module implements PageRank-like importance scoring based on
//! incoming edges from the chunk_edges table. Different edge types
//! contribute different weights to the importance score.

use crate::config::SearchConfig;
use crate::db::traits::StoreGraph;
use crate::db::SqliteStore;
use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use tracing::{debug, instrument};

/// Graph importance executor.
///
/// Calculates PageRank-like scores from chunk_edges table:
/// - Incoming calls: 0.3 weight
/// - Incoming imports: 0.2 weight
/// - Test coverage: 0.1 weight
///
/// Uses logarithmic scaling to dampen extreme values.
/// Over-fetches results (limit * 2) for fusion.
pub struct GraphExecutor;

impl GraphExecutor {
    /// Execute graph importance query.
    ///
    /// # Parameters
    /// - `store`: Database store
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `limit`: Maximum number of results (will over-fetch by 2x)
    /// - `config`: Optional search config to control quality-weighted scoring
    ///
    /// # Returns
    /// RankedResults with graph importance scores normalized to 0.0-1.0 range
    ///
    /// # Quality-Weighted Mode
    /// When `config.feature_flags.enable_quality_weighted_graph` is true,
    /// uses quality-weighted edge scoring with configurable weights from
    /// `config.graph_importance.edge_quality_weights` (SRCHREL-2002).
    /// When false or config is None, uses legacy graph scoring.
    ///
    /// # SQL Query (Legacy Mode)
    /// ```sql
    /// WITH edge_counts AS (
    ///   SELECT
    ///     dst_chunk_id as chunk_id,
    ///     COUNT(*) FILTER (WHERE type = 'calls') as callers,
    ///     COUNT(*) FILTER (WHERE type = 'imports') as importers,
    ///     COUNT(*) FILTER (WHERE type = 'test_of') as tests
    ///   FROM maproom.chunk_edges
    ///   GROUP BY dst_chunk_id
    /// )
    /// SELECT
    ///   c.id,
    ///   COALESCE(
    ///     LOG(2 + COALESCE(e.callers, 0)) * 0.3 +
    ///     LOG(2 + COALESCE(e.importers, 0)) * 0.2 +
    ///     LOG(2 + COALESCE(e.tests, 0)) * 0.1,
    ///     0
    ///   ) as graph_score
    /// FROM maproom.chunks c
    /// JOIN maproom.files f ON f.id = c.file_id
    /// LEFT JOIN edge_counts e ON e.chunk_id = c.id
    /// WHERE f.repo_id = $1
    ///   AND ($2::bigint IS NULL OR f.worktree_id = $2)
    /// ORDER BY graph_score DESC
    /// LIMIT $3;
    /// ```
    #[instrument(skip(store, config))]
    pub async fn execute(
        store: &SqliteStore,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        config: Option<&SearchConfig>,
    ) -> Result<RankedResults, GraphError> {
        // Over-fetch by 2x for fusion (graph signals are less selective than FTS/vector)
        let fetch_limit = (limit * 2) as i64;

        // Extract feature flag, default to false if config not provided
        let enable_quality = config
            .map(|c| c.feature_flags.enable_quality_weighted_graph)
            .unwrap_or(false);

        // Extract edge quality weights from config, or use defaults (SRCHREL-2002)
        let weights = config
            .map(|c| c.graph_importance.edge_quality_weights.clone())
            .unwrap_or_default();

        debug!(
            repo_id = repo_id,
            worktree_id = ?worktree_id,
            limit = limit,
            enable_quality = enable_quality,
            production_code_weight = weights.production_code,
            test_code_weight = weights.test_code,
            calls_weight = weights.calls,
            "Executing graph importance query"
        );

        // Delegate to SqliteStore's graph importance calculation with configurable weights
        let hits = store
            .calculate_graph_importance(
                repo_id,
                worktree_id,
                fetch_limit as usize,
                enable_quality,
                &weights,
            )
            .await
            .map_err(|e| GraphError::Database(e.to_string()))?;

        // Convert SearchHit to RankedResult
        let results: Vec<RankedResult> = hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| RankedResult::new(hit.chunk_id, hit.score as f32, i + 1))
            .collect();

        debug!("Graph search returned {} results", results.len());
        Ok(RankedResults::new(results, SearchSource::Graph))
    }

    /// Execute graph importance query for specific chunk IDs.
    ///
    /// This variant calculates graph scores only for a given set of chunks,
    /// useful when combining with other search results.
    #[instrument(skip(store, chunk_ids), fields(chunk_count = chunk_ids.len()))]
    pub async fn execute_for_chunks(
        store: &SqliteStore,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> Result<RankedResults, GraphError> {
        if chunk_ids.is_empty() {
            return Ok(RankedResults::empty(SearchSource::Graph));
        }

        debug!(
            "Executing graph importance query for {} specific chunks",
            chunk_ids.len()
        );

        // Delegate to SqliteStore's graph importance calculation for specific chunks
        let hits = store
            .calculate_graph_importance_for_chunks(chunk_ids, repo_id, worktree_id)
            .await
            .map_err(|e| GraphError::Database(e.to_string()))?;

        // Convert SearchHit to RankedResult
        let results: Vec<RankedResult> = hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| RankedResult::new(hit.chunk_id, hit.score as f32, i + 1))
            .collect();

        debug!("Graph search for chunks returned {} results", results.len());
        Ok(RankedResults::new(results, SearchSource::Graph))
    }
}

/// Errors that can occur during graph query execution.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    /// Database query error
    #[error("Database error: {0}")]
    Database(String),

    /// No edge data available
    #[error("No graph data available")]
    NoGraphData,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FeatureFlags;

    #[test]
    fn test_graph_executor_exists() {
        // Verify the executor type exists
        let _executor = GraphExecutor;
    }

    #[test]
    fn test_extract_quality_flag_none_config() {
        // None config should default to false
        let config: Option<&SearchConfig> = None;
        let enable_quality = config
            .map(|c| c.feature_flags.enable_quality_weighted_graph)
            .unwrap_or(false);
        assert!(!enable_quality, "None config should default to false");
    }

    #[test]
    fn test_extract_quality_flag_disabled() {
        // Config with flag disabled should return false
        let config = SearchConfig::default();
        assert!(
            !config.feature_flags.enable_quality_weighted_graph,
            "Default config should have quality flag disabled"
        );

        let enable_quality = Some(&config)
            .map(|c| c.feature_flags.enable_quality_weighted_graph)
            .unwrap_or(false);
        assert!(!enable_quality, "Disabled flag should return false");
    }

    #[test]
    fn test_extract_quality_flag_enabled() {
        // Config with flag enabled should return true
        let mut config = SearchConfig::default();
        config.feature_flags.enable_quality_weighted_graph = true;

        let enable_quality = Some(&config)
            .map(|c| c.feature_flags.enable_quality_weighted_graph)
            .unwrap_or(false);
        assert!(enable_quality, "Enabled flag should return true");
    }

    #[test]
    fn test_feature_flags_quality_graph_default() {
        // Verify the FeatureFlags default for quality weighted graph
        let flags = FeatureFlags::default();
        assert!(
            !flags.enable_quality_weighted_graph,
            "enable_quality_weighted_graph should default to false"
        );
    }

    // ===== SRCHREL-2002: Weight Extraction Tests =====

    #[test]
    fn test_extract_weights_none_config() {
        // None config should use default weights (SRCHREL-2002)
        let config: Option<&SearchConfig> = None;
        let weights = config
            .map(|c| c.graph_importance.edge_quality_weights.clone())
            .unwrap_or_default();

        assert!(
            (weights.production_code - 1.0).abs() < f32::EPSILON,
            "Default production_code weight should be 1.0"
        );
        assert!(
            (weights.test_code - 0.5).abs() < f32::EPSILON,
            "Default test_code weight should be 0.5"
        );
        assert!(
            (weights.calls - 1.0).abs() < f32::EPSILON,
            "Default calls weight should be 1.0"
        );
    }

    #[test]
    fn test_extract_weights_from_config() {
        // Config with custom weights should extract correctly (SRCHREL-2002)
        let mut config = SearchConfig::default();
        config.graph_importance.edge_quality_weights.production_code = 2.0;
        config.graph_importance.edge_quality_weights.test_code = 0.3;
        config.graph_importance.edge_quality_weights.calls = 1.5;

        let weights = Some(&config)
            .map(|c| c.graph_importance.edge_quality_weights.clone())
            .unwrap_or_default();

        assert!(
            (weights.production_code - 2.0).abs() < f32::EPSILON,
            "Custom production_code weight should be 2.0"
        );
        assert!(
            (weights.test_code - 0.3).abs() < f32::EPSILON,
            "Custom test_code weight should be 0.3"
        );
        assert!(
            (weights.calls - 1.5).abs() < f32::EPSILON,
            "Custom calls weight should be 1.5"
        );
    }

    #[test]
    fn test_extract_weights_default_from_config() {
        // Config with default weights should match EdgeQualityWeights::default() (SRCHREL-2002)
        let config = SearchConfig::default();

        let weights = Some(&config)
            .map(|c| c.graph_importance.edge_quality_weights.clone())
            .unwrap_or_default();

        assert!(
            weights.is_default(),
            "Default config should have default weights"
        );
    }

    // Note: Full integration tests with real database are in tests/search/executors_test.rs
}
