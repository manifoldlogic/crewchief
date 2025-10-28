//! Simple end-to-end workflow tests for validating the indexing pipeline.
//!
//! These tests validate:
//! 1. Docker stack health (PostgreSQL + Ollama)
//! 2. Indexed repository data exists and is complete
//! 3. FTS search functionality works
//! 4. Embedding quality is good
//! 5. Data persists correctly
//!
//! # Requirements
//!
//! - Docker stack running at ~/.maproom-mcp
//! - PostgreSQL on localhost:5433
//! - Ollama on localhost:11434 with nomic-embed-text
//! - Repository indexed (run: npx crewchief maproom scan --repo crewchief --root /workspace)
//!
//! # Running Tests
//!
//! ```bash
//! # Ensure stack is running and repo is indexed
//! docker compose --project-directory ~/.maproom-mcp ps
//!
//! # Run tests
//! cargo test --test e2e_workflow_simple -- --nocapture --test-threads=1
//! ```

use anyhow::Result;
use crewchief_maproom::db::pool::create_pool;
use crewchief_maproom::embedding::{
    CacheConfig, EmbeddingConfig, EmbeddingService, Provider, RetryConfig,
};
use serial_test::serial;
use std::time::{Duration, Instant};
use tokio_postgres::NoTls;

// =============================================================================
// Test Configuration
// =============================================================================

// Use Docker network hostname (works inside dev container)
const OLLAMA_ENDPOINT: &str = "http://maproom-ollama:11434";

// =============================================================================
// Helper Functions
// =============================================================================

async fn ollama_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let result = client
        .get(&format!("{}/api/tags", OLLAMA_ENDPOINT))
        .send()
        .await;

    if let Ok(response) = result {
        if response.status().is_success() {
            if let Ok(body) = response.text().await {
                return body.contains("nomic-embed-text");
            }
        }
    }
    false
}

async fn postgres_available_and_configure() -> bool {
    // Use Docker network hostname (works inside dev container)
    // User/pass/db all 'maproom' as configured in docker-compose.yml
    std::env::set_var("DATABASE_URL", "postgresql://maproom:maproom@maproom-postgres:5432/maproom");
    tokio_postgres::connect("postgresql://maproom:maproom@maproom-postgres:5432/maproom", NoTls)
        .await
        .is_ok()
}

async fn skip_if_services_unavailable() -> Option<()> {
    if !postgres_available_and_configure().await {
        eprintln!("WARNING: Skipping E2E test - PostgreSQL not available at maproom-postgres:5432");
        eprintln!("  Start with: docker compose --project-directory ~/.maproom-mcp up -d");
        return None;
    }

    if !ollama_available().await {
        eprintln!("WARNING: Skipping E2E test - Ollama not available at {}", OLLAMA_ENDPOINT);
        eprintln!("  Start with: docker compose --project-directory ~/.maproom-mcp up -d");
        return None;
    }

    Some(())
}

fn create_test_embedding_service() -> Result<EmbeddingService> {
    let config = EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nomic-embed-text".to_string(),
        dimension: 768,
        cache: CacheConfig {
            max_entries: 1000,
            ttl_seconds: 3600,
            enable_metrics: true,
        },
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: None,
        api_endpoint: Some(format!("{}/api/embed", OLLAMA_ENDPOINT)),
    };

    EmbeddingService::new(config).map_err(|e| anyhow::anyhow!("{:?}", e))
}

// =============================================================================
// E2E Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_01_stack_health_check() {
    println!("\n=== Test 1: Stack Health Check ===");

    let Some(_) = skip_if_services_unavailable().await else {
        return;
    };

    let pool = create_pool().await.expect("Failed to create pool");
    let client = pool.get().await.expect("Failed to get client");

    // Check schema
    let row = client
        .query_one(
            "SELECT COUNT(*) FROM information_schema.schemata WHERE schema_name = 'maproom'",
            &[],
        )
        .await
        .expect("Failed to check schema");

    let count: i64 = row.get(0);
    assert_eq!(count, 1, "maproom schema should exist");
    println!("✓ maproom schema exists");

    // Check tables
    let tables = vec!["repos", "worktrees", "chunks", "chunk_edges"];
    for table in &tables {
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM information_schema.tables
                 WHERE table_schema = 'maproom' AND table_name = $1",
                &[table],
            )
            .await
            .expect("Failed to check table");

        let count: i64 = row.get(0);
        assert_eq!(count, 1, "Table {} should exist", table);
    }
    println!("✓ All required tables exist");

    assert!(ollama_available().await);
    println!("✓ Ollama accessible with nomic-embed-text");

    println!("✓ Stack health check passed\n");
}

#[tokio::test]
#[serial]
async fn test_02_indexed_data_validation() {
    println!("\n=== Test 2: Indexed Data Validation ===");

    let Some(_) = skip_if_services_unavailable().await else {
        return;
    };

    let pool = create_pool().await.expect("Failed to create pool");
    let client = pool.get().await.expect("Failed to get client");

    // Check repos
    let row = client
        .query_one("SELECT COUNT(*), string_agg(name, ', ') FROM maproom.repos", &[])
        .await
        .expect("Failed to query repos");

    let repo_count: i64 = row.get(0);
    let repo_names: Option<String> = row.get(1);

    println!("Repositories: {} ({})", repo_count, repo_names.unwrap_or_default());
    assert!(repo_count > 0, "Should have at least 1 repo indexed");

    // Check worktrees
    let row = client
        .query_one(
            "SELECT COUNT(*), string_agg(DISTINCT name, ', ') FROM maproom.worktrees",
            &[],
        )
        .await
        .expect("Failed to query worktrees");

    let worktree_count: i64 = row.get(0);
    let worktree_names: Option<String> = row.get(1);

    println!("Worktrees: {} ({})", worktree_count, worktree_names.unwrap_or_default());
    assert!(worktree_count > 0, "Should have at least 1 worktree");

    // Check chunks
    let row = client
        .query_one(
            "SELECT
                COUNT(*) as total,
                COUNT(embedding) as with_embeddings,
                COUNT(DISTINCT language) as languages,
                COUNT(DISTINCT rel_path) as files
             FROM maproom.chunks",
            &[],
        )
        .await
        .expect("Failed to query chunks");

    let total_chunks: i64 = row.get(0);
    let chunks_with_embeddings: i64 = row.get(1);
    let language_count: i64 = row.get(2);
    let file_count: i64 = row.get(3);

    println!("Chunks: {} total", total_chunks);
    println!("  With embeddings: {}", chunks_with_embeddings);
    println!("  Languages: {}", language_count);
    println!("  Files: {}", file_count);

    assert!(total_chunks > 0, "Should have chunks");
    assert!(file_count > 0, "Should have files");

    let embedding_pct = (chunks_with_embeddings as f64 / total_chunks as f64) * 100.0;
    println!("  Embedding coverage: {:.1}%", embedding_pct);

    println!("✓ Indexed data validation passed\n");
}

#[tokio::test]
#[serial]
async fn test_03_fts_search_functionality() {
    println!("\n=== Test 3: FTS Search Functionality ===");

    let Some(_) = skip_if_services_unavailable().await else {
        return;
    };

    let pool = create_pool().await.expect("Failed to create pool");
    let client = pool.get().await.expect("Failed to get client");

    // Test FTS search for common code terms
    let search_queries = vec!["function", "async", "import", "class"];

    for query in search_queries {
        let start = Instant::now();

        let rows = client
            .query(
                "SELECT c.id, c.rel_path, c.content, ts_rank_cd(c.fts_vector, websearch_to_tsquery('english', $1)) as rank
                 FROM maproom.chunks c
                 WHERE c.fts_vector @@ websearch_to_tsquery('english', $1)
                 ORDER BY rank DESC
                 LIMIT 10",
                &[&query],
            )
            .await
            .expect("FTS search failed");

        let duration = start.elapsed();

        println!("Query '{}': {} results in {:?}", query, rows.len(), duration);

        if !rows.is_empty() {
            let top_path: String = rows[0].get(1);
            let top_rank: f32 = rows[0].get(3);
            println!("  Top result: {} (rank: {:.4})", top_path, top_rank);
        }

        // Performance check
        assert!(
            duration.as_millis() < 1000,
            "FTS search should complete in <1s"
        );
    }

    println!("✓ FTS search functionality passed\n");
}

#[tokio::test]
#[serial]
async fn test_04_embedding_quality() {
    println!("\n=== Test 4: Embedding Quality ===");

    let Some(_) = skip_if_services_unavailable().await else {
        return;
    };

    let pool = create_pool().await.expect("Failed to create pool");
    let client = pool.get().await.expect("Failed to get client");

    // Sample chunks with embeddings
    let rows = client
        .query(
            "SELECT id, content, embedding
             FROM maproom.chunks
             WHERE embedding IS NOT NULL
             ORDER BY RANDOM()
             LIMIT 10",
            &[],
        )
        .await
        .expect("Failed to query embeddings");

    println!("Sampled {} chunks with embeddings", rows.len());

    if rows.is_empty() {
        eprintln!("WARNING: No embeddings found. Run embedding generation first.");
        return;
    }

    for (i, row) in rows.iter().enumerate() {
        let chunk_id: i32 = row.get(0);
        let content: String = row.get(1);
        let embedding: Vec<f32> = row.get(2);

        println!("\nChunk {}:", i + 1);
        println!("  ID: {}", chunk_id);
        println!("  Content length: {} bytes", content.len());
        println!("  Embedding dimensions: {}", embedding.len());

        // Validate embedding
        assert_eq!(
            embedding.len(),
            768,
            "nomic-embed-text should produce 768-dimensional embeddings"
        );

        let non_zero_count = embedding.iter().filter(|&&v| v != 0.0).count();
        let finite_count = embedding.iter().filter(|&&v| v.is_finite()).count();

        println!("  Non-zero values: {} / {}", non_zero_count, embedding.len());
        println!("  Finite values: {} / {}", finite_count, embedding.len());

        assert!(
            non_zero_count > 700,
            "Embedding should have mostly non-zero values"
        );
        assert_eq!(
            finite_count,
            768,
            "All embedding values should be finite"
        );
    }

    println!("\n✓ Embedding quality check passed\n");
}

#[tokio::test]
#[serial]
async fn test_05_data_persistence() {
    println!("\n=== Test 5: Data Persistence ===");

    let Some(_) = skip_if_services_unavailable().await else {
        return;
    };

    let pool = create_pool().await.expect("Failed to create pool");
    let client = pool.get().await.expect("Failed to get client");

    // Verify data structures persist
    let row = client
        .query_one("SELECT id, name, root_path FROM maproom.repos LIMIT 1", &[])
        .await
        .expect("Repo should persist");

    let repo_id: i32 = row.get(0);
    let repo_name: String = row.get(1);
    let repo_root: String = row.get(2);

    println!("✓ Repository persisted:");
    println!("  ID: {}", repo_id);
    println!("  Name: {}", repo_name);
    println!("  Root: {}", repo_root);

    // Check worktrees
    let row = client
        .query_one(
            "SELECT id, name, commit_hash FROM maproom.worktrees WHERE repo_id = $1 LIMIT 1",
            &[&repo_id],
        )
        .await
        .expect("Worktree should persist");

    let worktree_id: i32 = row.get(0);
    let worktree_name: String = row.get(1);
    let commit_hash: String = row.get(2);

    println!("✓ Worktree persisted:");
    println!("  ID: {}", worktree_id);
    println!("  Name: {}", worktree_name);
    println!("  Commit: {}...", &commit_hash[..8]);

    // Check chunks and edges
    let row = client
        .query_one(
            "SELECT
                COUNT(*) as chunks,
                COUNT(embedding) as embeddings,
                (SELECT COUNT(*) FROM maproom.chunk_edges) as edges
             FROM maproom.chunks
             WHERE worktree_id = $1",
            &[&worktree_id],
        )
        .await
        .expect("Chunks should persist");

    let chunk_count: i64 = row.get(0);
    let embedding_count: i64 = row.get(1);
    let edge_count: i64 = row.get(2);

    println!("✓ Data persisted:");
    println!("  Chunks: {}", chunk_count);
    println!("  Embeddings: {}", embedding_count);
    println!("  Edges: {}", edge_count);

    assert!(chunk_count > 0, "Should have chunks");

    println!("\n✓ Data persistence test passed\n");
}

#[tokio::test]
#[serial]
async fn test_06_embedding_service_integration() {
    println!("\n=== Test 6: Embedding Service Integration ===");

    let Some(_) = skip_if_services_unavailable().await else {
        return;
    };

    let service = create_test_embedding_service().expect("Failed to create embedding service");

    // Test single embedding
    let text = "async function processData(input: string): Promise<Result>";
    let start = Instant::now();
    let embedding = service.embed_text(text).await;
    let duration = start.elapsed();

    assert!(embedding.is_ok(), "Embedding generation should succeed");

    let embedding = embedding.unwrap();
    assert_eq!(embedding.len(), 768, "Should generate 768-dimensional embedding");

    println!("Single embedding:");
    println!("  Time: {:?}", duration);
    println!("  Dimensions: {}", embedding.len());

    let non_zero = embedding.iter().filter(|&&v| v != 0.0).count();
    println!("  Non-zero: {} / {}", non_zero, embedding.len());

    assert!(non_zero > 700, "Should have mostly non-zero values");
    assert!(duration.as_secs() < 2, "Should complete in <2s");

    // Test batch embedding
    let texts = vec![
        "function calculateSum(a: number, b: number): number".to_string(),
        "class UserService { async getUser(id: string) {} }".to_string(),
        "const config = { apiUrl: 'http://localhost', timeout: 5000 }".to_string(),
    ];

    let start = Instant::now();
    let embeddings = service.embed_batch(texts.clone()).await;
    let duration = start.elapsed();

    assert!(embeddings.is_ok(), "Batch embedding should succeed");

    let embeddings = embeddings.unwrap();
    assert_eq!(embeddings.len(), 3, "Should generate 3 embeddings");

    println!("Batch embedding:");
    println!("  Time: {:?}", duration);
    println!("  Batch size: {}", embeddings.len());

    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), 768, "Embedding {} should have 768 dimensions", i);
    }

    println!("\n✓ Embedding service integration passed\n");
}

#[tokio::test]
#[serial]
async fn test_00_run_all_tests() {
    println!("\n");
    println!("================================================================================");
    println!("  E2E Workflow Test Suite (Simple)");
    println!("================================================================================");
    println!();
    println!("This test suite validates:");
    println!("  1. Stack health (PostgreSQL + Ollama)");
    println!("  2. Indexed data validation");
    println!("  3. FTS search functionality");
    println!("  4. Embedding quality");
    println!("  5. Data persistence");
    println!("  6. Embedding service integration");
    println!();
    println!("Prerequisites:");
    println!("  - Docker stack running: docker compose --project-directory ~/.maproom-mcp up -d");
    println!("  - Repository indexed: npx crewchief maproom scan --repo crewchief --root /workspace");
    println!();
    println!("Run tests:");
    println!("  cargo test --test e2e_workflow_simple -- --nocapture --test-threads=1");
    println!();
    println!("================================================================================");
    println!();
}
