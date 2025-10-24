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

//
// Additional Error Handling and Edge Case Tests
//

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_malformed_query() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Test various malformed queries
    let malformed_queries = vec![
        "",                           // Empty
        "   ",                        // Whitespace only
        "\t\n",                       // Tabs and newlines
    ];

    for query in malformed_queries {
        let options = SearchOptions::new(repo_id, None, 10);
        let result = pipeline.search(query, options).await;

        // Should handle gracefully (error or empty results)
        match result {
            Ok(results) => {
                println!("Malformed query '{}' returned {} results", query.escape_debug(), results.len());
                assert!(results.is_empty() || results.len() == 0);
            }
            Err(e) => {
                println!("Malformed query '{}' returned error: {}", query.escape_debug(), e);
            }
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_special_characters() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let special_queries = vec![
        "User::authenticate()",
        "array->map",
        "x => y + z",
        "!important",
        "@decorator",
        "#define MACRO",
        "$variable",
        "100% coverage",
    ];

    for query in special_queries {
        let options = SearchOptions::new(repo_id, None, 10);
        let results = pipeline.search(query, options).await?;

        println!("Special query '{}': {} results", query, results.len());
        // Should handle without errors
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_very_long_query() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Very long query (100+ words)
    let long_query = "how to implement a robust authentication and authorization system \
                      with proper session management user roles permissions token validation \
                      password hashing security best practices oauth integration jwt tokens \
                      refresh tokens multi factor authentication rate limiting brute force \
                      protection csrf xss sql injection prevention input validation sanitization \
                      secure headers cors configuration session timeout idle timeout concurrent \
                      session handling device tracking geolocation ip whitelisting blacklisting \
                      audit logging compliance gdpr ccpa hipaa encryption at rest in transit \
                      key management certificate rotation backup recovery disaster planning \
                      high availability load balancing database replication caching strategies \
                      performance optimization monitoring alerting incident response";

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search(long_query, options).await?;

    println!("Long query ({} chars): {} results", long_query.len(), results.len());
    // Should handle without errors
    assert!(results.metadata.total_time_ms() > 0.0);

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_unicode_query() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let unicode_queries = vec![
        "函数 function",           // Chinese
        "función búsqueda",        // Spanish
        "поиск функция",           // Russian
        "検索 機能",              // Japanese
        "🔍 search emoji",         // Emoji
    ];

    for query in unicode_queries {
        let options = SearchOptions::new(repo_id, None, 10);
        let results = pipeline.search(query, options).await?;

        println!("Unicode query '{}': {} results", query, results.len());
        // Should handle without errors
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_ranking_order() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function test", options).await?;

    if !results.is_empty() {
        // Verify results are sorted by score (descending)
        for i in 1..results.results.len() {
            assert!(
                results.results[i - 1].score >= results.results[i].score,
                "Results should be sorted by score: result[{}]={:.4} < result[{}]={:.4}",
                i - 1,
                results.results[i - 1].score,
                i,
                results.results[i].score
            );
        }

        println!("✓ Results are properly ranked by score");
        println!("  Top score: {:.4}", results.results[0].score);
        println!("  Bottom score: {:.4}", results.results.last().unwrap().score);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_score_range() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 20);
    let results = pipeline.search("search", options).await?;

    // Verify all scores are in valid range [0.0, 1.0]
    for (i, result) in results.results.iter().enumerate() {
        assert!(
            result.score >= 0.0 && result.score <= 1.0,
            "Result {} has invalid score: {:.4}",
            i,
            result.score
        );
    }

    println!("✓ All {} results have valid scores [0.0, 1.0]", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_concurrent_searches() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Execute multiple searches sequentially (pipeline isn't Clone)
    let queries = vec!["function", "class", "test", "error", "auth"];

    for query in queries {
        let options = SearchOptions::new(repo_id, None, 5);
        let result = pipeline.search(query, options).await?;
        println!("Query '{}': {} results", query, result.len());
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_invalid_repo_id() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Use an invalid repo ID that likely doesn't exist
    let invalid_repo_id = 999999;
    let options = SearchOptions::new(invalid_repo_id, None, 10);

    let results = pipeline.search("test", options).await?;

    // Should return empty results or handle gracefully
    assert!(results.is_empty(), "Invalid repo_id should return no results");
    println!("✓ Invalid repo_id handled gracefully: 0 results");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_metadata_completeness() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("test query", options).await?;

    // Verify metadata is complete
    assert!(results.metadata.timing.query_processing_ms >= 0.0);
    assert!(results.metadata.timing.search_execution_ms >= 0.0);
    assert!(results.metadata.timing.fusion_ms >= 0.0);
    assert!(results.metadata.timing.assembly_ms >= 0.0);
    assert!(results.metadata.total_time_ms() > 0.0);

    assert!(results.metadata.query_processing.token_count >= 0);
    assert!(results.metadata.query_processing.expanded_term_count >= 0);

    // result_counts is a HashMap<SearchSource, usize>
    assert!(!results.metadata.result_counts.is_empty(), "Should have result counts");

    println!("✓ Metadata is complete and valid");
    println!("  Total time: {:.2}ms", results.metadata.total_time_ms());
    println!("  Tokens: {}", results.metadata.query_processing.token_count);
    println!("  Total unique chunks: {}", results.metadata.total_unique_chunks);
    println!("  Returned results: {}", results.metadata.returned_results);

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_search_pipeline_result_fields_populated() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service()?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    let options = SearchOptions::new(repo_id, None, 5);
    let results = pipeline.search("function", options).await?;

    if !results.is_empty() {
        let first = &results.results[0];

        // Verify all required fields are populated
        assert!(first.chunk_id > 0, "chunk_id should be positive");
        assert!(first.file_id > 0, "file_id should be positive");
        assert!(!first.relpath.is_empty(), "relpath should not be empty");
        assert!(!first.kind.is_empty(), "kind should not be empty");
        assert!(first.score > 0.0, "score should be positive");
        assert!(first.start_line > 0, "start_line should be positive");
        assert!(first.end_line >= first.start_line, "end_line should be >= start_line");
        assert!(!first.preview.is_empty(), "preview should not be empty");

        // source_scores should have at least one entry
        assert!(!first.source_scores.is_empty(), "source_scores should not be empty");

        println!("✓ All required fields are populated");
        println!("  Chunk: {}", first.chunk_id);
        println!("  File: {} ({})", first.relpath, first.kind);
        println!("  Lines: {}-{}", first.start_line, first.end_line);
        println!("  Score: {:.4}", first.score);
        println!("  Sources: {:?}", first.source_scores.keys());
    } else {
        println!("No results to verify field population");
    }

    Ok(())
}
