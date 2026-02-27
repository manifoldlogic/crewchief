//! Reciprocal Rank Fusion (RRF) implementation.
//!
//! This module implements RRF, a rank-based fusion algorithm that is more robust
//! to score distribution differences across different search methods compared to
//! weighted averaging approaches.
//!
//! # Algorithm
//!
//! RRF uses the formula: `score = 1.0 / (k + rank + 1.0)`
//!
//! Where:
//! - `k` is a constant (typically 60, per Cormack et al., 2009)
//! - `rank` is the 0-based position in the result set
//!
//! The final score for each chunk is the sum of its RRF scores across all result sets.
//!
//! # Advantages over Weighted Fusion
//!
//! - **Score Distribution Independence**: Uses rank position instead of raw scores
//! - **No Manual Weight Tuning**: Eliminates need for carefully tuned weights
//! - **Proven Effectiveness**: Well-established in information retrieval literature
//! - **Simple and Interpretable**: Single parameter (k) with sensible default
//!
//! # References
//!
//! Cormack, G. V., Clarke, C. L., & Buettcher, S. (2009).
//! "Reciprocal rank fusion outperforms condorcet and individual rank learning methods."
//! SIGIR '09: Proceedings of the 32nd international ACM SIGIR conference.

use crate::search::executor_types::RankedResults;
use crate::search::fusion::{FusedResult, FusionWeights, ScoreFusion};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Reciprocal Rank Fusion implementation.
///
/// RRF combines multiple ranked result sets by computing reciprocal rank scores
/// for each result and summing them across all sources. This approach is robust
/// to score distribution differences and requires minimal parameter tuning.
///
/// # Examples
///
/// ```
/// use maproom::search::fusion::{RRFFusion, FusionWeights, ScoreFusion};
/// use maproom::search::executor_types::{RankedResults, RankedResult, SearchSource};
///
/// // Create RRF fusion with default k=60
/// let fusion = RRFFusion::default();
///
/// // Create sample result sets
/// let fts_results = RankedResults::new(
///     vec![
///         RankedResult::new(1, 0.9, 1),
///         RankedResult::new(2, 0.8, 2),
///     ],
///     SearchSource::FTS,
/// );
///
/// let vector_results = RankedResults::new(
///     vec![
///         RankedResult::new(1, 0.85, 1),
///         RankedResult::new(3, 0.75, 2),
///     ],
///     SearchSource::Vector,
/// );
///
/// // Fuse results (weights parameter is ignored by RRF)
/// let weights = FusionWeights::default();
/// let fused = fusion.fuse(vec![fts_results, vector_results], &weights, 10);
///
/// // Chunk 1 appears in both sources with rank 0 in each
/// // RRF score = 2 * (1.0 / (60 + 0 + 1)) = 2 * 0.01639 = 0.03279
/// assert_eq!(fused[0].chunk_id, 1);
/// assert!(fused[0].score > 0.032);
/// ```
pub struct RRFFusion {
    /// k parameter for RRF formula (typically 60)
    k: f32,
}

impl RRFFusion {
    /// Create a new RRFFusion with the specified k parameter.
    ///
    /// # Parameters
    /// - `k`: The k constant in the RRF formula (1.0 / (k + rank + 1.0))
    ///
    /// # Recommended Values
    /// - Default: 60 (from Cormack et al., 2009)
    /// - Higher k: More conservative, reduces impact of rank differences
    /// - Lower k: More aggressive, amplifies impact of rank differences
    ///
    /// # Examples
    ///
    /// ```
    /// use maproom::search::fusion::RRFFusion;
    ///
    /// let fusion = RRFFusion::new(60.0);
    /// ```
    pub fn new(k: f32) -> Self {
        Self { k }
    }

    /// Calculate the RRF score for a given rank position.
    ///
    /// Formula: `1.0 / (k + rank + 1.0)`
    ///
    /// # Parameters
    /// - `rank`: 0-based rank position in the result set
    ///
    /// # Returns
    /// RRF score for this rank position
    #[inline]
    fn rrf_score(&self, rank: usize) -> f32 {
        1.0 / (self.k + rank as f32 + 1.0)
    }
}

impl Default for RRFFusion {
    /// Create RRFFusion with default k=60.
    fn default() -> Self {
        Self::new(60.0)
    }
}

impl ScoreFusion for RRFFusion {
    #[instrument(skip(self, results), fields(
        k = self.k,
        num_result_sets = results.len(),
        limit = limit
    ))]
    fn fuse(
        &self,
        results: Vec<RankedResults>,
        _weights: &FusionWeights, // RRF doesn't use weights
        limit: usize,
    ) -> Vec<FusedResult> {
        // Build a map of chunk_id -> accumulated RRF score
        let mut chunk_scores: HashMap<i64, f32> = HashMap::new();
        let num_result_sets = results.len();

        // Also track source scores for transparency
        let mut chunk_source_scores: HashMap<
            i64,
            HashMap<crate::search::executor_types::SearchSource, f32>,
        > = HashMap::new();

        // Track exact_match_multiplier from FTS results
        let mut chunk_exact_match: HashMap<i64, f32> = HashMap::new();

        // Accumulate RRF scores from each result set
        for result_set in results {
            let source = result_set.source;

            // Iterate through results using enumerate to get 0-based rank
            for (rank, result) in result_set.results.iter().enumerate() {
                let rrf_score = self.rrf_score(rank);

                // Accumulate RRF score for this chunk
                *chunk_scores.entry(result.chunk_id).or_insert(0.0) += rrf_score;

                // Track original score from this source for transparency
                chunk_source_scores
                    .entry(result.chunk_id)
                    .or_default()
                    .insert(source, result.score);

                // Preserve exact_match_multiplier from FTS results
                if let Some(mult) = result.exact_match_multiplier {
                    chunk_exact_match.insert(result.chunk_id, mult);
                }
            }
        }

        debug!(
            "RRF fusing {} unique chunks from {} result sets with k={}",
            chunk_scores.len(),
            num_result_sets,
            self.k
        );

        // Convert to FusedResult vec
        let mut fused_results: Vec<FusedResult> = chunk_scores
            .into_iter()
            .map(|(chunk_id, score)| {
                let source_scores = chunk_source_scores.remove(&chunk_id).unwrap_or_default();
                let exact_mult = chunk_exact_match.get(&chunk_id).copied();
                FusedResult::with_exact_match(chunk_id, score, source_scores, exact_mult)
            })
            .collect();

        // Sort by RRF score descending
        fused_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        fused_results.truncate(limit);

        debug!(
            "RRF fusion produced {} results (requested: {})",
            fused_results.len(),
            limit
        );

        fused_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::executor_types::{RankedResult, SearchSource};

    #[test]
    fn test_rrf_score_calculation() {
        let fusion = RRFFusion::new(60.0);

        // Test specific rank positions
        assert!((fusion.rrf_score(0) - (1.0 / 61.0)).abs() < 0.0001);
        assert!((fusion.rrf_score(1) - (1.0 / 62.0)).abs() < 0.0001);
        assert!((fusion.rrf_score(9) - (1.0 / 70.0)).abs() < 0.0001);
    }

    #[test]
    fn test_rrf_default_k() {
        let fusion = RRFFusion::default();
        assert_eq!(fusion.k, 60.0);
    }

    #[test]
    fn test_rrf_custom_k() {
        let fusion = RRFFusion::new(100.0);
        assert_eq!(fusion.k, 100.0);

        // With higher k, scores should be lower
        assert!((fusion.rrf_score(0) - (1.0 / 101.0)).abs() < 0.0001);
    }

    #[test]
    fn test_rrf_fusion_single_source() {
        let fusion = RRFFusion::default();
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

        // Chunk 1 at rank 0: RRF = 1/(60+0+1) = 1/61 ≈ 0.01639
        assert_eq!(fused[0].chunk_id, 1);
        assert!((fused[0].score - (1.0 / 61.0)).abs() < 0.0001);

        // Chunk 2 at rank 1: RRF = 1/(60+1+1) = 1/62 ≈ 0.01613
        assert_eq!(fused[1].chunk_id, 2);
        assert!((fused[1].score - (1.0 / 62.0)).abs() < 0.0001);

        // Chunk 3 at rank 2: RRF = 1/(60+2+1) = 1/63 ≈ 0.01587
        assert_eq!(fused[2].chunk_id, 3);
        assert!((fused[2].score - (1.0 / 63.0)).abs() < 0.0001);
    }

    #[test]
    fn test_rrf_fusion_multiple_sources_same_chunk() {
        let fusion = RRFFusion::default();
        let weights = FusionWeights::default();

        let fts_results = RankedResults::new(vec![RankedResult::new(1, 0.9, 1)], SearchSource::FTS);

        let vector_results =
            RankedResults::new(vec![RankedResult::new(1, 0.8, 1)], SearchSource::Vector);

        let fused = fusion.fuse(vec![fts_results, vector_results], &weights, 10);

        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].chunk_id, 1);

        // Chunk 1 appears at rank 0 in both sources
        // RRF = 1/61 + 1/61 = 2/61 ≈ 0.03279
        let expected_score = 2.0 / 61.0;
        assert!((fused[0].score - expected_score).abs() < 0.0001);

        // Should track source scores from both
        assert_eq!(fused[0].source_scores.len(), 2);
        assert_eq!(fused[0].source_scores.get(&SearchSource::FTS), Some(&0.9));
        assert_eq!(
            fused[0].source_scores.get(&SearchSource::Vector),
            Some(&0.8)
        );
    }

    #[test]
    fn test_rrf_fusion_multiple_sources_different_ranks() {
        let fusion = RRFFusion::default();
        let weights = FusionWeights::default();

        let fts_results = RankedResults::new(
            vec![
                RankedResult::new(1, 0.9, 1), // rank 0
                RankedResult::new(2, 0.7, 2), // rank 1
            ],
            SearchSource::FTS,
        );

        let vector_results = RankedResults::new(
            vec![
                RankedResult::new(2, 0.8, 1), // rank 0
                RankedResult::new(3, 0.6, 2), // rank 1
            ],
            SearchSource::Vector,
        );

        let fused = fusion.fuse(vec![fts_results, vector_results], &weights, 10);

        assert_eq!(fused.len(), 3);

        // Chunk 2 appears in both: rank 1 in FTS, rank 0 in Vector
        // RRF = 1/62 + 1/61 ≈ 0.01613 + 0.01639 = 0.03252
        let chunk_2 = fused.iter().find(|r| r.chunk_id == 2).unwrap();
        let expected_score_2 = (1.0 / 62.0) + (1.0 / 61.0);
        assert!((chunk_2.score - expected_score_2).abs() < 0.0001);

        // Chunk 1 only in FTS at rank 0: RRF = 1/61 ≈ 0.01639
        let chunk_1 = fused.iter().find(|r| r.chunk_id == 1).unwrap();
        let expected_score_1 = 1.0 / 61.0;
        assert!((chunk_1.score - expected_score_1).abs() < 0.0001);

        // Chunk 3 only in Vector at rank 1: RRF = 1/62 ≈ 0.01613
        let chunk_3 = fused.iter().find(|r| r.chunk_id == 3).unwrap();
        let expected_score_3 = 1.0 / 62.0;
        assert!((chunk_3.score - expected_score_3).abs() < 0.0001);

        // Chunk 2 should be ranked first (highest RRF score)
        assert_eq!(fused[0].chunk_id, 2);
    }

    #[test]
    fn test_rrf_fusion_all_four_sources() {
        let fusion = RRFFusion::default();
        let weights = FusionWeights::default();

        let fts_results = RankedResults::new(
            vec![RankedResult::new(1, 0.9, 1)], // rank 0
            SearchSource::FTS,
        );

        let vector_results = RankedResults::new(
            vec![RankedResult::new(1, 0.85, 1)], // rank 0
            SearchSource::Vector,
        );

        let graph_results = RankedResults::new(
            vec![RankedResult::new(1, 0.75, 1)], // rank 0
            SearchSource::Graph,
        );

        let signal_results = RankedResults::new(
            vec![RankedResult::new(1, 0.65, 1)], // rank 0
            SearchSource::Signals,
        );

        let fused = fusion.fuse(
            vec![fts_results, vector_results, graph_results, signal_results],
            &weights,
            10,
        );

        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].chunk_id, 1);

        // Chunk 1 appears at rank 0 in all four sources
        // RRF = 4 * (1/61) ≈ 0.06557
        let expected_score = 4.0 / 61.0;
        assert!((fused[0].score - expected_score).abs() < 0.0001);

        // Should track all four source scores
        assert_eq!(fused[0].source_scores.len(), 4);
    }

    #[test]
    fn test_rrf_fusion_empty_results() {
        let fusion = RRFFusion::default();
        let weights = FusionWeights::default();

        let fused = fusion.fuse(vec![], &weights, 10);

        assert_eq!(fused.len(), 0);
    }

    #[test]
    fn test_rrf_fusion_limit() {
        let fusion = RRFFusion::default();
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
    fn test_rrf_k_parameter_effect() {
        let fusion_low_k = RRFFusion::new(10.0);
        let fusion_high_k = RRFFusion::new(100.0);
        let weights = FusionWeights::default();

        let fts_results = RankedResults::new(
            vec![
                RankedResult::new(1, 0.9, 1), // rank 0
                RankedResult::new(2, 0.8, 2), // rank 1
            ],
            SearchSource::FTS,
        );

        let fused_low = fusion_low_k.fuse(vec![fts_results.clone()], &weights, 10);
        let fused_high = fusion_high_k.fuse(vec![fts_results], &weights, 10);

        // Lower k should produce higher scores
        assert!(fused_low[0].score > fused_high[0].score);
        assert!(fused_low[1].score > fused_high[1].score);

        // Lower k should also have larger score differences between ranks
        let diff_low = fused_low[0].score - fused_low[1].score;
        let diff_high = fused_high[0].score - fused_high[1].score;
        assert!(diff_low > diff_high);
    }

    #[test]
    fn test_rrf_sorting_correctness() {
        let fusion = RRFFusion::default();
        let weights = FusionWeights::default();

        // Create results where chunk 3 should win due to appearing in multiple sources
        let fts_results = RankedResults::new(
            vec![
                RankedResult::new(1, 0.9, 1), // array position 0 (RRF uses enumerate index)
                RankedResult::new(3, 0.7, 3), // array position 1
            ],
            SearchSource::FTS,
        );

        let vector_results = RankedResults::new(
            vec![
                RankedResult::new(2, 0.8, 1),  // array position 0
                RankedResult::new(3, 0.75, 2), // array position 1
            ],
            SearchSource::Vector,
        );

        let graph_results = RankedResults::new(
            vec![
                RankedResult::new(3, 0.85, 1), // array position 0
                RankedResult::new(4, 0.6, 2),  // array position 1
            ],
            SearchSource::Graph,
        );

        let fused = fusion.fuse(
            vec![fts_results, vector_results, graph_results],
            &weights,
            10,
        );

        // Chunk 3 appears in all three sources at array positions: 1, 1, 0
        // RRF = 1/(60+1+1) + 1/(60+1+1) + 1/(60+0+1) = 1/62 + 1/62 + 1/61 ≈ 0.04868
        // This should be the highest score
        assert_eq!(fused[0].chunk_id, 3);

        // Verify chunk 3 has highest score
        let chunk_3_score = fused[0].score;
        let expected_3 = (1.0 / 62.0) + (1.0 / 62.0) + (1.0 / 61.0);
        assert!((chunk_3_score - expected_3).abs() < 0.0001);

        // All other chunks appear in only one source at rank 0
        for result in &fused[1..] {
            assert!(result.score < chunk_3_score);
        }
    }
}
