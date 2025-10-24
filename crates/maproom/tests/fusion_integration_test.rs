//! Integration tests for fusion strategies.
//!
//! These tests verify both BasicWeightedFusion and RRFFusion work correctly
//! in the complete search pipeline with real database queries.
//!
//! # Test Requirements
//!
//! - PostgreSQL database running with maproom schema
//! - Sample data indexed (at least a few files with chunks)
//! - Embedding service configured (for vector search)
//!
//! Run with:
//! ```bash
//! cargo test --test fusion_integration_test -- --nocapture
//! ```

use crewchief_maproom::embedding::EmbeddingService;
use crewchief_maproom::search::{
    BasicWeightedFusion, FusionWeights, QueryProcessor, RRFFusion, SearchExecutors, SearchOptions,
    SearchPipeline,
};
use std::sync::Arc;
use tokio_postgres::NoTls;

/// Helper to create a database connection for testing.
async fn create_test_connection() -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres@localhost/maproom".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;

    // Spawn connection in background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    Ok(client)
}

/// Helper to create embedding service from environment.
fn create_embedding_service() -> Result<EmbeddingService, Box<dyn std::error::Error>> {
    EmbeddingService::from_env().map_err(|e| e.into())
}

/// Helper to find a test repo ID from the database.
async fn find_test_repo(
    client: &tokio_postgres::Client,
) -> Result<Option<i64>, tokio_postgres::Error> {
    let row = client
        .query_opt("SELECT id FROM maproom.repos LIMIT 1", &[])
        .await?;

    Ok(row.map(|r| r.get(0)))
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    // Find a test repo
    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database. Please index some data first.");

    // Execute search with weighted fusion
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    // Verify results structure
    assert_eq!(results.query, "function");
    assert!(results.results.len() <= 10, "Should respect limit");

    // Verify fusion timing is recorded
    assert!(results.metadata.timing.fusion_ms > 0.0);

    println!(
        "Weighted fusion completed in {:.2}ms",
        results.metadata.timing.fusion_ms
    );

    // Verify results have scores and source tracking
    if !results.is_empty() {
        let first = &results.results[0];
        assert!(first.score > 0.0);
        assert!(first.score <= 1.0);
        assert!(!first.source_scores.is_empty(), "Should track sources");

        println!(
            "Top result: {} (score: {:.4}, sources: {})",
            first.relpath,
            first.score,
            first.source_scores.len()
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_fusion_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default()); // k=60
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    // Find a test repo
    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database. Please index some data first.");

    // Execute search with RRF fusion
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    // Verify results structure
    assert_eq!(results.query, "function");
    assert!(results.results.len() <= 10, "Should respect limit");

    // Verify fusion timing is recorded
    assert!(results.metadata.timing.fusion_ms > 0.0);

    println!(
        "RRF fusion completed in {:.2}ms",
        results.metadata.timing.fusion_ms
    );

    // Verify results have RRF scores
    if !results.is_empty() {
        let first = &results.results[0];
        // RRF scores are typically much smaller than weighted scores
        // (e.g., 0.016-0.065 range for single/multiple sources)
        assert!(first.score > 0.0);
        assert!(!first.source_scores.is_empty(), "Should track sources");

        println!(
            "Top RRF result: {} (RRF score: {:.6}, sources: {})",
            first.relpath,
            first.score,
            first.source_scores.len()
        );

        // RRF scores should be sorted descending
        for i in 1..results.results.len() {
            assert!(
                results.results[i - 1].score >= results.results[i].score,
                "RRF results should be sorted by score descending"
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_custom_k_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Setup with custom k parameter
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::new(100.0)); // Higher k = more conservative
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    // Find a test repo
    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database. Please index some data first.");

    // Execute search
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    // Verify results
    if !results.is_empty() {
        let first = &results.results[0];
        // With k=100, scores will be lower than k=60
        // (e.g., 1/(100+0+1) = 0.0099 vs 1/(60+0+1) = 0.0164)
        assert!(first.score > 0.0);
        assert!(first.score < 0.1); // Should be small with high k

        println!(
            "Top RRF result (k=100): {} (score: {:.6})",
            first.relpath, first.score
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_compare_fusion_strategies() -> Result<(), Box<dyn std::error::Error>> {
    // Setup both fusion strategies
    let client1 = create_test_connection().await?;
    let client2 = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);

    let processor1 = Arc::new(QueryProcessor::new(embedder.clone()));
    let executors1 = SearchExecutors::new(client1);
    let weighted_fusion = Box::new(BasicWeightedFusion::new());
    let pipeline_weighted = SearchPipeline::with_fusion(processor1, executors1, weighted_fusion);

    let processor2 = Arc::new(QueryProcessor::new(embedder));
    let executors2 = SearchExecutors::new(client2);
    let rrf_fusion = Box::new(RRFFusion::default());
    let pipeline_rrf = SearchPipeline::with_fusion(processor2, executors2, rrf_fusion);

    // Find a test repo
    let repo_id = find_test_repo(pipeline_weighted.client())
        .await?
        .expect("No repos found in database. Please index some data first.");

    // Execute same query with both strategies
    let query = "authentication";
    let options = SearchOptions::new(repo_id, None, 10);

    let results_weighted = pipeline_weighted.search(query, options.clone()).await?;
    let results_rrf = pipeline_rrf.search(query, options).await?;

    // Compare results
    println!("\n=== Fusion Strategy Comparison ===");
    println!("Query: {}", query);
    println!("\nWeighted Fusion:");
    println!(
        "  Time: {:.2}ms (fusion: {:.2}ms)",
        results_weighted.metadata.total_time_ms(),
        results_weighted.metadata.timing.fusion_ms
    );
    if !results_weighted.is_empty() {
        println!(
            "  Top result: {} (score: {:.4})",
            results_weighted.results[0].relpath, results_weighted.results[0].score
        );
    }

    println!("\nRRF Fusion:");
    println!(
        "  Time: {:.2}ms (fusion: {:.2}ms)",
        results_rrf.metadata.total_time_ms(),
        results_rrf.metadata.timing.fusion_ms
    );
    if !results_rrf.is_empty() {
        println!(
            "  Top result: {} (RRF score: {:.6})",
            results_rrf.results[0].relpath, results_rrf.results[0].score
        );
    }

    // Both should return valid results
    assert!(results_weighted.results.len() <= 10);
    assert!(results_rrf.results.len() <= 10);

    // Both should have completed fusion
    assert!(results_weighted.metadata.timing.fusion_ms > 0.0);
    assert!(results_rrf.metadata.timing.fusion_ms > 0.0);

    // Results may be different due to different scoring, which is expected
    // We just verify both strategies work correctly
    if !results_weighted.is_empty() && !results_rrf.is_empty() {
        println!("\nBoth fusion strategies produced valid results");
        println!(
            "Weighted top score: {:.4}, RRF top score: {:.6}",
            results_weighted.results[0].score, results_rrf.results[0].score
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_with_custom_weights_ignored() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    // Find a test repo
    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database. Please index some data first.");

    // Execute search with custom weights (should be ignored by RRF)
    let custom_weights = FusionWeights::new(0.5, 0.5, 0.0, 0.0, 0.0);
    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(custom_weights);
    let results = pipeline.search("function", options).await?;

    // Verify RRF still works (weights are ignored)
    assert!(results.results.len() <= 10);

    if !results.is_empty() {
        let first = &results.results[0];
        // RRF produces rank-based scores regardless of weights
        assert!(first.score > 0.0);

        println!(
            "RRF result (weights ignored): {} (score: {:.6})",
            first.relpath, first.score
        );
    }

    Ok(())
}
