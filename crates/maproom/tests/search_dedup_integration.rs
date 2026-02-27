//! Integration tests for search result deduplication.
//!
//! Run with: cargo test --test search_dedup_integration
//!
//! These tests validate deduplication behavior in multi-worktree scenarios
//! where the same code appears across different branches.

use maproom::db::SearchHit;
use maproom::search::dedup::{deduplicate, ChunkIdentity, DeduplicationConfig};
use maproom::search::executor_types::SearchSource;
use maproom::search::results::ChunkSearchResult;
use std::collections::HashMap;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test SearchHit with the specified parameters.
fn create_search_hit(
    score: f64,
    file_relpath: &str,
    symbol_name: Option<&str>,
    start_line: i32,
) -> SearchHit {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    SearchHit {
        chunk_id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        score,
        file_relpath: file_relpath.to_string(),
        symbol_name: symbol_name.map(|s| s.to_string()),
        kind: "function".to_string(),
        start_line,
        end_line: start_line + 10,
        base_score: None,
        kind_mult: None,
        exact_mult: None,
        preview: None,
    }
}

/// Create a test ChunkSearchResult with the specified parameters.
fn create_chunk_result(
    chunk_id: i64,
    relpath: &str,
    symbol_name: Option<&str>,
    start_line: i32,
    score: f32,
) -> ChunkSearchResult {
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
        HashMap::from([(SearchSource::FTS, score)]),
    )
}

/// Deduplicate SearchHit results (same logic as CLI's deduplicate_search_hits)
fn deduplicate_search_hits(hits: Vec<SearchHit>, limit: usize) -> Vec<SearchHit> {
    use std::collections::HashMap as StdHashMap;

    if hits.is_empty() {
        return hits;
    }

    let mut groups: StdHashMap<(String, Option<String>, i32), Vec<SearchHit>> = StdHashMap::new();
    for hit in hits {
        let key = (
            hit.file_relpath.clone(),
            hit.symbol_name.clone(),
            hit.start_line,
        );
        groups.entry(key).or_default().push(hit);
    }

    let mut deduped: Vec<SearchHit> = groups
        .into_values()
        .map(|mut group| {
            group.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            group.remove(0)
        })
        .collect();

    deduped.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    deduped.into_iter().take(limit).collect()
}

// ============================================================================
// ChunkIdentity Tests
// ============================================================================

#[test]
fn test_chunk_identity_same_code_different_worktrees() {
    // Same file/symbol/line in different worktrees should have same identity
    let chunk_main = create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9);
    let chunk_feature = create_chunk_result(2, "src/auth.rs", Some("validate"), 10, 0.8);

    let id_main = ChunkIdentity::from_result(&chunk_main);
    let id_feature = ChunkIdentity::from_result(&chunk_feature);

    assert_eq!(
        id_main, id_feature,
        "Same file/symbol/line should have equal identity"
    );
}

#[test]
fn test_chunk_identity_different_files() {
    let chunk1 = create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9);
    let chunk2 = create_chunk_result(2, "src/login.rs", Some("validate"), 10, 0.9);

    let id1 = ChunkIdentity::from_result(&chunk1);
    let id2 = ChunkIdentity::from_result(&chunk2);

    assert_ne!(id1, id2, "Different files should have different identity");
}

#[test]
fn test_chunk_identity_different_symbols() {
    let chunk1 = create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9);
    let chunk2 = create_chunk_result(2, "src/auth.rs", Some("authenticate"), 10, 0.9);

    let id1 = ChunkIdentity::from_result(&chunk1);
    let id2 = ChunkIdentity::from_result(&chunk2);

    assert_ne!(id1, id2, "Different symbols should have different identity");
}

#[test]
fn test_chunk_identity_different_lines() {
    // Same function moved by 1 line should be considered different
    let chunk1 = create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9);
    let chunk2 = create_chunk_result(2, "src/auth.rs", Some("validate"), 11, 0.9);

    let id1 = ChunkIdentity::from_result(&chunk1);
    let id2 = ChunkIdentity::from_result(&chunk2);

    assert_ne!(
        id1, id2,
        "Different start lines should have different identity"
    );
}

// ============================================================================
// Deduplication Function Tests
// ============================================================================

#[test]
fn test_deduplicate_removes_duplicates_across_worktrees() {
    // Simulate same function in main and feature branches
    let chunks = vec![
        create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9),
        create_chunk_result(2, "src/auth.rs", Some("validate"), 10, 0.8), // duplicate, lower score
        create_chunk_result(3, "src/utils.rs", Some("helper"), 20, 0.7),  // unique
    ];

    let config = DeduplicationConfig::default();
    let deduped = deduplicate(chunks, &config);

    assert_eq!(deduped.len(), 2, "Should have 2 unique results");
    assert_eq!(
        deduped[0].chunk_id, 1,
        "Should keep highest-scoring duplicate"
    );
}

#[test]
fn test_deduplicate_keeps_highest_score() {
    // Create duplicates with different scores
    let chunks = vec![
        create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.7),
        create_chunk_result(2, "src/auth.rs", Some("validate"), 10, 0.95), // highest
        create_chunk_result(3, "src/auth.rs", Some("validate"), 10, 0.8),
    ];

    let config = DeduplicationConfig::default();
    let deduped = deduplicate(chunks, &config);

    assert_eq!(deduped.len(), 1);
    assert_eq!(
        deduped[0].chunk_id, 2,
        "Should select chunk with highest score"
    );
    assert!((deduped[0].score - 0.95).abs() < 0.001);
}

#[test]
fn test_deduplicate_disabled() {
    let chunks = vec![
        create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9),
        create_chunk_result(2, "src/auth.rs", Some("validate"), 10, 0.8),
    ];

    let config = DeduplicationConfig {
        enabled: false,
        ..Default::default()
    };
    let deduped = deduplicate(chunks.clone(), &config);

    assert_eq!(
        deduped.len(),
        chunks.len(),
        "With dedup disabled, all results should be returned"
    );
}

#[test]
fn test_deduplicate_maintains_score_order() {
    let chunks = vec![
        create_chunk_result(1, "src/a.rs", Some("func_a"), 10, 0.5),
        create_chunk_result(2, "src/b.rs", Some("func_b"), 20, 0.9),
        create_chunk_result(3, "src/c.rs", Some("func_c"), 30, 0.7),
    ];

    let config = DeduplicationConfig::default();
    let deduped = deduplicate(chunks, &config);

    assert!(deduped[0].score >= deduped[1].score);
    assert!(deduped[1].score >= deduped[2].score);
}

// ============================================================================
// SearchHit Deduplication Tests (CLI-level)
// ============================================================================

#[test]
fn test_search_hit_dedup_multi_worktree() {
    // Simulate FTS search returning duplicates across worktrees
    let hits = vec![
        create_search_hit(0.95, "src/auth.rs", Some("validate"), 10), // worktree main
        create_search_hit(0.90, "src/auth.rs", Some("validate"), 10), // worktree feature
        create_search_hit(0.85, "src/auth.rs", Some("validate"), 10), // worktree hotfix
        create_search_hit(0.80, "src/utils.rs", Some("helper"), 20),  // unique
    ];

    let deduped = deduplicate_search_hits(hits, 10);

    assert_eq!(deduped.len(), 2, "Should dedupe to 2 unique results");
    assert!(
        (deduped[0].score - 0.95).abs() < 0.001,
        "Should keep highest score"
    );
}

#[test]
fn test_search_hit_dedup_respects_limit() {
    let hits = vec![
        create_search_hit(0.9, "src/a.rs", Some("a"), 10),
        create_search_hit(0.8, "src/b.rs", Some("b"), 10),
        create_search_hit(0.7, "src/c.rs", Some("c"), 10),
        create_search_hit(0.6, "src/d.rs", Some("d"), 10),
        create_search_hit(0.5, "src/e.rs", Some("e"), 10),
    ];

    let deduped = deduplicate_search_hits(hits, 3);

    assert_eq!(deduped.len(), 3, "Should respect limit");
    assert!(
        (deduped[0].score - 0.9).abs() < 0.001,
        "Should have highest scores"
    );
}

#[test]
fn test_search_hit_dedup_empty_input() {
    let hits: Vec<SearchHit> = vec![];
    let deduped = deduplicate_search_hits(hits, 10);
    assert!(deduped.is_empty());
}

#[test]
fn test_search_hit_dedup_null_symbol() {
    // Test that None symbol_name is handled correctly
    let hits = vec![
        create_search_hit(0.9, "src/module.rs", None, 1),
        create_search_hit(0.8, "src/module.rs", None, 1), // duplicate
    ];

    let deduped = deduplicate_search_hits(hits, 10);
    assert_eq!(deduped.len(), 1, "None symbol_name should be grouped");
}

// ============================================================================
// Scenario Tests
// ============================================================================

#[test]
fn test_realistic_multi_worktree_scenario() {
    // Simulate a realistic scenario:
    // - Same function exists in main, feature-auth, and feature-ui branches
    // - Feature branches have slightly different scores due to different activity
    let chunks = vec![
        // validate() in all 3 worktrees
        create_chunk_result(
            101,
            "src/auth/validate.rs",
            Some("validate_token"),
            15,
            0.92,
        ),
        create_chunk_result(
            201,
            "src/auth/validate.rs",
            Some("validate_token"),
            15,
            0.88,
        ),
        create_chunk_result(
            301,
            "src/auth/validate.rs",
            Some("validate_token"),
            15,
            0.85,
        ),
        // authenticate() only in main and feature-auth
        create_chunk_result(102, "src/auth/login.rs", Some("authenticate"), 30, 0.78),
        create_chunk_result(202, "src/auth/login.rs", Some("authenticate"), 30, 0.82),
        // render_ui() unique to feature-ui
        create_chunk_result(303, "src/ui/render.rs", Some("render_dashboard"), 10, 0.75),
    ];

    let config = DeduplicationConfig::default();
    let deduped = deduplicate(chunks, &config);

    assert_eq!(
        deduped.len(),
        3,
        "Should have 3 unique functions after dedup"
    );

    // Verify highest scores were kept
    let validate = deduped.iter().find(|c| c.relpath == "src/auth/validate.rs");
    assert!(validate.is_some());
    assert_eq!(
        validate.unwrap().chunk_id,
        101,
        "Should keep main's validate"
    );

    let authenticate = deduped.iter().find(|c| c.relpath == "src/auth/login.rs");
    assert!(authenticate.is_some());
    assert_eq!(
        authenticate.unwrap().chunk_id,
        202,
        "Should keep feature-auth's authenticate (higher score)"
    );
}

#[test]
fn test_dedup_with_line_drift() {
    // Simulate scenario where code moved by 1 line (not considered duplicate)
    let chunks = vec![
        create_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9),
        create_chunk_result(2, "src/auth.rs", Some("validate"), 11, 0.8), // shifted by 1 line
    ];

    let config = DeduplicationConfig::default();
    let deduped = deduplicate(chunks, &config);

    assert_eq!(
        deduped.len(),
        2,
        "Code shifted by 1 line should NOT be deduplicated (conservative approach)"
    );
}
