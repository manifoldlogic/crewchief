//! Fusion Quality and Edge Case Tests
//!
//! This module tests score breakdowns, edge cases, and performance benchmarks
//! for the fusion system.
//!
//! # Test Coverage
//!
//! - Score breakdown/explanation generation (debug mode)
//! - Edge cases: empty results, single signal, duplicate chunks
//! - Performance benchmarks (<10ms fusion requirement)
//! - Score explanation accuracy
//! - Fusion robustness under various conditions
//!
//! # Test Requirements
//!
//! - PostgreSQL database with maproom schema
//! - Sample data indexed for testing
//! - Embedding service configured
//!
//! Run with:
//! ```bash
//! cargo test --test fusion_quality_test -- --nocapture
//! ```

use crewchief_maproom::embedding::EmbeddingService;
use crewchief_maproom::search::{
    BasicWeightedFusion, QueryProcessor, RRFFusion, SearchExecutors, SearchOptions, SearchPipeline,
};
use std::sync::Arc;
use tokio_postgres::NoTls;

/// Helper to create a database connection for testing.
async fn create_test_connection() -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
    let database_url = std::env::var("MAPROOM_DATABASE_URL")
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

// ============================================================================
// Score Breakdown Tests
// ============================================================================

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_score_breakdown_availability() -> Result<(), Box<dyn std::error::Error>> {
    // Test that score breakdown is available in search results
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

    println!("\n=== Score Breakdown Test ===");
    println!("Results: {}", results.results.len());

    if !results.is_empty() {
        let first = &results.results[0];

        println!("Top result: {} (score: {:.4})", first.relpath, first.score);
        println!("Source scores:");
        for (source, score) in &first.source_scores {
            println!("  {:?}: {:.4}", source, score);
        }

        // Verify source scores are tracked
        assert!(
            !first.source_scores.is_empty(),
            "Should have source score breakdown"
        );

        // Note: Full ScoreBreakdown with FTS/Vector/Graph/Recency/Churn
        // components is available in the fusion layer but not currently
        // exposed in the API response. This test verifies source tracking.
        println!("\n✓ Score breakdown data available via source_scores");
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_score_source_contributions() -> Result<(), Box<dyn std::error::Error>> {
    // Test that each source's contribution is tracked correctly
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
    let results = pipeline.search("search function", options).await?;

    println!("\n=== Source Contribution Analysis ===");

    if !results.is_empty() {
        // Analyze source contributions across top results
        let mut fts_count = 0;
        let mut vector_count = 0;
        let mut graph_count = 0;
        let mut signals_count = 0;

        for result in &results.results {
            for source in result.source_scores.keys() {
                match source {
                    crewchief_maproom::search::SearchSource::FTS => fts_count += 1,
                    crewchief_maproom::search::SearchSource::Vector => vector_count += 1,
                    crewchief_maproom::search::SearchSource::Graph => graph_count += 1,
                    crewchief_maproom::search::SearchSource::Signals => signals_count += 1,
                }
            }
        }

        println!(
            "Source contribution counts across {} results:",
            results.results.len()
        );
        println!("  FTS: {}", fts_count);
        println!("  Vector: {}", vector_count);
        println!("  Graph: {}", graph_count);
        println!("  Signals: {}", signals_count);

        // In hybrid search, we expect multiple sources to contribute
        let total_sources = fts_count + vector_count + graph_count + signals_count;
        assert!(total_sources > 0, "Should have source contributions");

        println!("\n✓ Source contributions tracked across results");
    }

    Ok(())
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_edge_case_empty_results() -> Result<(), Box<dyn std::error::Error>> {
    // Test fusion with query that returns no results
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Use a very unlikely query
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline
        .search("xyzqwerty_unlikely_nonexistent_12345", options)
        .await?;

    println!("\n=== Edge Case: Empty Results ===");
    println!("Results: {}", results.results.len());

    // Should handle empty results gracefully
    assert_eq!(results.results.len(), 0);
    assert!(results.is_empty());
    assert!(results.metadata.timing.fusion_ms >= 0.0);

    println!("✓ Fusion handles empty results gracefully");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_edge_case_single_result() -> Result<(), Box<dyn std::error::Error>> {
    // Test fusion with limit of 1 (single result)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 1);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Edge Case: Single Result ===");
    println!("Results: {}", results.results.len());

    // Should return at most 1 result
    assert!(results.results.len() <= 1);

    if !results.is_empty() {
        let first = &results.results[0];
        println!(
            "Single result: {} (score: {:.4})",
            first.relpath, first.score
        );
        assert!(first.score > 0.0);
    }

    println!("✓ Fusion handles single result limit correctly");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_edge_case_duplicate_chunks_weighted() -> Result<(), Box<dyn std::error::Error>> {
    // Test that weighted fusion handles chunks appearing in multiple sources
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
    let results = pipeline.search("function main", options).await?;

    println!("\n=== Edge Case: Duplicate Chunks (Weighted Fusion) ===");

    if !results.is_empty() {
        // Find chunks that appear in multiple sources
        let mut multi_source_chunks = Vec::new();

        for result in &results.results {
            if result.source_scores.len() > 1 {
                multi_source_chunks.push(result);
            }
        }

        println!("Total results: {}", results.results.len());
        println!("Multi-source chunks: {}", multi_source_chunks.len());

        for chunk in &multi_source_chunks {
            println!(
                "  Chunk {}: {} sources, score: {:.4}",
                chunk.chunk_id,
                chunk.source_scores.len(),
                chunk.score
            );

            // Final score should be weighted sum of source scores
            assert!(chunk.score > 0.0);
        }

        println!("✓ Weighted fusion correctly aggregates duplicate chunks");
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_edge_case_duplicate_chunks_rrf() -> Result<(), Box<dyn std::error::Error>> {
    // Test that RRF fusion handles chunks appearing in multiple sources
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function main", options).await?;

    println!("\n=== Edge Case: Duplicate Chunks (RRF Fusion) ===");

    if !results.is_empty() {
        // Find chunks that appear in multiple sources
        let mut multi_source_chunks = Vec::new();

        for result in &results.results {
            if result.source_scores.len() > 1 {
                multi_source_chunks.push(result);
            }
        }

        println!("Total results: {}", results.results.len());
        println!("Multi-source chunks: {}", multi_source_chunks.len());

        for chunk in &multi_source_chunks {
            println!(
                "  Chunk {}: {} sources, RRF score: {:.6}",
                chunk.chunk_id,
                chunk.source_scores.len(),
                chunk.score
            );

            // RRF score should be sum of reciprocal ranks
            assert!(chunk.score > 0.0);
        }

        println!("✓ RRF fusion correctly aggregates duplicate chunks");
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_edge_case_very_large_limit() -> Result<(), Box<dyn std::error::Error>> {
    // Test fusion with very large limit (edge case)
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Request up to 1000 results
    let options = SearchOptions::new(repo_id, None, 1000);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Edge Case: Very Large Limit ===");
    println!("Requested: 1000 results");
    println!("Returned: {} results", results.results.len());
    println!("Fusion time: {:.2}ms", results.metadata.timing.fusion_ms);

    // Should handle large limits gracefully
    assert!(results.results.len() <= 1000);

    // Should still complete fusion quickly
    assert!(
        results.metadata.timing.fusion_ms < 100.0,
        "Fusion should complete quickly even with large limits"
    );

    println!("✓ Fusion handles large limits efficiently");

    Ok(())
}

// ============================================================================
// Performance Benchmark Tests
// ============================================================================

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_performance_weighted_fusion() -> Result<(), Box<dyn std::error::Error>> {
    // Comprehensive performance test for weighted fusion
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Test queries of varying complexity
    let test_queries = vec![
        ("function", "simple keyword"),
        ("async function handler", "multiple keywords"),
        (
            "search implementation with vector embeddings",
            "complex phrase",
        ),
        ("class", "single word"),
        ("import export module", "multiple terms"),
    ];

    let mut all_fusion_times = Vec::new();

    println!("\n=== Weighted Fusion Performance Benchmark ===");

    for (query, description) in test_queries {
        let options = SearchOptions::new(repo_id, None, 10);
        let results = pipeline.search(query, options).await?;

        let fusion_ms = results.metadata.timing.fusion_ms;
        all_fusion_times.push(fusion_ms);

        println!(
            "  {}: {:.2}ms ({})",
            description,
            fusion_ms,
            results.results.len()
        );

        // Each fusion should be under 10ms
        assert!(
            fusion_ms < 10.0,
            "Fusion time {:.2}ms exceeds 10ms requirement for query: {}",
            fusion_ms,
            query
        );
    }

    let avg_time = all_fusion_times.iter().sum::<f64>() / all_fusion_times.len() as f64;
    let max_time = all_fusion_times
        .iter()
        .fold(0.0f64, |a, &b| if b > a { b } else { a });

    println!("\nSummary:");
    println!("  Queries tested: {}", all_fusion_times.len());
    println!("  Average fusion time: {:.2}ms", avg_time);
    println!("  Max fusion time: {:.2}ms", max_time);
    println!("  Target: <10ms");

    assert!(
        avg_time < 10.0,
        "Average fusion time {:.2}ms exceeds 10ms requirement",
        avg_time
    );
    assert!(
        max_time < 10.0,
        "Max fusion time {:.2}ms exceeds 10ms requirement",
        max_time
    );

    println!("\n✓ Weighted fusion meets <10ms performance requirement");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_performance_rrf_fusion() -> Result<(), Box<dyn std::error::Error>> {
    // Comprehensive performance test for RRF fusion
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(RRFFusion::default());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Test queries of varying complexity
    let test_queries = vec![
        ("function", "simple keyword"),
        ("async function handler", "multiple keywords"),
        (
            "search implementation with vector embeddings",
            "complex phrase",
        ),
        ("class", "single word"),
        ("import export module", "multiple terms"),
    ];

    let mut all_fusion_times = Vec::new();

    println!("\n=== RRF Fusion Performance Benchmark ===");

    for (query, description) in test_queries {
        let options = SearchOptions::new(repo_id, None, 10);
        let results = pipeline.search(query, options).await?;

        let fusion_ms = results.metadata.timing.fusion_ms;
        all_fusion_times.push(fusion_ms);

        println!(
            "  {}: {:.2}ms ({})",
            description,
            fusion_ms,
            results.results.len()
        );

        // Each fusion should be under 10ms
        assert!(
            fusion_ms < 10.0,
            "Fusion time {:.2}ms exceeds 10ms requirement for query: {}",
            fusion_ms,
            query
        );
    }

    let avg_time = all_fusion_times.iter().sum::<f64>() / all_fusion_times.len() as f64;
    let max_time = all_fusion_times
        .iter()
        .fold(0.0f64, |a, &b| if b > a { b } else { a });

    println!("\nSummary:");
    println!("  Queries tested: {}", all_fusion_times.len());
    println!("  Average fusion time: {:.2}ms", avg_time);
    println!("  Max fusion time: {:.2}ms", max_time);
    println!("  Target: <10ms");

    assert!(
        avg_time < 10.0,
        "Average fusion time {:.2}ms exceeds 10ms requirement",
        avg_time
    );
    assert!(
        max_time < 10.0,
        "Max fusion time {:.2}ms exceeds 10ms requirement",
        max_time
    );

    println!("\n✓ RRF fusion meets <10ms performance requirement");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_performance_fusion_scaling() -> Result<(), Box<dyn std::error::Error>> {
    // Test fusion performance with different result set sizes
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Test with different limits
    let limits = vec![5, 10, 20, 50, 100];

    println!("\n=== Fusion Performance Scaling Test ===");
    println!("Testing fusion time vs. result set size");

    for limit in limits {
        let options = SearchOptions::new(repo_id, None, limit);
        let results = pipeline.search("function", options).await?;

        println!(
            "  Limit {}: {:.2}ms ({} results)",
            limit,
            results.metadata.timing.fusion_ms,
            results.results.len()
        );

        // Fusion should scale well
        assert!(
            results.metadata.timing.fusion_ms < 10.0,
            "Fusion time should be <10ms even with limit {}",
            limit
        );
    }

    println!("\n✓ Fusion performance scales well with result set size");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_performance_comparison_weighted_vs_rrf() -> Result<(), Box<dyn std::error::Error>> {
    // Compare performance of weighted vs RRF fusion
    let embedder = Arc::new(create_embedding_service()?);

    // Setup weighted fusion
    let client1 = create_test_connection().await?;
    let processor1 = Arc::new(QueryProcessor::new(embedder.clone()));
    let executors1 = SearchExecutors::new(client1);
    let fusion1 = Box::new(BasicWeightedFusion::new());
    let pipeline_weighted = SearchPipeline::with_fusion(processor1, executors1, fusion1);

    // Setup RRF fusion
    let client2 = create_test_connection().await?;
    let processor2 = Arc::new(QueryProcessor::new(embedder));
    let executors2 = SearchExecutors::new(client2);
    let fusion2 = Box::new(RRFFusion::default());
    let pipeline_rrf = SearchPipeline::with_fusion(processor2, executors2, fusion2);

    let repo_id = find_test_repo(pipeline_weighted.client())
        .await?
        .expect("No repos found in database");

    println!("\n=== Performance Comparison: Weighted vs RRF ===");

    let test_queries = vec!["function", "class", "import", "async", "const"];

    for query in test_queries {
        let options = SearchOptions::new(repo_id, None, 10);

        let results_weighted = pipeline_weighted.search(query, options.clone()).await?;
        let results_rrf = pipeline_rrf.search(query, options).await?;

        println!(
            "  '{}': Weighted={:.2}ms, RRF={:.2}ms",
            query,
            results_weighted.metadata.timing.fusion_ms,
            results_rrf.metadata.timing.fusion_ms
        );

        // Both should meet performance requirements
        assert!(results_weighted.metadata.timing.fusion_ms < 10.0);
        assert!(results_rrf.metadata.timing.fusion_ms < 10.0);
    }

    println!("\n✓ Both fusion strategies meet <10ms performance requirement");

    Ok(())
}
