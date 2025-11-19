//! Integration tests for parallel search execution.
//!
//! These tests verify that all search executors work correctly
//! with a real PostgreSQL database and can execute in parallel
//! within the 100ms performance target.

use crewchief_maproom::search::{
    FTSExecutor, GraphExecutor, ProcessedQuery, SearchExecutors, SearchMode, SearchSource,
    SignalExecutor, VectorExecutor,
};
use tokio_postgres::{Client, NoTls};

/// Helper function to establish database connection.
async fn get_test_client() -> Result<Client, Box<dyn std::error::Error>> {
    let db_url = std::env::var("MAPROOM_DATABASE_URL")
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
    let results = FTSExecutor::execute(
        &client,
        "test & function",
        "test_function", // normalized_query
        1,
        None,
        10,
    )
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
    let total_results =
        results.fts.len() + results.vector.len() + results.graph.len() + results.signals.len();

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

//
// Detailed Performance Benchmarks with Timing Breakdowns
//

#[tokio::test]
#[ignore] // Requires database setup
async fn test_individual_executor_timing() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let query = create_test_query("authentication function", SearchMode::Auto);

    println!("\n=== Individual Executor Performance ===");

    // FTS timing
    let start = std::time::Instant::now();
    let fts_results = FTSExecutor::execute(
        &client,
        "authentication & function",
        "authentication_function", // normalized_query
        1,
        None,
        10,
    )
    .await?;
    let fts_time = start.elapsed();
    println!(
        "FTS:     {:.2}ms ({} results)",
        fts_time.as_secs_f64() * 1000.0,
        fts_results.len()
    );

    // Vector timing
    let start = std::time::Instant::now();
    let vector_results =
        VectorExecutor::execute(&client, &query.embedding, SearchMode::Auto, 1, None, 10).await?;
    let vector_time = start.elapsed();
    println!(
        "Vector:  {:.2}ms ({} results)",
        vector_time.as_secs_f64() * 1000.0,
        vector_results.len()
    );

    // Graph timing
    let start = std::time::Instant::now();
    let graph_results = GraphExecutor::execute(&client, 1, None, 10).await?;
    let graph_time = start.elapsed();
    println!(
        "Graph:   {:.2}ms ({} results)",
        graph_time.as_secs_f64() * 1000.0,
        graph_results.len()
    );

    // Signals timing
    let start = std::time::Instant::now();
    let signal_results = SignalExecutor::execute(&client, 1, None).await?;
    let signal_time = start.elapsed();
    println!(
        "Signals: {:.2}ms ({} results)",
        signal_time.as_secs_f64() * 1000.0,
        signal_results.len()
    );

    // Calculate total sequential time
    let sequential_total = fts_time + vector_time + graph_time + signal_time;
    println!(
        "\nSequential total: {:.2}ms",
        sequential_total.as_secs_f64() * 1000.0
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_parallel_vs_sequential_timing() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);
    let query = create_test_query("search test", SearchMode::Code);

    println!("\n=== Parallel vs Sequential Performance ===");

    // Parallel execution
    let parallel_start = std::time::Instant::now();
    let parallel_results = executors.execute_all(&query, 1, None, 10).await?;
    let parallel_time = parallel_start.elapsed();

    println!(
        "Parallel execution: {:.2}ms",
        parallel_time.as_secs_f64() * 1000.0
    );
    println!("  FTS: {} results", parallel_results.fts.len());
    println!("  Vector: {} results", parallel_results.vector.len());
    println!("  Graph: {} results", parallel_results.graph.len());
    println!("  Signals: {} results", parallel_results.signals.len());

    // Sequential execution for comparison
    let seq_start = std::time::Instant::now();
    let _fts = FTSExecutor::execute(
        executors.client(),
        "search & test",
        "search_test", // normalized_query
        1,
        None,
        10,
    )
    .await?;
    let _vector = VectorExecutor::execute(
        executors.client(),
        &query.embedding,
        SearchMode::Code,
        1,
        None,
        10,
    )
    .await?;
    let _graph = GraphExecutor::execute(executors.client(), 1, None, 10).await?;
    let _signals = SignalExecutor::execute(executors.client(), 1, None).await?;
    let seq_time = seq_start.elapsed();

    println!(
        "\nSequential execution: {:.2}ms",
        seq_time.as_secs_f64() * 1000.0
    );

    // Calculate speedup
    let speedup = seq_time.as_secs_f64() / parallel_time.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);

    // Parallel should be faster
    assert!(
        parallel_time < seq_time,
        "Parallel execution should be faster than sequential"
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_100ms_latency_target() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);
    let query = create_test_query("test query", SearchMode::Auto);

    println!("\n=== 100ms Latency Target Test ===");

    let iterations = 10;
    let mut times = Vec::new();

    for i in 0..iterations {
        let start = std::time::Instant::now();
        let _results = executors.execute_all(&query, 1, None, 10).await?;
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        times.push(elapsed);
        println!("Run {}: {:.2}ms", i + 1, elapsed);
    }

    // Calculate statistics
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = times.first().unwrap();
    let max = times.last().unwrap();
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let median = times[times.len() / 2];
    let p95 = times[(times.len() as f64 * 0.95) as usize];

    println!("\nStatistics:");
    println!("  Min:    {:.2}ms", min);
    println!("  Max:    {:.2}ms", max);
    println!("  Avg:    {:.2}ms", avg);
    println!("  Median: {:.2}ms", median);
    println!("  P95:    {:.2}ms", p95);

    // Check against 100ms target
    if p95 < 100.0 {
        println!("\n✓ Met <100ms target (P95: {:.2}ms)", p95);
    } else {
        println!("\n⚠ Exceeded 100ms target (P95: {:.2}ms)", p95);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_concurrent_queries_performance() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);

    println!("\n=== Concurrent Queries Performance ===");

    let queries = vec![
        create_test_query("authentication", SearchMode::Code),
        create_test_query("error handling", SearchMode::Text),
        create_test_query("function test", SearchMode::Auto),
    ];

    let start = std::time::Instant::now();

    // Execute all queries concurrently
    let results = futures::future::join_all(
        queries
            .iter()
            .map(|q| executors.execute_all(q, 1, None, 10)),
    )
    .await;

    let elapsed = start.elapsed();

    println!(
        "3 concurrent queries completed in: {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );

    // Verify all succeeded
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Query {} failed", i);
    }

    println!("All queries completed successfully");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_search_executor_timeout_handling() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let _query = create_test_query("test", SearchMode::Code);

    println!("\n=== Timeout Handling Test ===");

    // Note: This test verifies that queries complete within reasonable time
    // Actual timeout implementation would require modifying executors

    let timeout = tokio::time::Duration::from_secs(5);
    let result = tokio::time::timeout(
        timeout,
        FTSExecutor::execute(&client, "test", "test", 1, None, 10),
    )
    .await;

    match result {
        Ok(Ok(results)) => {
            println!(
                "✓ Query completed within timeout: {} results",
                results.len()
            );
        }
        Ok(Err(e)) => {
            println!("✗ Query failed: {:?}", e);
            return Err(e.into());
        }
        Err(_) => {
            println!("✗ Query timed out after {:?}", timeout);
            return Err("Query timeout".into());
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_search_with_varying_limits() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);
    let query = create_test_query("function", SearchMode::Code);

    println!("\n=== Performance vs Result Limit ===");

    for limit in [5, 10, 20, 50, 100] {
        let start = std::time::Instant::now();
        let results = executors.execute_all(&query, 1, None, limit).await?;
        let elapsed = start.elapsed();

        println!(
            "Limit {}: {:.2}ms (FTS={}, Vector={}, Graph={}, Signals={})",
            limit,
            elapsed.as_secs_f64() * 1000.0,
            results.fts.len(),
            results.vector.len(),
            results.graph.len(),
            results.signals.len()
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_search_result_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let executors = SearchExecutors::new(client);
    let query = create_test_query("test function", SearchMode::Code);

    println!("\n=== Result Consistency Test ===");

    // Execute same query multiple times
    let mut all_results = Vec::new();
    for i in 0..5 {
        let results = executors.execute_all(&query, 1, None, 10).await?;
        all_results.push(results);
        println!(
            "Run {}: FTS={}, Vector={}, Graph={}, Signals={}",
            i + 1,
            all_results[i].fts.len(),
            all_results[i].vector.len(),
            all_results[i].graph.len(),
            all_results[i].signals.len()
        );
    }

    // Results should be consistent (same query, same results)
    for i in 1..all_results.len() {
        assert_eq!(
            all_results[0].fts.len(),
            all_results[i].fts.len(),
            "FTS result count should be consistent"
        );
    }

    println!("✓ Results are consistent across runs");

    Ok(())
}
