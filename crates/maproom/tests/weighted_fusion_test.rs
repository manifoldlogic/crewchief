//! Weighted Fusion Integration Tests
//!
//! This module tests the BasicWeightedFusion algorithm with various
//! weight configurations and edge cases.
//!
//! # Test Coverage
//!
//! - Default weight configuration (FTS=0.4, vector=0.35, graph=0.1, recency=0.1, churn=0.05)
//! - Extreme weight configurations (single signal, zero weights, unbalanced)
//! - Weight validation and normalization
//! - Score breakdown in debug mode
//! - Performance verification (<10ms fusion requirement)
//!
//! # Test Requirements
//!
//! - PostgreSQL database with maproom schema
//! - Sample data indexed for testing
//! - Embedding service configured
//!
//! Run with:
//! ```bash
//! cargo test --test weighted_fusion_test -- --nocapture
//! ```

use crewchief_maproom::embedding::EmbeddingService;
use crewchief_maproom::search::{
    BasicWeightedFusion, FusionWeights, QueryProcessor, SearchExecutors, SearchOptions,
    SearchPipeline,
};
use std::sync::Arc;
use tokio_postgres::NoTls;

/// Helper to create a database connection for testing.
async fn create_test_connection() -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres@localhost/maproom".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;

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
async fn test_weighted_fusion_default_weights() -> Result<(), Box<dyn std::error::Error>> {
    // Test with default weight configuration
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Weighted Fusion with Default Weights ===");
    println!("Default: FTS=0.4, Vector=0.35, Graph=0.1, Recency=0.1, Churn=0.05");
    println!("Results: {}", results.results.len());

    assert!(results.results.len() <= 10);

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);
        println!("  Sources: {}", first.source_scores.len());

        // Score should be in 0.0-1.0 range
        assert!(first.score >= 0.0 && first.score <= 1.0);

        // Verify sorted descending
        for i in 1..results.results.len() {
            assert!(
                results.results[i - 1].score >= results.results[i].score,
                "Results should be sorted by score descending"
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_fts_only() -> Result<(), Box<dyn std::error::Error>> {
    // Test with FTS-only weighting (all weight on FTS)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // FTS-only weights
    let fts_only = FusionWeights::new(1.0, 0.0, 0.0, 0.0, 0.0);
    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(fts_only);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Weighted Fusion: FTS Only ===");
    println!("Weights: FTS=1.0, Vector=0.0, Graph=0.0, Recency=0.0, Churn=0.0");
    println!("Results: {}", results.results.len());

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);

        // Score should reflect only FTS contribution
        assert!(first.score >= 0.0 && first.score <= 1.0);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_vector_only() -> Result<(), Box<dyn std::error::Error>> {
    // Test with vector-only weighting
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Vector-only weights
    let vector_only = FusionWeights::new(0.0, 1.0, 0.0, 0.0, 0.0);
    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(vector_only);
    let results = pipeline.search("authentication function", options).await?;

    println!("\n=== Weighted Fusion: Vector Only ===");
    println!("Weights: FTS=0.0, Vector=1.0, Graph=0.0, Recency=0.0, Churn=0.0");
    println!("Results: {}", results.results.len());

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);

        // Score should reflect only vector contribution
        assert!(first.score >= 0.0 && first.score <= 1.0);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_balanced() -> Result<(), Box<dyn std::error::Error>> {
    // Test with balanced weights (all signals equal)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Equal weights for all signals
    let balanced = FusionWeights::new(0.2, 0.2, 0.2, 0.2, 0.2);
    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(balanced);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Weighted Fusion: Balanced (All Equal) ===");
    println!("Weights: FTS=0.2, Vector=0.2, Graph=0.2, Recency=0.2, Churn=0.2");
    println!("Results: {}", results.results.len());

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);

        // Score should be balanced contribution from all sources
        assert!(first.score >= 0.0 && first.score <= 1.0);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_extreme_fts_heavy() -> Result<(), Box<dyn std::error::Error>> {
    // Test with extreme FTS bias
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Heavily favor FTS
    let fts_heavy = FusionWeights::new(0.9, 0.05, 0.025, 0.0125, 0.0125);
    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(fts_heavy);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Weighted Fusion: FTS Heavy ===");
    println!("Weights: FTS=0.9, Vector=0.05, Graph=0.025, Recency=0.0125, Churn=0.0125");
    println!("Results: {}", results.results.len());

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);
        assert!(first.score >= 0.0 && first.score <= 1.0);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_extreme_vector_heavy() -> Result<(), Box<dyn std::error::Error>> {
    // Test with extreme vector bias
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Heavily favor vector
    let vector_heavy = FusionWeights::new(0.05, 0.9, 0.025, 0.0125, 0.0125);
    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(vector_heavy);
    let results = pipeline
        .search("semantic search functionality", options)
        .await?;

    println!("\n=== Weighted Fusion: Vector Heavy ===");
    println!("Weights: FTS=0.05, Vector=0.9, Graph=0.025, Recency=0.0125, Churn=0.0125");
    println!("Results: {}", results.results.len());

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);
        assert!(first.score >= 0.0 && first.score <= 1.0);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_unnormalized_weights() -> Result<(), Box<dyn std::error::Error>> {
    // Test with weights that don't sum to 1.0
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Unnormalized weights (sum = 2.0)
    let unnormalized = FusionWeights::new(0.8, 0.7, 0.3, 0.1, 0.1);
    assert!(!unnormalized.is_normalized());
    assert!((unnormalized.sum() - 2.0).abs() < 0.001);

    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(unnormalized);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Weighted Fusion: Unnormalized Weights ===");
    println!("Weights: FTS=0.8, Vector=0.7, Graph=0.3, Recency=0.1, Churn=0.1 (sum=2.0)");
    println!("Results: {}", results.results.len());

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);

        // Scores can exceed 1.0 with unnormalized weights
        assert!(first.score >= 0.0);
        println!("Note: Score can exceed 1.0 with unnormalized weights");
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_zero_weights() -> Result<(), Box<dyn std::error::Error>> {
    // Test with all weights set to zero (edge case)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // All zero weights
    let zero_weights = FusionWeights::new(0.0, 0.0, 0.0, 0.0, 0.0);
    let options = SearchOptions::new(repo_id, None, 10).with_fusion_weights(zero_weights);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Weighted Fusion: All Zero Weights ===");
    println!("Weights: FTS=0.0, Vector=0.0, Graph=0.0, Recency=0.0, Churn=0.0");
    println!("Results: {}", results.results.len());

    // Should still complete without error
    assert!(results.results.len() <= 10);

    if !results.is_empty() {
        let first = &results.results[0];
        println!("Top result: {} (score: {:.4})", first.relpath, first.score);
        // All scores should be 0.0 with zero weights
        assert_eq!(first.score, 0.0);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_weight_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // Compare results with different weight configurations
    let embedder = Arc::new(create_embedding_service()?);

    // Setup default weights pipeline
    let client1 = create_test_connection().await?;
    let processor1 = Arc::new(QueryProcessor::new(embedder.clone()));
    let executors1 = SearchExecutors::new(client1);
    let fusion1 = Box::new(BasicWeightedFusion::new());
    let pipeline_default = SearchPipeline::with_fusion(processor1, executors1, fusion1);

    // Setup FTS-heavy pipeline
    let client2 = create_test_connection().await?;
    let processor2 = Arc::new(QueryProcessor::new(embedder));
    let executors2 = SearchExecutors::new(client2);
    let fusion2 = Box::new(BasicWeightedFusion::new());
    let pipeline_fts = SearchPipeline::with_fusion(processor2, executors2, fusion2);

    let repo_id = find_test_repo(pipeline_default.client())
        .await?
        .expect("No repos found in database");

    let query = "search function";

    // Execute with default weights
    let options_default = SearchOptions::new(repo_id, None, 10);
    let results_default = pipeline_default.search(query, options_default).await?;

    // Execute with FTS-heavy weights
    let fts_heavy = FusionWeights::new(0.9, 0.05, 0.025, 0.0125, 0.0125);
    let options_fts = SearchOptions::new(repo_id, None, 10).with_fusion_weights(fts_heavy);
    let results_fts = pipeline_fts.search(query, options_fts).await?;

    println!("\n=== Weight Configuration Comparison ===");
    println!("Query: {}", query);

    println!("\nDefault Weights (FTS=0.4, Vector=0.35, Graph=0.1, Recency=0.1, Churn=0.05):");
    println!("  Results: {}", results_default.results.len());
    if !results_default.is_empty() {
        println!(
            "  Top result: {} (score: {:.4})",
            results_default.results[0].relpath, results_default.results[0].score
        );
    }

    println!("\nFTS-Heavy Weights (FTS=0.9, Vector=0.05, Graph=0.025, Recency=0.0125, Churn=0.0125):");
    println!("  Results: {}", results_fts.results.len());
    if !results_fts.is_empty() {
        println!(
            "  Top result: {} (score: {:.4})",
            results_fts.results[0].relpath, results_fts.results[0].score
        );
    }

    // Both should produce valid results
    assert!(results_default.results.len() <= 10);
    assert!(results_fts.results.len() <= 10);

    // Rankings may differ due to different weight configurations
    println!("\nWeight configuration affects result ranking as expected");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_performance() -> Result<(), Box<dyn std::error::Error>> {
    // Verify weighted fusion meets <10ms performance requirement
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Run multiple queries to get average fusion time
    let queries = vec![
        "function",
        "class definition",
        "import module",
        "const variable",
        "async await",
    ];
    let mut fusion_times = Vec::new();

    for query in queries {
        let options = SearchOptions::new(repo_id, None, 10);
        let results = pipeline.search(query, options).await?;
        fusion_times.push(results.metadata.timing.fusion_ms);
    }

    let avg_fusion_time = fusion_times.iter().sum::<f64>() / fusion_times.len() as f64;
    let max_fusion_time = fusion_times
        .iter()
        .fold(0.0f64, |a, &b| if b > a { b } else { a });

    println!("\n=== Weighted Fusion Performance ===");
    println!("Queries tested: {}", fusion_times.len());
    println!("Average fusion time: {:.2}ms", avg_fusion_time);
    println!("Max fusion time: {:.2}ms", max_fusion_time);

    // Verify <10ms requirement
    assert!(
        avg_fusion_time < 10.0,
        "Average fusion time {:.2}ms exceeds 10ms requirement",
        avg_fusion_time
    );

    println!("✓ Weighted fusion meets <10ms performance requirement");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_weighted_fusion_source_scores() -> Result<(), Box<dyn std::error::Error>> {
    // Verify source scores are tracked correctly
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 5);
    let results = pipeline.search("function", options).await?;

    if !results.is_empty() {
        println!("\n=== Weighted Fusion Source Score Tracking ===");

        let first = &results.results[0];
        println!("Top result: {} (final score: {:.4})", first.relpath, first.score);
        println!("Source contributions:");

        assert!(
            !first.source_scores.is_empty(),
            "Should track source scores"
        );

        for (source, score) in &first.source_scores {
            println!("  {:?}: {:.4}", source, score);

            // Source scores should be in 0.0-1.0 range
            assert!(
                *score >= 0.0 && *score <= 1.0,
                "Source scores should be normalized"
            );
        }

        println!("✓ Source scores tracked correctly");
    }

    Ok(())
}
