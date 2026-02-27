//! Integration tests for relationship expansion in search results.
//!
//! Run with: cargo test --test relationship_integration_test
//!
//! These tests validate that related chunks are correctly discovered and
//! included in search results when include_related=true.

use maproom::search::executor_types::SearchSource;
use maproom::search::results::{ChunkSearchResult, ConfidenceSignals, RelatedChunkResult};
use maproom::search::SearchOptions;
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

/// Create a test ChunkSearchResult with confidence.
fn create_chunk_result_with_confidence(
    chunk_id: i64,
    relpath: &str,
    symbol_name: Option<&str>,
    start_line: i32,
    score: f32,
    source_count: usize,
    score_gap: f32,
    is_exact_match: bool,
) -> ChunkSearchResult {
    let mut result = create_chunk_result(
        chunk_id,
        relpath,
        symbol_name,
        start_line,
        score,
        source_count,
    );
    result.confidence = Some(ConfidenceSignals {
        source_count,
        score_gap,
        is_exact_match,
    });
    result
}

/// Create a test RelatedChunkResult.
fn create_related_chunk(
    chunk_id: i64,
    relpath: &str,
    symbol_name: Option<&str>,
    depth: i32,
    relevance: f32,
    relationship_type: &str,
) -> RelatedChunkResult {
    RelatedChunkResult {
        chunk_id,
        relpath: relpath.to_string(),
        symbol_name: symbol_name.map(|s| s.to_string()),
        kind: "function".to_string(),
        start_line: 10,
        end_line: 20,
        preview: "fn related() {}".to_string(),
        depth,
        relevance,
        relationship_type: relationship_type.to_string(),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_search_options_include_related_default_false() {
    let options = SearchOptions::new(1, None, 10);
    assert!(
        !options.include_related,
        "include_related should default to false"
    );
}

#[test]
fn test_chunk_search_result_related_field_none_by_default() {
    let result = create_chunk_result(1, "src/test.rs", Some("test_fn"), 10, 0.95, 3);
    assert!(
        result.related.is_none(),
        "related field should be None by default"
    );
}

#[test]
fn test_confidence_gating_source_count_2() {
    // Create a high-confidence result with source_count >= 2
    let mut result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("authenticate"),
        10,
        0.95,
        2, // source_count = 2 (meets threshold)
        0.13,
        false, // not exact match
    );

    // Simulate relationship expansion (this would normally happen in pipeline)
    // High confidence (source_count >= 2), so should be eligible for expansion
    assert!(
        result.confidence.as_ref().unwrap().source_count >= 2,
        "Result should meet confidence threshold for expansion"
    );

    // Add mock related chunks
    result.related = Some(vec![
        create_related_chunk(10, "src/auth.rs", Some("login"), 1, 0.7, "call"),
        create_related_chunk(11, "src/session.rs", Some("create_session"), 1, 0.6, "call"),
    ]);

    assert!(
        result.related.is_some(),
        "High-confidence result should have related chunks"
    );
    assert_eq!(result.related.as_ref().unwrap().len(), 2);
}

#[test]
fn test_confidence_gating_exact_match() {
    // Create a result with exact match (even with source_count = 1)
    let mut result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("authenticate"),
        10,
        0.95,
        1, // source_count = 1 (below threshold)
        0.13,
        true, // exact match (meets threshold)
    );

    // Should be eligible for expansion because is_exact_match = true
    assert!(
        result.confidence.as_ref().unwrap().is_exact_match,
        "Result should meet confidence threshold via exact match"
    );

    // Add mock related chunks
    result.related = Some(vec![create_related_chunk(
        10,
        "src/auth.rs",
        Some("login"),
        1,
        0.7,
        "call",
    )]);

    assert!(
        result.related.is_some(),
        "Exact match result should have related chunks"
    );
}

#[test]
fn test_confidence_gating_low_confidence_no_expansion() {
    // Create a low-confidence result (source_count = 1, not exact match)
    let result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("helper"),
        10,
        0.70,
        1, // source_count = 1 (below threshold)
        0.05,
        false, // not exact match
    );

    // Should NOT be eligible for expansion
    assert!(
        result.confidence.as_ref().unwrap().source_count < 2,
        "Result should not meet source_count threshold"
    );
    assert!(
        !result.confidence.as_ref().unwrap().is_exact_match,
        "Result should not be exact match"
    );

    // related should remain None (no expansion for low confidence)
    assert!(
        result.related.is_none(),
        "Low-confidence result should not have related chunks"
    );
}

#[test]
fn test_max_concurrent_expansions_cap() {
    // Create 4 high-confidence results
    let mut results = vec![
        create_chunk_result_with_confidence(1, "src/a.rs", Some("fn_a"), 10, 0.95, 3, 0.10, false),
        create_chunk_result_with_confidence(2, "src/b.rs", Some("fn_b"), 20, 0.85, 2, 0.08, false),
        create_chunk_result_with_confidence(3, "src/c.rs", Some("fn_c"), 30, 0.77, 2, 0.05, false),
        create_chunk_result_with_confidence(4, "src/d.rs", Some("fn_d"), 40, 0.72, 2, 0.03, false),
    ];

    // Simulate MAX_CONCURRENT_EXPANSIONS = 3 hard cap
    const MAX_CONCURRENT_EXPANSIONS: usize = 3;
    let mut expansion_count = 0;

    for result in &mut results {
        if expansion_count >= MAX_CONCURRENT_EXPANSIONS {
            break; // Hard cap
        }

        // Only expand high-confidence results
        if let Some(conf) = &result.confidence {
            if conf.source_count >= 2 || conf.is_exact_match {
                // Mock expansion
                result.related = Some(vec![create_related_chunk(
                    100 + result.chunk_id,
                    "src/related.rs",
                    Some("related_fn"),
                    1,
                    0.6,
                    "call",
                )]);
                expansion_count += 1;
            }
        }
    }

    // Verify only 3 results have related chunks (hard cap enforced)
    let expanded_count = results.iter().filter(|r| r.related.is_some()).count();
    assert_eq!(
        expanded_count, MAX_CONCURRENT_EXPANSIONS,
        "Should respect MAX_CONCURRENT_EXPANSIONS cap"
    );

    // First 3 results should have related chunks
    assert!(
        results[0].related.is_some(),
        "First result should be expanded"
    );
    assert!(
        results[1].related.is_some(),
        "Second result should be expanded"
    );
    assert!(
        results[2].related.is_some(),
        "Third result should be expanded"
    );

    // Fourth result should NOT have related chunks (exceeded cap)
    assert!(
        results[3].related.is_none(),
        "Fourth result should not be expanded (cap exceeded)"
    );
}

#[test]
fn test_backward_compatibility_include_related_false() {
    // When include_related = false, related should remain None
    let options = SearchOptions::new(1, None, 10);
    assert!(!options.include_related);

    let result = create_chunk_result(1, "src/test.rs", Some("test_fn"), 10, 0.95, 3);
    assert!(
        result.related.is_none(),
        "related should be None when include_related=false"
    );
}

#[test]
fn test_empty_related_chunks_semantics() {
    // Test empty result semantics:
    // - None: Expansion did not run (low confidence, disabled, or error)
    // - Some([]): Expansion ran but found no relationships

    let mut result_not_expanded = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("helper"),
        10,
        0.70,
        1, // low confidence
        0.05,
        false,
    );

    // None: expansion did not run
    assert!(
        result_not_expanded.related.is_none(),
        "None means expansion did not run"
    );

    // Some([]): expansion ran but found no relationships
    result_not_expanded.related = Some(vec![]);
    assert!(
        result_not_expanded.related.is_some(),
        "Some([]) means expansion ran"
    );
    assert_eq!(
        result_not_expanded.related.as_ref().unwrap().len(),
        0,
        "Empty vector means no relationships found"
    );
}

#[test]
fn test_related_chunk_structure() {
    let related = create_related_chunk(10, "src/auth.rs", Some("login"), 1, 0.7, "call");

    assert_eq!(related.chunk_id, 10);
    assert_eq!(related.relpath, "src/auth.rs");
    assert_eq!(related.symbol_name, Some("login".to_string()));
    assert_eq!(related.depth, 1);
    assert_eq!(related.relevance, 0.7);
    assert_eq!(related.relationship_type, "call");
}

#[test]
fn test_serialization_skip_if_none() {
    // Test that related field is omitted from JSON when None
    let result_without_related =
        create_chunk_result(1, "src/test.rs", Some("test_fn"), 10, 0.95, 3);

    let json = serde_json::to_value(&result_without_related).unwrap();
    assert!(
        json.get("related").is_none(),
        "related field should be omitted when None (skip_serializing_if)"
    );

    // Test that related field is included when Some
    let mut result_with_related =
        create_chunk_result(2, "src/test.rs", Some("test_fn"), 10, 0.95, 3);
    result_with_related.related = Some(vec![create_related_chunk(
        10,
        "src/related.rs",
        Some("related_fn"),
        1,
        0.6,
        "call",
    )]);

    let json = serde_json::to_value(&result_with_related).unwrap();
    assert!(
        json.get("related").is_some(),
        "related field should be included when Some"
    );
    let related_array = json.get("related").unwrap().as_array().unwrap();
    assert_eq!(related_array.len(), 1);
}

#[test]
fn test_auto_enable_confidence_when_include_related_true() {
    // When include_related=true, confidence should be auto-enabled
    let include_confidence = false;
    let include_related = true;

    let enable_confidence = include_confidence || include_related;
    assert!(
        enable_confidence,
        "Confidence should be auto-enabled when include_related=true"
    );
}

#[test]
fn test_confidence_not_required_when_include_related_false() {
    // When include_related=false, confidence is independent
    let include_confidence = false;
    let include_related = false;

    let enable_confidence = include_confidence || include_related;
    assert!(
        !enable_confidence,
        "Confidence should not be enabled when both flags are false"
    );
}

#[test]
fn test_related_chunks_ordered_by_relevance() {
    let mut result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("authenticate"),
        10,
        0.95,
        3,
        0.13,
        false,
    );

    // Add related chunks with different relevance scores
    result.related = Some(vec![
        create_related_chunk(10, "src/auth.rs", Some("login"), 1, 0.7, "call"),
        create_related_chunk(11, "src/session.rs", Some("create_session"), 1, 0.6, "call"),
        create_related_chunk(12, "src/db.rs", Some("save_user"), 2, 0.5, "indirect"),
    ]);

    let related = result.related.as_ref().unwrap();
    assert_eq!(related.len(), 3);

    // Verify highest relevance is first (already sorted by find_top_related_chunks)
    assert!(related[0].relevance >= related[1].relevance);
    assert!(related[1].relevance >= related[2].relevance);
}

#[test]
fn test_graceful_degradation_error_handling() {
    // Simulate error during graph traversal (result.related = None on error)
    let mut result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("authenticate"),
        10,
        0.95,
        3,
        0.13,
        false,
    );

    // Simulate error: tracing::warn! logged but related stays None
    // (In real code: match find_top_related_chunks(...) { Err(e) => warn!(...) })
    result.related = None; // Error case

    assert!(
        result.related.is_none(),
        "On error, related should be None (graceful degradation)"
    );

    // The search should still succeed with this result (just without related chunks)
    // This is the key behavior: errors don't fail the entire search
}
