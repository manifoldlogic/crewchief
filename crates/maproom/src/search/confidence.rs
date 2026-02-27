//! Confidence signal computation for search results.
//!
//! This module computes confidence signals that help users assess search result
//! quality. All signals are derived from existing search pipeline data with O(1)
//! computation per result and no database overhead.
//!
//! # Confidence Signals
//!
//! - **source_count**: Number of search strategies that found this result
//! - **score_gap**: Score difference from the next-ranked result
//! - **is_exact_match**: Whether result received exact match boost
//!
//! # Design Principles
//!
//! - Component-based (not single score) for transparency
//! - In-memory computation only (no database queries)
//! - Graceful degradation (handles None values, empty results)
//! - O(1) per-result complexity

use crate::search::fusion::FusedResult;
use crate::search::results::ConfidenceSignals;

/// Compute confidence signals for a single search result.
///
/// # Parameters
///
/// - `result`: The result to compute confidence for
/// - `all_results`: The complete sorted result list (for score_gap calculation)
/// - `index`: Position of `result` in `all_results` (0-based)
/// - `exact_match_multiplier`: Optional exact match boost applied during scoring
///
/// # Returns
///
/// `ConfidenceSignals` with three computed fields:
/// - `source_count`: Number of sources in `result.source_scores`
/// - `score_gap`: Score difference from next result (0.0 if last result)
/// - `is_exact_match`: True if `exact_match_multiplier >= 2.9`
///
/// # Examples
///
/// ```no_run
/// use maproom::search::confidence::compute_result_confidence;
/// use maproom::search::fusion::FusedResult;
/// use std::collections::HashMap;
///
/// let results = vec![
///     FusedResult::new(1, 0.95, HashMap::new()),
///     FusedResult::new(2, 0.82, HashMap::new()),
/// ];
///
/// let confidence = compute_result_confidence(&results[0], &results, 0, Some(3.0));
/// assert_eq!(confidence.score_gap, 0.13); // 0.95 - 0.82
/// assert!(confidence.is_exact_match); // 3.0 >= 2.9
/// ```
///
/// # Edge Cases
///
/// - **Last result**: `score_gap` is 0.0 (no next result to compare)
/// - **Single result**: `score_gap` is 0.0
/// - **Empty results**: Should never occur (caller should check), but returns safe defaults
/// - **None multiplier**: `is_exact_match` is false
pub fn compute_result_confidence(
    result: &FusedResult,
    all_results: &[FusedResult],
    index: usize,
    exact_match_multiplier: Option<f32>,
) -> ConfidenceSignals {
    // Source count: number of search strategies that found this result
    let source_count = result.source_scores.len();

    // Score gap: difference between this result and the next result
    // If this is the last result, gap is 0.0
    let score_gap = if index + 1 < all_results.len() {
        result.score - all_results[index + 1].score
    } else {
        0.0
    };

    // Exact match: multiplier >= 2.9 indicates exact match boost was applied
    let is_exact_match = exact_match_multiplier.map(|m| m >= 2.9).unwrap_or(false);

    ConfidenceSignals {
        source_count,
        score_gap,
        is_exact_match,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::executor_types::SearchSource;
    use std::collections::HashMap;

    fn make_result(chunk_id: i64, score: f32, source_count: usize) -> FusedResult {
        let mut source_scores = HashMap::new();
        // Add dummy sources based on count
        if source_count >= 1 {
            source_scores.insert(SearchSource::FTS, score);
        }
        if source_count >= 2 {
            source_scores.insert(SearchSource::Vector, score);
        }
        if source_count >= 3 {
            source_scores.insert(SearchSource::Graph, score);
        }
        if source_count >= 4 {
            source_scores.insert(SearchSource::Signals, score);
        }
        FusedResult::new(chunk_id, score, source_scores)
    }

    #[test]
    fn test_source_count() {
        let results = vec![
            make_result(1, 0.95, 3), // 3 sources
            make_result(2, 0.82, 1), // 1 source
        ];

        let confidence = compute_result_confidence(&results[0], &results, 0, None);
        assert_eq!(confidence.source_count, 3);

        let confidence = compute_result_confidence(&results[1], &results, 1, None);
        assert_eq!(confidence.source_count, 1);
    }

    #[test]
    fn test_score_gap_normal() {
        let results = vec![
            make_result(1, 0.95, 2),
            make_result(2, 0.82, 2),
            make_result(3, 0.70, 1),
        ];

        // First result: gap to second
        let confidence = compute_result_confidence(&results[0], &results, 0, None);
        assert!((confidence.score_gap - 0.13).abs() < 0.001);

        // Second result: gap to third
        let confidence = compute_result_confidence(&results[1], &results, 1, None);
        assert!((confidence.score_gap - 0.12).abs() < 0.001);
    }

    #[test]
    fn test_score_gap_last_result() {
        let results = vec![make_result(1, 0.95, 2), make_result(2, 0.82, 2)];

        // Last result: gap should be 0.0
        let confidence = compute_result_confidence(&results[1], &results, 1, None);
        assert_eq!(confidence.score_gap, 0.0);
    }

    #[test]
    fn test_score_gap_single_result() {
        let results = vec![make_result(1, 0.95, 2)];

        // Single result: gap should be 0.0
        let confidence = compute_result_confidence(&results[0], &results, 0, None);
        assert_eq!(confidence.score_gap, 0.0);
    }

    #[test]
    fn test_exact_match_none() {
        let results = vec![make_result(1, 0.95, 2)];

        let confidence = compute_result_confidence(&results[0], &results, 0, None);
        assert!(!confidence.is_exact_match);
    }

    #[test]
    fn test_exact_match_below_threshold() {
        let results = vec![make_result(1, 0.95, 2)];

        let confidence = compute_result_confidence(&results[0], &results, 0, Some(2.8));
        assert!(!confidence.is_exact_match);

        let confidence = compute_result_confidence(&results[0], &results, 0, Some(1.0));
        assert!(!confidence.is_exact_match);
    }

    #[test]
    fn test_exact_match_at_threshold() {
        let results = vec![make_result(1, 0.95, 2)];

        let confidence = compute_result_confidence(&results[0], &results, 0, Some(2.9));
        assert!(confidence.is_exact_match);
    }

    #[test]
    fn test_exact_match_above_threshold() {
        let results = vec![make_result(1, 0.95, 2)];

        let confidence = compute_result_confidence(&results[0], &results, 0, Some(3.0));
        assert!(confidence.is_exact_match);

        let confidence = compute_result_confidence(&results[0], &results, 0, Some(5.0));
        assert!(confidence.is_exact_match);
    }

    #[test]
    fn test_all_fields_together() {
        let results = vec![
            make_result(1, 0.95, 4), // All 4 sources
            make_result(2, 0.82, 2),
            make_result(3, 0.70, 1),
        ];

        let confidence = compute_result_confidence(&results[0], &results, 0, Some(3.5));

        assert_eq!(confidence.source_count, 4);
        assert!((confidence.score_gap - 0.13).abs() < 0.001);
        assert!(confidence.is_exact_match);
    }

    #[test]
    fn test_zero_score_gap() {
        // Edge case: two results with identical scores
        let results = vec![make_result(1, 0.95, 2), make_result(2, 0.95, 2)];

        let confidence = compute_result_confidence(&results[0], &results, 0, None);
        assert_eq!(confidence.score_gap, 0.0);
    }

    /// Test that exact_match_multiplier is accessible from FusedResult without debug mode.
    /// This verifies SRCHCONF-1002: exact match multiplier should be always available.
    #[test]
    fn test_exact_match_multiplier_always_available() {
        use crate::search::fusion::FusedResult;
        use std::collections::HashMap;

        // Create a FusedResult with exact_match_multiplier set
        let mut source_scores = HashMap::new();
        source_scores.insert(SearchSource::FTS, 0.95);

        let result = FusedResult::with_exact_match(1, 0.95, source_scores, Some(3.0));

        // Verify exact_match_multiplier is accessible
        assert_eq!(result.exact_match_multiplier, Some(3.0));

        // Verify confidence computation uses it correctly
        let results = vec![result];
        let confidence =
            compute_result_confidence(&results[0], &results, 0, results[0].exact_match_multiplier);

        // With multiplier >= 2.9, is_exact_match should be true
        assert!(confidence.is_exact_match);
    }

    /// Test that exact_match_multiplier correctly distinguishes exact matches (3.0) from non-exact (1.0).
    #[test]
    fn test_exact_match_multiplier_detection() {
        use crate::search::fusion::FusedResult;
        use std::collections::HashMap;

        // Test exact match (multiplier = 3.0)
        let result_exact = FusedResult::with_exact_match(1, 0.95, HashMap::new(), Some(3.0));
        assert_eq!(result_exact.exact_match_multiplier, Some(3.0));
        let conf_exact = compute_result_confidence(
            &result_exact,
            &vec![result_exact.clone()],
            0,
            result_exact.exact_match_multiplier,
        );
        assert!(
            conf_exact.is_exact_match,
            "Multiplier 3.0 should be detected as exact match"
        );

        // Test non-exact match (multiplier = 1.0)
        let result_non_exact = FusedResult::with_exact_match(2, 0.85, HashMap::new(), Some(1.0));
        assert_eq!(result_non_exact.exact_match_multiplier, Some(1.0));
        let conf_non_exact = compute_result_confidence(
            &result_non_exact,
            &vec![result_non_exact.clone()],
            0,
            result_non_exact.exact_match_multiplier,
        );
        assert!(
            !conf_non_exact.is_exact_match,
            "Multiplier 1.0 should NOT be detected as exact match"
        );

        // Test None (no FTS result)
        let result_none = FusedResult::with_exact_match(3, 0.75, HashMap::new(), None);
        assert_eq!(result_none.exact_match_multiplier, None);
        let conf_none = compute_result_confidence(
            &result_none,
            &vec![result_none.clone()],
            0,
            result_none.exact_match_multiplier,
        );
        assert!(
            !conf_none.is_exact_match,
            "None multiplier should NOT be detected as exact match"
        );
    }

    /// Test edge case: empty source_scores HashMap.
    #[test]
    fn test_empty_source_scores() {
        use std::collections::HashMap;

        let results = vec![FusedResult::new(1, 0.95, HashMap::new())];

        let confidence = compute_result_confidence(&results[0], &results, 0, None);
        assert_eq!(confidence.source_count, 0);
        assert_eq!(confidence.score_gap, 0.0);
        assert!(!confidence.is_exact_match);
    }

    /// Test serialization roundtrip: ConfidenceSignals -> JSON -> ConfidenceSignals.
    #[test]
    fn test_confidence_signals_serialization_roundtrip() {
        use crate::search::results::ConfidenceSignals;

        let original = ConfidenceSignals {
            source_count: 3,
            score_gap: 0.15,
            is_exact_match: true,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Failed to serialize");

        // Deserialize back
        let deserialized: ConfidenceSignals =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify all fields match
        assert_eq!(deserialized.source_count, original.source_count);
        assert_eq!(deserialized.score_gap, original.score_gap);
        assert_eq!(deserialized.is_exact_match, original.is_exact_match);

        // Verify JSON format is correct
        assert!(json.contains("\"source_count\":3"));
        assert!(json.contains("\"score_gap\":0.15"));
        assert!(json.contains("\"is_exact_match\":true"));
    }
}
