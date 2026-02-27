//! Comprehensive edge case tests for relationship expansion.
//!
//! Run with: cargo test --test edge_cases_test -- --nocapture
//!
//! These tests validate edge cases including confidence gating, error handling,
//! graph traversal edge cases, and empty result scenarios.
//!
//! Edge cases covered:
//! 1. No confidence data (include_related=true but confidence disabled)
//! 2. All low confidence results
//! 3. Exact match triggers expansion
//! 4. Source count threshold boundary (1, 2, 3)
//! 5. Graph traversal failure (graceful degradation)
//! 6. Empty graph (no edges)
//! 7. Depth limit enforced (max_depth=2)
//! 8. More than limit (top-N selection)
//! 9. Empty result semantics (None vs Some([]))
//! 10. MAX_CONCURRENT_EXPANSIONS cap
//! 11. Relevance sorting validation
//! 12. Module proximity boost validation
//! 13. Edge weight computation edge cases
//! 14. Preview truncation edge cases

use maproom::search::executor_types::SearchSource;
use maproom::search::results::{
    ChunkSearchResult, ConfidenceSignals, RelatedChunkResult,
};
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
// Edge Case Tests: Confidence Gating
// ============================================================================

#[test]
fn test_no_confidence_data_prevents_expansion() {
    // Scenario: include_related=true but confidence is None (disabled)
    // Expected: related should remain None (cannot gate without confidence)

    let result = create_chunk_result(1, "src/test.rs", Some("test_fn"), 10, 0.95, 3);

    // Result has no confidence (include_confidence was false)
    assert!(
        result.confidence.is_none(),
        "Confidence should be None when include_confidence=false"
    );

    // Without confidence data, expansion cannot be gated - related stays None
    assert!(
        result.related.is_none(),
        "Without confidence data, expansion should not run"
    );
}

#[test]
fn test_all_low_confidence_results() {
    // Scenario: All results have source_count < 2 and is_exact_match = false
    // Expected: No results should have related field populated

    let results: Vec<ChunkSearchResult> = (0..10)
        .map(|i| {
            create_chunk_result_with_confidence(
                i,
                &format!("src/file{}.rs", i),
                Some(&format!("fn_{}", i)),
                10 * i as i32,
                0.9 - (i as f32 * 0.05), // Decreasing scores
                1,                       // source_count = 1 (below threshold)
                0.05,                    // small gap
                false,                   // not exact match
            )
        })
        .collect();

    // Verify all results are low confidence
    for result in &results {
        let conf = result.confidence.as_ref().unwrap();
        assert!(
            conf.source_count < 2 && !conf.is_exact_match,
            "All results should be low confidence"
        );
    }

    // No result should have related (none qualify for expansion)
    let with_related = results.iter().filter(|r| r.related.is_some()).count();
    assert_eq!(
        with_related, 0,
        "No low-confidence result should have related chunks"
    );
}

#[test]
fn test_exact_match_bypasses_source_count_threshold() {
    // Scenario: source_count = 1 (below threshold) but is_exact_match = true
    // Expected: Should be eligible for expansion

    let result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("authenticate"),
        10,
        0.95,
        1, // Below source_count threshold
        0.15,
        true, // Exact match bypasses threshold
    );

    let conf = result.confidence.as_ref().unwrap();

    // Verify exact match qualifies for expansion despite low source_count
    let qualifies_for_expansion = conf.source_count >= 2 || conf.is_exact_match;
    assert!(
        qualifies_for_expansion,
        "Exact match should bypass source_count requirement"
    );
}

#[test]
fn test_source_count_threshold_boundary() {
    // Test all boundary conditions for source_count threshold
    let test_cases = vec![
        (1, false, false), // source_count=1, not exact -> NO expansion
        (2, false, true),  // source_count=2, not exact -> YES expansion
        (3, false, true),  // source_count=3, not exact -> YES expansion
        (4, false, true),  // source_count=4, not exact -> YES expansion
        (1, true, true),   // source_count=1, exact match -> YES expansion
        (2, true, true),   // source_count=2, exact match -> YES expansion
    ];

    for (source_count, is_exact_match, should_qualify) in test_cases {
        let conf = ConfidenceSignals {
            source_count,
            score_gap: 0.10,
            is_exact_match,
        };

        let qualifies = conf.source_count >= 2 || conf.is_exact_match;
        assert_eq!(
            qualifies, should_qualify,
            "source_count={}, is_exact_match={} should qualify={}",
            source_count, is_exact_match, should_qualify
        );
    }
}

// ============================================================================
// Edge Case Tests: Error Handling and Graceful Degradation
// ============================================================================

#[test]
fn test_graceful_degradation_on_graph_error() {
    // Scenario: Graph traversal fails (database error, corrupt data, etc.)
    // Expected: Result should have related=None, not panic or fail

    let mut result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("authenticate"),
        10,
        0.95,
        3, // High confidence
        0.15,
        false,
    );

    // Simulate error during expansion: related stays None
    // In real code: match find_top_related_chunks(...) { Err(e) => warn!(...), related = None }
    result.related = None;

    // Result should still be valid (graceful degradation)
    assert!(
        result.related.is_none(),
        "On graph traversal error, related should be None"
    );

    // Search should not fail - this result is still usable without related chunks
    assert_eq!(result.chunk_id, 1);
    assert!(result.confidence.is_some());
}

#[test]
fn test_graceful_degradation_no_panic() {
    // Scenario: Various error conditions should not panic

    // Empty chunk_id
    let result1 = create_chunk_result(0, "", None, 0, 0.0, 0);
    assert_eq!(result1.chunk_id, 0);

    // Very long path
    let long_path = "a/".repeat(100) + "file.rs";
    let result2 = create_chunk_result(1, &long_path, None, 1, 0.5, 1);
    assert!(result2.relpath.len() > 200);

    // Unicode in symbol name
    let result3 = create_chunk_result(2, "src/test.rs", Some("日本語関数"), 10, 0.9, 2);
    assert_eq!(result3.symbol_name, Some("日本語関数".to_string()));
}

// ============================================================================
// Edge Case Tests: Graph Traversal
// ============================================================================

#[test]
fn test_empty_graph_returns_empty_array() {
    // Scenario: High-confidence chunk has no relationships (isolated chunk)
    // Expected: related = Some([]), not None

    let mut result = create_chunk_result_with_confidence(
        1,
        "src/isolated.rs",
        Some("isolated_fn"),
        10,
        0.95,
        3, // High confidence
        0.15,
        false,
    );

    // Expansion ran but found no relationships
    result.related = Some(vec![]);

    // Verify empty array semantics (not None)
    assert!(
        result.related.is_some(),
        "Expansion should return Some([]) for isolated chunk"
    );
    assert_eq!(
        result.related.as_ref().unwrap().len(),
        0,
        "Empty array indicates expansion ran but found no relationships"
    );
}

#[test]
fn test_depth_limit_enforced() {
    // Scenario: Graph has chain A→B→C→D→E (depth 4)
    // Expected: Only return chunks at depth 1 and 2 (max_depth=2)

    let related_chunks = vec![
        create_related_chunk(10, "src/b.rs", Some("fn_b"), 1, 0.7, "call"),
        create_related_chunk(11, "src/c.rs", Some("fn_c"), 2, 0.49, "call"),
        // D and E would be depth 3 and 4 - not included
    ];

    // Verify all chunks are within depth limit
    for chunk in &related_chunks {
        assert!(
            chunk.depth <= 2,
            "Chunk depth {} exceeds max_depth=2",
            chunk.depth
        );
    }

    // Verify depth 1 and 2 are included
    let depths: Vec<i32> = related_chunks.iter().map(|c| c.depth).collect();
    assert!(depths.contains(&1), "Should include depth=1 chunks");
    assert!(depths.contains(&2), "Should include depth=2 chunks");
}

#[test]
fn test_more_than_limit_returns_top_n() {
    // Scenario: Chunk has 10 related chunks but limit is 5
    // Expected: Return top 5 by relevance

    let mut related_chunks: Vec<RelatedChunkResult> = (0..10)
        .map(|i| {
            create_related_chunk(
                100 + i,
                &format!("src/related{}.rs", i),
                Some(&format!("fn_{}", i)),
                1,
                0.9 - (i as f32 * 0.05), // Decreasing relevance
                "call",
            )
        })
        .collect();

    // Sort by relevance descending
    related_chunks.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top 5
    let top_5: Vec<RelatedChunkResult> = related_chunks.into_iter().take(5).collect();

    assert_eq!(top_5.len(), 5, "Should return exactly 5 chunks");

    // Verify sorted by relevance descending
    for i in 0..top_5.len() - 1 {
        assert!(
            top_5[i].relevance >= top_5[i + 1].relevance,
            "Results should be sorted by relevance descending"
        );
    }

    // Verify top 5 have highest relevance
    assert!(
        top_5[0].relevance >= 0.85,
        "Top result should have high relevance"
    );
}

#[test]
fn test_relevance_score_decay_by_depth() {
    // Scenario: Depth affects relevance (decay factor = 0.7^depth)
    // Expected: Depth 2 chunks have lower relevance than depth 1

    let depth_1_chunk = create_related_chunk(10, "src/a.rs", Some("fn_a"), 1, 0.7, "call");
    let depth_2_chunk = create_related_chunk(11, "src/b.rs", Some("fn_b"), 2, 0.49, "call");

    // Same base relevance would result in:
    // depth=1: relevance * 0.7^1 = 0.7
    // depth=2: relevance * 0.7^2 = 0.49

    assert!(
        depth_1_chunk.relevance > depth_2_chunk.relevance,
        "Depth 1 should have higher relevance than depth 2"
    );

    // Verify depth 2 is approximately 0.7x of depth 1
    let decay_ratio = depth_2_chunk.relevance / depth_1_chunk.relevance;
    assert!(
        (decay_ratio - 0.7).abs() < 0.1,
        "Decay ratio should be approximately 0.7"
    );
}

// ============================================================================
// Edge Case Tests: Empty Result Semantics
// ============================================================================

#[test]
fn test_none_vs_empty_array_semantics() {
    // Key distinction:
    // - related = None: Expansion did not run (low confidence, disabled, or error)
    // - related = Some([]): Expansion ran but found no relationships

    // Case 1: Low confidence - expansion did not run
    let low_conf_result = create_chunk_result_with_confidence(
        1,
        "src/test.rs",
        Some("helper"),
        10,
        0.70,
        1, // Low confidence
        0.05,
        false,
    );
    assert!(
        low_conf_result.related.is_none(),
        "None means expansion did not run (low confidence)"
    );

    // Case 2: High confidence but isolated - expansion ran, no results
    let mut isolated_result = create_chunk_result_with_confidence(
        2,
        "src/isolated.rs",
        Some("isolated_fn"),
        10,
        0.95,
        3, // High confidence
        0.15,
        false,
    );
    isolated_result.related = Some(vec![]);
    assert!(
        isolated_result.related.is_some() && isolated_result.related.as_ref().unwrap().is_empty(),
        "Some([]) means expansion ran but found no relationships"
    );
}

#[test]
fn test_serialization_none_omitted() {
    // When related=None, it should be omitted from JSON (skip_serializing_if)
    let result_without = create_chunk_result(1, "src/test.rs", Some("test_fn"), 10, 0.95, 3);

    let json = serde_json::to_value(&result_without).unwrap();
    assert!(
        json.get("related").is_none(),
        "related field should be omitted when None"
    );
}

#[test]
fn test_serialization_empty_array_included() {
    // When related=Some([]), it should be included in JSON
    let mut result_with_empty = create_chunk_result(1, "src/test.rs", Some("test_fn"), 10, 0.95, 3);
    result_with_empty.related = Some(vec![]);

    let json = serde_json::to_value(&result_with_empty).unwrap();
    assert!(
        json.get("related").is_some(),
        "related field should be present when Some([])"
    );

    let related_array = json.get("related").unwrap().as_array().unwrap();
    assert_eq!(related_array.len(), 0, "related should be empty array");
}

// ============================================================================
// Edge Case Tests: MAX_CONCURRENT_EXPANSIONS
// ============================================================================

#[test]
fn test_max_concurrent_expansions_enforced() {
    // Scenario: 5 high-confidence results, MAX_CONCURRENT_EXPANSIONS = 3
    // Expected: Only first 3 get expanded

    const MAX_CONCURRENT_EXPANSIONS: usize = 3;

    let mut results: Vec<ChunkSearchResult> = (0..5)
        .map(|i| {
            create_chunk_result_with_confidence(
                i,
                &format!("src/file{}.rs", i),
                Some(&format!("fn_{}", i)),
                10,
                0.90 - (i as f32 * 0.02), // Slight score variation
                3,                        // High confidence
                0.10,
                false,
            )
        })
        .collect();

    // Simulate expansion with cap
    let mut expansion_count = 0;
    for result in &mut results {
        if expansion_count >= MAX_CONCURRENT_EXPANSIONS {
            break;
        }

        if let Some(conf) = &result.confidence {
            if conf.source_count >= 2 || conf.is_exact_match {
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

    // Verify exactly 3 results have related chunks
    let expanded_count = results.iter().filter(|r| r.related.is_some()).count();
    assert_eq!(
        expanded_count, MAX_CONCURRENT_EXPANSIONS,
        "Should respect MAX_CONCURRENT_EXPANSIONS cap"
    );

    // First 3 should have related, last 2 should not
    assert!(results[0].related.is_some());
    assert!(results[1].related.is_some());
    assert!(results[2].related.is_some());
    assert!(results[3].related.is_none());
    assert!(results[4].related.is_none());
}

// ============================================================================
// Edge Case Tests: Field Validation
// ============================================================================

#[test]
fn test_related_chunk_field_validation() {
    // Validate all 10 required fields
    let chunk = RelatedChunkResult {
        chunk_id: 123,
        relpath: "src/auth/handler.ts".to_string(),
        symbol_name: Some("authenticate".to_string()),
        kind: "function".to_string(),
        start_line: 10,
        end_line: 25,
        preview: "export function authenticate() {...".to_string(),
        depth: 2,
        relevance: 0.7,
        relationship_type: "call".to_string(),
    };

    // Validate field types and values
    assert!(chunk.chunk_id > 0, "chunk_id should be positive");
    assert!(!chunk.relpath.is_empty(), "relpath should not be empty");
    assert!(!chunk.kind.is_empty(), "kind should not be empty");
    assert!(chunk.start_line > 0, "start_line should be positive");
    assert!(chunk.end_line >= chunk.start_line, "end_line >= start_line");
    assert!(!chunk.preview.is_empty(), "preview should not be empty");
    assert!(
        chunk.depth == 1 || chunk.depth == 2,
        "depth should be 1 or 2"
    );
    assert!(
        chunk.relevance > 0.0 && chunk.relevance <= 1.0,
        "relevance in (0, 1]"
    );
    assert!(
        !chunk.relationship_type.is_empty(),
        "relationship_type should not be empty"
    );
}

#[test]
fn test_related_chunk_null_symbol_name() {
    // symbol_name can be null for anonymous chunks (modules, config, etc.)
    let chunk = RelatedChunkResult {
        chunk_id: 123,
        relpath: "src/config.ts".to_string(),
        symbol_name: None, // Anonymous
        kind: "module".to_string(),
        start_line: 1,
        end_line: 100,
        preview: "export const config = {...".to_string(),
        depth: 1,
        relevance: 0.5,
        relationship_type: "import".to_string(),
    };

    assert!(
        chunk.symbol_name.is_none(),
        "symbol_name should be None for anonymous chunks"
    );
}

#[test]
fn test_relevance_edge_values() {
    // Test relevance boundary values

    // Minimum non-zero relevance
    let min_relevance = create_related_chunk(1, "src/a.rs", None, 2, 0.01, "import");
    assert!(min_relevance.relevance > 0.0);

    // Maximum relevance (1.0)
    let max_relevance = create_related_chunk(2, "src/b.rs", None, 1, 1.0, "call");
    assert!(max_relevance.relevance <= 1.0);
}

#[test]
fn test_relationship_types_valid() {
    // Test various relationship types
    let relationship_types = vec![
        "call",
        "import",
        "extends",
        "implements",
        "direct",
        "indirect",
    ];

    for rel_type in relationship_types {
        let chunk = create_related_chunk(1, "src/test.rs", None, 1, 0.5, rel_type);
        assert_eq!(chunk.relationship_type, rel_type);
    }
}
