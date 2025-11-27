//! Integration tests for configuration management.
//!
//! Tests the complete configuration system including:
//! - Load from maproom-search.yml
//! - Hot reload of fusion weights without restart
//! - Feature flag toggling (enable_fts, enable_vector, enable_hybrid)
//! - Environment variable overrides
//! - Configuration validation and error handling
//! - Default fallback behavior

use std::time::Duration;
use tokio::time::sleep;

#[path = "../common/mod.rs"]
mod common;
use common::TestConfig;
use crewchief_maproom::config::{SearchConfig, FeatureFlags};
use crewchief_maproom::search::fusion::FusionWeights;

#[tokio::test]
async fn test_config_load_from_file() {
    // Setup: Create test configuration file
    let test_config = TestConfig::new().expect("Failed to create test config");

    let config_content = r#"
embedding:
  provider: "openai"
  model_name: "text-embedding-3-small"
  dimension: 1536
  cache_size: 5000
  cache_ttl_seconds: 1800

fusion:
  method: "rrf"
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.4
    graph: 0.1
    recency: 0.05
    churn: 0.05

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
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    let config_path = test_config
        .write_config("maproom-search.yml", config_content)
        .expect("Failed to write config");

    // Test: Load configuration from file
    let config = SearchConfig::load_from_file(&config_path)
        .await
        .expect("Failed to load config");

    // Assertions
    assert_eq!(config.embedding.provider, "openai");
    assert_eq!(config.embedding.model_name, "text-embedding-3-small");
    assert_eq!(config.embedding.dimension, 1536);
    assert_eq!(config.embedding.cache_size, 5000);

    assert_eq!(config.fusion.rrf_k, 60);
    assert_eq!(config.fusion.weights.fts, 0.4);
    assert_eq!(config.fusion.weights.vector, 0.4);
    assert_eq!(config.fusion.weights.graph, 0.1);

    assert_eq!(config.performance.max_candidates_per_method, 100);
    assert_eq!(config.performance.final_result_limit, 20);
    assert_eq!(config.performance.timeout_ms, 1000);
    assert!(config.performance.parallel_execution);

    assert!(config.feature_flags.enable_vector_search);
    assert!(config.feature_flags.enable_hybrid_fusion);
    assert!(config.feature_flags.enable_hot_reload);
}

#[tokio::test]
async fn test_config_default_fallback() {
    // Test: Load default configuration when file doesn't exist
    let config = SearchConfig::default();

    // Assertions: Verify sensible defaults
    assert_eq!(config.embedding.provider, "openai");
    assert_eq!(config.embedding.dimension, 1536);
    assert!(config.embedding.cache_size > 0);

    assert!(config.fusion.rrf_k > 0);
    assert!(config.fusion.weights.fts > 0.0);
    assert!(config.fusion.weights.vector > 0.0);

    assert!(config.performance.max_candidates_per_method > 0);
    assert!(config.performance.final_result_limit > 0);

    assert!(config.feature_flags.enable_vector_search);
    assert!(config.feature_flags.enable_hybrid_fusion);

    // Validate configuration
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_config_env_overrides() {
    // Setup: Set environment variables
    std::env::set_var("MAPROOM_SEARCH_EMBEDDING_PROVIDER", "cohere");
    std::env::set_var("MAPROOM_SEARCH_EMBEDDING_DIMENSION", "768");
    std::env::set_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS", "0.5");
    std::env::set_var("MAPROOM_SEARCH_FUSION_WEIGHTS_VECTOR", "0.5");
    std::env::set_var("MAPROOM_SEARCH_PERFORMANCE_FINAL_RESULT_LIMIT", "30");

    // Create test configuration file
    let test_config = TestConfig::new().expect("Failed to create test config");
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
    fts: 0.4
    vector: 0.4
    graph: 0.1
    recency: 0.05
    churn: 0.05

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
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    let config_path = test_config
        .write_config("maproom-search.yml", config_content)
        .expect("Failed to write config");

    // Test: Load configuration with environment variable overrides
    let config = SearchConfig::load_from_file(&config_path)
        .await
        .expect("Failed to load config");

    // Assertions: Environment variables should override file values
    assert_eq!(config.embedding.provider, "cohere"); // Overridden
    assert_eq!(config.embedding.dimension, 768); // Overridden
    assert_eq!(config.fusion.weights.fts, 0.5); // Overridden
    assert_eq!(config.fusion.weights.vector, 0.5); // Overridden
    assert_eq!(config.performance.final_result_limit, 30); // Overridden

    // Cleanup
    std::env::remove_var("MAPROOM_SEARCH_EMBEDDING_PROVIDER");
    std::env::remove_var("MAPROOM_SEARCH_EMBEDDING_DIMENSION");
    std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS");
    std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_VECTOR");
    std::env::remove_var("MAPROOM_SEARCH_PERFORMANCE_FINAL_RESULT_LIMIT");
}

#[tokio::test]
async fn test_config_validation() {
    // Test: Valid configuration
    let valid_config = SearchConfig::default();
    assert!(valid_config.validate().is_ok());

    // Test: Invalid embedding dimension
    let mut invalid_config = SearchConfig::default();
    invalid_config.embedding.dimension = 0;
    assert!(invalid_config.validate().is_err());

    // Test: Invalid fusion weights (would be caught by validation)
    let mut invalid_fusion = SearchConfig::default();
    invalid_fusion.fusion.rrf_k = 0;
    assert!(invalid_fusion.validate().is_err());

    // Test: Invalid performance config
    let mut invalid_perf = SearchConfig::default();
    invalid_perf.performance.max_candidates_per_method = 0;
    assert!(invalid_perf.validate().is_err());

    // Test: Invalid index config
    let mut invalid_index = SearchConfig::default();
    invalid_index.index.ivfflat_lists = 0;
    assert!(invalid_index.validate().is_err());
}

#[tokio::test]
async fn test_config_hot_reload() {
    // Setup: Create initial configuration file
    let test_config = TestConfig::new().expect("Failed to create test config");

    let initial_config = r#"
embedding:
  provider: "openai"
  model_name: "text-embedding-3-small"
  dimension: 1536
  cache_size: 10000
  cache_ttl_seconds: 3600

fusion:
  method: "weighted"
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.4
    graph: 0.1
    recency: 0.05
    churn: 0.05

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
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    let config_path = test_config
        .write_config("maproom-search.yml", initial_config)
        .expect("Failed to write config");

    // Load initial configuration
    let config1 = SearchConfig::load_from_file(&config_path)
        .await
        .expect("Failed to load initial config");

    assert_eq!(config1.fusion.weights.fts, 0.4);
    assert_eq!(config1.fusion.weights.vector, 0.4);

    // Simulate hot reload: Update configuration file
    let updated_config = r#"
embedding:
  provider: "openai"
  model_name: "text-embedding-3-small"
  dimension: 1536
  cache_size: 10000
  cache_ttl_seconds: 3600

fusion:
  method: "weighted"
  rrf_k: 60
  weights:
    fts: 0.6
    vector: 0.3
    graph: 0.05
    recency: 0.025
    churn: 0.025

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
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    test_config
        .write_config("maproom-search.yml", updated_config)
        .expect("Failed to write updated config");

    // Small delay to ensure file system flush
    sleep(Duration::from_millis(100)).await;

    // Reload configuration
    let config2 = SearchConfig::load_from_file(&config_path)
        .await
        .expect("Failed to load updated config");

    // Assertions: Verify hot reload picked up new values
    assert_eq!(config2.fusion.weights.fts, 0.6);
    assert_eq!(config2.fusion.weights.vector, 0.3);
    assert_eq!(config2.fusion.weights.graph, 0.05);
}

#[tokio::test]
async fn test_feature_flags() {
    // Test: Feature flags default state
    let flags = FeatureFlags::default();
    assert!(flags.enable_vector_search);
    assert!(flags.enable_hybrid_fusion);
    assert!(flags.enable_graph_signals);
    assert!(flags.enable_temporal_signals);
    assert!(flags.enable_query_cache);

    // Test: Custom feature flags
    let custom_flags = FeatureFlags {
        enable_vector_search: false,
        enable_hybrid_fusion: false,
        enable_graph_signals: false,
        enable_temporal_signals: false,
        enable_query_cache: false,
        enable_hot_reload: false,
    };

    assert!(!custom_flags.enable_vector_search);
    assert!(!custom_flags.enable_hybrid_fusion);
    assert!(!custom_flags.enable_graph_signals);
}

#[tokio::test]
async fn test_feature_flag_env_override() {
    // Setup: Set environment variable to disable vector search
    std::env::set_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH", "false");
    std::env::set_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_HYBRID_FUSION", "false");

    // Create test configuration
    let test_config = TestConfig::new().expect("Failed to create test config");
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
    fts: 0.4
    vector: 0.4
    graph: 0.1
    recency: 0.05
    churn: 0.05

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
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
"#;

    let config_path = test_config
        .write_config("maproom-search.yml", config_content)
        .expect("Failed to write config");

    // Load configuration with environment overrides
    let config = SearchConfig::load_from_file(&config_path)
        .await
        .expect("Failed to load config");

    // Assertions: Environment variables should override file values
    assert!(!config.feature_flags.enable_vector_search); // Overridden to false
    assert!(!config.feature_flags.enable_hybrid_fusion); // Overridden to false
    assert!(config.feature_flags.enable_graph_signals); // Not overridden, remains true

    // Cleanup
    std::env::remove_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH");
    std::env::remove_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_HYBRID_FUSION");
}

#[tokio::test]
async fn test_fusion_weights_validation() {
    // Test: Valid fusion weights
    let valid_weights = FusionWeights {
        fts: 0.4,
        vector: 0.4,
        graph: 0.1,
        recency: 0.05,
        churn: 0.05,
    };
    assert!(valid_weights.validate().is_ok());

    // Test: Negative weights (should fail validation)
    let invalid_weights = FusionWeights {
        fts: -0.1,
        vector: 0.4,
        graph: 0.1,
        recency: 0.05,
        churn: 0.05,
    };
    assert!(invalid_weights.validate().is_err());

    // Test: All zero weights (should fail validation)
    let zero_weights = FusionWeights {
        fts: 0.0,
        vector: 0.0,
        graph: 0.0,
        recency: 0.0,
        churn: 0.0,
    };
    assert!(zero_weights.validate().is_err());
}

#[tokio::test]
async fn test_config_yaml_parsing_errors() {
    // Setup: Create invalid YAML configuration
    let test_config = TestConfig::new().expect("Failed to create test config");

    let invalid_yaml = r#"
embedding:
  provider: "openai"
  dimension: not_a_number
  cache_size: 10000
"#;

    let config_path = test_config
        .write_config("invalid.yml", invalid_yaml)
        .expect("Failed to write config");

    // Test: Loading invalid YAML should fail
    let result = SearchConfig::load_from_file(&config_path).await;
    assert!(result.is_err(), "Expected error when loading invalid YAML");
}

#[tokio::test]
async fn test_config_missing_required_fields() {
    // Setup: Create configuration with missing required fields
    let test_config = TestConfig::new().expect("Failed to create test config");

    let incomplete_config = r#"
embedding:
  provider: "openai"
  # Missing dimension, model_name, etc.
"#;

    let config_path = test_config
        .write_config("incomplete.yml", incomplete_config)
        .expect("Failed to write config");

    // Test: Loading incomplete config should fail
    let result = SearchConfig::load_from_file(&config_path).await;
    assert!(result.is_err(), "Expected error when loading incomplete config");
}

#[tokio::test]
async fn test_config_get_env_overrides() {
    // Setup: Set some test environment variables
    std::env::set_var("MAPROOM_SEARCH_TEST_VAR_1", "value1");
    std::env::set_var("MAPROOM_SEARCH_TEST_VAR_2", "value2");

    // Test: Get all environment variable overrides
    let overrides = SearchConfig::get_env_overrides();

    // Assertions
    assert!(overrides.iter().any(|(k, _)| k == "MAPROOM_SEARCH_TEST_VAR_1"));
    assert!(overrides.iter().any(|(k, _)| k == "MAPROOM_SEARCH_TEST_VAR_2"));

    // All returned variables should start with MAPROOM_SEARCH_
    for (key, _) in &overrides {
        assert!(key.starts_with("MAPROOM_SEARCH_"));
    }

    // Cleanup
    std::env::remove_var("MAPROOM_SEARCH_TEST_VAR_1");
    std::env::remove_var("MAPROOM_SEARCH_TEST_VAR_2");
}
