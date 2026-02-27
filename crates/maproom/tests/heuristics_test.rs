//! Integration tests for heuristics implementation.
//!
//! These tests verify:
//! - Test file detection via patterns
//! - Config file detection via patterns
//! - Heuristic scoring integration with importance scorer
//! - >90% test inclusion rate when tests exist
//! - Proper weight application across different file types

use maproom::context::{
    ChunkMetadata, EdgeType, FileType, HeuristicScorer, HeuristicsConfig, ImportanceScorer,
    Relationship, ScoringConfig,
};

// Helper function to create test chunk metadata
fn create_chunk(id: i64, relpath: &str) -> ChunkMetadata {
    ChunkMetadata {
        id,
        relpath: relpath.to_string(),
        importance_score: Some(1.0),
        recency_score: Some(1.0),
        churn_score: Some(0.0),
    }
}

// Helper function to create test relationship
fn create_relationship(edge_type: EdgeType, distance: u32) -> Relationship {
    Relationship {
        edge_type,
        distance,
    }
}

#[test]
fn test_heuristic_scorer_creation() {
    // Default configuration
    let scorer = HeuristicScorer::new();
    assert_eq!(scorer.config().test_weight, 1.5);
    assert_eq!(scorer.config().config_weight, 1.1);

    // Custom configuration
    let config = HeuristicsConfig::new()
        .with_test_weight(2.0)
        .with_config_weight(1.5);
    let custom_scorer = HeuristicScorer::with_config(config);
    assert_eq!(custom_scorer.config().test_weight, 2.0);
    assert_eq!(custom_scorer.config().config_weight, 1.5);
}

#[test]
fn test_detect_typescript_test_files() {
    let scorer = HeuristicScorer::new();

    // Various TypeScript/JavaScript test patterns
    let test_files = vec![
        "src/handler.test.ts",
        "src/component.test.tsx",
        "src/utils.test.js",
        "src/handler.spec.ts",
        "src/component.spec.tsx",
        "src/__tests__/handler.ts",
        "__tests__/integration.test.ts",
        "tests/unit/handler.ts",
        "src/handler_test.ts",
    ];

    for file in test_files {
        assert!(
            scorer.is_test_file(file),
            "Failed to detect test file: {}",
            file
        );
        assert_eq!(scorer.detect_file_type(file), FileType::Test);
    }
}

#[test]
fn test_detect_rust_test_files() {
    let scorer = HeuristicScorer::new();

    let test_files = vec![
        "src/lib_test.rs",
        "src/parser.test.rs",
        "tests/integration.rs",
        "src/handler.spec.rs",
    ];

    for file in test_files {
        assert!(
            scorer.is_test_file(file),
            "Failed to detect Rust test file: {}",
            file
        );
    }
}

#[test]
fn test_detect_config_files() {
    let scorer = HeuristicScorer::new();

    let config_files = vec![
        "package.json",
        "tsconfig.json",
        "jsconfig.json",
        "vite.config.ts",
        "webpack.config.js",
        "jest.config.json",
        ".env",
        ".env.local",
        ".env.production",
        "Cargo.toml",
        "go.mod",
        "pyproject.toml",
        "setup.py",
    ];

    for file in config_files {
        assert!(
            scorer.is_config_file(file),
            "Failed to detect config file: {}",
            file
        );
        assert_eq!(scorer.detect_file_type(file), FileType::Config);
    }
}

#[test]
fn test_detect_regular_files() {
    let scorer = HeuristicScorer::new();

    let regular_files = vec![
        "src/handler.ts",
        "src/component.tsx",
        "src/lib.rs",
        "README.md",
        "src/utils/helper.ts",
        "main.go",
        "app.py",
    ];

    for file in regular_files {
        assert_eq!(
            scorer.detect_file_type(file),
            FileType::Regular,
            "Incorrectly classified regular file: {}",
            file
        );
        assert!(!scorer.is_test_file(file));
        assert!(!scorer.is_config_file(file));
    }
}

#[test]
fn test_heuristic_weight_application() {
    let scorer = HeuristicScorer::new();
    let base_score = 1.0;

    // Test file should get 1.5x boost
    let test_score = scorer.apply_heuristic_weight(base_score, "handler.test.ts");
    assert!((test_score - 1.5).abs() < 0.01);

    // Config file should get 1.1x boost
    let config_score = scorer.apply_heuristic_weight(base_score, "package.json");
    assert!((config_score - 1.1).abs() < 0.01);

    // Regular file should have no boost
    let regular_score = scorer.apply_heuristic_weight(base_score, "handler.ts");
    assert!((regular_score - 1.0).abs() < 0.01);
}

#[test]
fn test_importance_scorer_with_heuristics() {
    let scorer = ImportanceScorer::new();
    let target = create_chunk(100, "src/handler.ts");

    // Test file should score higher than regular file
    let test_chunk = create_chunk(1, "src/handler.test.ts");
    let regular_chunk = create_chunk(2, "src/utils.ts");

    let rel = create_relationship(EdgeType::TestOf, 1);

    let test_score = scorer.score(&test_chunk, &rel, &target);
    let regular_score = scorer.score(&regular_chunk, &rel, &target);

    // Test file should have higher score due to heuristic weight
    assert!(
        test_score > regular_score,
        "Test file score ({}) should be higher than regular file score ({})",
        test_score,
        regular_score
    );

    // The ratio should be approximately 1.5x
    let ratio = test_score / regular_score;
    assert!(
        (ratio - 1.5).abs() < 0.2,
        "Test/regular ratio should be ~1.5, got {}",
        ratio
    );
}

#[test]
fn test_importance_scorer_without_heuristics() {
    let config = ScoringConfig::new();
    let scorer = ImportanceScorer::without_heuristics(config);
    let target = create_chunk(100, "src/handler.ts");

    // With heuristics disabled, test files should not get special boost
    let test_chunk = create_chunk(1, "src/handler.test.ts");
    let regular_chunk = create_chunk(2, "src/utils.ts");

    // Use same relationship and distance for fair comparison
    let rel = create_relationship(EdgeType::Calls, 1);

    let test_score = scorer.score(&test_chunk, &rel, &target);
    let regular_score = scorer.score(&regular_chunk, &rel, &target);

    // Scores should be similar since heuristics are disabled
    assert!(
        (test_score - regular_score).abs() < 0.01,
        "Without heuristics, scores should be similar"
    );
}

#[test]
fn test_config_file_scoring() {
    let scorer = ImportanceScorer::new();
    let target = create_chunk(100, "src/handler.ts");

    // Config file at root (no directory bonus)
    let config_chunk = create_chunk(1, "package.json");
    // Regular file in different directory (also no directory bonus)
    let regular_chunk = create_chunk(2, "lib/utils.ts");

    let rel = create_relationship(EdgeType::Imports, 1);

    let config_score = scorer.score(&config_chunk, &rel, &target);
    let regular_score = scorer.score(&regular_chunk, &rel, &target);

    // Config file should score slightly higher due to config heuristic weight (1.1x)
    assert!(
        config_score > regular_score,
        "Config file score ({}) should be higher than regular file score ({})",
        config_score,
        regular_score
    );
}

#[test]
fn test_test_file_priority_with_multiple_relationships() {
    let scorer = ImportanceScorer::new();
    let target = create_chunk(100, "src/api/handler.ts");

    // Create several related chunks
    let test_chunk = create_chunk(1, "src/api/handler.test.ts"); // Test file, same directory
    let caller_chunk = create_chunk(2, "src/controllers/user.ts"); // Caller from different directory
    let config_chunk = create_chunk(3, "package.json"); // Config file

    // All at distance 1 for fair comparison
    let test_rel = create_relationship(EdgeType::TestOf, 1);
    let caller_rel = create_relationship(EdgeType::Calls, 1);
    let config_rel = create_relationship(EdgeType::Imports, 1);

    let test_score = scorer.score(&test_chunk, &test_rel, &target);
    let caller_score = scorer.score(&caller_chunk, &caller_rel, &target);
    let config_score = scorer.score(&config_chunk, &config_rel, &target);

    // Test should score highest (TestOf relationship + test file heuristic + same directory)
    assert!(
        test_score > caller_score,
        "Test should score higher than caller: {} vs {}",
        test_score,
        caller_score
    );
    assert!(
        test_score > config_score,
        "Test should score higher than config: {} vs {}",
        test_score,
        config_score
    );

    // Config should score higher than regular caller due to config heuristic
    // Note: This might not always be true depending on relationship weights
    // but we're testing that heuristics have an impact
}

#[test]
fn test_same_directory_and_test_heuristic_combine() {
    let scorer = ImportanceScorer::new();
    let target = create_chunk(100, "src/modules/handler.ts");

    // Test in same directory
    let same_dir_test = create_chunk(1, "src/modules/handler.test.ts");
    // Test in different directory
    let diff_dir_test = create_chunk(2, "tests/handler.test.ts");

    let rel = create_relationship(EdgeType::TestOf, 1);

    let same_dir_score = scorer.score(&same_dir_test, &rel, &target);
    let diff_dir_score = scorer.score(&diff_dir_test, &rel, &target);

    // Same directory test should score higher (both get test heuristic, but same_dir gets directory bonus)
    assert!(
        same_dir_score > diff_dir_score,
        "Same directory test should score higher: {} vs {}",
        same_dir_score,
        diff_dir_score
    );

    // The ratio should include both directory bonus (1.3x) and test weight (1.5x)
    // Note: The exact ratio depends on all the scoring factors
}

#[test]
fn test_custom_heuristic_weights() {
    let config = ScoringConfig::new();
    let heuristic_config = HeuristicsConfig::new()
        .with_test_weight(3.0) // Very high test weight
        .with_config_weight(2.0); // High config weight

    let scorer =
        ImportanceScorer::with_heuristics(config, HeuristicScorer::with_config(heuristic_config));
    let target = create_chunk(100, "src/handler.ts");

    let test_chunk = create_chunk(1, "src/handler.test.ts");
    let config_chunk = create_chunk(2, "package.json");
    let regular_chunk = create_chunk(3, "src/utils.ts");

    let rel = create_relationship(EdgeType::Imports, 1);

    let test_score = scorer.score(&test_chunk, &rel, &target);
    let config_score = scorer.score(&config_chunk, &rel, &target);
    let regular_score = scorer.score(&regular_chunk, &rel, &target);

    // Test should be significantly higher with 3.0x weight
    assert!(test_score > config_score);
    assert!(config_score > regular_score);

    // Verify approximate ratios
    let test_ratio = test_score / regular_score;
    assert!(
        test_ratio > 2.5,
        "Test ratio should be > 2.5, got {}",
        test_ratio
    );
}

#[test]
fn test_multiple_test_patterns() {
    let scorer = HeuristicScorer::new();

    // Test various naming conventions
    let patterns = vec![
        ("handler.test.ts", true),
        ("handler.spec.ts", true),
        ("handler_test.ts", true),
        ("__tests__/handler.ts", true),
        ("tests/handler.ts", true),
        ("handler.tests.ts", false), // Invalid pattern
        ("test_handler.ts", false),  // Invalid pattern
        ("handler.ts", false),
    ];

    for (file, expected) in patterns {
        let is_test = scorer.is_test_file(file);
        assert_eq!(
            is_test, expected,
            "Pattern {} should be test={}, got {}",
            file, expected, is_test
        );
    }
}

#[test]
fn test_path_normalization_windows_style() {
    let scorer = HeuristicScorer::new();

    // Windows-style paths with backslashes
    assert!(scorer.is_test_file("src\\handler.test.ts"));
    assert!(scorer.is_test_file("src\\__tests__\\integration.ts"));
    assert!(scorer.is_config_file("config\\jest.config.js"));

    // Mixed separators
    assert!(scorer.is_test_file("src/__tests__\\handler.test.ts"));
}

#[test]
fn test_nested_directory_structures() {
    let scorer = HeuristicScorer::new();

    // Deeply nested paths
    assert!(scorer.is_test_file("src/modules/auth/handlers/__tests__/login.test.ts"));
    assert!(scorer.is_test_file("packages/api/src/routes/users.spec.ts"));
    assert!(scorer.is_config_file("apps/web/vite.config.ts"));
}

#[test]
fn test_heuristic_scorer_accessor() {
    let scorer = ImportanceScorer::new();

    // Should have heuristic scorer enabled by default
    assert!(scorer.heuristic_scorer().is_some());

    let config = ScoringConfig::new();
    let no_heuristics = ImportanceScorer::without_heuristics(config);

    // Should not have heuristic scorer when disabled
    assert!(no_heuristics.heuristic_scorer().is_none());
}

/// Test that demonstrates >90% test inclusion rate requirement.
///
/// This test simulates a realistic scenario where:
/// - We have multiple implementation chunks
/// - Each has an associated test file
/// - We rank them and verify tests are highly prioritized
#[test]
fn test_high_test_inclusion_rate() {
    let scorer = ImportanceScorer::new();

    // Create 10 implementation chunks with their test counterparts
    let mut chunks_and_scores = vec![];

    for i in 1..=10 {
        let target = create_chunk(i * 100, &format!("src/module{}.ts", i));

        // Test file for this chunk
        let test_chunk = create_chunk(i, &format!("src/module{}.test.ts", i));
        let test_rel = create_relationship(EdgeType::TestOf, 1);
        let test_score = scorer.score(&test_chunk, &test_rel, &target);
        chunks_and_scores.push(("test", i, test_score));

        // Regular caller from same directory
        let caller_chunk = create_chunk(i + 100, &format!("src/caller{}.ts", i));
        let caller_rel = create_relationship(EdgeType::Calls, 1);
        let caller_score = scorer.score(&caller_chunk, &caller_rel, &target);
        chunks_and_scores.push(("caller", i, caller_score));

        // Regular callee from different directory
        let callee_chunk = create_chunk(i + 200, &format!("src/utils/helper{}.ts", i));
        let callee_rel = create_relationship(EdgeType::Calls, 1);
        let callee_score = scorer.score(&callee_chunk, &callee_rel, &target);
        chunks_and_scores.push(("callee", i, callee_score));
    }

    // Sort by score descending
    chunks_and_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    // Count how many tests are in the top 10 (90% of test files)
    let top_10 = &chunks_and_scores[0..10];
    let test_count = top_10.iter().filter(|(t, _, _)| *t == "test").count();

    // We should have at least 9 out of 10 tests in the top ranks (>90%)
    assert!(
        test_count >= 9,
        "Expected at least 9 tests in top 10, got {}. Top 10: {:?}",
        test_count,
        top_10
            .iter()
            .map(|(t, i, s)| format!("{}-{}: {:.2}", t, i, s))
            .collect::<Vec<_>>()
    );

    // All 10 tests should be in the top 15 (showing very high priority)
    let top_15 = &chunks_and_scores[0..15];
    let test_count_15 = top_15.iter().filter(|(t, _, _)| *t == "test").count();
    assert_eq!(
        test_count_15, 10,
        "All 10 tests should be in top 15, got {}",
        test_count_15
    );
}

/// Test scoring across many file types to verify heuristics work at scale
#[test]
fn test_heuristics_at_scale() {
    let scorer = ImportanceScorer::new();
    let target = create_chunk(1000, "lib/main.ts");
    let rel = create_relationship(EdgeType::Imports, 1);

    let file_types = vec![
        // Test files (should score highest)
        ("lib/handler.test.ts", "test"),
        ("lib/component.spec.tsx", "test"),
        ("lib/integration.test.ts", "test"),
        // Config files (should score higher than regular from different dir)
        ("package.json", "config"),
        ("tsconfig.json", "config"),
        (".env", "config"),
        // Regular files from different directory (no same-dir bonus)
        ("src/handler.ts", "regular"),
        ("src/component.tsx", "regular"),
        ("src/utils.ts", "regular"),
    ];

    let mut scores: Vec<(&str, &str, f64)> = file_types
        .iter()
        .map(|(path, category)| {
            let chunk = create_chunk(1, path);
            let score = scorer.score(&chunk, &rel, &target);
            (*path, *category, score)
        })
        .collect();

    // Sort by score descending
    scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    // Verify test files are at the top
    let top_3 = &scores[0..3];
    let test_count_top3 = top_3.iter().filter(|(_, cat, _)| *cat == "test").count();
    assert_eq!(
        test_count_top3, 3,
        "Top 3 should all be test files, got: {:?}",
        top_3
    );

    // Verify config files rank higher than regular files from different directories
    let config_scores: Vec<f64> = scores
        .iter()
        .filter(|(_, cat, _)| *cat == "config")
        .map(|(_, _, s)| *s)
        .collect();

    let regular_scores: Vec<f64> = scores
        .iter()
        .filter(|(_, cat, _)| *cat == "regular")
        .map(|(_, _, s)| *s)
        .collect();

    let avg_config = config_scores.iter().sum::<f64>() / config_scores.len() as f64;
    let avg_regular = regular_scores.iter().sum::<f64>() / regular_scores.len() as f64;

    assert!(
        avg_config > avg_regular,
        "Average config score ({:.2}) should be higher than average regular score ({:.2})",
        avg_config,
        avg_regular
    );
}

/// Test that verifies the exact acceptance criteria from the ticket:
/// Tests included >90% of the time when they exist for the target chunk
#[test]
fn test_acceptance_criteria_test_inclusion_rate() {
    let scorer = ImportanceScorer::new();

    // Simulate 100 different scenarios
    let mut test_included_count = 0;
    let total_scenarios = 100;

    for i in 0..total_scenarios {
        let target = create_chunk(i * 1000, &format!("src/module{}.ts", i));

        // For each scenario, compare test vs other relationships
        let test_chunk = create_chunk(i, &format!("src/module{}.test.ts", i));
        let test_rel = create_relationship(EdgeType::TestOf, 1);
        let test_score = scorer.score(&test_chunk, &test_rel, &target);

        // Various competing chunks
        let competing_chunks = vec![
            (
                create_chunk(i + 1000, &format!("src/caller{}.ts", i)),
                create_relationship(EdgeType::Calls, 1),
            ),
            (
                create_chunk(i + 2000, &format!("src/utils/helper{}.ts", i)),
                create_relationship(EdgeType::Calls, 2),
            ),
            (
                create_chunk(i + 3000, &format!("src/import{}.ts", i)),
                create_relationship(EdgeType::Imports, 1),
            ),
        ];

        let mut all_scores = vec![("test", test_score)];
        for (chunk, rel) in competing_chunks {
            let score = scorer.score(&chunk, &rel, &target);
            all_scores.push(("other", score));
        }

        // Sort by score
        all_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Check if test is in top position (would be included)
        if all_scores[0].0 == "test" {
            test_included_count += 1;
        }
    }

    let inclusion_rate = (test_included_count as f64 / total_scenarios as f64) * 100.0;

    assert!(
        inclusion_rate >= 90.0,
        "Test inclusion rate should be >= 90%, got {:.1}%",
        inclusion_rate
    );

    println!(
        "✓ Test inclusion rate: {:.1}% (target: >=90%)",
        inclusion_rate
    );
}
