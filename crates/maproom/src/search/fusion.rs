//! Score fusion for hybrid search results.
//!
//! This module implements score fusion strategies that combine results from
//! multiple search strategies (FTS, vector, graph, signals) into a single
//! ranked result set.
//!
//! # Phase 2 Implementation
//!
//! The current implementation uses a simple weighted average approach as a
//! baseline. More sophisticated fusion algorithms (RRF, learned weights,
//! cross-encoder reranking) will be implemented in Phase 3.
//!
//! # Score Normalization
//!
//! All scores are normalized to the 0.0-1.0 range before fusion to ensure
//! fair combination across different search types with different score ranges.

use crate::search::executor_types::{RankedResults, SearchSource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Configuration for score fusion weights.
///
/// These weights determine how much each search strategy contributes to
/// the final combined score. All weights should sum to 1.0 for proper
/// normalization, but this is not enforced to allow experimentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionWeights {
    /// Weight for full-text search scores (default: 0.4)
    pub fts: f32,
    /// Weight for vector similarity scores (default: 0.4)
    pub vector: f32,
    /// Weight for graph importance scores (default: 0.2)
    pub graph: f32,
    /// Weight for temporal signal scores (default: 0.0 for basic fusion)
    pub signals: f32,
}

impl Default for FusionWeights {
    fn default() -> Self {
        Self {
            fts: 0.4,
            vector: 0.4,
            graph: 0.2,
            signals: 0.0, // Signals are used within other searches, not as separate weight
        }
    }
}

impl FusionWeights {
    /// Create custom fusion weights.
    ///
    /// # Validation
    /// Weights should ideally sum to 1.0, but this is not enforced to allow
    /// experimentation with different scaling approaches.
    pub fn new(fts: f32, vector: f32, graph: f32, signals: f32) -> Self {
        Self {
            fts,
            vector,
            graph,
            signals,
        }
    }

    /// Get the total sum of all weights.
    pub fn sum(&self) -> f32 {
        self.fts + self.vector + self.graph + self.signals
    }

    /// Check if weights are normalized (sum to 1.0 within tolerance).
    pub fn is_normalized(&self) -> bool {
        (self.sum() - 1.0).abs() < 0.001
    }
}

/// Trait for score fusion strategies.
///
/// Implementations combine results from multiple search strategies into
/// a single ranked result set with fused scores.
pub trait ScoreFusion: Send + Sync {
    /// Fuse multiple result sets into a single ranked list.
    ///
    /// # Parameters
    /// - `results`: Vector of RankedResults from different search strategies
    /// - `weights`: Weights for each search type
    /// - `limit`: Maximum number of results to return
    ///
    /// # Returns
    /// Vector of FusedResult with combined scores, sorted by score descending
    fn fuse(
        &self,
        results: Vec<RankedResults>,
        weights: &FusionWeights,
        limit: usize,
    ) -> Vec<FusedResult>;
}

/// A single search result with fused score from multiple sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedResult {
    /// Chunk ID from maproom.chunks table
    pub chunk_id: i64,

    /// Combined score after fusion (0.0-1.0)
    pub score: f32,

    /// Individual scores from each search source that found this chunk
    pub source_scores: HashMap<SearchSource, f32>,
}

impl FusedResult {
    /// Create a new FusedResult.
    pub fn new(chunk_id: i64, score: f32, source_scores: HashMap<SearchSource, f32>) -> Self {
        Self {
            chunk_id,
            score,
            source_scores,
        }
    }
}

/// Basic weighted fusion implementation.
///
/// Combines scores using a simple weighted average:
/// ```text
/// final_score = w_fts * fts_score + w_vector * vector_score + w_graph * graph_score + w_signals * signals_score
/// ```
///
/// All input scores are already normalized to 0.0-1.0 by the individual
/// search executors, so no additional normalization is needed.
pub struct BasicWeightedFusion;

impl BasicWeightedFusion {
    /// Create a new BasicWeightedFusion.
    pub fn new() -> Self {
        Self
    }
}

impl Default for BasicWeightedFusion {
    fn default() -> Self {
        Self::new()
    }
}

impl ScoreFusion for BasicWeightedFusion {
    #[instrument(skip(self, results), fields(
        num_result_sets = results.len(),
        limit = limit
    ))]
    fn fuse(
        &self,
        results: Vec<RankedResults>,
        weights: &FusionWeights,
        limit: usize,
    ) -> Vec<FusedResult> {
        // Build a map of chunk_id -> scores from each source
        let mut chunk_scores: HashMap<i64, HashMap<SearchSource, f32>> = HashMap::new();
        let num_result_sets = results.len();

        for result_set in results {
            let source = result_set.source;
            for result in result_set.results {
                chunk_scores
                    .entry(result.chunk_id)
                    .or_insert_with(HashMap::new)
                    .insert(source, result.score);
            }
        }

        debug!(
            "Fusing {} unique chunks from {} result sets",
            chunk_scores.len(),
            num_result_sets
        );

        // Calculate weighted scores for each chunk
        let mut fused_results: Vec<FusedResult> = chunk_scores
            .into_iter()
            .map(|(chunk_id, source_scores)| {
                // Calculate weighted sum based on which sources found this chunk
                let mut weighted_sum = 0.0;

                if let Some(&fts_score) = source_scores.get(&SearchSource::FTS) {
                    weighted_sum += weights.fts * fts_score;
                }
                if let Some(&vector_score) = source_scores.get(&SearchSource::Vector) {
                    weighted_sum += weights.vector * vector_score;
                }
                if let Some(&graph_score) = source_scores.get(&SearchSource::Graph) {
                    weighted_sum += weights.graph * graph_score;
                }
                if let Some(&signal_score) = source_scores.get(&SearchSource::Signals) {
                    weighted_sum += weights.signals * signal_score;
                }

                FusedResult::new(chunk_id, weighted_sum, source_scores)
            })
            .collect();

        // Sort by score descending
        fused_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        fused_results.truncate(limit);

        debug!(
            "Fusion produced {} results (requested: {})",
            fused_results.len(),
            limit
        );

        fused_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::executor_types::RankedResult;

    #[test]
    fn test_fusion_weights_default() {
        let weights = FusionWeights::default();
        assert_eq!(weights.fts, 0.4);
        assert_eq!(weights.vector, 0.4);
        assert_eq!(weights.graph, 0.2);
        assert_eq!(weights.signals, 0.0);
        assert!(weights.is_normalized());
    }

    #[test]
    fn test_fusion_weights_custom() {
        let weights = FusionWeights::new(0.5, 0.3, 0.15, 0.05);
        assert_eq!(weights.sum(), 1.0);
        assert!(weights.is_normalized());
    }

    #[test]
    fn test_fusion_weights_not_normalized() {
        let weights = FusionWeights::new(0.6, 0.6, 0.3, 0.0);
        assert_eq!(weights.sum(), 1.5);
        assert!(!weights.is_normalized());
    }

    #[test]
    fn test_basic_weighted_fusion_single_source() {
        let fusion = BasicWeightedFusion::new();
        let weights = FusionWeights::default();

        let fts_results = RankedResults::new(
            vec![
                RankedResult::new(1, 0.9, 1),
                RankedResult::new(2, 0.8, 2),
                RankedResult::new(3, 0.7, 3),
            ],
            SearchSource::FTS,
        );

        let fused = fusion.fuse(vec![fts_results], &weights, 10);

        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].chunk_id, 1);
        assert!((fused[0].score - 0.36).abs() < 0.01); // 0.9 * 0.4
        assert_eq!(fused[1].chunk_id, 2);
        assert!((fused[1].score - 0.32).abs() < 0.01); // 0.8 * 0.4
    }

    #[test]
    fn test_basic_weighted_fusion_multiple_sources() {
        let fusion = BasicWeightedFusion::new();
        let weights = FusionWeights::default();

        let fts_results = RankedResults::new(
            vec![RankedResult::new(1, 0.9, 1), RankedResult::new(2, 0.7, 2)],
            SearchSource::FTS,
        );

        let vector_results = RankedResults::new(
            vec![RankedResult::new(1, 0.8, 1), RankedResult::new(3, 0.6, 2)],
            SearchSource::Vector,
        );

        let fused = fusion.fuse(vec![fts_results, vector_results], &weights, 10);

        assert_eq!(fused.len(), 3);

        // Chunk 1 appears in both FTS and Vector
        assert_eq!(fused[0].chunk_id, 1);
        // Score = 0.4 * 0.9 + 0.4 * 0.8 = 0.36 + 0.32 = 0.68
        assert!((fused[0].score - 0.68).abs() < 0.01);
        assert_eq!(fused[0].source_scores.len(), 2);

        // Chunk 2 only in FTS
        let chunk_2 = fused.iter().find(|r| r.chunk_id == 2).unwrap();
        assert!((chunk_2.score - 0.28).abs() < 0.01); // 0.4 * 0.7
        assert_eq!(chunk_2.source_scores.len(), 1);

        // Chunk 3 only in Vector
        let chunk_3 = fused.iter().find(|r| r.chunk_id == 3).unwrap();
        assert!((chunk_3.score - 0.24).abs() < 0.01); // 0.4 * 0.6
        assert_eq!(chunk_3.source_scores.len(), 1);
    }

    #[test]
    fn test_basic_weighted_fusion_limit() {
        let fusion = BasicWeightedFusion::new();
        let weights = FusionWeights::default();

        let fts_results = RankedResults::new(
            vec![
                RankedResult::new(1, 0.9, 1),
                RankedResult::new(2, 0.8, 2),
                RankedResult::new(3, 0.7, 3),
                RankedResult::new(4, 0.6, 4),
                RankedResult::new(5, 0.5, 5),
            ],
            SearchSource::FTS,
        );

        let fused = fusion.fuse(vec![fts_results], &weights, 3);

        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].chunk_id, 1);
        assert_eq!(fused[1].chunk_id, 2);
        assert_eq!(fused[2].chunk_id, 3);
    }

    #[test]
    fn test_basic_weighted_fusion_empty_results() {
        let fusion = BasicWeightedFusion::new();
        let weights = FusionWeights::default();

        let fused = fusion.fuse(vec![], &weights, 10);

        assert_eq!(fused.len(), 0);
    }

    #[test]
    fn test_basic_weighted_fusion_all_sources() {
        let fusion = BasicWeightedFusion::new();
        let weights = FusionWeights::new(0.3, 0.3, 0.3, 0.1);

        let fts_results = RankedResults::new(
            vec![RankedResult::new(1, 0.8, 1)],
            SearchSource::FTS,
        );

        let vector_results = RankedResults::new(
            vec![RankedResult::new(1, 0.9, 1)],
            SearchSource::Vector,
        );

        let graph_results = RankedResults::new(
            vec![RankedResult::new(1, 0.7, 1)],
            SearchSource::Graph,
        );

        let signal_results = RankedResults::new(
            vec![RankedResult::new(1, 0.6, 1)],
            SearchSource::Signals,
        );

        let fused = fusion.fuse(
            vec![fts_results, vector_results, graph_results, signal_results],
            &weights,
            10,
        );

        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].chunk_id, 1);
        // Score = 0.3*0.8 + 0.3*0.9 + 0.3*0.7 + 0.1*0.6 = 0.24 + 0.27 + 0.21 + 0.06 = 0.78
        assert!((fused[0].score - 0.78).abs() < 0.01);
        assert_eq!(fused[0].source_scores.len(), 4);
    }
}
