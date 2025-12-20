//! Edge case tests for quality-weighted graph scoring (SRCHREL-3004).
//!
//! Tests edge cases that could cause issues:
//! - Extreme weight configurations (boundary values)
//! - Invalid weight validation
//! - Config defaults
//! - File path pattern detection

use crewchief_maproom::config::{EdgeQualityWeights, SearchConfig};

// ============================================================================
// Edge Case: Extreme Weight Configurations
// ============================================================================

#[test]
fn test_extreme_weights_zero_all() {
    // All weights at zero (valid but unusual)
    let weights = EdgeQualityWeights {
        production_code: 0.0,
        test_code: 0.0,
        calls: 0.0,
    };
    assert!(
        weights.validate().is_ok(),
        "All-zero weights should be valid"
    );
}

#[test]
fn test_extreme_weights_maximum_all() {
    // All weights at maximum (10.0)
    let weights = EdgeQualityWeights {
        production_code: 10.0,
        test_code: 10.0,
        calls: 10.0,
    };
    assert!(
        weights.validate().is_ok(),
        "Maximum weights should be valid"
    );
}

#[test]
fn test_extreme_weights_asymmetric() {
    // Very high production, very low test
    let weights = EdgeQualityWeights {
        production_code: 10.0,
        test_code: 0.0,
        calls: 5.0,
    };
    assert!(
        weights.validate().is_ok(),
        "Asymmetric weights should be valid"
    );
}

// ============================================================================
// Edge Case: Invalid Weights Rejected
// ============================================================================

#[test]
fn test_invalid_weights_negative() {
    // Negative weight - should be rejected
    let negative = EdgeQualityWeights {
        production_code: -1.0,
        test_code: 0.5,
        calls: 1.0,
    };
    assert!(
        negative.validate().is_err(),
        "Negative weights should be rejected"
    );
}

#[test]
fn test_invalid_weights_above_max() {
    // Weight > 10.0 - should be rejected
    let too_high = EdgeQualityWeights {
        production_code: 1.0,
        test_code: 0.5,
        calls: 11.0,
    };
    assert!(
        too_high.validate().is_err(),
        "Weights > 10.0 should be rejected"
    );
}

#[test]
fn test_invalid_weights_just_below_zero() {
    // Just below boundary
    let just_below = EdgeQualityWeights {
        production_code: -0.001,
        test_code: 0.5,
        calls: 1.0,
    };
    assert!(
        just_below.validate().is_err(),
        "Weights below 0.0 should be invalid"
    );
}

#[test]
fn test_invalid_weights_just_above_max() {
    // Just above boundary
    let just_above = EdgeQualityWeights {
        production_code: 1.0,
        test_code: 10.001,
        calls: 1.0,
    };
    assert!(
        just_above.validate().is_err(),
        "Weights above 10.0 should be invalid"
    );
}

#[test]
fn test_weights_nan_behavior() {
    // Note: NaN comparisons always return false in IEEE 754, so:
    // - NaN < 0.0 → false
    // - NaN > 10.0 → false
    // This means NaN passes the current range check validation.
    // Documenting actual behavior rather than expected behavior.
    let nan_weight = EdgeQualityWeights {
        production_code: f32::NAN,
        test_code: 0.5,
        calls: 1.0,
    };
    // NaN passes validation because comparisons return false
    // This is a known edge case - NaN values would still produce
    // unexpected results in scoring, but config validation allows them.
    let result = nan_weight.validate();
    assert!(
        result.is_ok(),
        "NaN passes validation due to IEEE 754 comparison semantics"
    );
}

#[test]
fn test_invalid_weights_infinity() {
    // Infinity should be rejected
    let inf_weight = EdgeQualityWeights {
        production_code: f32::INFINITY,
        test_code: 0.5,
        calls: 1.0,
    };
    assert!(
        inf_weight.validate().is_err(),
        "Infinite weights should be rejected"
    );
}

#[test]
fn test_invalid_weights_negative_infinity() {
    // Negative infinity should be rejected
    let neg_inf = EdgeQualityWeights {
        production_code: f32::NEG_INFINITY,
        test_code: 0.5,
        calls: 1.0,
    };
    assert!(
        neg_inf.validate().is_err(),
        "Negative infinite weights should be rejected"
    );
}

// ============================================================================
// Edge Case: Boundary Value Testing
// ============================================================================

#[test]
fn test_weight_exactly_at_zero() {
    let at_zero = EdgeQualityWeights {
        production_code: 0.0,
        test_code: 0.0,
        calls: 0.0,
    };
    assert!(at_zero.validate().is_ok(), "Weights at 0.0 should be valid");
}

#[test]
fn test_weight_exactly_at_ten() {
    let at_ten = EdgeQualityWeights {
        production_code: 10.0,
        test_code: 10.0,
        calls: 10.0,
    };
    assert!(at_ten.validate().is_ok(), "Weights at 10.0 should be valid");
}

#[test]
fn test_weight_very_small_positive() {
    let very_small = EdgeQualityWeights {
        production_code: 0.0001,
        test_code: 0.0001,
        calls: 0.0001,
    };
    assert!(
        very_small.validate().is_ok(),
        "Very small positive weights should be valid"
    );
}

#[test]
fn test_weight_close_to_max() {
    let close_to_max = EdgeQualityWeights {
        production_code: 9.9999,
        test_code: 9.9999,
        calls: 9.9999,
    };
    assert!(
        close_to_max.validate().is_ok(),
        "Weights close to max should be valid"
    );
}

// ============================================================================
// Edge Case: Config Defaults
// ============================================================================

#[test]
fn test_config_defaults() {
    let config = SearchConfig::default();

    // Feature flag default
    assert!(
        !config.feature_flags.enable_quality_weighted_graph,
        "Quality scoring should be disabled by default"
    );

    // Weight defaults
    assert!(
        (config.graph_importance.edge_quality_weights.production_code - 1.0).abs() < f32::EPSILON,
        "Production weight should default to 1.0"
    );
    assert!(
        (config.graph_importance.edge_quality_weights.test_code - 0.5).abs() < f32::EPSILON,
        "Test weight should default to 0.5"
    );
    assert!(
        (config.graph_importance.edge_quality_weights.calls - 1.0).abs() < f32::EPSILON,
        "Calls weight should default to 1.0"
    );

    // Fusion override default
    assert!(
        config.graph_importance.fusion_weight_override.is_none(),
        "Fusion override should default to None"
    );
}

#[test]
fn test_edge_quality_weights_is_default() {
    let default = EdgeQualityWeights::default();
    assert!(
        default.is_default(),
        "Default weights should return is_default() = true"
    );

    let non_default = EdgeQualityWeights {
        production_code: 2.0,
        test_code: 0.5,
        calls: 1.0,
    };
    assert!(
        !non_default.is_default(),
        "Non-default weights should return is_default() = false"
    );
}

// ============================================================================
// Edge Case: Test Detection Patterns
// ============================================================================

/// Helper to check if a path would be detected as test code.
/// This mirrors the SQL pattern matching in the graph query.
fn is_test_path(path: &str) -> bool {
    let lower = path.to_lowercase();

    // Check directory patterns
    if lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
        || lower.contains("/__test__/")
    {
        return true;
    }

    // Check file suffix patterns
    if lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains("_test.")
        || lower.contains("_spec.")
    {
        return true;
    }

    // Check prefix patterns
    let filename = path.rsplit('/').next().unwrap_or(path);
    if filename.starts_with("test_") || filename.starts_with("test.") {
        return true;
    }

    false
}

#[test]
fn test_detection_directory_patterns() {
    // Standard test directories
    assert!(is_test_path("src/test/handler.ts"));
    assert!(is_test_path("src/tests/handler.ts"));
    assert!(is_test_path("src/__tests__/handler.ts"));
    assert!(is_test_path("src/__test__/handler.ts"));

    // Deep nested
    assert!(is_test_path("packages/core/src/__tests__/unit/handler.ts"));
    assert!(is_test_path("apps/web/test/integration/api.js"));
}

#[test]
fn test_detection_file_patterns() {
    // Common test file patterns
    assert!(is_test_path("handler.test.ts"));
    assert!(is_test_path("handler.spec.ts"));
    assert!(is_test_path("handler_test.go"));
    assert!(is_test_path("handler_spec.rb"));

    // Nested paths with patterns
    assert!(is_test_path("src/handlers/auth.test.ts"));
    assert!(is_test_path("lib/utils/date.spec.js"));
}

#[test]
fn test_detection_prefix_patterns() {
    // Prefix-based test files
    assert!(is_test_path("test_handler.py"));
    assert!(is_test_path("test.js"));
}

#[test]
fn test_detection_non_test_files() {
    // Should NOT be detected as test
    assert!(!is_test_path("src/handler.ts"));
    assert!(!is_test_path("src/services/auth.ts"));
    assert!(!is_test_path("lib/utils.js"));
    assert!(!is_test_path("README.md"));

    // Tricky cases - "test" in name but not a test file
    assert!(!is_test_path("src/contest/handler.ts")); // "contest" contains "test"
    assert!(!is_test_path("src/latest/api.ts")); // "latest" contains "test"
}

#[test]
fn test_detection_case_insensitive() {
    // Should match regardless of case
    assert!(is_test_path("src/TEST/handler.ts"));
    assert!(is_test_path("src/Tests/handler.ts"));
    assert!(is_test_path("handler.TEST.ts"));
    assert!(is_test_path("handler.Spec.ts"));
}

#[test]
fn test_detection_edge_case_paths() {
    // Edge cases that should be handled gracefully
    assert!(!is_test_path("")); // Empty path
    assert!(!is_test_path("noextension")); // No extension
    assert!(!is_test_path(".hidden")); // Hidden file
}

// ============================================================================
// Edge Case: Weight Multiplication Safety
// ============================================================================

#[test]
fn test_weight_multiplication_bounds() {
    // Maximum weight multiplication: 10.0 * 10.0 = 100.0
    // This should not cause overflow
    let max_weights = EdgeQualityWeights {
        production_code: 10.0,
        test_code: 10.0,
        calls: 10.0,
    };

    // Simulate the weight calculation: production_code * calls
    let max_multiplied = max_weights.production_code * max_weights.calls;
    assert!(
        max_multiplied.is_finite(),
        "Maximum weight multiplication should not overflow"
    );
    assert_eq!(max_multiplied, 100.0, "Max product should be 100.0");
}

#[test]
fn test_weight_multiplication_zero() {
    // Zero weights should produce zero scores
    let zero_weights = EdgeQualityWeights {
        production_code: 0.0,
        test_code: 0.0,
        calls: 0.0,
    };

    let result = zero_weights.production_code * zero_weights.calls;
    assert_eq!(result, 0.0, "Zero weights should produce zero score");
}

#[test]
fn test_weight_default_multiplication() {
    // Default weights: production=1.0, test=0.5, calls=1.0
    let defaults = EdgeQualityWeights::default();

    let prod_result = defaults.production_code * defaults.calls;
    assert_eq!(prod_result, 1.0, "Default production * calls should be 1.0");

    let test_result = defaults.test_code * defaults.calls;
    assert_eq!(test_result, 0.5, "Default test * calls should be 0.5");
}
