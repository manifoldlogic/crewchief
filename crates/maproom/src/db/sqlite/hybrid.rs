//! Hybrid search module combining FTS5 and vector search using Reciprocal Rank Fusion
//!
//! Hybrid search provides the best of both worlds:
//! - FTS5 for keyword matching and exact term relevance
//! - Vector search for semantic similarity and conceptual matching
//!
//! Results are combined using RRF (Reciprocal Rank Fusion), a proven algorithm
//! for merging ranked lists without requiring score normalization.
//!
//! Semantic ranking applies domain-specific adjustments:
//! - Kind multipliers (functions/classes rank higher than variables)
//! - Exact match boost when symbol name matches query
//! - Recency factor for recently modified chunks

use std::collections::{HashMap, HashSet};

use super::fts::FtsResult;
use super::vector::VectorResult;
use crate::search::fts::normalize_for_exact_match;

pub use crate::db::types::ChunkMetadata;
pub use crate::db::types::HybridResult;
pub use crate::db::types::HybridWeights;
pub use crate::db::types::RankedSearchHit;
pub use crate::db::types::SemanticRanking;

/// Standard RRF constant (k=60 is widely used in IR research)
const RRF_K: f64 = 60.0;

/// Apply semantic ranking multipliers to search results
///
/// Modifies scores in-place based on:
/// 1. Kind multipliers (functions/classes score higher)
/// 2. Exact match boost if symbol name contains query
/// 3. Recency factor (small boost for recently modified)
///
/// Re-sorts results by adjusted score after applying multipliers.
pub fn apply_semantic_ranking(
    results: &mut [RankedSearchHit],
    query: &str,
    ranking: &SemanticRanking,
) {
    let normalized_query = normalize_for_exact_match(query);

    for hit in results.iter_mut() {
        let mut multiplier = 1.0;

        // Apply kind multiplier (default 1.0 for unknown kinds)
        if let Some(&kind_mult) = ranking.kind_multipliers.get(&hit.kind) {
            multiplier *= kind_mult;
        }

        // Apply exact match boost if symbol name contains normalized query
        if let Some(ref symbol) = hit.symbol_name {
            let normalized_symbol = normalize_for_exact_match(symbol);
            if normalized_symbol
                .to_lowercase()
                .contains(&normalized_query.to_lowercase())
            {
                multiplier *= ranking.exact_match_boost;
            }
        }

        // Apply recency factor: 1.0 + (recency_score * weight)
        // recency_score is 0-1 where 1 = most recent
        let recency_boost = 1.0 + (hit.recency_score * ranking.recency_weight);
        multiplier *= recency_boost;

        hit.score *= multiplier;
    }

    // Re-sort after applying multipliers
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
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
pub fn rrf_score(fts_rank: Option<usize>, vec_rank: Option<usize>, weights: &HybridWeights) -> f64 {
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
    let all_chunk_ids: HashSet<i64> = fts_ranks.keys().chain(vec_ranks.keys()).copied().collect();

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
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

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
        assert!(
            (score - 1.0 / 60.0).abs() < 1e-6,
            "Score at rank 0 in both: {}",
            score
        );
    }

    #[test]
    fn test_rrf_score_fts_only() {
        let weights = HybridWeights::default();

        // Item at rank 0 in FTS only
        let score = rrf_score(Some(0), None, &weights);

        // Expected: 0.3/60 = 0.005
        assert!(
            (score - 0.3 / 60.0).abs() < 1e-6,
            "Score at rank 0 FTS only: {}",
            score
        );
    }

    #[test]
    fn test_rrf_score_vector_only() {
        let weights = HybridWeights::default();

        // Item at rank 0 in vector only
        let score = rrf_score(None, Some(0), &weights);

        // Expected: 0.7/60 ≈ 0.0117
        assert!(
            (score - 0.7 / 60.0).abs() < 1e-6,
            "Score at rank 0 vector only: {}",
            score
        );
    }

    #[test]
    fn test_rrf_score_neither_source() {
        let weights = HybridWeights::default();

        // Item in neither list (shouldn't happen in practice)
        let score = rrf_score(None, None, &weights);

        assert!(
            (score - 0.0).abs() < 1e-6,
            "Score with no ranks should be 0"
        );
    }

    #[test]
    fn test_rrf_score_rank_decay() {
        let weights = HybridWeights::default();

        // Scores should decrease as rank increases
        let score_0 = rrf_score(Some(0), Some(0), &weights);
        let score_1 = rrf_score(Some(1), Some(1), &weights);
        let score_10 = rrf_score(Some(10), Some(10), &weights);

        assert!(score_0 > score_1, "Rank 0 should score higher than rank 1");
        assert!(
            score_1 > score_10,
            "Rank 1 should score higher than rank 10"
        );
    }

    #[test]
    fn test_rrf_score_both_beats_single() {
        let weights = HybridWeights::default();

        // Item in both lists (even at lower ranks) should beat item in one list at rank 0
        let both_rank_5 = rrf_score(Some(5), Some(5), &weights);
        let fts_only_rank_0 = rrf_score(Some(0), None, &weights);
        let vec_only_rank_0 = rrf_score(None, Some(0), &weights);

        assert!(
            both_rank_5 > fts_only_rank_0,
            "Both at rank 5 should beat FTS-only at rank 0"
        );
        assert!(
            both_rank_5 > vec_only_rank_0,
            "Both at rank 5 should beat vector-only at rank 0"
        );
    }

    #[test]
    fn test_combine_results_basic() {
        let fts_results = vec![
            FtsResult {
                chunk_id: 1,
                rank: -1.0,
                normalized_rank: 0.5,
                position: 0,
            },
            FtsResult {
                chunk_id: 2,
                rank: -0.5,
                normalized_rank: 0.67,
                position: 1,
            },
        ];

        let vec_results = vec![
            VectorResult {
                chunk_id: 2,
                distance: 0.1,
                similarity: 0.91,
            },
            VectorResult {
                chunk_id: 3,
                distance: 0.2,
                similarity: 0.83,
            },
        ];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert_eq!(results.len(), 3, "Should have 3 unique chunks");

        // Chunk 2 should be first (appears in both lists)
        assert_eq!(
            results[0].chunk_id, 2,
            "Chunk 2 should be ranked first (in both lists)"
        );
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
                position: i as usize,
            })
            .collect();

        let vec_results: Vec<VectorResult> = (5..15)
            .map(|i| VectorResult {
                chunk_id: i,
                distance: i as f64 * 0.1,
                similarity: 0.9,
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
            VectorResult {
                chunk_id: 1,
                distance: 0.1,
                similarity: 0.91,
            },
            VectorResult {
                chunk_id: 2,
                distance: 0.2,
                similarity: 0.83,
            },
        ];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert_eq!(
            results.len(),
            2,
            "Should return vector results when FTS is empty"
        );
        assert!(results.iter().all(|r| r.source == "vector"));
    }

    #[test]
    fn test_combine_results_empty_vector() {
        let fts_results = vec![
            FtsResult {
                chunk_id: 1,
                rank: -1.0,
                normalized_rank: 0.5,
                position: 0,
            },
            FtsResult {
                chunk_id: 2,
                rank: -0.5,
                normalized_rank: 0.67,
                position: 1,
            },
        ];

        let vec_results: Vec<VectorResult> = vec![];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert_eq!(
            results.len(),
            2,
            "Should return FTS results when vector is empty"
        );
        assert!(results.iter().all(|r| r.source == "fts"));
    }

    #[test]
    fn test_combine_results_both_empty() {
        let fts_results: Vec<FtsResult> = vec![];
        let vec_results: Vec<VectorResult> = vec![];

        let weights = HybridWeights::default();
        let results = combine_results(&fts_results, &vec_results, &weights, 10);

        assert!(
            results.is_empty(),
            "Should return empty when both sources are empty"
        );
    }

    #[test]
    fn test_combine_results_sorted_by_score() {
        let fts_results = vec![
            FtsResult {
                chunk_id: 1,
                rank: -1.0,
                normalized_rank: 0.5,
                position: 0,
            },
            FtsResult {
                chunk_id: 2,
                rank: -0.5,
                normalized_rank: 0.67,
                position: 1,
            },
            FtsResult {
                chunk_id: 3,
                rank: -0.3,
                normalized_rank: 0.77,
                position: 2,
            },
        ];

        let vec_results = vec![
            VectorResult {
                chunk_id: 3,
                distance: 0.1,
                similarity: 0.91,
            },
            VectorResult {
                chunk_id: 2,
                distance: 0.2,
                similarity: 0.83,
            },
            VectorResult {
                chunk_id: 4,
                distance: 0.3,
                similarity: 0.77,
            },
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

        assert!(
            (fts_only - vec_only).abs() < 1e-6,
            "Equal weights should give same contribution"
        );
    }

    #[test]
    fn test_weights_fts_heavy() {
        let weights = HybridWeights::fts_heavy();

        let fts_only = rrf_score(Some(0), None, &weights);
        let vec_only = rrf_score(None, Some(0), &weights);

        assert!(
            fts_only > vec_only,
            "FTS-heavy weights should favor FTS results"
        );
    }

    #[test]
    fn test_weights_vector_heavy() {
        let weights = HybridWeights::vector_heavy();

        let fts_only = rrf_score(Some(0), None, &weights);
        let vec_only = rrf_score(None, Some(0), &weights);

        assert!(
            vec_only > fts_only,
            "Vector-heavy weights should favor vector results"
        );
    }

    // ========================================================================
    // Semantic Ranking Tests
    // ========================================================================

    #[test]
    fn test_semantic_ranking_defaults() {
        let ranking = SemanticRanking::default();

        // Check default multipliers
        assert_eq!(ranking.kind_multipliers.get("function"), Some(&1.2));
        assert_eq!(ranking.kind_multipliers.get("method"), Some(&1.2));
        assert_eq!(ranking.kind_multipliers.get("class"), Some(&1.1));
        assert_eq!(ranking.kind_multipliers.get("variable"), Some(&0.8));
        assert_eq!(ranking.kind_multipliers.get("import"), Some(&0.7));

        // Check default boost values
        assert!((ranking.exact_match_boost - 1.5).abs() < 1e-6);
        assert!((ranking.recency_weight - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_semantic_ranking_identity() {
        let ranking = SemanticRanking::identity();

        // Identity ranking should have no multipliers
        assert!(ranking.kind_multipliers.is_empty());
        assert!((ranking.exact_match_boost - 1.0).abs() < 1e-6);
        assert!((ranking.recency_weight - 0.0).abs() < 1e-6);
    }

    fn create_test_hit(
        chunk_id: i64,
        score: f64,
        kind: &str,
        symbol_name: Option<&str>,
        recency_score: f64,
    ) -> RankedSearchHit {
        RankedSearchHit {
            chunk_id,
            score,
            fts_rank: Some(0),
            vector_rank: Some(0),
            kind: kind.to_string(),
            symbol_name: symbol_name.map(|s| s.to_string()),
            recency_score,
            source: "both".to_string(),
        }
    }

    #[test]
    fn test_apply_semantic_ranking_kind_multipliers() {
        let ranking = SemanticRanking::default();

        let mut results = vec![
            create_test_hit(1, 1.0, "function", None, 0.0),
            create_test_hit(2, 1.0, "variable", None, 0.0),
            create_test_hit(3, 1.0, "import", None, 0.0),
        ];

        apply_semantic_ranking(&mut results, "query", &ranking);

        // Function should have highest score (1.0 * 1.2 = 1.2)
        // Variable should be middle (1.0 * 0.8 = 0.8)
        // Import should be lowest (1.0 * 0.7 = 0.7)
        assert!(
            (results[0].score - 1.2).abs() < 1e-6,
            "Function score: {}",
            results[0].score
        );
        assert!(
            (results[1].score - 0.8).abs() < 1e-6,
            "Variable score: {}",
            results[1].score
        );
        assert!(
            (results[2].score - 0.7).abs() < 1e-6,
            "Import score: {}",
            results[2].score
        );

        // Should be sorted by score descending
        assert_eq!(results[0].kind, "function");
        assert_eq!(results[1].kind, "variable");
        assert_eq!(results[2].kind, "import");
    }

    #[test]
    fn test_apply_semantic_ranking_exact_match_boost() {
        let ranking = SemanticRanking::default();

        let mut results = vec![
            create_test_hit(1, 1.0, "function", Some("validateUser"), 0.0),
            create_test_hit(2, 1.0, "function", Some("processData"), 0.0),
        ];

        apply_semantic_ranking(&mut results, "validate", &ranking);

        // First hit should get exact match boost (1.0 * 1.2 * 1.5 = 1.8)
        // Second hit should only get kind multiplier (1.0 * 1.2 = 1.2)
        assert!(
            (results[0].score - 1.8).abs() < 1e-6,
            "Match score: {}",
            results[0].score
        );
        assert!(
            (results[1].score - 1.2).abs() < 1e-6,
            "No match score: {}",
            results[1].score
        );

        // Verify sorting
        assert_eq!(results[0].symbol_name, Some("validateUser".to_string()));
        assert_eq!(results[1].symbol_name, Some("processData".to_string()));
    }

    #[test]
    fn test_apply_semantic_ranking_exact_match_camel_case() {
        let ranking = SemanticRanking::default();

        let mut results = vec![
            create_test_hit(1, 1.0, "function", Some("getUserName"), 0.0),
            create_test_hit(2, 1.0, "function", Some("processData"), 0.0),
        ];

        // Query "user_name" should match after normalization
        // getUserName -> get_user_name which contains "user_name"
        apply_semantic_ranking(&mut results, "user_name", &ranking);

        // getUserName normalized is "get_user_name" which contains "user_name"
        // So it should get the boost
        assert!(
            results[0].score > results[1].score,
            "Camel case match should boost score"
        );
    }

    #[test]
    fn test_apply_semantic_ranking_exact_match_partial_name() {
        let ranking = SemanticRanking::default();

        let mut results = vec![
            create_test_hit(1, 1.0, "function", Some("validateUserCredentials"), 0.0),
            create_test_hit(2, 1.0, "function", Some("processData"), 0.0),
        ];

        // Query "user" should match "validateUserCredentials" after normalization
        // validateUserCredentials -> validate_user_credentials
        apply_semantic_ranking(&mut results, "user", &ranking);

        // Should match since "validate_user_credentials" contains "user"
        assert!(
            results[0].score > results[1].score,
            "Partial name match should boost score"
        );
    }

    #[test]
    fn test_apply_semantic_ranking_recency_factor() {
        let ranking = SemanticRanking::default();

        let mut results = vec![
            create_test_hit(1, 1.0, "enum", None, 1.0), // Most recent
            create_test_hit(2, 1.0, "enum", None, 0.0), // Not recent
        ];

        apply_semantic_ranking(&mut results, "query", &ranking);

        // First hit: 1.0 * 1.0 * (1.0 + 1.0 * 0.1) = 1.0 * 1.1 = 1.1
        // Second hit: 1.0 * 1.0 * (1.0 + 0.0 * 0.1) = 1.0 * 1.0 = 1.0
        assert!(
            (results[0].score - 1.1).abs() < 1e-6,
            "Recent score: {}",
            results[0].score
        );
        assert!(
            (results[1].score - 1.0).abs() < 1e-6,
            "Old score: {}",
            results[1].score
        );
    }

    #[test]
    fn test_apply_semantic_ranking_combined_factors() {
        let ranking = SemanticRanking::default();

        let mut results = vec![
            create_test_hit(1, 1.0, "function", Some("validateInput"), 1.0),
            create_test_hit(2, 1.0, "variable", None, 0.0),
        ];

        apply_semantic_ranking(&mut results, "validate", &ranking);

        // First hit: function (1.2) * exact match (1.5) * recency (1.1) = 1.98
        // Second hit: variable (0.8) * no match (1.0) * no recency (1.0) = 0.8
        assert!(
            (results[0].score - 1.98).abs() < 1e-6,
            "Combined score: {}",
            results[0].score
        );
        assert!(
            (results[1].score - 0.8).abs() < 1e-6,
            "Base score: {}",
            results[1].score
        );
    }

    #[test]
    fn test_apply_semantic_ranking_reorders_results() {
        let ranking = SemanticRanking::default();

        // Start with variable ranked higher than function
        let mut results = vec![
            create_test_hit(1, 2.0, "variable", None, 0.0), // Higher base score
            create_test_hit(2, 1.0, "function", Some("targetFunction"), 0.0),
        ];

        apply_semantic_ranking(&mut results, "target", &ranking);

        // After ranking:
        // Variable: 2.0 * 0.8 = 1.6
        // Function with match: 1.0 * 1.2 * 1.5 = 1.8
        // Function should now be ranked first
        assert_eq!(results[0].kind, "function");
        assert_eq!(results[1].kind, "variable");
    }

    #[test]
    fn test_apply_semantic_ranking_unknown_kind() {
        let ranking = SemanticRanking::default();

        let mut results = vec![create_test_hit(1, 1.0, "unknown_kind", None, 0.0)];

        apply_semantic_ranking(&mut results, "query", &ranking);

        // Unknown kind should use default multiplier of 1.0
        assert!(
            (results[0].score - 1.0).abs() < 1e-6,
            "Unknown kind score: {}",
            results[0].score
        );
    }

    #[test]
    fn test_apply_semantic_ranking_empty_results() {
        let ranking = SemanticRanking::default();
        let mut results: Vec<RankedSearchHit> = vec![];

        // Should not panic on empty results
        apply_semantic_ranking(&mut results, "query", &ranking);
        assert!(results.is_empty());
    }

    #[test]
    fn test_apply_semantic_ranking_no_symbol_name() {
        let ranking = SemanticRanking::default();

        let mut results = vec![create_test_hit(1, 1.0, "function", None, 0.0)];

        apply_semantic_ranking(&mut results, "validate", &ranking);

        // Should only apply kind multiplier, no exact match boost
        assert!(
            (results[0].score - 1.2).abs() < 1e-6,
            "No symbol score: {}",
            results[0].score
        );
    }

    #[test]
    fn test_semantic_ranking_custom() {
        let mut kind_multipliers = HashMap::new();
        kind_multipliers.insert("custom".to_string(), 2.0);

        let ranking = SemanticRanking::new(kind_multipliers, 3.0, 0.5);

        let mut results = vec![create_test_hit(1, 1.0, "custom", Some("matchThis"), 1.0)];

        apply_semantic_ranking(&mut results, "match", &ranking);

        // custom (2.0) * exact match (3.0) * recency (1.0 + 1.0 * 0.5 = 1.5) = 9.0
        assert!(
            (results[0].score - 9.0).abs() < 1e-6,
            "Custom ranking score: {}",
            results[0].score
        );
    }
}
