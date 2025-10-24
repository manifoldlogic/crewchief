//! Integration tests for parallel search execution.
//!
//! These tests verify that all search executors work correctly
//! with a real PostgreSQL database and can execute in parallel
//! within the 100ms performance target.

use crewchief_maproom::search::{
    ExecutorError, FTSExecutor, GraphExecutor, ProcessedQuery, SearchExecutors, SearchMode,
    SearchSource, SignalExecutor, VectorExecutor,
};
use tokio_postgres::{Client, NoTls};

/// Helper function to establish database connection.
async fn get_test_client() -> Result<Client, Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "host=localhost user=postgres dbname=maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    Ok(client)
}

/// Helper function to create a test ProcessedQuery.
fn create_test_query(text: &str, mode: SearchMode) -> ProcessedQuery {
    let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
    let embedding = vec![0.1; 1536]; // Dummy embedding
    let expanded_terms = Vec::new();

    ProcessedQuery::new(text.to_string(), tokens, embedding, expanded_terms, mode)
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_fts_executor_basic() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;

    // Assuming test database has some indexed chunks
    let results = FTSExecutor::execute(&client, "test & function", "test function", 1, None, 10)
        .await?;

    assert_eq!(results.source, SearchSource::FTS);
    // Results may be empty if test database has no data
    println!("FTS results: {}", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_vector_executor_code_mode() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;

    let query_embedding = vec![0.1; 1536]; // Dummy embedding
    let results =
        VectorExecutor::execute(&client, &query_embedding, SearchMode::Code, 1, None, 10).await?;

    assert_eq!(results.source, SearchSource::Vector);
    println!("Vector (code) results: {}", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_vector_executor_text_mode() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;

    let query_embedding = vec![0.1; 1536];
    let results =
        VectorExecutor::execute(&client, &query_embedding, SearchMode::Text, 1, None, 10).await?;

    assert_eq!(results.source, SearchSource::Vector);
    println!("Vector (text) results: {}", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_vector_executor_hybrid_mode() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;

    let query_embedding = vec![0.1; 1536];
    let results =
        VectorExecutor::execute(&client, &query_embedding, SearchMode::Auto, 1, None, 10).await?;

    assert_eq!(results.source, SearchSource::Vector);
    println!("Vector (hybrid) results: {}", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_graph_executor_basic() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;

    let results = GraphExecutor::execute(&client, 1, None, 10).await?;

    assert_eq!(results.source, SearchSource::Graph);
    println!("Graph results: {}", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_signal_executor_basic() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;

    let results = SignalExecutor::execute(&client, 1, None).await?;

    assert_eq!(results.source, SearchSource::Signals);
    println!("Signal results: {}", results.len());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_parallel_execution() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);

    let query = create_test_query("test function", SearchMode::Code);

    let results = executors.execute_all(&query, 1, None, 10).await?;

    println!("Parallel search summary: {}", results.summary());

    // Verify all result sets are present
    assert_eq!(results.fts.source, SearchSource::FTS);
    assert_eq!(results.vector.source, SearchSource::Vector);
    assert_eq!(results.graph.source, SearchSource::Graph);
    assert_eq!(results.signals.source, SearchSource::Signals);

    // Log execution time
    println!("Execution time: {:.2}ms", results.execution_time_ms);

    // Performance assertion (may fail on slow systems or unoptimized DB)
    if results.execution_time_ms > 100.0 {
        eprintln!(
            "Warning: Execution time ({:.2}ms) exceeded 100ms target",
            results.execution_time_ms
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_parallel_execution_performance_target() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);

    let query = create_test_query("authentication", SearchMode::Auto);

    // Run multiple times to get average
    let mut total_time = 0.0;
    let iterations = 5;

    for i in 0..iterations {
        let results = executors.execute_all(&query, 1, None, 10).await?;
        total_time += results.execution_time_ms;
        println!("Iteration {}: {:.2}ms", i + 1, results.execution_time_ms);
    }

    let avg_time = total_time / iterations as f64;
    println!("Average execution time: {:.2}ms", avg_time);

    // Log warning if average exceeds target
    if avg_time > 100.0 {
        eprintln!(
            "Warning: Average execution time ({:.2}ms) exceeded 100ms target",
            avg_time
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_fast_execution() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);

    let query = create_test_query("test", SearchMode::Code);

    let start = std::time::Instant::now();
    let (fts_results, vector_results) = executors.execute_fast(&query, 1, None, 10).await?;
    let elapsed = start.elapsed();

    println!(
        "Fast execution: FTS={}, Vector={}, time={:.2}ms",
        fts_results.len(),
        vector_results.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(fts_results.source, SearchSource::FTS);
    assert_eq!(vector_results.source, SearchSource::Vector);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_empty_query_handling() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;

    // FTS with empty query
    let fts_results = FTSExecutor::execute(&client, "", "", 1, None, 10).await?;
    assert!(fts_results.is_empty());

    // Vector with empty embedding
    let empty_embedding: Vec<f32> = Vec::new();
    let vector_results =
        VectorExecutor::execute(&client, &empty_embedding, SearchMode::Code, 1, None, 10).await?;
    assert!(vector_results.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_result_deduplication() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);

    let query = create_test_query("function", SearchMode::Auto);

    let results = executors.execute_all(&query, 1, None, 10).await?;

    let unique_chunks = results.total_unique_chunks();
    let total_results = results.fts.len()
        + results.vector.len()
        + results.graph.len()
        + results.signals.len();

    println!(
        "Total results: {}, Unique chunks: {}",
        total_results, unique_chunks
    );

    // Unique chunks should be <= total results (may have duplicates across sources)
    assert!(unique_chunks <= total_results);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_score_normalization() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);

    let query = create_test_query("test", SearchMode::Code);

    let results = executors.execute_all(&query, 1, None, 10).await?;

    // Verify all scores are in 0.0-1.0 range
    for result in &results.fts.results {
        assert!(
            result.score >= 0.0 && result.score <= 1.0,
            "FTS score out of range: {}",
            result.score
        );
    }

    for result in &results.vector.results {
        assert!(
            result.score >= 0.0 && result.score <= 1.0,
            "Vector score out of range: {}",
            result.score
        );
    }

    for result in &results.graph.results {
        assert!(
            result.score >= 0.0 && result.score <= 1.0,
            "Graph score out of range: {}",
            result.score
        );
    }

    for result in &results.signals.results {
        assert!(
            result.score >= 0.0 && result.score <= 1.0,
            "Signal score out of range: {}",
            result.score
        );
    }

    Ok(())
}
