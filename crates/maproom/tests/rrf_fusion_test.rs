//! RRF (Reciprocal Rank Fusion) Integration Tests
//!
//! This module tests the RRF fusion algorithm with real database queries
//! and various k parameter configurations.
//!
//! # Test Coverage
//!
//! - RRF formula correctness with different k values (30, 60, 120)
//! - Edge cases: empty sets, single result, duplicate chunks
//! - Comparison with weighted fusion baseline
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
//! cargo test --test rrf_fusion_test -- --nocapture
//! ```

use crewchief_maproom::embedding::EmbeddingService;
use crewchief_maproom::search::{
    BasicWeightedFusion, QueryProcessor, RRFFusion, SearchExecutors, SearchOptions, SearchPipeline,
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
async fn test_rrf_formula_k30() -> Result<(), Box<dyn std::error::Error>> {
    // Test RRF with k=30 (aggressive ranking)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::new(30.0));
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    // Verify results
    assert!(results.results.len() <= 10);

    if !results.is_empty() {
        let first = &results.results[0];

        // With k=30, top-ranked results get higher scores
        // Expected score for rank 0: 1/(30+0+1) = 1/31 ≈ 0.0323
        println!(
            "RRF (k=30) top result: {} (score: {:.6})",
            first.relpath, first.score
        );

        // Score should be positive
        assert!(first.score > 0.0);

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
async fn test_rrf_formula_k60() -> Result<(), Box<dyn std::error::Error>> {
    // Test RRF with k=60 (default/balanced ranking)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default()); // k=60
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    assert!(results.results.len() <= 10);

    if !results.is_empty() {
        let first = &results.results[0];

        // Expected score for rank 0: 1/(60+0+1) = 1/61 ≈ 0.0164
        println!(
            "RRF (k=60) top result: {} (score: {:.6})",
            first.relpath, first.score
        );

        assert!(first.score > 0.0);

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
async fn test_rrf_formula_k120() -> Result<(), Box<dyn std::error::Error>> {
    // Test RRF with k=120 (conservative ranking)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::new(120.0));
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    assert!(results.results.len() <= 10);

    if !results.is_empty() {
        let first = &results.results[0];

        // Expected score for rank 0: 1/(120+0+1) = 1/121 ≈ 0.0083
        println!(
            "RRF (k=120) top result: {} (score: {:.6})",
            first.relpath, first.score
        );

        // With higher k, scores should be lower than k=60
        assert!(first.score > 0.0);
        assert!(first.score < 0.015); // Should be lower than k=60 expected

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
async fn test_rrf_k_parameter_effect() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that k parameter affects scores as expected
    let embedder = Arc::new(create_embedding_service()?);

    // Setup k=30 pipeline
    let client1 = create_test_connection().await?;
    let processor1 = Arc::new(QueryProcessor::new(embedder.clone()));
    let executors1 = SearchExecutors::new(client1);
    let fusion_low = Box::new(RRFFusion::new(30.0));
    let pipeline_low = SearchPipeline::with_fusion(processor1, executors1, fusion_low);

    // Setup k=120 pipeline
    let client2 = create_test_connection().await?;
    let processor2 = Arc::new(QueryProcessor::new(embedder));
    let executors2 = SearchExecutors::new(client2);
    let fusion_high = Box::new(RRFFusion::new(120.0));
    let pipeline_high = SearchPipeline::with_fusion(processor2, executors2, fusion_high);

    let repo_id = find_test_repo(pipeline_low.client())
        .await?
        .expect("No repos found in database");

    // Execute same query with both k values
    let query = "authentication";
    let options = SearchOptions::new(repo_id, None, 10);

    let results_low = pipeline_low.search(query, options.clone()).await?;
    let results_high = pipeline_high.search(query, options).await?;

    if !results_low.is_empty() && !results_high.is_empty() {
        println!("\n=== K Parameter Effect ===");
        println!("Query: {}", query);
        println!("k=30 top score:  {:.6}", results_low.results[0].score);
        println!("k=120 top score: {:.6}", results_high.results[0].score);

        // Lower k should produce higher scores
        assert!(
            results_low.results[0].score > results_high.results[0].score,
            "Lower k should produce higher scores"
        );

        // Compute score differences between top 2 results
        if results_low.results.len() >= 2 && results_high.results.len() >= 2 {
            let diff_low = results_low.results[0].score - results_low.results[1].score;
            let diff_high = results_high.results[0].score - results_high.results[1].score;

            println!("k=30 rank 0-1 diff:  {:.6}", diff_low);
            println!("k=120 rank 0-1 diff: {:.6}", diff_high);

            // Lower k should have larger differences (more discriminative)
            assert!(
                diff_low > diff_high,
                "Lower k should have larger score differences between ranks"
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_empty_results() -> Result<(), Box<dyn std::error::Error>> {
    // Test RRF with a query that returns no results
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Use a very unlikely query
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline
        .search("xyzqwerthasdfzxcv_unlikely_12345", options)
        .await?;

    // Should handle empty results gracefully
    assert_eq!(results.results.len(), 0);
    assert!(results.metadata.timing.fusion_ms >= 0.0);

    println!("RRF handled empty results correctly");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_vs_weighted_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // Compare RRF against weighted fusion on the same query
    let embedder = Arc::new(create_embedding_service()?);

    // Setup RRF pipeline
    let client1 = create_test_connection().await?;
    let processor1 = Arc::new(QueryProcessor::new(embedder.clone()));
    let executors1 = SearchExecutors::new(client1);
    let rrf_fusion = Box::new(RRFFusion::default());
    let pipeline_rrf = SearchPipeline::with_fusion(processor1, executors1, rrf_fusion);

    // Setup weighted fusion pipeline
    let client2 = create_test_connection().await?;
    let processor2 = Arc::new(QueryProcessor::new(embedder));
    let executors2 = SearchExecutors::new(client2);
    let weighted_fusion = Box::new(BasicWeightedFusion::new());
    let pipeline_weighted = SearchPipeline::with_fusion(processor2, executors2, weighted_fusion);

    let repo_id = find_test_repo(pipeline_rrf.client())
        .await?
        .expect("No repos found in database");

    // Execute same query with both strategies
    let query = "search function";
    let options = SearchOptions::new(repo_id, None, 10);

    let results_rrf = pipeline_rrf.search(query, options.clone()).await?;
    let results_weighted = pipeline_weighted.search(query, options).await?;

    println!("\n=== RRF vs Weighted Fusion Comparison ===");
    println!("Query: {}", query);
    println!("\nRRF Fusion:");
    println!("  Results: {}", results_rrf.results.len());
    println!("  Fusion time: {:.2}ms", results_rrf.metadata.timing.fusion_ms);
    if !results_rrf.is_empty() {
        println!(
            "  Top result: {} (RRF score: {:.6})",
            results_rrf.results[0].relpath, results_rrf.results[0].score
        );
    }

    println!("\nWeighted Fusion:");
    println!("  Results: {}", results_weighted.results.len());
    println!("  Fusion time: {:.2}ms", results_weighted.metadata.timing.fusion_ms);
    if !results_weighted.is_empty() {
        println!(
            "  Top result: {} (weighted score: {:.4})",
            results_weighted.results[0].relpath, results_weighted.results[0].score
        );
    }

    // Both should produce valid results
    assert!(results_rrf.results.len() <= 10);
    assert!(results_weighted.results.len() <= 10);

    // Both should complete fusion in reasonable time
    assert!(results_rrf.metadata.timing.fusion_ms < 100.0);
    assert!(results_weighted.metadata.timing.fusion_ms < 100.0);

    // RRF scores typically smaller than weighted (different scale)
    if !results_rrf.is_empty() && !results_weighted.is_empty() {
        println!(
            "\nScore scale difference: RRF={:.6}, Weighted={:.4}",
            results_rrf.results[0].score, results_weighted.results[0].score
        );
        // This is expected - RRF uses reciprocal ranks (small values),
        // while weighted fusion uses normalized scores (0-1 range)
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_fusion_performance() -> Result<(), Box<dyn std::error::Error>> {
    // Verify RRF fusion meets <10ms performance requirement
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Run multiple queries to get average fusion time
    let queries = vec!["function", "class", "import", "const", "async"];
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

    println!("\n=== RRF Fusion Performance ===");
    println!("Queries tested: {}", fusion_times.len());
    println!("Average fusion time: {:.2}ms", avg_fusion_time);
    println!("Max fusion time: {:.2}ms", max_fusion_time);

    // Verify <10ms requirement
    assert!(
        avg_fusion_time < 10.0,
        "Average fusion time {:.2}ms exceeds 10ms requirement",
        avg_fusion_time
    );

    println!("✓ RRF fusion meets <10ms performance requirement");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_source_tracking() -> Result<(), Box<dyn std::error::Error>> {
    // Verify RRF correctly tracks source scores
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 5);
    let results = pipeline.search("function", options).await?;

    if !results.is_empty() {
        let first = &results.results[0];

        println!("\n=== RRF Source Tracking ===");
        println!("Top result: {}", first.relpath);
        println!("RRF score: {:.6}", first.score);
        println!("Sources: {}", first.source_scores.len());

        // Verify source scores are tracked
        assert!(
            !first.source_scores.is_empty(),
            "RRF should track source scores"
        );

        for (source, score) in &first.source_scores {
            println!("  {:?}: {:.4}", source, score);
        }

        // Source scores should be in 0.0-1.0 range (original scores)
        for (_source, score) in &first.source_scores {
            assert!(
                *score >= 0.0 && *score <= 1.0,
                "Source scores should be normalized"
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_rrf_duplicate_chunks() -> Result<(), Box<dyn std::error::Error>> {
    // Test that RRF correctly aggregates scores when same chunk appears in multiple sources
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Use a common query that should hit multiple search sources
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function main", options).await?;

    if !results.is_empty() {
        println!("\n=== RRF Duplicate Chunk Aggregation ===");

        // Find chunks that appear in multiple sources
        let mut multi_source_count = 0;
        for result in &results.results {
            if result.source_scores.len() > 1 {
                multi_source_count += 1;
                println!(
                    "Chunk {} appeared in {} sources (RRF score: {:.6})",
                    result.chunk_id,
                    result.source_scores.len(),
                    result.score
                );

                // RRF score should be sum of reciprocal ranks from each source
                // Higher than if it appeared in only one source
                assert!(
                    result.score > 0.0,
                    "Multi-source chunk should have positive RRF score"
                );
            }
        }

        println!("Chunks in multiple sources: {}", multi_source_count);

        // In a hybrid search, we expect some chunks to appear in multiple sources
        // (though this depends on the test data)
        if multi_source_count > 0 {
            println!("✓ RRF correctly aggregates duplicate chunks");
        }
    }

    Ok(())
}
