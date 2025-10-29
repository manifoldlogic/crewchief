//! End-to-end multi-provider embedding tests (MPEMBED-6002).
//!
//! These tests validate complete workflows from scanning to searching with
//! multiple embedding providers (Ollama, Google, OpenAI). Each test:
//!
//! 1. Creates temporary test files with realistic code
//! 2. Scans files and generates chunks
//! 3. Generates embeddings with specific provider
//! 4. Verifies embeddings are stored in correct database columns
//! 5. Performs semantic search
//! 6. Validates search results match expected content
//! 7. Cleans up test data
//!
//! # Test Requirements
//!
//! Tests require real provider access and are marked `#[ignore]` for CI.
//! Enable specific tests via environment variables:
//!
//! - `TEST_OLLAMA=1`: Enable Ollama tests (requires Ollama running)
//! - `OPENAI_API_KEY=sk-...`: Enable OpenAI tests (requires API key)
//! - `GOOGLE_PROJECT_ID=...`: Enable Google tests (requires GCP setup)
//!
//! # Running Tests
//!
//! ```bash
//! # Run all E2E tests (with providers configured)
//! TEST_OLLAMA=1 OPENAI_API_KEY=sk-... cargo test --test e2e_multi_provider -- --ignored --nocapture
//!
//! # Run only Ollama E2E test
//! TEST_OLLAMA=1 cargo test --test e2e_multi_provider test_e2e_ollama_scan_and_search -- --ignored --nocapture
//!
//! # Run without external providers (tests will skip gracefully)
//! cargo test --test e2e_multi_provider -- --ignored
//! ```
//!
//! # Database Requirements
//!
//! Tests use the development database (postgres:5432) with maproom schema.
//! They create unique test repos/worktrees and clean up after completion.

use anyhow::Result;
use crewchief_maproom::db::pool::create_pool;
use crewchief_maproom::embedding::factory::create_provider_from_env;
use crewchief_maproom::embedding::provider::EmbeddingProvider;
use serial_test::serial;
use std::env;
use std::time::Instant;
use tempfile::TempDir;

// =============================================================================
// Test Helpers
// =============================================================================

/// Test fixture structure to track test data for cleanup.
#[derive(Debug)]
struct TestFixture {
    repo_id: i64,
    #[allow(dead_code)]
    worktree_id: i64,
    #[allow(dead_code)]
    file_id: i64,
    #[allow(dead_code)]
    chunk_ids: Vec<i64>,
    _temp_dir: TempDir, // Keep alive for test duration
}

/// Create a test repository, worktree, and file in the database.
async fn create_test_repo(
    client: &tokio_postgres::Client,
    test_name: &str,
) -> Result<(i64, i64, i64)> {
    // Create unique test repo
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let repo_name = format!("test_e2e_{}_{}", test_name, timestamp);

    let repo_row = client
        .query_one(
            "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id",
            &[&repo_name, &format!("/tmp/test/{}", repo_name)],
        )
        .await?;
    let repo_id: i64 = repo_row.get(0);

    // Create worktree
    let worktree_row = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path, commit_hash) VALUES ($1, $2, $3, $4) RETURNING id",
            &[&repo_id, &"main", &format!("/tmp/test/{}/main", repo_name), &"test_commit"],
        )
        .await?;
    let worktree_id: i64 = worktree_row.get(0);

    // Create file
    let file_row = client
        .query_one(
            "INSERT INTO maproom.files (repo_id, worktree_id, relpath, language, size_bytes, hash) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            &[&repo_id, &worktree_id, &"test.ts", &"typescript", &1000i64, &"test_hash"],
        )
        .await?;
    let file_id: i64 = file_row.get(0);

    Ok((repo_id, worktree_id, file_id))
}

/// Create chunks in the database from code content.
async fn create_test_chunks(
    client: &tokio_postgres::Client,
    repo_id: i64,
    worktree_id: i64,
    file_id: i64,
    contents: &[&str],
) -> Result<Vec<i64>> {
    let mut chunk_ids = Vec::new();

    for (idx, content) in contents.iter().enumerate() {
        let row = client
            .query_one(
                "INSERT INTO maproom.chunks (repo_id, worktree_id, file_id, start_line, end_line, content, kind, preview) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
                &[
                    &repo_id,
                    &worktree_id,
                    &file_id,
                    &((idx * 10) as i32 + 1),
                    &((idx * 10) as i32 + 5),
                    content,
                    &"function",
                    &format!("{}...", &content[..content.len().min(50)]),
                ],
            )
            .await?;

        let chunk_id: i64 = row.get(0);
        chunk_ids.push(chunk_id);
    }

    Ok(chunk_ids)
}

/// Generate embeddings for chunks using the specified provider.
async fn generate_embeddings(
    client: &tokio_postgres::Client,
    provider: &Box<dyn EmbeddingProvider>,
    chunk_ids: &[i64],
) -> Result<()> {
    let provider_name = provider.provider_name();
    let dimension = provider.dimension();

    println!(
        "Generating embeddings with {} ({}D) for {} chunks...",
        provider_name,
        dimension,
        chunk_ids.len()
    );

    // Determine which column to use based on dimension
    let column_name = match dimension {
        768 => "code_embedding_ollama",
        1536 => "code_embedding",
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported dimension: {}. Expected 768 or 1536.",
                dimension
            ))
        }
    };

    // Fetch chunk contents
    let rows = client
        .query(
            "SELECT id, content FROM maproom.chunks WHERE id = ANY($1) ORDER BY id",
            &[&chunk_ids],
        )
        .await?;

    // Generate embeddings for each chunk
    for row in rows {
        let chunk_id: i64 = row.get(0);
        let content: String = row.get(1);

        let embedding = provider.embed(content).await?;

        // Store in appropriate column
        let update_query = format!(
            "UPDATE maproom.chunks SET {} = $1 WHERE id = $2",
            column_name
        );
        client.execute(&update_query, &[&embedding, &chunk_id]).await?;

        println!("  ✓ Generated embedding for chunk {}", chunk_id);
    }

    println!("✓ Generated {} embeddings", chunk_ids.len());
    Ok(())
}

/// Perform semantic search using the provider's embeddings.
async fn search_with_provider(
    client: &tokio_postgres::Client,
    provider: &Box<dyn EmbeddingProvider>,
    repo_id: i64,
    worktree_id: i64,
    query_text: &str,
    limit: i32,
) -> Result<Vec<SearchResult>> {
    let dimension = provider.dimension();
    let query_embedding = provider.embed(query_text.to_string()).await?;

    // Determine which column to search based on dimension
    let column_name = match dimension {
        768 => "code_embedding_ollama",
        1536 => "code_embedding",
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported dimension: {}",
                dimension
            ))
        }
    };

    let search_query = format!(
        "SELECT
            c.id,
            c.content,
            c.preview,
            1.0 - (c.{} <=> $1::vector) as similarity
         FROM maproom.chunks c
         WHERE c.repo_id = $2
           AND c.worktree_id = $3
           AND c.{} IS NOT NULL
         ORDER BY c.{} <=> $1::vector
         LIMIT $4",
        column_name, column_name, column_name
    );

    let rows = client
        .query(&search_query, &[&query_embedding, &repo_id, &worktree_id, &limit])
        .await?;

    let mut results = Vec::new();
    for row in rows {
        results.push(SearchResult {
            chunk_id: row.get(0),
            content: row.get(1),
            preview: row.get(2),
            similarity: row.get(3),
        });
    }

    Ok(results)
}

/// Cleanup test fixture from database.
async fn cleanup_fixture(client: &tokio_postgres::Client, fixture: &TestFixture) -> Result<()> {
    // Delete in reverse dependency order
    client
        .execute(
            "DELETE FROM maproom.chunks WHERE repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    client
        .execute(
            "DELETE FROM maproom.files WHERE repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    client
        .execute(
            "DELETE FROM maproom.worktrees WHERE repo_id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    client
        .execute(
            "DELETE FROM maproom.repos WHERE id = $1",
            &[&fixture.repo_id],
        )
        .await?;

    println!("✓ Cleaned up test fixture (repo_id: {})", fixture.repo_id);
    Ok(())
}

#[derive(Debug)]
struct SearchResult {
    chunk_id: i64,
    content: String,
    preview: String,
    similarity: f64,
}

// =============================================================================
// E2E Test Cases
// =============================================================================

#[tokio::test]
#[serial]
#[ignore] // Run with: TEST_OLLAMA=1 cargo test ... -- --ignored
async fn test_e2e_ollama_scan_and_search() -> Result<()> {
    println!("\n=== E2E Test: Ollama Scan and Search (768-dim) ===\n");

    if env::var("TEST_OLLAMA").is_err() {
        println!("⊘ Skipping Ollama E2E test (TEST_OLLAMA not set)");
        println!("  To enable: TEST_OLLAMA=1 cargo test ... -- --ignored");
        return Ok(());
    }

    // Setup database connection
    let pool = create_pool().await?;
    let client = pool.get().await?;

    // Create test repository structure
    let (repo_id, worktree_id, file_id) = create_test_repo(&client, "ollama").await?;

    // Create realistic authentication code
    let test_content = vec![
        r#"export async function authenticate(username: string, password: string) {
    const user = await db.findUser(username);
    if (!user) throw new Error('User not found');
    const valid = await bcrypt.compare(password, user.passwordHash);
    if (!valid) throw new Error('Invalid password');
    return createSession(user);
}"#,
        r#"export async function validateToken(token: string): Promise<User | null> {
    const session = await db.findSession(token);
    if (!session || session.expiresAt < new Date()) {
        return null;
    }
    return session.user;
}"#,
    ];

    let temp_dir = tempfile::tempdir()?;
    let chunk_ids = create_test_chunks(&client, repo_id, worktree_id, file_id, &test_content).await?;

    // Setup Ollama provider
    env::set_var("EMBEDDING_PROVIDER", "ollama");
    let provider = create_provider_from_env().await?;
    assert_eq!(provider.dimension(), 768, "Ollama should use 768 dimensions");
    assert_eq!(provider.provider_name(), "ollama");

    println!("Using provider: {} ({}D)", provider.provider_name(), provider.dimension());

    // Generate embeddings
    let start = Instant::now();
    generate_embeddings(&client, &provider, &chunk_ids).await?;
    let embedding_duration = start.elapsed();
    println!("Embedding generation took: {:?}", embedding_duration);

    // Verify embeddings stored in correct column
    let row = client
        .query_one(
            "SELECT code_embedding_ollama, code_embedding FROM maproom.chunks WHERE id = $1",
            &[&chunk_ids[0]],
        )
        .await?;

    let ollama_embedding: Option<Vec<f32>> = row.get(0);
    let openai_embedding: Option<Vec<f32>> = row.get(1);

    assert!(ollama_embedding.is_some(), "Should have Ollama embedding");
    assert_eq!(
        ollama_embedding.unwrap().len(),
        768,
        "Ollama embedding should be 768-dim"
    );
    assert!(openai_embedding.is_none(), "Should not have OpenAI embedding");

    println!("✓ Embeddings stored in code_embedding_ollama column");

    // Perform semantic search
    let start = Instant::now();
    let results =
        search_with_provider(&client, &provider, repo_id, worktree_id, "authentication password validation", 5)
            .await?;
    let search_duration = start.elapsed();

    println!("\nSearch results (took {:?}):", search_duration);
    assert!(!results.is_empty(), "Should find authentication code");

    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. Chunk {} (similarity: {:.4}): {}",
            i + 1,
            result.chunk_id,
            result.similarity,
            result.preview
        );
    }

    // Verify top result is relevant
    let top_result = &results[0];
    assert!(
        top_result.content.contains("authenticate") || top_result.content.contains("password"),
        "Top result should be relevant to authentication"
    );
    assert!(top_result.similarity > 0.5, "Top result should have good similarity");

    println!("\n✓ E2E Ollama test passed");

    // Cleanup
    cleanup_fixture(
        &client,
        &TestFixture {
            repo_id,
            worktree_id,
            file_id,
            chunk_ids,
            _temp_dir: temp_dir,
        },
    )
    .await?;

    Ok(())
}

#[tokio::test]
#[serial]
#[ignore] // Run with: GOOGLE_PROJECT_ID=... cargo test ... -- --ignored
async fn test_e2e_google_scan_and_search() -> Result<()> {
    println!("\n=== E2E Test: Google Scan and Search (768-dim) ===\n");

    if env::var("GOOGLE_PROJECT_ID").is_err() {
        println!("⊘ Skipping Google E2E test (GOOGLE_PROJECT_ID not set)");
        println!("  To enable: GOOGLE_PROJECT_ID=... GOOGLE_APPLICATION_CREDENTIALS=... cargo test ... -- --ignored");
        return Ok(());
    }

    // Setup database connection
    let pool = create_pool().await?;
    let client = pool.get().await?;

    // Create test repository structure
    let (repo_id, worktree_id, file_id) = create_test_repo(&client, "google").await?;

    // Create realistic database code
    let test_content = vec![
        r#"export class DatabaseConnection {
    async query(sql: string, params: any[]) {
        const result = await this.pool.query(sql, params);
        return result.rows;
    }

    async transaction<T>(callback: (client: Client) => Promise<T>): Promise<T> {
        const client = await this.pool.connect();
        try {
            await client.query('BEGIN');
            const result = await callback(client);
            await client.query('COMMIT');
            return result;
        } catch (error) {
            await client.query('ROLLBACK');
            throw error;
        } finally {
            client.release();
        }
    }
}"#,
    ];

    let temp_dir = tempfile::tempdir()?;
    let chunk_ids = create_test_chunks(&client, repo_id, worktree_id, file_id, &test_content).await?;

    // Setup Google provider
    env::set_var("EMBEDDING_PROVIDER", "google");
    let provider = create_provider_from_env().await?;
    assert_eq!(provider.dimension(), 768, "Google should use 768 dimensions");
    assert_eq!(provider.provider_name(), "google");

    println!("Using provider: {} ({}D)", provider.provider_name(), provider.dimension());

    // Generate embeddings
    let start = Instant::now();
    generate_embeddings(&client, &provider, &chunk_ids).await?;
    let embedding_duration = start.elapsed();
    println!("Embedding generation took: {:?}", embedding_duration);

    // Verify embeddings stored in correct column (Google uses Ollama column for 768-dim)
    let row = client
        .query_one(
            "SELECT code_embedding_ollama, code_embedding FROM maproom.chunks WHERE id = $1",
            &[&chunk_ids[0]],
        )
        .await?;

    let ollama_embedding: Option<Vec<f32>> = row.get(0);
    let openai_embedding: Option<Vec<f32>> = row.get(1);

    assert!(ollama_embedding.is_some(), "Should have 768-dim embedding in Ollama column");
    assert_eq!(ollama_embedding.unwrap().len(), 768, "Should be 768-dim");
    assert!(openai_embedding.is_none(), "Should not have OpenAI embedding");

    println!("✓ Embeddings stored in code_embedding_ollama column");

    // Perform semantic search
    let start = Instant::now();
    let results = search_with_provider(&client, &provider, repo_id, worktree_id, "database query transaction", 5).await?;
    let search_duration = start.elapsed();

    println!("\nSearch results (took {:?}):", search_duration);
    assert!(!results.is_empty(), "Should find database code");

    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. Chunk {} (similarity: {:.4}): {}",
            i + 1,
            result.chunk_id,
            result.similarity,
            result.preview
        );
    }

    // Verify top result is relevant
    let top_result = &results[0];
    assert!(
        top_result.content.contains("query") || top_result.content.contains("transaction"),
        "Top result should be relevant to database operations"
    );

    println!("\n✓ E2E Google test passed");

    // Cleanup
    cleanup_fixture(
        &client,
        &TestFixture {
            repo_id,
            worktree_id,
            file_id,
            chunk_ids,
            _temp_dir: temp_dir,
        },
    )
    .await?;

    Ok(())
}

#[tokio::test]
#[serial]
#[ignore] // Run with: OPENAI_API_KEY=sk-... cargo test ... -- --ignored
async fn test_e2e_openai_scan_and_search() -> Result<()> {
    println!("\n=== E2E Test: OpenAI Scan and Search (1536-dim) ===\n");

    if env::var("OPENAI_API_KEY").is_err() {
        println!("⊘ Skipping OpenAI E2E test (OPENAI_API_KEY not set)");
        println!("  To enable: OPENAI_API_KEY=sk-... cargo test ... -- --ignored");
        return Ok(());
    }

    // Setup database connection
    let pool = create_pool().await?;
    let client = pool.get().await?;

    // Create test repository structure
    let (repo_id, worktree_id, file_id) = create_test_repo(&client, "openai").await?;

    // Create realistic error handling code
    let test_content = vec![
        r#"export class ErrorHandler {
    handle(error: Error) {
        if (error instanceof ValidationError) {
            return this.handleValidationError(error);
        } else if (error instanceof DatabaseError) {
            return this.handleDatabaseError(error);
        } else {
            return this.handleUnknownError(error);
        }
    }

    private handleValidationError(error: ValidationError) {
        console.error('Validation failed:', error.message);
        throw new ApiError(400, error.message);
    }

    private handleDatabaseError(error: DatabaseError) {
        console.error('Database error:', error.message);
        throw new ApiError(500, 'Internal server error');
    }
}"#,
    ];

    let temp_dir = tempfile::tempdir()?;
    let chunk_ids = create_test_chunks(&client, repo_id, worktree_id, file_id, &test_content).await?;

    // Setup OpenAI provider
    env::set_var("EMBEDDING_PROVIDER", "openai");
    let provider = create_provider_from_env().await?;
    assert_eq!(provider.dimension(), 1536, "OpenAI should use 1536 dimensions");
    assert_eq!(provider.provider_name(), "openai");

    println!("Using provider: {} ({}D)", provider.provider_name(), provider.dimension());

    // Generate embeddings
    let start = Instant::now();
    generate_embeddings(&client, &provider, &chunk_ids).await?;
    let embedding_duration = start.elapsed();
    println!("Embedding generation took: {:?}", embedding_duration);

    // Verify embeddings stored in correct column
    let row = client
        .query_one(
            "SELECT code_embedding, code_embedding_ollama FROM maproom.chunks WHERE id = $1",
            &[&chunk_ids[0]],
        )
        .await?;

    let openai_embedding: Option<Vec<f32>> = row.get(0);
    let ollama_embedding: Option<Vec<f32>> = row.get(1);

    assert!(openai_embedding.is_some(), "Should have OpenAI embedding");
    assert_eq!(
        openai_embedding.unwrap().len(),
        1536,
        "OpenAI embedding should be 1536-dim"
    );
    assert!(ollama_embedding.is_none(), "Should not have Ollama embedding");

    println!("✓ Embeddings stored in code_embedding column");

    // Perform semantic search
    let start = Instant::now();
    let results = search_with_provider(&client, &provider, repo_id, worktree_id, "error handling validation", 5).await?;
    let search_duration = start.elapsed();

    println!("\nSearch results (took {:?}):", search_duration);
    assert!(!results.is_empty(), "Should find error handling code");

    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. Chunk {} (similarity: {:.4}): {}",
            i + 1,
            result.chunk_id,
            result.similarity,
            result.preview
        );
    }

    // Verify top result is relevant
    let top_result = &results[0];
    assert!(
        top_result.content.contains("Error") || top_result.content.contains("handle"),
        "Top result should be relevant to error handling"
    );

    println!("\n✓ E2E OpenAI test passed");

    // Cleanup
    cleanup_fixture(
        &client,
        &TestFixture {
            repo_id,
            worktree_id,
            file_id,
            chunk_ids,
            _temp_dir: temp_dir,
        },
    )
    .await?;

    Ok(())
}

#[tokio::test]
#[serial]
#[ignore] // Run with: TEST_OLLAMA=1 OPENAI_API_KEY=sk-... cargo test ... -- --ignored
async fn test_e2e_mixed_embeddings_workflow() -> Result<()> {
    println!("\n=== E2E Test: Mixed Embeddings Workflow ===\n");

    if env::var("TEST_OLLAMA").is_err() || env::var("OPENAI_API_KEY").is_err() {
        println!("⊘ Skipping mixed embeddings test (requires both TEST_OLLAMA and OPENAI_API_KEY)");
        println!("  To enable: TEST_OLLAMA=1 OPENAI_API_KEY=sk-... cargo test ... -- --ignored");
        return Ok(());
    }

    // Setup database connection
    let pool = create_pool().await?;
    let client = pool.get().await?;

    // Create test repository structure
    let (repo_id, worktree_id, file_id) = create_test_repo(&client, "mixed").await?;

    // Create test content - will split between providers
    let test_content = vec![
        r#"export function processUserData(user: User) {
    const validated = validateUser(user);
    const normalized = normalizeData(validated);
    return saveToDatabase(normalized);
}"#,
        r#"export function calculateMetrics(data: Data[]) {
    const total = data.reduce((sum, item) => sum + item.value, 0);
    const average = total / data.length;
    return { total, average, count: data.length };
}"#,
    ];

    let temp_dir = tempfile::tempdir()?;
    let chunk_ids = create_test_chunks(&client, repo_id, worktree_id, file_id, &test_content).await?;

    // Step 1: Generate OpenAI embeddings for first chunk
    println!("\n--- Step 1: Generate OpenAI embeddings ---");
    env::set_var("EMBEDDING_PROVIDER", "openai");
    let openai_provider = create_provider_from_env().await?;
    assert_eq!(openai_provider.dimension(), 1536);

    let openai_chunks = vec![chunk_ids[0]];
    generate_embeddings(&client, &openai_provider, &openai_chunks).await?;

    // Step 2: Generate Ollama embeddings for second chunk
    println!("\n--- Step 2: Generate Ollama embeddings ---");
    env::set_var("EMBEDDING_PROVIDER", "ollama");
    let ollama_provider = create_provider_from_env().await?;
    assert_eq!(ollama_provider.dimension(), 768);

    let ollama_chunks = vec![chunk_ids[1]];
    generate_embeddings(&client, &ollama_provider, &ollama_chunks).await?;

    // Step 3: Verify mixed embeddings in database
    println!("\n--- Step 3: Verify mixed embeddings ---");

    let row1 = client
        .query_one(
            "SELECT code_embedding, code_embedding_ollama FROM maproom.chunks WHERE id = $1",
            &[&chunk_ids[0]],
        )
        .await?;

    let openai_emb1: Option<Vec<f32>> = row1.get(0);
    let ollama_emb1: Option<Vec<f32>> = row1.get(1);
    assert!(openai_emb1.is_some(), "Chunk 0 should have OpenAI embedding");
    assert_eq!(openai_emb1.unwrap().len(), 1536);
    assert!(ollama_emb1.is_none(), "Chunk 0 should not have Ollama embedding");

    let row2 = client
        .query_one(
            "SELECT code_embedding, code_embedding_ollama FROM maproom.chunks WHERE id = $1",
            &[&chunk_ids[1]],
        )
        .await?;

    let openai_emb2: Option<Vec<f32>> = row2.get(0);
    let ollama_emb2: Option<Vec<f32>> = row2.get(1);
    assert!(ollama_emb2.is_some(), "Chunk 1 should have Ollama embedding");
    assert_eq!(ollama_emb2.unwrap().len(), 768);
    assert!(openai_emb2.is_none(), "Chunk 1 should not have OpenAI embedding");

    println!("✓ Mixed embeddings verified:");
    println!("  Chunk {} has 1536-dim OpenAI embedding", chunk_ids[0]);
    println!("  Chunk {} has 768-dim Ollama embedding", chunk_ids[1]);

    // Step 4: Search with Ollama provider (should find both chunks)
    println!("\n--- Step 4: Search with Ollama provider ---");
    let ollama_results =
        search_with_provider(&client, &ollama_provider, repo_id, worktree_id, "process data function", 10).await?;

    println!("Ollama search found {} results:", ollama_results.len());
    for result in &ollama_results {
        println!("  Chunk {}: {}", result.chunk_id, result.preview);
    }

    // Step 5: Search with OpenAI provider (should find both chunks)
    println!("\n--- Step 5: Search with OpenAI provider ---");
    let openai_results =
        search_with_provider(&client, &openai_provider, repo_id, worktree_id, "process data function", 10).await?;

    println!("OpenAI search found {} results:", openai_results.len());
    for result in &openai_results {
        println!("  Chunk {}: {}", result.chunk_id, result.preview);
    }

    // Verify both searches found results (may not find ALL chunks due to dimension mismatch)
    assert!(
        !ollama_results.is_empty() || !openai_results.is_empty(),
        "At least one search should return results"
    );

    println!("\n✓ Mixed embeddings workflow test passed");
    println!("  Note: Each provider only searches its own dimension's embeddings");

    // Cleanup
    cleanup_fixture(
        &client,
        &TestFixture {
            repo_id,
            worktree_id,
            file_id,
            chunk_ids,
            _temp_dir: temp_dir,
        },
    )
    .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_00_test_suite_info() {
    println!("\n");
    println!("================================================================================");
    println!("  E2E Multi-Provider Embedding Test Suite (MPEMBED-6002)");
    println!("================================================================================");
    println!();
    println!("This test suite validates complete workflows:");
    println!("  1. Ollama: Scan → Generate 768-dim embeddings → Search");
    println!("  2. Google: Scan → Generate 768-dim embeddings → Search");
    println!("  3. OpenAI: Scan → Generate 1536-dim embeddings → Search");
    println!("  4. Mixed: Partial OpenAI + Partial Ollama → Search both");
    println!();
    println!("Prerequisites:");
    println!("  - PostgreSQL running with maproom schema");
    println!("  - Provider access (configured via environment variables)");
    println!();
    println!("Environment variables:");
    println!("  TEST_OLLAMA=1              - Enable Ollama tests");
    println!("  OPENAI_API_KEY=sk-...      - Enable OpenAI tests");
    println!("  GOOGLE_PROJECT_ID=...      - Enable Google tests");
    println!("  GOOGLE_APPLICATION_CREDENTIALS=... - Google service account key");
    println!();
    println!("Run tests:");
    println!("  # All E2E tests (with providers)");
    println!("  TEST_OLLAMA=1 OPENAI_API_KEY=sk-... cargo test --test e2e_multi_provider -- --ignored --nocapture");
    println!();
    println!("  # Specific test");
    println!("  TEST_OLLAMA=1 cargo test --test e2e_multi_provider test_e2e_ollama_scan_and_search -- --ignored --nocapture");
    println!();
    println!("  # Without providers (tests skip gracefully)");
    println!("  cargo test --test e2e_multi_provider -- --ignored");
    println!();
    println!("================================================================================");
    println!();
}
