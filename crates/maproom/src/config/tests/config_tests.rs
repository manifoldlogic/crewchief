//! Tests for configuration loading and validation.

use crate::config::{
    EmbeddingConfig, FusionConfig, FusionMethod, IndexConfig, PerformanceConfig, SearchConfig,
};
use crate::search::fusion::FusionWeights;
use std::io::Write;
use std::sync::Mutex;
use tempfile::NamedTempFile;

// Mutex to serialize tests that modify environment variables
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_load_valid_yaml_config() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: text-embedding-3-small
  dimension: 1536
  cache_size: 10000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = SearchConfig::load_from_file(temp_file.path())
        .await
        .unwrap();

    assert_eq!(config.embedding.provider, "openai");
    assert_eq!(config.embedding.dimension, 1536);
    assert_eq!(config.fusion.method, FusionMethod::RRF);
    assert_eq!(config.fusion.weights.fts, 0.4);
    assert!(config.feature_flags.enable_vector_search);
}

#[tokio::test]
async fn test_load_invalid_yaml_syntax() {
    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  invalid yaml syntax here
"#;
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = SearchConfig::load_from_file(temp_file.path()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_missing_file() {
    let result = SearchConfig::load_from_file(std::path::Path::new("nonexistent.yml")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_env_override_fusion_weights() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Set environment variable override
    std::env::set_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS", "0.6");

    let config = SearchConfig::load_from_file(temp_file.path())
        .await
        .unwrap();

    assert_eq!(config.fusion.weights.fts, 0.6);
    assert_eq!(config.fusion.weights.vector, 0.35); // Unchanged

    // Clean up
    std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS");
}

#[tokio::test]
async fn test_env_override_performance() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Set environment variable overrides
    std::env::set_var("MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS", "500");
    std::env::set_var("MAPROOM_SEARCH_PERFORMANCE_PARALLEL_EXECUTION", "false");

    let config = SearchConfig::load_from_file(temp_file.path())
        .await
        .unwrap();

    assert_eq!(config.performance.timeout_ms, 500);
    assert!(!config.performance.parallel_execution);

    // Clean up
    std::env::remove_var("MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS");
    std::env::remove_var("MAPROOM_SEARCH_PERFORMANCE_PARALLEL_EXECUTION");
}

#[tokio::test]
async fn test_env_override_feature_flags() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Set environment variable override
    std::env::set_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH", "false");

    let config = SearchConfig::load_from_file(temp_file.path())
        .await
        .unwrap();

    assert!(!config.feature_flags.enable_vector_search);
    assert!(config.feature_flags.enable_hybrid_fusion); // Unchanged

    // Clean up
    std::env::remove_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH");
}

#[test]
fn test_fusion_method_from_str() {
    assert_eq!(FusionMethod::from_str("rrf").unwrap(), FusionMethod::RRF);
    assert_eq!(
        FusionMethod::from_str("weighted").unwrap(),
        FusionMethod::Weighted
    );
    assert_eq!(
        FusionMethod::from_str("learned").unwrap(),
        FusionMethod::Learned
    );

    // Case insensitive
    assert_eq!(FusionMethod::from_str("RRF").unwrap(), FusionMethod::RRF);
    assert_eq!(
        FusionMethod::from_str("WEIGHTED").unwrap(),
        FusionMethod::Weighted
    );

    // Invalid
    assert!(FusionMethod::from_str("invalid").is_err());
}

#[tokio::test]
async fn test_validation_negative_weights() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: -0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = SearchConfig::load_from_file(temp_file.path()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validation_zero_dimension() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 0
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = SearchConfig::load_from_file(temp_file.path()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validation_zero_rrf_k() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 0
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = SearchConfig::load_from_file(temp_file.path()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validation_zero_performance_limits() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.4
    vector: 0.35
    graph: 0.1
    recency: 0.1
    churn: 0.05

performance:
  max_candidates_per_method: 0
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
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = SearchConfig::load_from_file(temp_file.path()).await;
    assert!(result.is_err());
}

#[test]
fn test_default_config_is_valid() {
    let config = SearchConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_embedding_config_validation() {
    let mut config = EmbeddingConfig::default();
    assert!(config.validate().is_ok());

    config.provider = "".to_string();
    assert!(config.validate().is_err());

    config = EmbeddingConfig::default();
    config.model_name = "".to_string();
    assert!(config.validate().is_err());

    config = EmbeddingConfig::default();
    config.dimension = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_fusion_config_validation() {
    let mut config = FusionConfig::default();
    assert!(config.validate().is_ok());

    config.rrf_k = 0;
    assert!(config.validate().is_err());

    config = FusionConfig::default();
    config.weights = FusionWeights::new(-0.1, 0.35, 0.1, 0.1, 0.05);
    assert!(config.validate().is_err());
}

#[test]
fn test_performance_config_validation() {
    let mut config = PerformanceConfig::default();
    assert!(config.validate().is_ok());

    config.max_candidates_per_method = 0;
    assert!(config.validate().is_err());

    config = PerformanceConfig::default();
    config.final_result_limit = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_index_config_validation() {
    let mut config = IndexConfig::default();
    assert!(config.validate().is_ok());

    config.ivfflat_lists = 0;
    assert!(config.validate().is_err());

    config = IndexConfig::default();
    config.ivfflat_probes = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_complete_config_with_all_overrides() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut temp_file = NamedTempFile::new().unwrap();
    let yaml = r#"
embedding:
  provider: openai
  model_name: base-model
  dimension: 1000
  cache_size: 5000
  cache_ttl_seconds: 1800

fusion:
  method: weighted
  rrf_k: 30
  weights:
    fts: 0.3
    vector: 0.3
    graph: 0.2
    recency: 0.15
    churn: 0.05

performance:
  max_candidates_per_method: 50
  final_result_limit: 10
  timeout_ms: 500
  parallel_execution: false

index:
  ivfflat_lists: 50
  ivfflat_probes: 5
  refresh_interval_seconds: 1800

feature_flags:
  enable_vector_search: false
  enable_hybrid_fusion: false
  enable_graph_signals: false
  enable_temporal_signals: false
  enable_query_cache: false
  enable_hot_reload: false
"#;
    temp_file.write_all(yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Set multiple overrides
    std::env::set_var("MAPROOM_SEARCH_EMBEDDING_DIMENSION", "2000");
    std::env::set_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS", "0.5");
    std::env::set_var("MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS", "2000");
    std::env::set_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH", "true");

    let config = SearchConfig::load_from_file(temp_file.path())
        .await
        .unwrap();

    // Check overrides applied
    assert_eq!(config.embedding.dimension, 2000);
    assert_eq!(config.fusion.weights.fts, 0.5);
    assert_eq!(config.performance.timeout_ms, 2000);
    assert!(config.feature_flags.enable_vector_search);

    // Check non-overridden values
    assert_eq!(config.embedding.provider, "openai");
    assert_eq!(config.fusion.weights.vector, 0.3);

    // Clean up
    std::env::remove_var("MAPROOM_SEARCH_EMBEDDING_DIMENSION");
    std::env::remove_var("MAPROOM_SEARCH_FUSION_WEIGHTS_FTS");
    std::env::remove_var("MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS");
    std::env::remove_var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH");
}
