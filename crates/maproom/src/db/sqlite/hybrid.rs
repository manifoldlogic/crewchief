//! Hybrid search module combining FTS5 and vector search using Reciprocal Rank Fusion
//!
//! Hybrid search provides the best of both worlds:
//! - FTS5 for keyword matching and exact term relevance
//! - Vector search for semantic similarity and conceptual matching
//!
//! Results are combined using RRF (Reciprocal Rank Fusion), a proven algorithm
//! for merging ranked lists without requiring score normalization.

use std::collections::{HashMap, HashSet};

use super::fts::FtsResult;
use super::vector::VectorResult;

/// Standard RRF constant (k=60 is widely used in IR research)
const RRF_K: f64 = 60.0;

/// Weights for combining FTS and vector search contributions
#[derive(Debug, Clone)]
pub struct HybridWeights {
    /// Weight for FTS (keyword) contribution (default 0.3)
    pub fts_weight: f64,
    /// Weight for vector (semantic) contribution (default 0.7)
    pub vector_weight: f64,
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            fts_weight: 0.3,
            vector_weight: 0.7,
        }
    }
}

impl HybridWeights {
    /// Create weights with custom values
    pub fn new(fts_weight: f64, vector_weight: f64) -> Self {
        Self { fts_weight, vector_weight }
    }

    /// Equal weights for FTS and vector (0.5 each)
    pub fn equal() -> Self {
        Self { fts_weight: 0.5, vector_weight: 0.5 }
    }

    /// FTS-heavy weights (0.7 FTS, 0.3 vector)
    pub fn fts_heavy() -> Self {
        Self { fts_weight: 0.7, vector_weight: 0.3 }
    }

    /// Vector-heavy weights (0.3 FTS, 0.7 vector) - default
    pub fn vector_heavy() -> Self {
        Self::default()
    }
}

/// Result from hybrid search combining FTS and vector scores
#[derive(Debug, Clone)]
pub struct HybridResult {
    /// Chunk ID in the chunks table
    pub chunk_id: i64,
    /// Combined RRF score (higher = better)
    pub score: f64,
    /// Position in FTS results (None if not found by FTS)
    pub fts_rank: Option<usize>,
    /// Position in vector results (None if not found by vector search)
    pub vector_rank: Option<usize>,
    /// Source indicator: "fts", "vector", or "both"
    pub source: String,
}

/// Calculate RRF score for a single result
///
/// RRF formula: weight / (k + rank) where:
/// - rank is 0-indexed position in result list
/// - k is a constant (60) that balances contribution across ranks
/// - weight scales the contribution from each source
///
/// Items appearing in both lists get contributions from both sources,
/// naturally boosting results that match both keyword and semantic criteria.
pub fn rrf_score(
    fts_rank: Option<usize>,
    vec_rank: Option<usize>,
    weights: &HybridWeights,
) -> f64 {
    let fts_contribution = fts_rank
        .map(|r| weights.fts_weight / (RRF_K + r as f64))
        .unwrap_or(0.0);

    let vec_contribution = vec_rank
        .map(|r| weights.vector_weight / (RRF_K + r as f64))
        .unwrap_or(0.0);

    fts_contribution + vec_contribution
}

/// Combine FTS and vector search results using RRF
///
/// This is the core hybrid search implementation that:
/// 1. Takes pre-computed FTS and vector results
/// 2. Builds lookup maps for rank by chunk_id
/// 3. Calculates RRF scores for all unique chunks
/// 4. Sorts by combined score and returns top N
///
/// # Arguments
/// * `fts_results` - Results from FTS5 search
/// * `vec_results` - Results from vector similarity search
/// * `weights` - Weights for combining FTS and vector contributions
/// * `limit` - Maximum number of results to return
pub fn combine_results(
    fts_results: &[FtsResult],
    vec_results: &[VectorResult],
    weights: &HybridWeights,
    limit: usize,
) -> Vec<HybridResult> {
    // Build lookup maps: chunk_id -> rank (0-indexed position)
    let fts_ranks: HashMap<i64, usize> = fts_results
        .iter()
        .map(|r| (r.chunk_id, r.position))
        .collect();

    let vec_ranks: HashMap<i64, usize> = vec_results
        .iter()
        .enumerate()
        .map(|(i, r)| (r.chunk_id, i))
        .collect();

    // Get all unique chunk_ids from both sources
    let all_chunk_ids: HashSet<i64> = fts_ranks.keys()
        .chain(vec_ranks.keys())
        .copied()
        .collect();

    // Calculate RRF scores for all chunks
    let mut hits: Vec<HybridResult> = all_chunk_ids
        .into_iter()
        .map(|chunk_id| {
            let fts_rank = fts_ranks.get(&chunk_id).copied();
            let vec_rank = vec_ranks.get(&chunk_id).copied();
            let source = match (fts_rank.is_some(), vec_rank.is_some()) {
                (true, true) => "both".to_string(),
                (true, false) => "fts".to_string(),
                (false, true) => "vector".to_string(),
                (false, false) => "unknown".to_string(), // Should never happen
            };
            HybridResult {
                chunk_id,
                score: rrf_score(fts_rank, vec_rank, weights),
                fts_rank,
                vector_rank: vec_rank,
                source,
            }
        })
        .collect();

    // Sort by score (descending) - higher RRF score is better
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N results
    hits.truncate(limit);

    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_score_both_sources() {
        let weights = HybridWeights::default();

        // Item at rank 0 in both lists
        let score = rrf_score(Some(0), Some(0), &weights);

        // Expected: 0.3/60 + 0.7/60 = 1.0/60 ≈ 0.0167
        assert!((score - 1.0/60.0).abs() < 1e-6, "Score at rank 0 in both: {}", score);
    }

    #[test]
    fn test_rrf_score_fts_only() {
        let weights = HybridWeights::default();

        // Item at rank 0 in FTS only
        let score = rrf_score(Some(0), None, &weights);

        // Expected: 0.3/60 = 0.005
        assert!((score - 0.3/60.0).abs() < 1e-6, "Score at rank 0 FTS only: {}", score);
    }

    #[test]
    fn test_rrf_score_vector_only() {
        let weights = HybridWeights::default();

        // Item at rank 0 in vector only
        let score = rrf_score(None, Some(0), &weights);

        // Expected: 0.7/60 ≈ 0.0117
        assert!((score - 0.7/60.0).abs() < 1e-6, "Score at rank 0 vector only: {}", score);
    }

    #[test]
    fn test_rrf_score_neither_source() {
        let weights = HybridWeights::default();

        // Item in neither list (shouldn't happen in practice)
        let score = rrf_score(None, None, &weights);

        assert!((score - 0.0).abs() < 1e-6, "Score with no ranks should be 0");
    }

    #[test]
    fn test_rrf_score_rank_decay() {
        let weights = HybridWeights::default();

        // Scores should decrease as rank increases
        let score_0 = rrf_score(Some(0), Some(0), &weights);
        let score_1 = rrf_score(Some(1), Some(1), &weights);
        let score_10 = rrf_score(Some(10), Some(10), &weights);

        assert!(score_0 > score_1, "Rank 0 should score higher than rank 1");
        assert!(score_1 > score_10, "Rank 1 should score higher than rank 10");
    }

    #[test]
    fn test_rrf_score_both_beats_single() {
        let weights = HybridWeights::default();

        // Item in both lists (even at lower ranks) should beat item in one list at rank 0
        let both_rank_5 = rrf_score(Some(5), Some(5), &weights);
        let fts_only_rank_0 = rrf_score(Some(0), None, &weights);
        let vec_only_rank_0 = rrf_score(None, Some(0), &weights);

        assert!(both_rank_5 > fts_only_rank_0, "Both at rank 5 should beat FTS-only at rank 0");
        assert!(both_rank_5 > vec_only_rank_0, "Both at rank 5 should beat vector-only at rank 0");
    }

    #[test]
    fn test_combine_results_basic() {
        let fts_results = vec![
            FtsResult { chunk_id: 1, rank: -1.0, normalized_rank: 0.5, position: 0 },
            FtsResult { chunk_id: 2, rank: -0.5, normalized_rank: 0.67, position: 1 },
        ];

        let vec_results = vec![
            VectorResult { chunk_id: 2, distance: 0.1, similarity: 0.91 },
            VectorResult { chunk_id: 3, distance: 0.2, similarity: 0.83 },
        ];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert_eq!(results.len(), 3, "Should have 3 unique chunks");

        // Chunk 2 should be first (appears in both lists)
        assert_eq!(results[0].chunk_id, 2, "Chunk 2 should be ranked first (in both lists)");
        assert_eq!(results[0].source, "both");
        assert!(results[0].fts_rank.is_some());
        assert!(results[0].vector_rank.is_some());
    }

    #[test]
    fn test_combine_results_respects_limit() {
        let fts_results: Vec<FtsResult> = (0..10)
            .map(|i| FtsResult {
                chunk_id: i,
                rank: -(i as f64),
                normalized_rank: 0.5,
                position: i as usize
            })
            .collect();

        let vec_results: Vec<VectorResult> = (5..15)
            .map(|i| VectorResult {
                chunk_id: i,
                distance: i as f64 * 0.1,
                similarity: 0.9
            })
            .collect();

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 5);

        assert_eq!(results.len(), 5, "Should respect limit of 5");
    }

    #[test]
    fn test_combine_results_empty_fts() {
        let fts_results: Vec<FtsResult> = vec![];

        let vec_results = vec![
            VectorResult { chunk_id: 1, distance: 0.1, similarity: 0.91 },
            VectorResult { chunk_id: 2, distance: 0.2, similarity: 0.83 },
        ];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert_eq!(results.len(), 2, "Should return vector results when FTS is empty");
        assert!(results.iter().all(|r| r.source == "vector"));
    }

    #[test]
    fn test_combine_results_empty_vector() {
        let fts_results = vec![
            FtsResult { chunk_id: 1, rank: -1.0, normalized_rank: 0.5, position: 0 },
            FtsResult { chunk_id: 2, rank: -0.5, normalized_rank: 0.67, position: 1 },
        ];

        let vec_results: Vec<VectorResult> = vec![];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert_eq!(results.len(), 2, "Should return FTS results when vector is empty");
        assert!(results.iter().all(|r| r.source == "fts"));
    }

    #[test]
    fn test_combine_results_both_empty() {
        let fts_results: Vec<FtsResult> = vec![];
        let vec_results: Vec<VectorResult> = vec![];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert!(results.is_empty(), "Should return empty when both sources are empty");
    }

    #[test]
    fn test_combine_results_sorted_by_score() {
        let fts_results = vec![
            FtsResult { chunk_id: 1, rank: -1.0, normalized_rank: 0.5, position: 0 },
            FtsResult { chunk_id: 2, rank: -0.5, normalized_rank: 0.67, position: 1 },
            FtsResult { chunk_id: 3, rank: -0.3, normalized_rank: 0.77, position: 2 },
        ];

        let vec_results = vec![
            VectorResult { chunk_id: 3, distance: 0.1, similarity: 0.91 },
            VectorResult { chunk_id: 2, distance: 0.2, similarity: 0.83 },
            VectorResult { chunk_id: 4, distance: 0.3, similarity: 0.77 },
        ];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        // Verify scores are in descending order
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_weights_equal() {
        let weights = HybridWeights::equal();

        let fts_only = rrf_score(Some(0), None, &weights);
        let vec_only = rrf_score(None, Some(0), &weights);

        assert!((fts_only - vec_only).abs() < 1e-6, "Equal weights should give same contribution");
    }

    #[test]
    fn test_weights_fts_heavy() {
        let weights = HybridWeights::fts_heavy();

        let fts_only = rrf_score(Some(0), None, &weights);
        let vec_only = rrf_score(None, Some(0), &weights);

        assert!(fts_only > vec_only, "FTS-heavy weights should favor FTS results");
    }

    #[test]
    fn test_weights_vector_heavy() {
        let weights = HybridWeights::vector_heavy();

        let fts_only = rrf_score(Some(0), None, &weights);
        let vec_only = rrf_score(None, Some(0), &weights);

        assert!(vec_only > fts_only, "Vector-heavy weights should favor vector results");
    }
}
