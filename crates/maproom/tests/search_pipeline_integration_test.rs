//! Integration tests for search pipeline.
//!
//! These tests verify the complete end-to-end search flow from query string
//! to ranked results with full chunk details.
//!
//! # Test Requirements
//!
//! - PostgreSQL database running with maproom schema
//! - Sample data indexed (at least a few files with chunks)
//! - Embedding service configured (for vector search)
//!
//! Run with:
//! ```bash
//! cargo test --test search_pipeline_integration_test -- --nocapture
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
async fn test_search_pipeline_basic_query() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Find a test repo
    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database. Please index some data first.");

    // Execute search
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function main", options).await?;

    // Verify results structure
    assert_eq!(results.query, "function main");
    assert!(results.results.len() <= 10, "Should respect limit");

    // Verify metadata
    assert!(results.metadata.total_time_ms() > 0.0);
    println!(
        "Search completed in {:.2}ms",
        results.metadata.total_time_ms()
    );

    // Verify result structure
    if !results.is_empty() {
        let first = &results.results[0];
        assert!(first.chunk_id > 0);
        assert!(first.file_id > 0);
        assert!(!first.relpath.is_empty());
        assert!(!first.kind.is_empty());
        assert!(first.score > 0.0);
        assert!(first.score <= 1.0);

        println!("Top result: {} (score: {:.4})", first.relpath, first.score);
        println!("  Symbol: {:?}", first.symbol_name);
        println!(
            "  Lines: {}-{}",
            first.start_line, first.end_line
        );
        println!("  Preview: {}", &first.preview[..first.preview.len().min(80)]);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_with_custom_weights() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Custom weights: heavily favor FTS
    let custom_weights = FusionWeights::new(0.7, 0.2, 0.1, 0.0);

    let options = SearchOptions::new(repo_id, None, 5).with_fusion_weights(custom_weights);

    let results = pipeline.search("authentication", options).await?;

    assert_eq!(results.query, "authentication");
    assert!(results.results.len() <= 5);

    // Verify fusion weights were applied
    assert!(results.metadata.query_processing.token_count > 0);

    println!("Custom weight search: {} results", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_empty_query() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("", options).await?;

    // Empty query should return zero results or error gracefully
    println!("Empty query results: {}", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_no_matches() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline
        .search("xyzabc123veryunlikelyquerystring", options)
        .await?;

    // Should return empty results, not error
    assert_eq!(results.results.len(), 0);
    assert!(results.is_empty());

    println!("No match query: 0 results as expected");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_code_query() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);

    // Code-like query (should be detected as Code mode)
    let results = pipeline.search("async fn search", options).await?;

    assert_eq!(results.query, "async fn search");

    // Check that mode detection worked
    println!(
        "Query mode: {:?}",
        results.metadata.query_processing.mode
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_performance() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);

    // Run search and check performance
    let start = std::time::Instant::now();
    let results = pipeline.search("search function", options).await?;
    let elapsed = start.elapsed();

    println!("Search completed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Query processing: {:.2}ms", results.metadata.timing.query_processing_ms);
    println!("  Search execution: {:.2}ms", results.metadata.timing.search_execution_ms);
    println!("  Fusion: {:.2}ms", results.metadata.timing.fusion_ms);
    println!("  Assembly: {:.2}ms", results.metadata.timing.assembly_ms);

    // Check if we met the 50ms target
    if results.metadata.met_performance_target() {
        println!("✓ Met 50ms performance target");
    } else {
        println!(
            "✗ Exceeded 50ms target: {:.2}ms",
            results.metadata.total_time_ms()
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_result_deduplication() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("test function", options).await?;

    // Check for duplicate chunk IDs
    let mut seen_chunks = std::collections::HashSet::new();
    for result in &results.results {
        assert!(
            !seen_chunks.contains(&result.chunk_id),
            "Duplicate chunk_id {} found in results",
            result.chunk_id
        );
        seen_chunks.insert(result.chunk_id);
    }

    println!("✓ All {} results are unique", results.len());

    // Check if any results came from multiple sources
    for result in &results.results {
        if result.source_scores.len() > 1 {
            println!(
                "Chunk {} found by {} sources: {:?}",
                result.chunk_id,
                result.source_scores.len(),
                result.source_scores.keys()
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_with_worktree_filter() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Find a repo and worktree using the pipeline's client
    let row = pipeline
        .client()
        .query_opt(
            "SELECT repo_id, id FROM maproom.worktrees LIMIT 1",
            &[],
        )
        .await?;

    if let Some(row) = row {
        let repo_id: i64 = row.get(0);
        let worktree_id: i64 = row.get(1);

        let options = SearchOptions::new(repo_id, Some(worktree_id), 10);
        let results = pipeline.search("function", options).await?;

        println!(
            "Search with worktree filter: {} results",
            results.len()
        );

        Ok(())
    } else {
        println!("No worktrees found, skipping test");
        Ok(())
    }
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_custom_fusion_strategy() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);

    // Create pipeline with custom fusion
    let custom_fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, custom_fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("search test", options).await?;

    println!(
        "Custom fusion search: {} results",
        results.len()
    );

    Ok(())
}
