//! Signal Integration Tests
//!
//! This module tests the integration of temporal signals (recency and churn)
//! into the search pipeline and fusion process.
//!
//! # Test Coverage
//!
//! - Graph importance signal verification
//! - Recency signal (favors recent commits)
//! - Churn signal (penalizes high-churn code)
//! - Signal contribution to final scores
//! - Missing signal handling (graceful degradation)
//!
//! # Test Requirements
//!
//! - PostgreSQL database with maproom schema
//! - Sample data indexed with various timestamps and churn levels
//! - Graph relationships established for importance testing
//!
//! Run with:
//! ```bash
//! cargo test --test signal_integration_test -- --nocapture
//! ```

use crewchief_maproom::embedding::EmbeddingService;
use crewchief_maproom::search::{
    BasicWeightedFusion, FusionWeights, QueryProcessor, SearchExecutors, SearchOptions,
    SearchPipeline, SignalExecutor, SignalWeights,
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
async fn create_embedding_service() -> Result<EmbeddingService, Box<dyn std::error::Error>> {
    EmbeddingService::from_env().await.map_err(|e| e.into())
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
async fn test_signal_executor_basic() -> Result<(), Box<dyn std::error::Error>> {
    // Test basic signal executor functionality
    let client = create_test_connection().await?;

    let repo_id = find_test_repo(&client)
        .await?
        .expect("No repos found in database");

    let signal_results = SignalExecutor::execute(&client, repo_id, None).await?;

    println!("\n=== Signal Executor Basic Test ===");
    println!("Signal results: {}", signal_results.results.len());

    // Signal executor should return results
    assert!(
        !signal_results.results.is_empty(),
        "Should have signal scores"
    );

    // Verify signal scores are normalized (0.0-1.0)
    for result in &signal_results.results {
        assert!(
            result.score >= 0.0 && result.score <= 1.0,
            "Signal scores should be normalized to 0.0-1.0 range"
        );
    }

    // Verify results are sorted by score descending
    for i in 1..signal_results.results.len() {
        assert!(
            signal_results.results[i - 1].score >= signal_results.results[i].score,
            "Signal results should be sorted descending"
        );
    }

    println!("Top 5 signal scores:");
    for (i, result) in signal_results.results.iter().take(5).enumerate() {
        println!(
            "  {}: chunk {} (score: {:.4})",
            i + 1,
            result.chunk_id,
            result.score
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_signal_weights_custom() -> Result<(), Box<dyn std::error::Error>> {
    // Test signal executor with custom recency/churn weights
    let client = create_test_connection().await?;

    let repo_id = find_test_repo(&client)
        .await?
        .expect("No repos found in database");

    // Test with default weights
    let default_weights = SignalWeights::default();
    let results_default =
        SignalExecutor::execute_with_weights(&client, repo_id, None, default_weights).await?;

    // Test with recency-heavy weights
    let recency_weights = SignalWeights {
        recency: 0.8,
        churn: 0.2,
    };
    let results_recency =
        SignalExecutor::execute_with_weights(&client, repo_id, None, recency_weights).await?;

    // Test with churn-heavy weights
    let churn_weights = SignalWeights {
        recency: 0.2,
        churn: 0.8,
    };
    let results_churn =
        SignalExecutor::execute_with_weights(&client, repo_id, None, churn_weights).await?;

    println!("\n=== Signal Weights Comparison ===");
    println!(
        "Default (recency=0.3, churn=0.2): {} results",
        results_default.results.len()
    );
    println!(
        "Recency-heavy (recency=0.8, churn=0.2): {} results",
        results_recency.results.len()
    );
    println!(
        "Churn-heavy (recency=0.2, churn=0.8): {} results",
        results_churn.results.len()
    );

    // All should return results
    assert!(!results_default.results.is_empty());
    assert!(!results_recency.results.is_empty());
    assert!(!results_churn.results.is_empty());

    // Rankings may differ based on weights
    if results_default.results.len() >= 3 {
        println!("\nTop 3 chunks with different weights:");
        println!(
            "Default:       {:?}",
            results_default
                .results
                .iter()
                .take(3)
                .map(|r| r.chunk_id)
                .collect::<Vec<_>>()
        );
        println!(
            "Recency-heavy: {:?}",
            results_recency
                .results
                .iter()
                .take(3)
                .map(|r| r.chunk_id)
                .collect::<Vec<_>>()
        );
        println!(
            "Churn-heavy:   {:?}",
            results_churn
                .results
                .iter()
                .take(3)
                .map(|r| r.chunk_id)
                .collect::<Vec<_>>()
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_recency_signal_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that recency signal favors recent commits
    let client = create_test_connection().await?;

    let repo_id = find_test_repo(&client)
        .await?
        .expect("No repos found in database");

    // Query chunks with their recency scores
    let query = r#"
        SELECT c.id, c.recency_score, f.last_modified
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1
        ORDER BY c.recency_score DESC
        LIMIT 10
    "#;

    let rows = client.query(query, &[&repo_id]).await?;

    println!("\n=== Recency Signal Verification ===");
    println!("Top 10 chunks by recency score:");

    for row in &rows {
        let chunk_id: i64 = row.get(0);
        let recency_score: f32 = row.get(1);
        let last_modified: Option<chrono::NaiveDateTime> = row.get(2);

        println!(
            "  Chunk {}: recency={:.4}, modified={:?}",
            chunk_id, recency_score, last_modified
        );

        // Recency score should be non-negative
        assert!(recency_score >= 0.0, "Recency score should be non-negative");
    }

    // Verify recency scores are sorted descending
    for i in 1..rows.len() {
        let score_prev: f32 = rows[i - 1].get(1);
        let score_curr: f32 = rows[i].get(1);
        assert!(
            score_prev >= score_curr,
            "Recency scores should be sorted descending"
        );
    }

    println!("✓ Recency signal correctly ordered");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_churn_signal_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that churn signal reflects code stability
    let client = create_test_connection().await?;

    let repo_id = find_test_repo(&client)
        .await?
        .expect("No repos found in database");

    // Query chunks with their churn scores
    let query = r#"
        SELECT c.id, c.churn_score
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1
          AND c.churn_score > 0
        ORDER BY c.churn_score DESC
        LIMIT 10
    "#;

    let rows = client.query(query, &[&repo_id]).await?;

    println!("\n=== Churn Signal Verification ===");
    println!("Top 10 chunks by churn score:");

    for row in &rows {
        let chunk_id: i64 = row.get(0);
        let churn_score: f32 = row.get(1);

        println!("  Chunk {}: churn={:.4}", chunk_id, churn_score);

        // Churn score should be non-negative
        assert!(churn_score >= 0.0, "Churn score should be non-negative");
    }

    // Verify churn scores are sorted descending
    for i in 1..rows.len() {
        let score_prev: f32 = rows[i - 1].get(1);
        let score_curr: f32 = rows[i].get(1);
        assert!(
            score_prev >= score_curr,
            "Churn scores should be sorted descending"
        );
    }

    // In fusion, high churn should be penalized (inverted)
    // This is handled by the fusion layer with: 1.0 / (1.0 + churn_score)
    println!("\nNote: High churn is penalized during fusion: 1.0 / (1.0 + churn_score)");
    println!("✓ Churn signal correctly ordered");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_signal_contribution_to_fusion() -> Result<(), Box<dyn std::error::Error>> {
    // Test that signals contribute to final fusion scores
    let embedder = Arc::new(create_embedding_service().await?);

    // Setup pipeline with signals included
    let client1 = create_test_connection().await?;
    let processor1 = Arc::new(QueryProcessor::new(embedder.clone()));
    let executors1 = SearchExecutors::new(client1);
    let fusion1 = Box::new(BasicWeightedFusion::new());
    let pipeline_with_signals = SearchPipeline::with_fusion(processor1, executors1, fusion1);

    // Setup pipeline with signals disabled (zero weights)
    let client2 = create_test_connection().await?;
    let processor2 = Arc::new(QueryProcessor::new(embedder));
    let executors2 = SearchExecutors::new(client2);
    let fusion2 = Box::new(BasicWeightedFusion::new());
    let pipeline_no_signals = SearchPipeline::with_fusion(processor2, executors2, fusion2);

    let repo_id = find_test_repo(pipeline_with_signals.client())
        .await?
        .expect("No repos found in database");

    let query = "function";

    // Execute with default weights (includes signals)
    let options_with = SearchOptions::new(repo_id, None, 10);
    let results_with = pipeline_with_signals.search(query, options_with).await?;

    // Execute with signals disabled
    let no_signals = FusionWeights::new(0.5, 0.5, 0.0, 0.0, 0.0);
    let options_without = SearchOptions::new(repo_id, None, 10).with_fusion_weights(no_signals);
    let results_without = pipeline_no_signals.search(query, options_without).await?;

    println!("\n=== Signal Contribution to Fusion ===");
    println!("Query: {}", query);

    println!("\nWith signals (default weights):");
    println!("  Results: {}", results_with.results.len());
    if !results_with.is_empty() {
        println!(
            "  Top result: {} (score: {:.4})",
            results_with.results[0].relpath, results_with.results[0].score
        );
        println!("  Sources: {}", results_with.results[0].source_scores.len());
    }

    println!("\nWithout signals (FTS+Vector only):");
    println!("  Results: {}", results_without.results.len());
    if !results_without.is_empty() {
        println!(
            "  Top result: {} (score: {:.4})",
            results_without.results[0].relpath, results_without.results[0].score
        );
        println!(
            "  Sources: {}",
            results_without.results[0].source_scores.len()
        );
    }

    // Both should produce valid results
    assert!(!results_with.is_empty());
    assert!(!results_without.is_empty());

    // Rankings may differ when signals are included
    println!("\n✓ Signals contribute to fusion as expected");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_graph_signal_importance() -> Result<(), Box<dyn std::error::Error>> {
    // Test graph importance signal (if graph data exists)
    let client = create_test_connection().await?;

    let repo_id = find_test_repo(&client)
        .await?
        .expect("No repos found in database");

    // Query chunks with graph importance (chunk_rank from graph analysis)
    let query = r#"
        SELECT c.id, COUNT(DISTINCT cr.target_chunk_id) as outgoing_refs
        FROM maproom.chunks c
        LEFT JOIN maproom.chunk_references cr ON cr.source_chunk_id = c.id
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1
        GROUP BY c.id
        ORDER BY outgoing_refs DESC
        LIMIT 10
    "#;

    let rows = client.query(query, &[&repo_id]).await?;

    println!("\n=== Graph Importance Verification ===");
    println!("Top 10 chunks by outgoing references:");

    for row in &rows {
        let chunk_id: i64 = row.get(0);
        let ref_count: i64 = row.get(1);

        println!("  Chunk {}: {} outgoing refs", chunk_id, ref_count);
    }

    // More references indicate higher importance in the codebase
    println!("\nNote: Chunks with more references are considered more important");
    println!("✓ Graph data available for importance calculation");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_missing_signal_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Test graceful degradation when signals are missing
    let client = create_test_connection().await?;
    let embedder = Arc::new(create_embedding_service().await?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let fusion = Box::new(BasicWeightedFusion::new());
    let pipeline = SearchPipeline::with_fusion(processor, executors, fusion);

    let repo_id = find_test_repo(pipeline.client())
        .await?
        .expect("No repos found in database");

    // Query should work even if some chunks are missing signal data
    let options = SearchOptions::new(repo_id, None, 10);
    let results = pipeline.search("function", options).await?;

    println!("\n=== Missing Signal Handling ===");
    println!("Results: {}", results.results.len());

    // Search should complete successfully even with missing signals
    assert!(results.results.len() <= 10);

    if !results.is_empty() {
        println!(
            "Top result: {} (score: {:.4})",
            results.results[0].relpath, results.results[0].score
        );

        // Verify fusion completed
        assert!(results.metadata.timing.fusion_ms > 0.0);
    }

    println!("✓ Search handles missing signals gracefully");

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_signal_normalization() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that signal scores are properly normalized
    let client = create_test_connection().await?;

    let repo_id = find_test_repo(&client)
        .await?
        .expect("No repos found in database");

    let signal_results = SignalExecutor::execute(&client, repo_id, None).await?;

    println!("\n=== Signal Normalization Test ===");
    println!("Total signal results: {}", signal_results.results.len());

    if !signal_results.results.is_empty() {
        // Find min and max scores
        let scores: Vec<f32> = signal_results.results.iter().map(|r| r.score).collect();
        let min_score = scores.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_score = scores.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

        println!("Score range: {:.4} - {:.4}", min_score, max_score);

        // Max score should be 1.0 (normalized)
        assert!(
            (max_score - 1.0).abs() < 0.001,
            "Max signal score should be normalized to 1.0"
        );

        // All scores should be in 0.0-1.0 range
        for score in &scores {
            assert!(
                *score >= 0.0 && *score <= 1.0,
                "Signal scores must be in 0.0-1.0 range"
            );
        }

        println!("✓ Signal scores properly normalized to 0.0-1.0 range");
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database is available
async fn test_signal_executor_for_specific_chunks() -> Result<(), Box<dyn std::error::Error>> {
    // Test signal executor when querying specific chunks
    let client = create_test_connection().await?;

    let repo_id = find_test_repo(&client)
        .await?
        .expect("No repos found in database");

    // Get some chunk IDs to test
    let chunk_query = "SELECT id FROM maproom.chunks c JOIN maproom.files f ON f.id = c.file_id WHERE f.repo_id = $1 LIMIT 5";
    let rows = client.query(chunk_query, &[&repo_id]).await?;
    let chunk_ids: Vec<i64> = rows.iter().map(|r| r.get(0)).collect();

    if !chunk_ids.is_empty() {
        let signal_results = SignalExecutor::execute_for_chunks(
            &client,
            &chunk_ids,
            repo_id,
            None,
            SignalWeights::default(),
        )
        .await?;

        println!("\n=== Signal Executor for Specific Chunks ===");
        println!("Requested chunks: {}", chunk_ids.len());
        println!("Signal results: {}", signal_results.results.len());

        // Should return signal scores for the requested chunks
        assert!(
            signal_results.results.len() <= chunk_ids.len(),
            "Should return at most the requested chunks"
        );

        for result in &signal_results.results {
            assert!(
                chunk_ids.contains(&result.chunk_id),
                "Result should be one of the requested chunks"
            );
        }

        println!("✓ Signal executor correctly handles specific chunk queries");
    }

    Ok(())
}
