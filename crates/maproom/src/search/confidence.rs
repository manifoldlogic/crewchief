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
/// use crewchief_maproom::search::confidence::compute_result_confidence;
/// use crewchief_maproom::search::fusion::FusedResult;
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
}
