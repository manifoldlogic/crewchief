//! Integration tests for query understanding metadata (SRCHTRN-2002).
//!
//! Tests that QueryUnderstanding metadata is properly assembled and attached
//! to search responses, including tokens, mode, filters, and timing breakdown.

#![allow(unused_imports)] // Justification: imports used across test helper functions and #[ignore] test bodies

use crewchief_maproom::search::results::{
    FinalSearchResults, QueryFilters, QueryProcessingDetails, QueryUnderstanding, SearchMetadata,
    SearchOptions, SearchTiming, TimingBreakdown,
};
use crewchief_maproom::search::types::SearchMode;
use std::collections::HashMap;

/// Helper to create test search results with understanding metadata
fn create_test_results_with_understanding(
    query: &str,
    mode: SearchMode,
    tokens: Vec<String>,
    expanded_terms: Vec<String>,
    repo_id: i64,
    worktree_id: Option<i64>,
) -> FinalSearchResults {
    let query_processing = QueryProcessingDetails::new(
        query.to_string(),
        mode,
        tokens.len(),
        expanded_terms.len(),
        tokens.join(" & "),
        false,
    );

    let result_counts = HashMap::new();
    let timing = SearchTiming::new(5.0, 30.0, 2.0, 5.0);

    let filters = QueryFilters {
        repo_id,
        worktree_id,
        file_types: vec![],
        recency_threshold: None,
    };

    let timing_breakdown = TimingBreakdown::new(5.0, 30.0, 2.0, 5.0);

    let understanding = QueryUnderstanding::from_query_data(
        mode,
        tokens,
        expanded_terms,
        filters,
        "basic_weighted".to_string(),
        timing_breakdown,
    );

    let metadata = SearchMetadata::with_understanding(
        query_processing,
        result_counts,
        timing,
        0,
        0,
        understanding,
    );

    FinalSearchResults::new(query.to_string(), vec![], metadata)
}

#[test]
fn test_query_understanding_in_response() {
    let result = create_test_results_with_understanding(
        "authenticate user",
        SearchMode::Auto,
        vec!["authenticate".to_string(), "user".to_string()],
        vec![],
        1,
        None,
    );

    // Verify metadata includes understanding
    assert!(result.metadata.understanding.is_some());
    let understanding = result.metadata.understanding.unwrap();

    // Verify tokens extracted
    assert_eq!(understanding.tokens, vec!["authenticate", "user"]);

    // Verify mode detected
    assert_eq!(understanding.mode, SearchMode::Auto);

    // Verify timing data present
    assert!(understanding.timing.total_ms > 0.0);
    assert!(understanding.timing.query_processing_ms > 0.0);

    // Verify timing breakdown is accurate (sum equals total)
    let sum = understanding.timing.query_processing_ms
        + understanding.timing.search_execution_ms
        + understanding.timing.score_fusion_ms
        + understanding.timing.result_assembly_ms;

    // Allow small floating point variance
    assert!((sum - understanding.timing.total_ms).abs() < 0.1);

    // Verify filters populated
    assert_eq!(understanding.filters.repo_id, 1);
}

#[test]
fn test_query_understanding_with_expanded_terms() {
    let result = create_test_results_with_understanding(
        "search",
        SearchMode::Code,
        vec!["search".to_string()],
        vec!["find".to_string(), "lookup".to_string()],
        42,
        Some(123),
    );

    let understanding = result.metadata.understanding.unwrap();

    // Verify tokens and expanded terms
    assert_eq!(understanding.tokens, vec!["search"]);
    assert_eq!(understanding.expanded_terms, vec!["find", "lookup"]);

    // Verify mode
    assert_eq!(understanding.mode, SearchMode::Code);

    // Verify filters include worktree
    assert_eq!(understanding.filters.repo_id, 42);
    assert_eq!(understanding.filters.worktree_id, Some(123));
}

#[test]
fn test_query_understanding_with_file_filters() {
    let query_processing = QueryProcessingDetails::new(
        "function".to_string(),
        SearchMode::Code,
        1,
        0,
        "function".to_string(),
        false,
    );

    let result_counts = HashMap::new();
    let timing = SearchTiming::new(4.0, 25.0, 1.5, 3.5);

    let filters = QueryFilters {
        repo_id: 1,
        worktree_id: None,
        file_types: vec!["ts".to_string(), "tsx".to_string(), "js".to_string()],
        recency_threshold: Some("7 days".to_string()),
    };

    let timing_breakdown = TimingBreakdown::new(4.0, 25.0, 1.5, 3.5);

    let understanding = QueryUnderstanding::from_query_data(
        SearchMode::Code,
        vec!["function".to_string()],
        vec![],
        filters,
        "basic_weighted".to_string(),
        timing_breakdown,
    );

    let metadata = SearchMetadata::with_understanding(
        query_processing,
        result_counts,
        timing,
        0,
        0,
        understanding,
    );

    let understanding = metadata.understanding.unwrap();

    // Verify file type filters
    assert_eq!(understanding.filters.file_types, vec!["ts", "tsx", "js"]);

    // Verify recency threshold
    assert_eq!(
        understanding.filters.recency_threshold,
        Some("7 days".to_string())
    );
}

#[test]
fn test_timing_breakdown_accuracy() {
    let timing = TimingBreakdown::new(4.2, 35.8, 2.1, 6.4);

    // Total should be sum of all stages
    assert_eq!(timing.total_ms, 48.5);

    // Individual timings should match
    assert_eq!(timing.query_processing_ms, 4.2);
    assert_eq!(timing.search_execution_ms, 35.8);
    assert_eq!(timing.score_fusion_ms, 2.1);
    assert_eq!(timing.result_assembly_ms, 6.4);
}

#[test]
fn test_fusion_strategy_recorded() {
    let result = create_test_results_with_understanding(
        "test query",
        SearchMode::Auto,
        vec!["test".to_string(), "query".to_string()],
        vec![],
        1,
        None,
    );

    let understanding = result.metadata.understanding.unwrap();

    // Verify fusion strategy is recorded
    assert_eq!(understanding.fusion_strategy, "basic_weighted");
}

#[test]
fn test_understanding_serialization() {
    let result = create_test_results_with_understanding(
        "search query",
        SearchMode::Text,
        vec!["search".to_string(), "query".to_string()],
        vec!["find".to_string()],
        1,
        Some(2),
    );

    // Serialize to JSON
    let json = serde_json::to_string(&result).unwrap();

    // Should contain understanding fields
    assert!(json.contains("understanding"));
    assert!(json.contains("tokens"));
    assert!(json.contains("search"));
    assert!(json.contains("query"));
    assert!(json.contains("basic_weighted"));

    // Deserialize back
    let deserialized: FinalSearchResults = serde_json::from_str(&json).unwrap();

    // Verify understanding is preserved
    assert!(deserialized.metadata.understanding.is_some());
    let understanding = deserialized.metadata.understanding.unwrap();
    assert_eq!(understanding.tokens, vec!["search", "query"]);
    assert_eq!(understanding.mode, SearchMode::Text);
}

#[test]
fn test_understanding_omitted_for_backward_compatibility() {
    // Create metadata without understanding (old format)
    let query_processing = QueryProcessingDetails::new(
        "test".to_string(),
        SearchMode::Auto,
        1,
        0,
        "test".to_string(),
        false,
    );
    let result_counts = HashMap::new();
    let timing = SearchTiming::new(1.0, 1.0, 1.0, 1.0);
    let metadata = SearchMetadata::new(query_processing, result_counts, timing, 0, 0);

    // Serialize to JSON
    let json = serde_json::to_value(&metadata).unwrap();

    // Understanding field should be omitted (not present as null)
    assert!(json.get("understanding").is_none());
}
