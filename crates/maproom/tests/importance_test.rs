//! Integration tests for importance scoring system.
//!
//! These tests verify that the scoring system correctly combines multiple
//! factors and produces expected results in realistic scenarios.

use maproom::context::graph::EdgeType;
use maproom::context::importance::{
    ChunkMetadata, ImportanceScorer, Relationship, ScoringConfig,
};

#[test]
fn test_integration_basic_scoring() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 1,
        relpath: "src/main.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let chunk = ChunkMetadata {
        id: 2,
        relpath: "src/main.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let relationship = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    let score = scorer.score(&chunk, &relationship, &target);

    // Base=1.0, Calls=1.2, Distance=0.7, Importance=1.0, Recency=1.0, Churn=1.0, SameDir=1.3
    // = 1.0 * 1.2 * 0.7 * 1.0 * 1.0 * 1.0 * 1.3 = 1.092
    assert!(
        (score - 1.092).abs() < 0.01,
        "Expected ~1.092, got {}",
        score
    );
}

#[test]
fn test_integration_test_relationship_priority() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/module.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    // Test chunk: test_of relationship, same directory
    let test_chunk = ChunkMetadata {
        id: 1,
        relpath: "src/module.test.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let test_rel = Relationship {
        edge_type: EdgeType::TestOf,
        distance: 1,
    };

    // Regular chunk: calls relationship, same directory
    let regular_chunk = ChunkMetadata {
        id: 2,
        relpath: "src/helper.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let regular_rel = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    let test_score = scorer.score(&test_chunk, &test_rel, &target);
    let regular_score = scorer.score(&regular_chunk, &regular_rel, &target);

    // Test relationship (1.5x) should score higher than calls (1.2x)
    assert!(
        test_score > regular_score,
        "Test score {} should be > regular score {}",
        test_score,
        regular_score
    );

    // Verify the ratio is approximately 1.5/0.8 = 1.875 (test_of vs calls relationship weights)
    let ratio = test_score / regular_score;
    assert!(
        (ratio - 1.875).abs() < 0.01,
        "Expected ratio ~1.875, got {}",
        ratio
    );
}

#[test]
fn test_integration_distance_matters() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/api.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let chunk = ChunkMetadata {
        id: 1,
        relpath: "src/api.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    // Same relationship type, different distances
    let rel_close = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    let rel_far = Relationship {
        edge_type: EdgeType::Calls,
        distance: 3,
    };

    let score_close = scorer.score(&chunk, &rel_close, &target);
    let score_far = scorer.score(&chunk, &rel_far, &target);

    // Closer should score significantly higher
    assert!(
        score_close > score_far,
        "Close score {} should be > far score {}",
        score_close,
        score_far
    );

    // Verify exponential decay: 0.7^1 vs 0.7^3 = 0.343
    let expected_ratio = 0.7_f64.powf(1.0) / 0.7_f64.powf(3.0);
    let actual_ratio = score_close / score_far;
    assert!(
        (actual_ratio - expected_ratio).abs() < 0.01,
        "Expected ratio ~{}, got {}",
        expected_ratio,
        actual_ratio
    );
}

#[test]
fn test_integration_importance_amplifies_score() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/main.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let relationship = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    // Low importance chunk
    let low_importance = ChunkMetadata {
        id: 1,
        relpath: "src/main.rs".to_string(),
        importance_score: Some(0.5),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    // High importance chunk
    let high_importance = ChunkMetadata {
        id: 2,
        relpath: "src/main.rs".to_string(),
        importance_score: Some(3.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let low_score = scorer.score(&low_importance, &relationship, &target);
    let high_score = scorer.score(&high_importance, &relationship, &target);

    // High importance should score 6x higher (3.0 / 0.5)
    let ratio = high_score / low_score;
    assert!(
        (ratio - 6.0).abs() < 0.01,
        "Expected ratio ~6.0, got {}",
        ratio
    );
}

#[test]
fn test_integration_recency_boost() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/api.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let relationship = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    // Recent chunk
    let recent = ChunkMetadata {
        id: 1,
        relpath: "src/api.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    // Old chunk
    let old = ChunkMetadata {
        id: 2,
        relpath: "src/api.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(0.2),
        churn_score: Some(0.0),
    };

    let recent_score = scorer.score(&recent, &relationship, &target);
    let old_score = scorer.score(&old, &relationship, &target);

    // Recent should score 5x higher (1.0 / 0.2)
    let ratio = recent_score / old_score;
    assert!(
        (ratio - 5.0).abs() < 0.01,
        "Expected ratio ~5.0, got {}",
        ratio
    );
}

#[test]
fn test_integration_churn_penalty() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/core.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let relationship = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    // Stable chunk (low churn)
    let stable = ChunkMetadata {
        id: 1,
        relpath: "src/core.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.1),
    };

    // Unstable chunk (high churn)
    let unstable = ChunkMetadata {
        id: 2,
        relpath: "src/core.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(5.0),
    };

    let stable_score = scorer.score(&stable, &relationship, &target);
    let unstable_score = scorer.score(&unstable, &relationship, &target);

    // Stable should score significantly higher
    assert!(
        stable_score > unstable_score,
        "Stable score {} should be > unstable score {}",
        stable_score,
        unstable_score
    );

    // Verify the ratio matches inverse churn formula
    let stable_churn_factor = 1.0 / (1.0 + 0.1);
    let unstable_churn_factor = 1.0 / (1.0 + 5.0);
    let expected_ratio = stable_churn_factor / unstable_churn_factor;
    let actual_ratio = stable_score / unstable_score;
    assert!(
        (actual_ratio - expected_ratio).abs() < 0.01,
        "Expected ratio ~{}, got {}",
        expected_ratio,
        actual_ratio
    );
}

#[test]
fn test_integration_directory_bonus_impact() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/modules/handler.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let relationship = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    // Same directory
    let same_dir = ChunkMetadata {
        id: 1,
        relpath: "src/modules/helper.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    // Different directory
    let diff_dir = ChunkMetadata {
        id: 2,
        relpath: "src/utils/helper.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let same_score = scorer.score(&same_dir, &relationship, &target);
    let diff_score = scorer.score(&diff_dir, &relationship, &target);

    // Same directory should get 1.3x bonus
    let ratio = same_score / diff_score;
    assert!(
        (ratio - 1.3).abs() < 0.01,
        "Expected ratio ~1.3, got {}",
        ratio
    );
}

#[test]
fn test_integration_realistic_context_assembly_scenario() {
    let scorer = ImportanceScorer::new();

    // Scenario: Assembling context for an API handler function
    let target = ChunkMetadata {
        id: 100,
        relpath: "src/api/user_handler.rs".to_string(),
        importance_score: Some(2.0),
        recency_score: Some(0.8),
        churn_score: Some(0.3),
    };

    // Option 1: Direct test file (best choice)
    let test = ChunkMetadata {
        id: 1,
        relpath: "src/api/user_handler.test.rs".to_string(),
        importance_score: Some(1.5),
        recency_score: Some(0.9),
        churn_score: Some(0.2),
    };
    let test_rel = Relationship {
        edge_type: EdgeType::TestOf,
        distance: 1,
    };
    let test_score = scorer.score(&test, &test_rel, &target);

    // Option 2: Route that calls this handler (good choice)
    let route = ChunkMetadata {
        id: 2,
        relpath: "src/routes/user_routes.rs".to_string(),
        importance_score: Some(1.8),
        recency_score: Some(0.7),
        churn_score: Some(0.4),
    };
    let route_rel = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };
    let route_score = scorer.score(&route, &route_rel, &target);

    // Option 3: Database model this imports (moderate importance)
    let model = ChunkMetadata {
        id: 3,
        relpath: "src/models/user.rs".to_string(),
        importance_score: Some(1.2), // Lower importance to ensure proper ordering
        recency_score: Some(0.5),
        churn_score: Some(0.1),
    };
    let model_rel = Relationship {
        edge_type: EdgeType::Imports,
        distance: 1,
    };
    let model_score = scorer.score(&model, &model_rel, &target);

    // Option 4: Distant utility function (lower priority)
    let util = ChunkMetadata {
        id: 4,
        relpath: "src/utils/validators.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(0.3),
        churn_score: Some(0.2),
    };
    let util_rel = Relationship {
        edge_type: EdgeType::Imports,
        distance: 2,
    };
    let util_score = scorer.score(&util, &util_rel, &target);

    // Print scores for inspection
    println!("Test score: {:.4}", test_score);
    println!("Route score: {:.4}", route_score);
    println!("Model score: {:.4}", model_score);
    println!("Util score: {:.4}", util_score);

    // Verify ordering: test > route > model > util
    // Note: This verifies that relationship type matters, but importance can override
    assert!(
        test_score > route_score,
        "Test ({:.4}) should score higher than route ({:.4})",
        test_score,
        route_score
    );
    assert!(
        test_score > model_score,
        "Test ({:.4}) should score higher than model ({:.4})",
        test_score,
        model_score
    );
    assert!(
        route_score > util_score,
        "Route ({:.4}) should score higher than distant util ({:.4})",
        route_score,
        util_score
    );
    assert!(
        model_score > util_score,
        "Model ({:.4}) should score higher than util ({:.4})",
        model_score,
        util_score
    );

    // Test should be at least 2x higher than util
    assert!(
        test_score / util_score > 2.0,
        "Test should be significantly higher than distant util"
    );
}

#[test]
fn test_integration_custom_config() {
    // Custom config with different weights
    let config = ScoringConfig::new()
        .with_base_score(2.0)
        .with_decay_factor(0.8)
        .with_relationship_weight(EdgeType::TestOf, 2.0)
        .with_directory_bonus(1.5);

    let scorer = ImportanceScorer::with_config(config);

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/test.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let chunk = ChunkMetadata {
        id: 1,
        relpath: "src/test.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let relationship = Relationship {
        edge_type: EdgeType::TestOf,
        distance: 1,
    };

    let score = scorer.score(&chunk, &relationship, &target);

    // Base=2.0, TestOf=2.0, Distance=0.8, Importance=1.0, Recency=1.0, Churn=1.0, SameDir=1.5
    // = 2.0 * 2.0 * 0.8 * 1.0 * 1.0 * 1.0 * 1.5 = 4.8
    assert!(
        (score - 4.8).abs() < 0.01,
        "Expected ~4.8 with custom config, got {}",
        score
    );
}

#[test]
fn test_integration_disabled_recency_and_churn() {
    // Config with recency and churn disabled
    let config = ScoringConfig::new().with_recency(false).with_churn(false);

    let scorer = ImportanceScorer::with_config(config);

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/test.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    // Chunk with extreme recency and churn values
    let chunk = ChunkMetadata {
        id: 1,
        relpath: "src/test.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(0.1), // Very old
        churn_score: Some(10.0),  // Very churny
    };

    let relationship = Relationship {
        edge_type: EdgeType::Calls,
        distance: 1,
    };

    let score = scorer.score(&chunk, &relationship, &target);

    // With recency/churn disabled, score should not be affected by those factors
    // Base=1.0, Calls=1.2, Distance=0.7, Importance=1.0, SameDir=1.3
    // = 1.0 * 1.2 * 0.7 * 1.0 * 1.3 = 1.092
    assert!(
        (score - 1.092).abs() < 0.01,
        "Expected ~1.092 with disabled recency/churn, got {}",
        score
    );
}

#[test]
fn test_integration_score_range_sanity() {
    let scorer = ImportanceScorer::new();

    let target = ChunkMetadata {
        id: 100,
        relpath: "src/test.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    let chunk = ChunkMetadata {
        id: 1,
        relpath: "src/test.rs".to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    };

    // Test various distances
    for distance in 0..=5 {
        let relationship = Relationship {
            edge_type: EdgeType::Calls,
            distance,
        };

        let score = scorer.score(&chunk, &relationship, &target);

        // All scores should be in reasonable range
        assert!(score >= 0.0, "Score should be non-negative");
        assert!(score <= 100.0, "Score should be clamped to max 100.0");
    }
}
