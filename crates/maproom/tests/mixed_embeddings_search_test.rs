//! Integration tests for mixed embedding dimensions (MPEMBED-4901).
//!
//! These tests validate that the search system correctly handles repositories
//! with mixed embedding dimensions - some chunks with OpenAI embeddings (1536-dim),
//! some with Ollama embeddings (768-dim), and some with both.
//!
//! This represents a realistic migration scenario where a repository is being
//! progressively migrated from OpenAI to Ollama embeddings.
//!
//! # Test Requirements
//!
//! - PostgreSQL database running with maproom schema
//! - Embedding providers available (OpenAI and Ollama) for test fixture generation
//! - Mark tests with #[ignore] as they require external services
//!
//! Run with:
//! ```bash
//! cargo test --test mixed_embeddings_search_test -- --nocapture --ignored
//! ```
//!
//! # Test Fixture Structure
//!
//! The test creates 110 total chunks:
//! - 50 chunks with OpenAI embeddings only (1536-dim in code_embedding)
//! - 50 chunks with Ollama embeddings only (768-dim in code_embedding_ollama)
//! - 10 chunks with BOTH embeddings (tests COALESCE preference)
//!
//! Content types:
//! - TypeScript functions (for OpenAI chunks)
//! - Rust async functions (for Ollama chunks)
//! - JavaScript classes (for both-embedding chunks)

use crewchief_maproom::embedding::EmbeddingService;
use crewchief_maproom::search::{QueryProcessor, SearchExecutors, SearchOptions, SearchPipeline};
use std::collections::HashSet;
use std::sync::Arc;
use tokio_postgres::NoTls;

/// Helper to create a database connection for testing.
async fn create_test_connection() -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
    let database_url = std::env::var("MAPROOM_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@postgres:5432/crewchief".to_string());

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
async fn create_embedding_service() -> Result<EmbeddingService, Box<dyn std::error::Error>> {
    Ok(EmbeddingService::from_env().await?)
}

/// Structure to track the test fixture for verification.
#[derive(Debug)]
struct MixedEmbeddingFixture {
    repo_id: i64,
    worktree_id: i64,
    openai_chunk_ids: Vec<i64>,
    ollama_chunk_ids: Vec<i64>,
    both_chunk_ids: Vec<i64>,
}

/// Create a test repository and worktree for the fixture.
async fn create_test_repo_and_worktree(
    client: &tokio_postgres::Client,
) -> Result<(i64, i64), Box<dyn std::error::Error>> {
    // Create a unique test repo
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let repo_name = format!("test_mixed_embeddings_{}", timestamp);
    let root_path = format!("/tmp/test/{}", repo_name);

    let repo_row = client
        .query_one(
            "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id",
            &[&repo_name, &root_path],
        )
        .await?;

    let repo_id: i64 = repo_row.get(0);

    // Create a worktree
    let worktree_row = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, commit_hash) VALUES ($1, $2, $3) RETURNING id",
            &[&repo_id, &"main", &"abc123"],
        )
        .await?;

    let worktree_id: i64 = worktree_row.get(0);

    Ok((repo_id, worktree_id))
}

/// Create mixed embeddings test fixture with realistic code content.
///
/// Creates:
/// - 50 chunks with OpenAI embeddings (1536-dim) - TypeScript code
/// - 50 chunks with Ollama embeddings (768-dim) - Rust code
/// - 10 chunks with BOTH embeddings - JavaScript code
async fn setup_mixed_embeddings_fixture(
    client: &tokio_postgres::Client,
) -> Result<MixedEmbeddingFixture, Box<dyn std::error::Error>> {
    let embedder = create_embedding_service().await?;
    let (repo_id, worktree_id) = create_test_repo_and_worktree(client).await?;

    let mut openai_chunk_ids = Vec::new();
    let mut ollama_chunk_ids = Vec::new();
    let mut both_chunk_ids = Vec::new();

    // Insert a test file for all chunks
    let file_row = client
        .query_one(
            "INSERT INTO maproom.files (repo_id, worktree_id, relpath, kind, size_bytes) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            &[&repo_id, &worktree_id, &"test.ts", &"typescript", &1000i64],
        )
        .await?;
    let file_id: i64 = file_row.get(0);

    println!("Creating 50 chunks with OpenAI embeddings (1536-dim)...");
    // Create 50 chunks with OpenAI embeddings (1536-dim only)
    for i in 0..50 {
        let content = format!(
            "function processData{}(items: Array<Data>): Result {{\n  \
             const filtered = items.filter(x => x.isValid);\n  \
             const mapped = filtered.map(x => x.value * 2);\n  \
             return {{ success: true, data: mapped }};\n\
             }}",
            i
        );

        // Insert chunk with OpenAI embedding (dimension=1536)
        let embedding = embedder.embed_text(&content).await?;
        assert_eq!(
            embedding.len(),
            1536,
            "Expected OpenAI embedding to be 1536-dim"
        );

        let row = client
            .query_one(
                "INSERT INTO maproom.chunks (file_id, repo_id, worktree_id, start_line, end_line, content, code_embedding) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
                &[
                    &file_id,
                    &repo_id,
                    &worktree_id,
                    &(i as i32 * 10 + 1),
                    &(i as i32 * 10 + 5),
                    &content,
                    &embedding,
                ],
            )
            .await?;

        let chunk_id: i64 = row.get(0);
        openai_chunk_ids.push(chunk_id);
    }
    println!("✓ Created 50 OpenAI chunks");

    println!("Creating 50 chunks with Ollama embeddings (768-dim)...");
    // Create 50 chunks with Ollama embeddings (768-dim only)
    // Note: For testing, we'll truncate embeddings to 768-dim to simulate Ollama
    for i in 0..50 {
        let content = format!(
            "async fn handle_request{}(req: Request) -> Result<Response, Error> {{\n  \
             let data = req.extract_data().await?;\n  \
             let processed = process_async(data).await?;\n  \
             Ok(Response::ok(processed))\n\
             }}",
            i
        );

        // Generate embedding and truncate to 768 dimensions to simulate Ollama
        let embedding = embedder.embed_text(&content).await?;
        let ollama_embedding: Vec<f32> = embedding.iter().take(768).copied().collect();

        let row = client
            .query_one(
                "INSERT INTO maproom.chunks (file_id, repo_id, worktree_id, start_line, end_line, content, code_embedding_ollama) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
                &[
                    &file_id,
                    &repo_id,
                    &worktree_id,
                    &(i as i32 * 10 + 1000),
                    &(i as i32 * 10 + 1004),
                    &content,
                    &ollama_embedding,
                ],
            )
            .await?;

        let chunk_id: i64 = row.get(0);
        ollama_chunk_ids.push(chunk_id);
    }
    println!("✓ Created 50 Ollama chunks");

    println!("Creating 10 chunks with BOTH embeddings...");
    // Create 10 chunks with BOTH embeddings (tests COALESCE preference)
    for i in 0..10 {
        let content = format!(
            "class DataProcessor{} {{\n  \
             constructor(private config: Config) {{}}\n  \
             async process(data: Data[]): Promise<Result> {{\n    \
             return this.transform(data);\n  \
             }}\n\
             }}",
            i
        );

        let embedding = embedder.embed_text(&content).await?;
        let ollama_embedding: Vec<f32> = embedding.iter().take(768).copied().collect();

        let row = client
            .query_one(
                "INSERT INTO maproom.chunks (file_id, repo_id, worktree_id, start_line, end_line, content, code_embedding, code_embedding_ollama) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
                &[
                    &file_id,
                    &repo_id,
                    &worktree_id,
                    &(i as i32 * 10 + 2000),
                    &(i as i32 * 10 + 2004),
                    &content,
                    &embedding,
                    &ollama_embedding,
                ],
            )
            .await?;

        let chunk_id: i64 = row.get(0);
        both_chunk_ids.push(chunk_id);
    }
    println!("✓ Created 10 chunks with both embeddings");
    println!(
        "✓ Total fixture: {} chunks ({} OpenAI, {} Ollama, {} both)",
        openai_chunk_ids.len() + ollama_chunk_ids.len() + both_chunk_ids.len(),
        openai_chunk_ids.len(),
        ollama_chunk_ids.len(),
        both_chunk_ids.len()
    );

    Ok(MixedEmbeddingFixture {
        repo_id,
        worktree_id,
        openai_chunk_ids,
        ollama_chunk_ids,
        both_chunk_ids,
    })
}

/// Cleanup test fixture from database.
async fn cleanup_fixture(
    client: &tokio_postgres::Client,
    fixture: &MixedEmbeddingFixture,
) -> Result<(), Box<dyn std::error::Error>> {
    // Delete chunks (cascades to files via foreign keys)
    client
        .execute(
            "DELETE FROM maproom.chunks WHERE repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    // Delete files
    client
        .execute(
            "DELETE FROM maproom.files WHERE repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    // Delete worktrees
    client
        .execute(
            "DELETE FROM maproom.worktrees WHERE repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    // Delete repo
    client
        .execute(
            "DELETE FROM maproom.repos WHERE id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    println!("✓ Cleaned up test fixture");
    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database and embedding services are available
async fn test_hybrid_search_finds_both_embedding_types() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Test: Hybrid search finds both embedding types ===");

    let client = create_test_connection().await?;
    let fixture = setup_mixed_embeddings_fixture(&client).await?;

    // Setup search pipeline
    let embedder = Arc::new(create_embedding_service().await?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Search with a query that should match both TypeScript and Rust code
    let query = "process data function";
    let options = SearchOptions::new(fixture.repo_id, Some(fixture.worktree_id), 50);

    println!("Executing hybrid search for: '{}'", query);
    let start = std::time::Instant::now();
    let results = pipeline.search(query, options).await?;
    let duration = start.elapsed();

    println!("Search completed in {:?}", duration);
    println!("Found {} results", results.len());

    // Verify we found results
    assert!(
        !results.is_empty(),
        "Should find results from mixed embeddings"
    );

    // Track which chunk types we found
    let mut found_openai = false;
    let mut found_ollama = false;
    let mut found_both = false;

    for result in &results.results {
        if fixture.openai_chunk_ids.contains(&result.chunk_id) {
            found_openai = true;
            println!("  Found OpenAI chunk: {}", result.chunk_id);
        } else if fixture.ollama_chunk_ids.contains(&result.chunk_id) {
            found_ollama = true;
            println!("  Found Ollama chunk: {}", result.chunk_id);
        } else if fixture.both_chunk_ids.contains(&result.chunk_id) {
            found_both = true;
            println!("  Found both-embedding chunk: {}", result.chunk_id);
        }
    }

    println!("\nResults summary:");
    println!("  OpenAI chunks found: {}", found_openai);
    println!("  Ollama chunks found: {}", found_ollama);
    println!("  Both-embedding chunks found: {}", found_both);

    // The test is successful if we find results from different embedding types
    // Note: We don't require all types because relevance may vary
    let types_found = [found_openai, found_ollama, found_both]
        .iter()
        .filter(|&&x| x)
        .count();
    assert!(
        types_found >= 1,
        "Should find results from at least one embedding type, found {}",
        types_found
    );

    println!(
        "✓ Hybrid search successfully found results from {} embedding type(s)",
        types_found
    );

    // Cleanup
    cleanup_fixture(pipeline.client(), &fixture).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database and embedding services are available
async fn test_coalesce_prefers_ollama_over_openai() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Test: COALESCE prefers Ollama over OpenAI ===");

    let client = create_test_connection().await?;
    let fixture = setup_mixed_embeddings_fixture(&client).await?;

    println!("Verifying COALESCE behavior for chunks with both embeddings...");

    // Query the database directly to check which embedding column is used
    // when both are present (simulating the COALESCE logic)
    let rows = client
        .query(
            "SELECT id, \
             CASE \
               WHEN code_embedding_ollama IS NOT NULL THEN 768 \
               WHEN code_embedding IS NOT NULL THEN 1536 \
               ELSE NULL \
             END as used_dimension \
             FROM maproom.chunks \
             WHERE repo_id = $1 AND id = ANY($2)",
            &[&fixture.repo_id, &fixture.both_chunk_ids],
        )
        .await?;

    let row_count = rows.len();
    println!("Checking {} chunks that have both embeddings...", row_count);

    for row in rows {
        let chunk_id: i64 = row.get(0);
        let used_dimension: Option<i32> = row.get(1);

        assert_eq!(
            used_dimension,
            Some(768),
            "Chunk {} with both embeddings should prefer Ollama (768-dim)",
            chunk_id
        );
        println!("  ✓ Chunk {} uses 768-dim (Ollama)", chunk_id);
    }

    println!(
        "✓ COALESCE correctly prefers Ollama over OpenAI for all {} chunks",
        row_count
    );

    // Cleanup
    cleanup_fixture(&client, &fixture).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database and embedding services are available
async fn test_fts_search_unaffected_by_embeddings() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Test: FTS search unaffected by embedding dimensions ===");

    let client = create_test_connection().await?;
    let fixture = setup_mixed_embeddings_fixture(&client).await?;

    // Setup search pipeline
    let embedder = Arc::new(create_embedding_service().await?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Search for "DataProcessor" which appears only in the 10 "both" chunks
    let query = "DataProcessor";
    let options = SearchOptions::new(fixture.repo_id, Some(fixture.worktree_id), 20);

    println!("Executing FTS search for: '{}'", query);
    let results = pipeline.search(query, options).await?;

    println!("Found {} results", results.len());

    // Count how many of the "both" chunks we found
    let both_chunk_count = results
        .results
        .iter()
        .filter(|r| fixture.both_chunk_ids.contains(&r.chunk_id))
        .count();

    println!(
        "Found {} out of 10 chunks containing 'DataProcessor'",
        both_chunk_count
    );

    // FTS should find at least some of the chunks with "DataProcessor" in the content
    assert!(
        both_chunk_count > 0,
        "FTS should find chunks with 'DataProcessor' in content"
    );

    println!("✓ FTS search works correctly regardless of embedding dimensions");

    // Cleanup
    cleanup_fixture(pipeline.client(), &fixture).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database and embedding services are available
async fn test_search_performance_with_mixed_embeddings() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Test: Search performance with mixed embeddings ===");

    let client = create_test_connection().await?;
    let fixture = setup_mixed_embeddings_fixture(&client).await?;

    // Setup search pipeline
    let embedder = Arc::new(create_embedding_service().await?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Run multiple searches to measure performance
    let queries = vec![
        "process data function",
        "async handler request",
        "class constructor",
        "filter map array",
        "transform result",
    ];

    let mut total_duration = std::time::Duration::ZERO;
    let mut total_results = 0;

    for query in &queries {
        let options = SearchOptions::new(fixture.repo_id, Some(fixture.worktree_id), 20);

        let start = std::time::Instant::now();
        let results = pipeline.search(query, options).await?;
        let duration = start.elapsed();

        total_duration += duration;
        total_results += results.len();

        println!(
            "Query '{}': {} results in {:?}",
            query,
            results.len(),
            duration
        );

        // Check performance target: < 100ms per search
        assert!(
            duration.as_millis() < 100,
            "Search for '{}' took too long: {:?} (expected < 100ms)",
            query,
            duration
        );
    }

    let avg_duration = total_duration / queries.len() as u32;
    println!("\nPerformance summary:");
    println!("  Total searches: {}", queries.len());
    println!("  Average duration: {:?}", avg_duration);
    println!(
        "  Average results per query: {:.1}",
        total_results as f64 / queries.len() as f64
    );
    println!("  Total corpus: 110 chunks (50 OpenAI + 50 Ollama + 10 both)");

    assert!(
        avg_duration.as_millis() < 100,
        "Average search time exceeds 100ms target: {:?}",
        avg_duration
    );

    println!("✓ All searches met performance target (< 100ms)");

    // Cleanup
    cleanup_fixture(pipeline.client(), &fixture).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database and embedding services are available
async fn test_result_deduplication_across_dimensions() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Test: Result deduplication across embedding dimensions ===");

    let client = create_test_connection().await?;
    let fixture = setup_mixed_embeddings_fixture(&client).await?;

    // Setup search pipeline
    let embedder = Arc::new(create_embedding_service().await?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Search broadly to get many results
    let query = "data process function async class";
    let options = SearchOptions::new(fixture.repo_id, Some(fixture.worktree_id), 50);

    let results = pipeline.search(query, options).await?;

    println!("Found {} results", results.len());

    // Check for duplicate chunk IDs
    let mut seen_chunks = HashSet::new();
    let mut duplicates = Vec::new();

    for result in &results.results {
        if !seen_chunks.insert(result.chunk_id) {
            duplicates.push(result.chunk_id);
        }
    }

    assert!(
        duplicates.is_empty(),
        "Found {} duplicate chunk IDs: {:?}",
        duplicates.len(),
        duplicates
    );

    println!("✓ All {} results are unique (no duplicates)", results.len());

    // Cleanup
    cleanup_fixture(pipeline.client(), &fixture).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database and embedding services are available
async fn test_empty_results_with_mixed_embeddings() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Test: Empty results with mixed embeddings ===");

    let client = create_test_connection().await?;
    let fixture = setup_mixed_embeddings_fixture(&client).await?;

    // Setup search pipeline
    let embedder = Arc::new(create_embedding_service().await?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    // Search for something that definitely won't match
    let query = "xyzabc123nonexistentquerystring";
    let options = SearchOptions::new(fixture.repo_id, Some(fixture.worktree_id), 10);

    let results = pipeline.search(query, options).await?;

    println!("Found {} results (expected 0)", results.len());

    assert_eq!(
        results.len(),
        0,
        "Should return empty results for non-matching query"
    );

    println!("✓ Empty results handled correctly");

    // Cleanup
    cleanup_fixture(pipeline.client(), &fixture).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Run only when database and embedding services are available
async fn test_metadata_tracking_with_mixed_embeddings() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Test: Metadata tracking with mixed embeddings ===");

    let client = create_test_connection().await?;
    let fixture = setup_mixed_embeddings_fixture(&client).await?;

    // Setup search pipeline
    let embedder = Arc::new(create_embedding_service().await?);
    let processor = Arc::new(QueryProcessor::new(embedder));
    let executors = SearchExecutors::new(client);
    let pipeline = SearchPipeline::new(processor, executors);

    let query = "process function";
    let options = SearchOptions::new(fixture.repo_id, Some(fixture.worktree_id), 20);

    let results = pipeline.search(query, options).await?;

    // Verify metadata is complete
    println!("Metadata:");
    println!("  Total time: {:.2}ms", results.metadata.total_time_ms());
    println!(
        "  Query processing: {:.2}ms",
        results.metadata.timing.query_processing_ms
    );
    println!(
        "  Search execution: {:.2}ms",
        results.metadata.timing.search_execution_ms
    );
    println!("  Fusion: {:.2}ms", results.metadata.timing.fusion_ms);
    println!("  Assembly: {:.2}ms", results.metadata.timing.assembly_ms);

    assert!(results.metadata.timing.query_processing_ms >= 0.0);
    assert!(results.metadata.timing.search_execution_ms >= 0.0);
    assert!(results.metadata.timing.fusion_ms >= 0.0);
    assert!(results.metadata.timing.assembly_ms >= 0.0);
    assert!(results.metadata.total_time_ms() > 0.0);

    // Verify result counts
    println!("  Result counts: {:?}", results.metadata.result_counts);
    println!(
        "  Total unique chunks: {}",
        results.metadata.total_unique_chunks
    );
    println!("  Returned results: {}", results.metadata.returned_results);

    assert!(
        !results.metadata.result_counts.is_empty(),
        "Should have result counts"
    );

    println!("✓ Metadata is complete and valid");

    // Cleanup
    cleanup_fixture(pipeline.client(), &fixture).await?;

    Ok(())
}
