//! Tests for hot reload functionality.

use crate::config::{ConfigReloader, SearchConfig};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;
use tokio::sync::RwLock;

// Mutex to serialize tests that modify or load configuration
static CONFIG_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_config_reloader_creation() {
    let config = Arc::new(RwLock::new(SearchConfig::default()));
    let temp_file = NamedTempFile::new().unwrap();

    let reloader = ConfigReloader::new(config, temp_file.path());
    assert!(reloader.is_ok());
}

#[tokio::test]
async fn test_config_reloader_with_nonexistent_file() {
    let config = Arc::new(RwLock::new(SearchConfig::default()));

    // Creating reloader with non-existent file should succeed (with warning)
    let reloader = ConfigReloader::new(config, "nonexistent-file.yml");
    assert!(reloader.is_ok());
}

#[tokio::test]
async fn test_manual_reload_updates_weights() {
    let _guard = CONFIG_MUTEX.lock().unwrap();

    let config = Arc::new(RwLock::new(SearchConfig::default()));
    let mut temp_file = NamedTempFile::new().unwrap();

    // Write initial config with different weights
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
    fts: 0.5
    vector: 0.3
    graph: 0.1
    recency: 0.08
    churn: 0.02

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

    let reloader = ConfigReloader::new(config.clone(), temp_file.path()).unwrap();

    // Initial state
    {
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.weights.fts, 0.4); // Default
    }

    // Reload configuration
    reloader.reload().await.unwrap();

    // Check that weights were updated
    {
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.weights.fts, 0.5);
        assert_eq!(cfg.fusion.weights.vector, 0.3);
        assert_eq!(cfg.fusion.weights.graph, 0.1);
    }
}

#[tokio::test]
async fn test_hot_reload_preserves_valid_config_on_error() {
    let _guard = CONFIG_MUTEX.lock().unwrap();

    let config = Arc::new(RwLock::new(SearchConfig::default()));
    let mut temp_file = NamedTempFile::new().unwrap();

    // Write valid config first
    let valid_yaml = r#"
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
    fts: 0.5
    vector: 0.3
    graph: 0.1
    recency: 0.08
    churn: 0.02

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
    temp_file.write_all(valid_yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let reloader = ConfigReloader::new(config.clone(), temp_file.path()).unwrap();

    // Load valid config
    reloader.reload().await.unwrap();

    {
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.weights.fts, 0.5);
    }

    // Now write invalid config (negative weight)
    let invalid_yaml = r#"
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
    fts: -0.5
    vector: 0.3
    graph: 0.1
    recency: 0.08
    churn: 0.02

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
    temp_file.write_all(invalid_yaml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Reload should fail
    let result = reloader.reload().await;
    assert!(result.is_err());

    // Original valid config should be preserved
    {
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.weights.fts, 0.5); // Still the valid value
    }
}

#[tokio::test]
async fn test_reload_updates_rrf_k() {
    let _guard = CONFIG_MUTEX.lock().unwrap();

    let config = Arc::new(RwLock::new(SearchConfig::default()));
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
  rrf_k: 100
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

    let reloader = ConfigReloader::new(config.clone(), temp_file.path()).unwrap();

    // Initial state
    {
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.rrf_k, 60); // Default
    }

    // Reload configuration
    reloader.reload().await.unwrap();

    // Check that rrf_k was updated
    {
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.rrf_k, 100);
    }
}

#[tokio::test]
async fn test_reload_updates_fusion_method() {
    let _guard = CONFIG_MUTEX.lock().unwrap();

    let config = Arc::new(RwLock::new(SearchConfig::default()));
    let mut temp_file = NamedTempFile::new().unwrap();

    let yaml = r#"
embedding:
  provider: openai
  model_name: test-model
  dimension: 1536
  cache_size: 1000
  cache_ttl_seconds: 3600

fusion:
  method: weighted
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

    let reloader = ConfigReloader::new(config.clone(), temp_file.path()).unwrap();

    // Reload configuration
    reloader.reload().await.unwrap();

    // Check that fusion method was updated
    {
        let cfg = config.read().await;
        assert_eq!(cfg.fusion.method, crate::config::FusionMethod::Weighted);
    }
}

#[tokio::test]
async fn test_concurrent_reads_during_reload() {
    let _guard = CONFIG_MUTEX.lock().unwrap();

    use std::time::Duration;

    let config = Arc::new(RwLock::new(SearchConfig::default()));
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
    fts: 0.5
    vector: 0.3
    graph: 0.1
    recency: 0.08
    churn: 0.02

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

    let reloader = ConfigReloader::new(config.clone(), temp_file.path()).unwrap();

    // Spawn multiple readers
    let mut handles = vec![];
    for _ in 0..10 {
        let cfg = config.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                let _c = cfg.read().await;
                tokio::time::sleep(Duration::from_micros(1)).await;
            }
        });
        handles.push(handle);
    }

    // Perform reload while readers are active
    tokio::time::sleep(Duration::from_millis(10)).await;
    let reload_result = reloader.reload().await;

    // Wait for all readers to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Reload should succeed
    assert!(reload_result.is_ok());
}

#[test]
fn test_config_path() {
    let config = Arc::new(RwLock::new(SearchConfig::default()));
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let reloader = ConfigReloader::new(config, path).unwrap();
    assert_eq!(reloader.config_path(), path);
}
