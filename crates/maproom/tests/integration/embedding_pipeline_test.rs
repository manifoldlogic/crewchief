//! Integration tests for the embedding generation pipeline.
//!
//! These tests require:
//! - A running PostgreSQL database with pgvector extension
//! - OPENAI_API_KEY environment variable set
//! - DATABASE_URL environment variable set
//!
//! Run with: cargo test --test embedding_pipeline_test -- --nocapture

use crewchief_maproom::embedding::{
    EmbeddingConfig, EmbeddingPipeline, EmbeddingService, PipelineConfig,
};
use crewchief_maproom::db;

#[tokio::test]
#[ignore] // Ignore by default since it requires API key and database
async fn test_embedding_pipeline_incremental() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Connect to database
    let client = db::connect().await.expect("Failed to connect to database");

    // Run migrations
    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Create embedding service
    let service = EmbeddingService::from_env().expect("Failed to create embedding service");

    // Configure pipeline for testing with small sample
    let config = PipelineConfig {
        batch_size: 10,
        incremental: true,
        dry_run: false,
        sample_size: Some(20), // Only process 20 chunks
        batch_delay_ms: 200,
        max_cost_usd: Some(0.10), // Limit cost to 10 cents
    };

    // Create and run pipeline
    let pipeline = EmbeddingPipeline::new(service, config);
    let stats = pipeline.run(&client).await.expect("Pipeline failed");

    // Verify stats
    assert!(stats.total_chunks <= 20);
    assert!(stats.embeddings_generated > 0 || stats.embeddings_cached > 0);
    assert_eq!(stats.failed_chunks, 0);
    assert!(stats.estimated_cost_usd <= 0.10);

    println!("Pipeline stats: {:#?}", stats);
    println!("{}", stats.summary());
}

#[tokio::test]
#[ignore] // Ignore by default since it requires API key and database
async fn test_embedding_pipeline_dry_run() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Connect to database
    let client = db::connect().await.expect("Failed to connect to database");

    // Create embedding service
    let service = EmbeddingService::from_env().expect("Failed to create embedding service");

    // Configure pipeline for dry run
    let config = PipelineConfig {
        batch_size: 10,
        incremental: true,
        dry_run: true, // Don't write to database
        sample_size: Some(10),
        batch_delay_ms: 100,
        max_cost_usd: None,
    };

    // Create and run pipeline
    let pipeline = EmbeddingPipeline::new(service, config);
    let stats = pipeline.run(&client).await.expect("Pipeline failed");

    // Verify stats (dry run should still generate embeddings but not write)
    assert!(stats.total_chunks <= 10);

    println!("Dry run stats: {:#?}", stats);
    println!("{}", stats.summary());
}

#[tokio::test]
#[ignore] // Ignore by default since it requires API key and database
async fn test_embedding_pipeline_cache_hit_rate() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Connect to database
    let client = db::connect().await.expect("Failed to connect to database");

    // Create embedding service
    let service = EmbeddingService::from_env().expect("Failed to create embedding service");

    // First run: generate embeddings (cold cache)
    let config1 = PipelineConfig {
        batch_size: 10,
        incremental: false,
        dry_run: true,
        sample_size: Some(10),
        batch_delay_ms: 100,
        max_cost_usd: None,
    };

    let pipeline1 = EmbeddingPipeline::new(service.clone(), config1);
    let stats1 = pipeline1.run(&client).await.expect("First run failed");

    println!("First run (cold cache): {}", stats1.summary());

    // Second run: should have high cache hit rate
    let config2 = PipelineConfig {
        batch_size: 10,
        incremental: false,
        dry_run: true,
        sample_size: Some(10),
        batch_delay_ms: 100,
        max_cost_usd: None,
    };

    let pipeline2 = EmbeddingPipeline::new(service, config2);
    let stats2 = pipeline2.run(&client).await.expect("Second run failed");

    println!("Second run (warm cache): {}", stats2.summary());

    // Cache hit rate should be very high on second run
    assert!(
        stats2.cache_hit_rate > 0.8,
        "Expected cache hit rate >80%, got {:.1}%",
        stats2.cache_hit_rate * 100.0
    );
}

#[tokio::test]
#[ignore] // Ignore by default since it requires database
async fn test_embedding_pipeline_no_chunks() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Connect to database
    let client = db::connect().await.expect("Failed to connect to database");

    // Create a mock embedding config (won't actually make API calls)
    let mut config = EmbeddingConfig::default();
    config.api_key = Some("test-key".to_string());

    let service = EmbeddingService::new(config).expect("Failed to create service");

    // Configure pipeline to process 0 chunks (sample_size = 0 not allowed, so use incremental on empty table)
    let pipeline_config = PipelineConfig {
        batch_size: 10,
        incremental: true,
        dry_run: true,
        sample_size: Some(0), // This will result in 0 chunks
        batch_delay_ms: 100,
        max_cost_usd: None,
    };

    let pipeline = EmbeddingPipeline::new(service, pipeline_config);
    let stats = pipeline.run(&client).await.expect("Pipeline failed");

    // Should handle 0 chunks gracefully
    assert_eq!(stats.total_chunks, 0);
    assert_eq!(stats.embeddings_generated, 0);
    assert_eq!(stats.embeddings_cached, 0);
}

#[test]
fn test_pipeline_config_defaults() {
    let config = PipelineConfig::default();

    assert_eq!(config.batch_size, 100);
    assert!(config.incremental);
    assert!(!config.dry_run);
    assert_eq!(config.sample_size, None);
    assert_eq!(config.batch_delay_ms, 100);
    assert_eq!(config.max_cost_usd, None);
}

#[test]
fn test_pipeline_stats_calculations() {
    let stats = crewchief_maproom::embedding::PipelineStats {
        total_chunks: 1000,
        embeddings_generated: 200,
        embeddings_cached: 800,
        failed_chunks: 0,
        api_calls: 10,
        total_tokens: 50000,
        estimated_cost_usd: 1.0,
        cache_hit_rate: 0.8,
        duration_secs: 10.0,
    };

    assert_eq!(stats.chunks_per_second(), 100.0);

    let summary = stats.summary();
    assert!(summary.contains("1000 chunks"));
    assert!(summary.contains("10.0s"));
    assert!(summary.contains("200"));
    assert!(summary.contains("800"));
    assert!(summary.contains("80.0%"));
}
