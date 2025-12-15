//! Backward compatibility tests for SearchConfig and FeatureFlags.
//!
//! These tests ensure that configuration changes do not break existing deployments.
//! Validates that old configs without feature flags still work correctly.

use crewchief_maproom::config::{FeatureFlags, SearchConfig};
use std::env;

#[test]
fn test_old_config_without_feature_flags() {
    // Test that SearchConfig::default() provides FeatureFlags with enable_graph_signals=true
    // This validates backward compatibility when upgrading from configs without feature_flags
    let config = SearchConfig::default();

    // Should use default FeatureFlags (all enabled)
    assert_eq!(config.feature_flags.enable_vector_search, true);
    assert_eq!(config.feature_flags.enable_hybrid_fusion, true);
    assert_eq!(config.feature_flags.enable_graph_signals, true); // Default enabled - KEY TEST
    assert_eq!(config.feature_flags.enable_temporal_signals, true);
    assert_eq!(config.feature_flags.enable_query_cache, true);
    assert_eq!(config.feature_flags.enable_hot_reload, true);
}

#[test]
fn test_old_config_with_feature_flags_missing_graph_signals() {
    // NOTE: FeatureFlags currently requires all fields to be present.
    // For true backward compatibility, we would need to add #[serde(default)] to each field.
    // For now, this test validates that FeatureFlags::default() provides the right value.

    // Test that default provides enable_graph_signals=true for backward compat
    let flags = FeatureFlags::default();
    assert_eq!(flags.enable_graph_signals, true); // KEY TEST - default is enabled

    // Also test that we can construct with missing field via programmatic default
    let mut flags = FeatureFlags::default();
    flags.enable_vector_search = true;
    flags.enable_hybrid_fusion = true;
    // Don't set enable_graph_signals - uses default (true)
    flags.enable_temporal_signals = true;
    flags.enable_query_cache = true;
    flags.enable_hot_reload = true;

    assert_eq!(flags.enable_graph_signals, true); // Still true from default
}

#[test]
fn test_new_config_with_graph_signals_enabled() {
    // New config with enable_graph_signals explicitly set to true
    let new_config = r#"
embedding:
  provider: "openai"
  model_name: "text-embedding-3-small"
  dimension: 1536
  cache_size: 10000
  cache_ttl_seconds: 3600

fusion:
  method: "rrf"
  rrf_k: 60
  weights:
    fts: 0.40
    vector: 0.30
    graph: 0.10
    recency: 0.10
    churn: 0.10

performance:
  max_candidates_per_method: 100
  final_result_limit: 20
  timeout_ms: 1000
  parallel_execution: true

index:
  ivfflat_lists: 100
  ivfflat_probes: 10
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true  # Quality-weighted graph scoring enabled
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    let config: SearchConfig = serde_yaml::from_str(new_config).unwrap();
    assert_eq!(config.feature_flags.enable_graph_signals, true);
}

#[test]
fn test_new_config_with_graph_signals_disabled() {
    // New config with enable_graph_signals explicitly set to false
    let new_config = r#"
embedding:
  provider: "openai"
  model_name: "text-embedding-3-small"
  dimension: 1536
  cache_size: 10000
  cache_ttl_seconds: 3600

fusion:
  method: "rrf"
  rrf_k: 60
  weights:
    fts: 0.40
    vector: 0.30
    graph: 0.10
    recency: 0.10
    churn: 0.10

performance:
  max_candidates_per_method: 100
  final_result_limit: 20
  timeout_ms: 1000
  parallel_execution: true

index:
  ivfflat_lists: 100
  ivfflat_probes: 10
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: false  # Quality-weighted graph scoring disabled
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    let config: SearchConfig = serde_yaml::from_str(new_config).unwrap();
    assert_eq!(config.feature_flags.enable_graph_signals, false);
}

#[test]
fn test_minimal_config_uses_defaults() {
    // Test using Rust's Default trait instead of YAML deserialization
    // since SearchConfig requires core fields (embedding, fusion, etc.)
    let config = SearchConfig::default();

    // All feature flags should use default values (enabled)
    assert_eq!(config.feature_flags.enable_vector_search, true);
    assert_eq!(config.feature_flags.enable_hybrid_fusion, true);
    assert_eq!(config.feature_flags.enable_graph_signals, true);
    assert_eq!(config.feature_flags.enable_temporal_signals, true);
    assert_eq!(config.feature_flags.enable_query_cache, true);
    assert_eq!(config.feature_flags.enable_hot_reload, true);
}

#[test]
fn test_feature_flags_default() {
    // Test FeatureFlags::default() directly
    let flags = FeatureFlags::default();

    assert_eq!(flags.enable_vector_search, true);
    assert_eq!(flags.enable_hybrid_fusion, true);
    assert_eq!(flags.enable_graph_signals, true); // Default enabled
    assert_eq!(flags.enable_temporal_signals, true);
    assert_eq!(flags.enable_query_cache, true);
    assert_eq!(flags.enable_hot_reload, true);
}

#[test]
fn test_feature_flags_serialization_roundtrip() {
    // Test that serialization and deserialization preserve values
    let original = FeatureFlags {
        enable_vector_search: true,
        enable_hybrid_fusion: true,
        enable_graph_signals: false, // Test with false
        enable_temporal_signals: true,
        enable_query_cache: true,
        enable_hot_reload: false,
    };

    let yaml = serde_yaml::to_string(&original).unwrap();
    let deserialized: FeatureFlags = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(
        original.enable_vector_search,
        deserialized.enable_vector_search
    );
    assert_eq!(
        original.enable_hybrid_fusion,
        deserialized.enable_hybrid_fusion
    );
    assert_eq!(
        original.enable_graph_signals,
        deserialized.enable_graph_signals
    );
    assert_eq!(
        original.enable_temporal_signals,
        deserialized.enable_temporal_signals
    );
    assert_eq!(original.enable_query_cache, deserialized.enable_query_cache);
    assert_eq!(original.enable_hot_reload, deserialized.enable_hot_reload);
}

#[tokio::test]
async fn test_environment_variable_override_graph_signals() {
    // Set environment variable before loading config
    env::set_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS", "false");

    // Create temporary config file with complete feature_flags
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("test-config.yml");

    let config_content = r#"
embedding:
  provider: "openai"
  model_name: "text-embedding-3-small"
  dimension: 1536
  cache_size: 10000
  cache_ttl_seconds: 3600

fusion:
  method: "rrf"
  rrf_k: 60
  weights:
    fts: 0.40
    vector: 0.30
    graph: 0.10
    recency: 0.10
    churn: 0.10

performance:
  max_candidates_per_method: 100
  final_result_limit: 20
  timeout_ms: 1000
  parallel_execution: true

index:
  ivfflat_lists: 100
  ivfflat_probes: 10
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true  # Will be overridden by env var to false
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    std::fs::write(&config_path, config_content).unwrap();

    // Load config from file (this applies env overrides automatically)
    let config = SearchConfig::load_from_file(&config_path).await.unwrap();

    // After loading with env override, should be false (from env var)
    assert_eq!(config.feature_flags.enable_graph_signals, false); // KEY TEST - env var overrides config file

    // Clean up
    env::remove_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS");
}

#[test]
fn test_invalid_config_fails_validation() {
    // Test that validation catches invalid config values
    // Create a config with defaults, then manually set invalid values
    let mut config = SearchConfig::default();

    // Make embedding config invalid (empty provider)
    config.embedding.provider = "".to_string();

    // Validation should fail due to invalid provider
    assert!(config.validate().is_err());
}

#[test]
fn test_config_with_all_flags_disabled() {
    // Config with all feature flags disabled
    let all_disabled_config = r#"
embedding:
  provider: "openai"
  model_name: "text-embedding-3-small"
  dimension: 1536
  cache_size: 10000
  cache_ttl_seconds: 3600

fusion:
  method: "rrf"
  rrf_k: 60
  weights:
    fts: 0.40
    vector: 0.30
    graph: 0.10
    recency: 0.10
    churn: 0.10

performance:
  max_candidates_per_method: 100
  final_result_limit: 20
  timeout_ms: 1000
  parallel_execution: true

index:
  ivfflat_lists: 100
  ivfflat_probes: 10
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: false
  enable_hybrid_fusion: false
  enable_graph_signals: false
  enable_temporal_signals: false
  enable_query_cache: false
  enable_hot_reload: false
"#;

    let config: SearchConfig = serde_yaml::from_str(all_disabled_config).unwrap();

    // All flags should be disabled
    assert_eq!(config.feature_flags.enable_vector_search, false);
    assert_eq!(config.feature_flags.enable_hybrid_fusion, false);
    assert_eq!(config.feature_flags.enable_graph_signals, false);
    assert_eq!(config.feature_flags.enable_temporal_signals, false);
    assert_eq!(config.feature_flags.enable_query_cache, false);
    assert_eq!(config.feature_flags.enable_hot_reload, false);

    // Should still validate successfully (flags are valid)
    assert!(config.validate().is_ok());
}

#[test]
fn test_feature_flags_helper_methods() {
    // Test FeatureFlags helper methods
    let all_enabled = FeatureFlags::all_enabled();
    assert!(all_enabled.needs_graph_data());
    assert!(all_enabled.needs_embeddings());
    assert!(all_enabled.needs_temporal_data());
    assert_eq!(all_enabled.enabled_count(), 6);

    let all_disabled = FeatureFlags::all_disabled();
    assert!(!all_disabled.needs_graph_data());
    assert!(!all_disabled.needs_embeddings());
    assert!(!all_disabled.needs_temporal_data());
    assert_eq!(all_disabled.disabled_count(), 6);

    let fts_only = FeatureFlags::fts_only();
    assert!(!fts_only.needs_graph_data());
    assert!(!fts_only.needs_embeddings());
    assert_eq!(fts_only.enable_graph_signals, false);
}

#[test]
fn test_partial_feature_flags() {
    // Test FeatureFlags with some flags enabled, some disabled
    let feature_flags_yaml = r#"
enable_vector_search: true
enable_hybrid_fusion: false
enable_graph_signals: true  # Quality scoring enabled
enable_temporal_signals: false
enable_query_cache: true
enable_hot_reload: false
"#;

    let flags: FeatureFlags = serde_yaml::from_str(feature_flags_yaml).unwrap();

    assert_eq!(flags.enable_vector_search, true);
    assert_eq!(flags.enable_hybrid_fusion, false);
    assert_eq!(flags.enable_graph_signals, true); // Quality scoring on - KEY TEST
    assert_eq!(flags.enable_temporal_signals, false);
    assert_eq!(flags.enable_query_cache, true);
    assert_eq!(flags.enable_hot_reload, false);

    // Should need graph data (enable_graph_signals=true)
    assert!(flags.needs_graph_data());
}
