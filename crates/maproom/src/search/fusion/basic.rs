//! Basic weighted fusion implementation.
//!
//! This module implements simple weighted average fusion as a baseline approach
//! from Phase 2. More sophisticated fusion algorithms (RRF) are available in
//! other modules for Phase 3.

use crate::search::executor_types::{RankedResults, SearchSource};
use crate::search::fusion::{FusedResult, ScoreFusion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Configuration for score fusion weights.
///
/// These weights determine how much each search strategy contributes to
/// the final combined score. All weights should sum to 1.0 for proper
/// normalization, but this is not enforced to allow experimentation.
///
/// # Default Weights Rationale
/// - **FTS (0.4)**: Highest weight for keyword/exact matches
/// - **Vector (0.35)**: Strong semantic similarity signal
/// - **Graph (0.1)**: Moderate boost for important/central code
/// - **Recency (0.1)**: Moderate boost for recently changed code
/// - **Churn (0.05)**: Slight penalty for high-churn (unstable) code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionWeights {
    /// Weight for full-text search scores (default: 0.4)
    pub fts: f32,
    /// Weight for vector similarity scores (default: 0.35)
    pub vector: f32,
    /// Weight for graph importance scores (default: 0.1)
    pub graph: f32,
    /// Weight for recency signal scores (default: 0.1)
    pub recency: f32,
    /// Weight for churn signal scores (default: 0.05)
    /// Note: Churn is inverted during fusion: churn_contrib = weight * (1.0 / (1.0 + churn_score))
    pub churn: f32,
}

impl Default for FusionWeights {
    fn default() -> Self {
        Self {
            fts: 0.4,
            vector: 0.35,
            graph: 0.1,
            recency: 0.1,
            churn: 0.05,
        }
    }
}

impl FusionWeights {
    /// Create custom fusion weights.
    ///
    /// # Validation
    /// Weights should ideally sum to 1.0, but this is not enforced to allow
    /// experimentation with different scaling approaches.
    pub fn new(fts: f32, vector: f32, graph: f32, recency: f32, churn: f32) -> Self {
        Self {
            fts,
            vector,
            graph,
            recency,
            churn,
        }
    }

    /// Get the total sum of all weights.
    pub fn sum(&self) -> f32 {
        self.fts + self.vector + self.graph + self.recency + self.churn
    }

    /// Check if weights are normalized (sum to 1.0 within tolerance).
    pub fn is_normalized(&self) -> bool {
        (self.sum() - 1.0).abs() < 0.001
    }

    /// Validate weights to ensure all are non-negative.
    ///
    /// # Errors
    /// Returns an error if any weight is negative.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.fts < 0.0 {
            anyhow::bail!("FTS weight must be non-negative, got: {}", self.fts);
        }
        if self.vector < 0.0 {
            anyhow::bail!("Vector weight must be non-negative, got: {}", self.vector);
        }
        if self.graph < 0.0 {
            anyhow::bail!("Graph weight must be non-negative, got: {}", self.graph);
        }
        if self.recency < 0.0 {
            anyhow::bail!("Recency weight must be non-negative, got: {}", self.recency);
        }
        if self.churn < 0.0 {
            anyhow::bail!("Churn weight must be non-negative, got: {}", self.churn);
        }
        Ok(())
    }

    /// Normalize weights to sum to 1.0.
    ///
    /// If the sum is zero or very close to zero, all weights are set to equal values.
    pub fn normalize(&mut self) {
        let sum = self.sum();
        if sum < 0.0001 {
            // If sum is effectively zero, set to equal weights
            let equal_weight = 1.0 / 5.0; // 5 signals
            self.fts = equal_weight;
            self.vector = equal_weight;
            self.graph = equal_weight;
            self.recency = equal_weight;
            self.churn = equal_weight;
        } else {
            self.fts /= sum;
            self.vector /= sum;
            self.graph /= sum;
            self.recency /= sum;
            self.churn /= sum;
        }
    }

    /// Create a normalized copy of these weights.
    pub fn normalized(&self) -> Self {
        let mut weights = self.clone();
        weights.normalize();
        weights
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
                    .or_default()
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
                let fts_contrib = source_scores
                    .get(&SearchSource::FTS)
                    .map(|&score| weights.fts * score)
                    .unwrap_or(0.0);

                let vector_contrib = source_scores
                    .get(&SearchSource::Vector)
                    .map(|&score| weights.vector * score)
                    .unwrap_or(0.0);

                let graph_contrib = source_scores
                    .get(&SearchSource::Graph)
                    .map(|&score| weights.graph * score)
                    .unwrap_or(0.0);

                // For now, SearchSource::Signals combines recency and churn
                // We use the combined (recency + churn) weight for the Signals source
                // In Phase 3, signals will be decomposed into separate sources
                let signals_weight = weights.recency + weights.churn;
                let signals_contrib = source_scores
                    .get(&SearchSource::Signals)
                    .map(|&score| signals_weight * score)
                    .unwrap_or(0.0);

                let weighted_sum = fts_contrib + vector_contrib + graph_contrib + signals_contrib;

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
        assert_eq!(weights.vector, 0.35);
        assert_eq!(weights.graph, 0.1);
        assert_eq!(weights.recency, 0.1);
        assert_eq!(weights.churn, 0.05);
        assert!(weights.is_normalized());
    }

    #[test]
    fn test_fusion_weights_custom() {
        let weights = FusionWeights::new(0.5, 0.3, 0.1, 0.08, 0.02);
        assert_eq!(weights.sum(), 1.0);
        assert!(weights.is_normalized());
    }

    #[test]
    fn test_fusion_weights_not_normalized() {
        let weights = FusionWeights::new(0.6, 0.6, 0.3, 0.2, 0.1);
        // Use approximate comparison for floating point
        assert!((weights.sum() - 1.8).abs() < 0.001);
        assert!(!weights.is_normalized());
    }

    #[test]
    fn test_fusion_weights_validate() {
        let valid_weights = FusionWeights::new(0.4, 0.35, 0.1, 0.1, 0.05);
        assert!(valid_weights.validate().is_ok());

        let invalid_weights = FusionWeights::new(-0.1, 0.35, 0.1, 0.1, 0.05);
        assert!(invalid_weights.validate().is_err());
    }

    #[test]
    fn test_fusion_weights_normalize() {
        let mut weights = FusionWeights::new(0.8, 0.7, 0.3, 0.2, 0.0);
        weights.normalize();
        assert!(weights.is_normalized());
        assert!((weights.sum() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_fusion_weights_normalized_copy() {
        let weights = FusionWeights::new(2.0, 2.0, 1.0, 0.5, 0.5);
        let normalized = weights.normalized();
        assert!(normalized.is_normalized());
        // Original unchanged
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
        // Score = 0.4 * 0.9 + 0.35 * 0.8 = 0.36 + 0.28 = 0.64
        assert!((fused[0].score - 0.64).abs() < 0.01);
        assert_eq!(fused[0].source_scores.len(), 2);

        // Chunk 2 only in FTS
        let chunk_2 = fused.iter().find(|r| r.chunk_id == 2).unwrap();
        assert!((chunk_2.score - 0.28).abs() < 0.01); // 0.4 * 0.7
        assert_eq!(chunk_2.source_scores.len(), 1);

        // Chunk 3 only in Vector
        let chunk_3 = fused.iter().find(|r| r.chunk_id == 3).unwrap();
        assert!((chunk_3.score - 0.21).abs() < 0.01); // 0.35 * 0.6
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
        // recency=0.08, churn=0.02, combined signals weight = 0.1
        let weights = FusionWeights::new(0.3, 0.3, 0.3, 0.08, 0.02);

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
        // Score = 0.3*0.8 + 0.3*0.9 + 0.3*0.7 + (0.08+0.02)*0.6
        // = 0.24 + 0.27 + 0.21 + 0.06 = 0.78
        assert!((fused[0].score - 0.78).abs() < 0.01);
        assert_eq!(fused[0].source_scores.len(), 4);
    }
}
