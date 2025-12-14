//! Integration tests for confidence scoring in search results.
//!
//! Run with: cargo test --test confidence_integration_test
//!
//! These tests validate that confidence signals are correctly computed and
//! included in search results when include_confidence=true.

use crewchief_maproom::search::executor_types::SearchSource;
use crewchief_maproom::search::fusion::FusedResult;
use crewchief_maproom::search::results::{ChunkSearchResult, ConfidenceSignals};
use crewchief_maproom::search::SearchOptions;
use std::collections::HashMap;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test ChunkSearchResult with the specified parameters.
fn create_chunk_result(
    chunk_id: i64,
    relpath: &str,
    symbol_name: Option<&str>,
    start_line: i32,
    score: f32,
    source_count: usize,
) -> ChunkSearchResult {
    let mut source_scores = HashMap::new();
    if source_count >= 1 {
        source_scores.insert(SearchSource::FTS, score);
    }
    if source_count >= 2 {
        source_scores.insert(SearchSource::Vector, score * 0.9);
    }
    if source_count >= 3 {
        source_scores.insert(SearchSource::Graph, score * 0.8);
    }
    if source_count >= 4 {
        source_scores.insert(SearchSource::Signals, score * 0.7);
    }

    ChunkSearchResult::new(
        chunk_id,
        1, // file_id
        relpath.to_string(),
        symbol_name.map(|s| s.to_string()),
        "function".to_string(),
        start_line,
        start_line + 10,
        "fn test() {}".to_string(),
        score,
        source_scores,
    )
}

/// Create a test FusedResult for confidence computation.
fn create_fused_result(
    chunk_id: i64,
    score: f32,
    source_count: usize,
    exact_match_multiplier: Option<f32>,
) -> FusedResult {
    let mut source_scores = HashMap::new();
    if source_count >= 1 {
        source_scores.insert(SearchSource::FTS, score);
    }
    if source_count >= 2 {
        source_scores.insert(SearchSource::Vector, score * 0.9);
    }
    if source_count >= 3 {
        source_scores.insert(SearchSource::Graph, score * 0.8);
    }
    if source_count >= 4 {
        source_scores.insert(SearchSource::Signals, score * 0.7);
    }

    FusedResult::with_exact_match(chunk_id, score, source_scores, exact_match_multiplier)
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_search_with_include_confidence_true_returns_confidence() {
    // Create test results
    let fused_results = vec![
        create_fused_result(1, 0.95, 3, Some(3.0)), // exact match
        create_fused_result(2, 0.82, 2, Some(1.0)), // not exact match
        create_fused_result(3, 0.70, 1, None),      // no FTS
    ];

    // Simulate what pipeline does: compute confidence for each result
    let results_with_confidence: Vec<ChunkSearchResult> = fused_results
        .iter()
        .enumerate()
        .map(|(index, fused)| {
            let confidence = Some(
                crewchief_maproom::search::confidence::compute_result_confidence(
                    fused,
                    &fused_results,
                    index,
                    fused.exact_match_multiplier,
                ),
            );

            let mut result = create_chunk_result(
                fused.chunk_id,
                "src/test.rs",
                Some("test_fn"),
                10,
                fused.score,
                fused.source_scores.len(),
            );
            result.confidence = confidence;
            result
        })
        .collect();

    // Verify all results have confidence
    assert_eq!(results_with_confidence.len(), 3);
    for result in &results_with_confidence {
        assert!(
            result.confidence.is_some(),
            "Result should have confidence when include_confidence=true"
        );
    }

    // Verify first result has correct confidence signals
    let first_confidence = results_with_confidence[0].confidence.as_ref().unwrap();
    assert_eq!(first_confidence.source_count, 3);
    assert!(
        first_confidence.score_gap > 0.0,
        "First result should have positive score gap"
    );
    assert!(
        first_confidence.is_exact_match,
        "Multiplier 3.0 should be detected as exact match"
    );

    // Verify second result
    let second_confidence = results_with_confidence[1].confidence.as_ref().unwrap();
    assert_eq!(second_confidence.source_count, 2);
    assert!(second_confidence.score_gap > 0.0);
    assert!(
        !second_confidence.is_exact_match,
        "Multiplier 1.0 should NOT be exact match"
    );

    // Verify third result (last result has 0.0 score gap)
    let third_confidence = results_with_confidence[2].confidence.as_ref().unwrap();
    assert_eq!(third_confidence.source_count, 1);
    assert_eq!(
        third_confidence.score_gap, 0.0,
        "Last result should have 0.0 score gap"
    );
    assert!(
        !third_confidence.is_exact_match,
        "None multiplier should NOT be exact match"
    );
}

#[test]
fn test_search_with_include_confidence_false_returns_none() {
    // Create test results WITHOUT confidence
    let fused_results = vec![
        create_fused_result(1, 0.95, 3, Some(3.0)),
        create_fused_result(2, 0.82, 2, Some(1.0)),
    ];

    // Simulate what pipeline does when include_confidence=false: confidence is None
    let results_without_confidence: Vec<ChunkSearchResult> = fused_results
        .iter()
        .map(|fused| {
            create_chunk_result(
                fused.chunk_id,
                "src/test.rs",
                Some("test_fn"),
                10,
                fused.score,
                fused.source_scores.len(),
            )
            // confidence remains None (default from constructor)
        })
        .collect();

    // Verify all results have confidence=None
    assert_eq!(results_without_confidence.len(), 2);
    for result in &results_without_confidence {
        assert!(
            result.confidence.is_none(),
            "Result should have confidence=None when include_confidence=false"
        );
    }
}

#[test]
fn test_default_search_options_has_include_confidence_false() {
    let options = SearchOptions::new(1, None, 10);

    assert!(
        !options.include_confidence,
        "Default SearchOptions should have include_confidence=false for backward compatibility"
    );
}

#[test]
fn test_confidence_fields_have_correct_values() {
    let fused_results = vec![
        create_fused_result(1, 0.95, 4, Some(3.5)), // All 4 sources, exact match
        create_fused_result(2, 0.80, 3, Some(1.0)), // 3 sources, not exact
        create_fused_result(3, 0.65, 2, None),      // 2 sources, no FTS
    ];

    let results: Vec<ChunkSearchResult> = fused_results
        .iter()
        .enumerate()
        .map(|(index, fused)| {
            let confidence = Some(
                crewchief_maproom::search::confidence::compute_result_confidence(
                    fused,
                    &fused_results,
                    index,
                    fused.exact_match_multiplier,
                ),
            );

            let mut result = create_chunk_result(
                fused.chunk_id,
                "src/test.rs",
                Some("test_fn"),
                10,
                fused.score,
                fused.source_scores.len(),
            );
            result.confidence = confidence;
            result
        })
        .collect();

    // Test first result
    let conf1 = results[0].confidence.as_ref().unwrap();
    assert_eq!(conf1.source_count, 4, "First result should have 4 sources");
    assert!(
        (conf1.score_gap - 0.15).abs() < 0.01,
        "Score gap should be ~0.15 (0.95 - 0.80)"
    );
    assert!(conf1.is_exact_match, "Multiplier 3.5 >= 2.9 is exact match");

    // Test second result
    let conf2 = results[1].confidence.as_ref().unwrap();
    assert_eq!(conf2.source_count, 3, "Second result should have 3 sources");
    assert!(
        (conf2.score_gap - 0.15).abs() < 0.01,
        "Score gap should be ~0.15 (0.80 - 0.65)"
    );
    assert!(
        !conf2.is_exact_match,
        "Multiplier 1.0 < 2.9 is NOT exact match"
    );

    // Test third result (last)
    let conf3 = results[2].confidence.as_ref().unwrap();
    assert_eq!(conf3.source_count, 2, "Third result should have 2 sources");
    assert_eq!(
        conf3.score_gap, 0.0,
        "Last result should have 0.0 score gap"
    );
    assert!(!conf3.is_exact_match, "None multiplier is NOT exact match");
}

#[test]
fn test_confidence_serialization_omits_none() {
    // Create result without confidence
    let result_without = create_chunk_result(1, "src/test.rs", Some("test_fn"), 10, 0.95, 2);
    assert!(result_without.confidence.is_none());

    // Serialize to JSON
    let json = serde_json::to_value(&result_without).expect("Failed to serialize");

    // Verify confidence field is omitted from JSON
    assert!(
        json.get("confidence").is_none(),
        "confidence field should be omitted when None (skip_serializing_if)"
    );

    // Create result with confidence
    let mut result_with = create_chunk_result(2, "src/test.rs", Some("test_fn"), 20, 0.85, 3);
    result_with.confidence = Some(ConfidenceSignals {
        source_count: 3,
        score_gap: 0.15,
        is_exact_match: true,
    });

    // Serialize to JSON
    let json = serde_json::to_value(&result_with).expect("Failed to serialize");

    // Verify confidence field is present
    assert!(
        json.get("confidence").is_some(),
        "confidence field should be present when Some(...)"
    );

    let confidence = json.get("confidence").unwrap();
    assert_eq!(confidence.get("source_count").unwrap(), 3);
    // Use approximate comparison for floating point
    let score_gap = confidence.get("score_gap").unwrap().as_f64().unwrap();
    assert!((score_gap - 0.15).abs() < 0.01, "score_gap should be ~0.15");
    assert_eq!(confidence.get("is_exact_match").unwrap(), true);
}

#[test]
fn test_score_gap_calculation_across_multiple_results() {
    let fused_results = vec![
        create_fused_result(1, 0.95, 2, None),
        create_fused_result(2, 0.90, 2, None),
        create_fused_result(3, 0.82, 2, None),
        create_fused_result(4, 0.75, 2, None),
        create_fused_result(5, 0.70, 2, None),
    ];

    let results: Vec<ChunkSearchResult> = fused_results
        .iter()
        .enumerate()
        .map(|(index, fused)| {
            let confidence = Some(
                crewchief_maproom::search::confidence::compute_result_confidence(
                    fused,
                    &fused_results,
                    index,
                    fused.exact_match_multiplier,
                ),
            );

            let mut result = create_chunk_result(
                fused.chunk_id,
                "src/test.rs",
                Some("test_fn"),
                10,
                fused.score,
                fused.source_scores.len(),
            );
            result.confidence = confidence;
            result
        })
        .collect();

    // Verify score gaps
    assert!(
        (results[0].confidence.as_ref().unwrap().score_gap - 0.05).abs() < 0.01,
        "Gap 1: 0.95 - 0.90 = 0.05"
    );
    assert!(
        (results[1].confidence.as_ref().unwrap().score_gap - 0.08).abs() < 0.01,
        "Gap 2: 0.90 - 0.82 = 0.08"
    );
    assert!(
        (results[2].confidence.as_ref().unwrap().score_gap - 0.07).abs() < 0.01,
        "Gap 3: 0.82 - 0.75 = 0.07"
    );
    assert!(
        (results[3].confidence.as_ref().unwrap().score_gap - 0.05).abs() < 0.01,
        "Gap 4: 0.75 - 0.70 = 0.05"
    );
    assert_eq!(
        results[4].confidence.as_ref().unwrap().score_gap,
        0.0,
        "Last result has 0.0 gap"
    );
}

#[test]
fn test_exact_match_threshold_detection() {
    // Test various multiplier values around the 2.9 threshold
    let test_cases = vec![
        (Some(0.0), false), // Well below threshold
        (Some(1.0), false), // Below threshold
        (Some(2.8), false), // Just below threshold
        (Some(2.9), true),  // At threshold
        (Some(3.0), true),  // Above threshold
        (Some(5.0), true),  // Well above threshold
        (None, false),      // No FTS result
    ];

    for (multiplier, expected_exact) in test_cases {
        let fused = create_fused_result(1, 0.95, 2, multiplier);
        let confidence = crewchief_maproom::search::confidence::compute_result_confidence(
            &fused,
            &vec![fused.clone()],
            0,
            multiplier,
        );

        assert_eq!(
            confidence.is_exact_match, expected_exact,
            "Multiplier {:?} should have is_exact_match={}",
            multiplier, expected_exact
        );
    }
}
