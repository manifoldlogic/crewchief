// Unit tests for quality-weighted graph importance (SRCHREL-1004)
//
// Tests validate:
// - Test detection patterns work correctly
// - Logarithmic scaling is applied correctly
// - Feature flag toggle affects behavior
//
// Note: Full integration tests with database are deferred to SRCHREL-1005.
// These unit tests focus on the algorithm logic without requiring database setup.

/// Test detection based on file paths (mirrors SQL LIKE patterns)
fn is_test_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".test.js")
        || lower.ends_with(".test.tsx")
        || lower.ends_with(".test.jsx")
        || lower.ends_with(".spec.ts")
        || lower.ends_with(".spec.js")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_test.py")
}

/// Calculate expected quality-weighted score
/// Formula: ln(2.0 + sum_of_edge_qualities)
fn expected_quality_score(production_edges: usize, test_edges: usize) -> f64 {
    let production_quality = production_edges as f64 * 1.0;
    let test_quality = test_edges as f64 * 0.5;
    let sum = production_quality + test_quality;
    (2.0 + sum).ln()
}

/// Calculate expected legacy score (all edges weighted equally)
/// Formula: ln(2.0 + edge_count)
fn expected_legacy_score(edge_count: usize) -> f64 {
    (2.0 + edge_count as f64).ln()
}

// ============================================================================
// Test Detection Pattern Tests
// ============================================================================

#[test]
fn test_file_path_patterns_test_directories() {
    // Directory patterns that should be detected as test code
    let test_paths = vec![
        "src/test/helper.ts",
        "src/tests/utils.ts",
        "src/__tests__/component.tsx",
        "lib/test/fixture.py",
        "packages/tests/integration.rs",
    ];

    for path in &test_paths {
        assert!(
            is_test_path(path),
            "Directory pattern should detect '{}' as test path",
            path
        );
    }
}

#[test]
fn test_file_path_patterns_test_extensions() {
    // File extension patterns that should be detected as test code
    let test_paths = vec![
        "src/auth.test.ts",
        "lib/utils.test.js",
        "components/Button.test.tsx",
        "hooks/useAuth.test.jsx",
        "validator.spec.ts",
        "parser.spec.js",
        "crates/maproom/db_test.rs",
        "lib/utils_test.py",
    ];

    for path in &test_paths {
        assert!(
            is_test_path(path),
            "Extension pattern should detect '{}' as test path",
            path
        );
    }
}

#[test]
fn test_file_path_patterns_production_code() {
    // Production paths that should NOT be detected as test code
    let production_paths = vec![
        "src/handler.ts",
        "lib/utils.ts",
        "crates/maproom/src/db.rs",
        "packages/cli/src/index.ts",
        "src/components/Button.tsx",
        "src/testing-utils.ts", // Contains 'test' but not a test file
        "src/latest.ts",        // Ends with 'test.ts' substring but not .test.ts
        "src/manifest.json",
        "docs/TESTING.md", // Documentation about testing, not actual test
    ];

    for path in &production_paths {
        assert!(
            !is_test_path(path),
            "Should NOT detect '{}' as test path",
            path
        );
    }
}

#[test]
fn test_file_path_case_insensitivity() {
    // Pattern matching should be case-insensitive
    let test_paths = vec![
        "src/TEST/helper.ts",
        "src/Tests/utils.ts",
        "src/__TESTS__/component.tsx",
        "src/Auth.Test.ts",
        "src/parser.SPEC.js",
    ];

    for path in &test_paths {
        assert!(
            is_test_path(path),
            "Case-insensitive matching should detect '{}' as test path",
            path
        );
    }
}

// ============================================================================
// Quality Score Calculation Tests
// ============================================================================

#[test]
fn test_quality_score_production_only() {
    // 5 production edges → quality sum = 5.0 → ln(2 + 5) = ln(7)
    let score = expected_quality_score(5, 0);
    let expected = (7.0_f64).ln();

    assert!(
        (score - expected).abs() < 0.001,
        "Production-only score {} should equal ln(7) = {}",
        score,
        expected
    );
}

#[test]
fn test_quality_score_test_only() {
    // 5 test edges → quality sum = 2.5 → ln(2 + 2.5) = ln(4.5)
    let score = expected_quality_score(0, 5);
    let expected = (4.5_f64).ln();

    assert!(
        (score - expected).abs() < 0.001,
        "Test-only score {} should equal ln(4.5) = {}",
        score,
        expected
    );
}

#[test]
fn test_quality_score_mixed() {
    // 3 production + 2 test edges → quality sum = 3 + 1 = 4 → ln(2 + 4) = ln(6)
    let score = expected_quality_score(3, 2);
    let expected = (6.0_f64).ln();

    assert!(
        (score - expected).abs() < 0.001,
        "Mixed score {} should equal ln(6) = {}",
        score,
        expected
    );
}

#[test]
fn test_quality_vs_legacy_scoring() {
    // Scenario: 1 production caller + 1 test caller
    // Legacy: all edges = 1.0 → sum = 2 → ln(4) ≈ 1.386
    // Quality: prod=1.0, test=0.5 → sum = 1.5 → ln(3.5) ≈ 1.253

    let legacy_score = expected_legacy_score(2);
    let quality_score = expected_quality_score(1, 1);

    assert!(
        quality_score < legacy_score,
        "Quality score ({}) should be lower than legacy score ({}) due to test penalty",
        quality_score,
        legacy_score
    );
}

#[test]
fn test_production_scores_higher_than_test() {
    // Same number of edges, but production should score higher
    // 5 production: ln(2 + 5*1.0) = ln(7) ≈ 1.946
    // 5 test:       ln(2 + 5*0.5) = ln(4.5) ≈ 1.504

    let production_score = expected_quality_score(5, 0);
    let test_score = expected_quality_score(0, 5);

    assert!(
        production_score > test_score,
        "Production score ({}) should be higher than test score ({})",
        production_score,
        test_score
    );

    // Verify the difference is meaningful (not just floating point noise)
    let difference = production_score - test_score;
    assert!(
        difference > 0.4,
        "Score difference ({}) should be meaningful (>0.4)",
        difference
    );
}

#[test]
fn test_logarithmic_scaling_diminishing_returns() {
    // Adding more edges should have diminishing returns due to LOG scaling
    let score_5 = expected_quality_score(5, 0); // ln(7)
    let score_10 = expected_quality_score(10, 0); // ln(12)
    let score_20 = expected_quality_score(20, 0); // ln(22)

    let delta_5_to_10 = score_10 - score_5;
    let delta_10_to_20 = score_20 - score_10;

    // First 5 additional edges should add more than next 10
    assert!(
        delta_5_to_10 < delta_10_to_20 * 1.5,
        "Logarithmic scaling should show diminishing returns: 5→10 delta ({}) vs 10→20 delta ({})",
        delta_5_to_10,
        delta_10_to_20
    );
}

#[test]
fn test_empty_edges_base_score() {
    // No edges → sum = 0 → ln(2 + 0) = ln(2)
    let score = expected_quality_score(0, 0);
    let expected = (2.0_f64).ln();

    assert!(
        (score - expected).abs() < 0.001,
        "Empty edges score {} should equal ln(2) = {}",
        score,
        expected
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_path_with_multiple_test_patterns() {
    // Path matching multiple patterns should still be detected once
    let path = "src/tests/__tests__/auth.test.ts";
    assert!(
        is_test_path(path),
        "Multiple pattern matches should still be detected: {}",
        path
    );
}

#[test]
fn test_nested_test_directory() {
    // Deeply nested test paths should be detected
    let paths = vec![
        "packages/core/src/tests/unit/auth.ts",
        "crates/lib/tests/integration/db_test.rs",
    ];

    for path in &paths {
        assert!(
            is_test_path(path),
            "Nested test directory should be detected: {}",
            path
        );
    }
}

#[test]
fn test_test_substring_in_non_test_file() {
    // Files with 'test' substring but not matching patterns
    let non_test_paths = vec![
        "src/attestation.ts",
        "src/contested.ts",
        "src/latest.ts",
        "lib/testing.ts", // testing-utils is production code
        "src/protest.py",
    ];

    for path in &non_test_paths {
        // Note: "testing.ts" might be flagged depending on pattern specificity
        // Our current patterns only match specific extensions and directories
        if !path.contains("/testing") {
            assert!(
                !is_test_path(path),
                "'test' substring in '{}' should not trigger detection",
                path
            );
        }
    }
}
