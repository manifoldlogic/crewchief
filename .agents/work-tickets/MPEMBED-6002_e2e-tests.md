# Ticket: MPEMBED-6002: End-to-end multi-provider tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Create end-to-end test scenarios covering: Scan with Ollama → Search, Scan with Google → Search, Scan with OpenAI → Search, and Mixed embeddings scenario. Verify complete workflows from indexing to retrieval.

## Background
This ticket implements comprehensive E2E testing for Phase 6 (Testing and Validation) to ensure the entire multi-provider system works correctly from scan to search. These tests validate the integration of all components: provider abstraction, database layer, search queries, and MCP tools.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-6-testing-validation.md

## Acceptance Criteria
- [ ] E2E test: Scan with Ollama, then search (768-dim workflow)
- [ ] E2E test: Scan with Google, then search (768-dim workflow)
- [ ] E2E test: Scan with OpenAI, then search (1536-dim workflow)
- [ ] E2E test: Mixed embeddings (partial Ollama, partial OpenAI, search both)
- [ ] All tests pass with real database
- [ ] Tests verify search returns correct results
- [ ] Tests verify embedding dimensions stored correctly
- [ ] Tests clean up after themselves

## Technical Requirements
- Use sqlx::test for database setup/teardown
- Create test fixtures with realistic code samples
- Test complete workflows: scan → database → search
- Verify search ranking quality
- Test incremental updates
- Measure end-to-end latency
- Document test scenarios for future reference

## Implementation Notes
```rust
// crates/maproom/tests/e2e_multi_provider.rs
#![cfg(test)]

use maproom::embedding::factory::create_provider;
use maproom::embedding::pipeline::EmbeddingPipeline;
use maproom::search::hybrid::hybrid_search;
use maproom::scanner::Scanner;
use sqlx::PgPool;
use std::path::PathBuf;

#[sqlx::test]
async fn e2e_ollama_scan_and_search(pool: PgPool) -> anyhow::Result<()> {
    if std::env::var("TEST_OLLAMA").is_err() {
        println!("Skipping Ollama E2E test (TEST_OLLAMA not set)");
        return Ok(());
    }

    // Setup: Create test repository files
    let temp_dir = tempfile::tempdir()?;
    let test_file = temp_dir.path().join("auth.ts");
    std::fs::write(
        &test_file,
        r#"
        export async function authenticate(username: string, password: string) {
            const user = await db.findUser(username);
            if (!user) throw new Error('User not found');
            const valid = await bcrypt.compare(password, user.passwordHash);
            if (!valid) throw new Error('Invalid password');
            return createSession(user);
        }
        "#,
    )?;

    // Step 1: Scan with Ollama provider
    std::env::set_var("EMBEDDING_PROVIDER", "ollama");
    let provider = create_provider("ollama")?;
    assert_eq!(provider.dimension(), 768);

    let scanner = Scanner::new(pool.clone());
    let scan_result = scanner.scan(
        temp_dir.path(),
        "test-repo",
        "main",
    ).await?;

    println!("Scanned {} chunks", scan_result.chunk_ids.len());
    assert!(scan_result.chunk_ids.len() > 0);

    // Step 2: Generate embeddings
    let pipeline = EmbeddingPipeline::new(provider.clone(), pool.clone());
    let stats = pipeline.process_chunks(scan_result.chunk_ids.clone()).await?;

    println!("Generated embeddings: {}", stats);
    assert_eq!(stats.chunks_processed, scan_result.chunk_ids.len());
    assert_eq!(stats.dimension, 768);

    // Step 3: Verify embeddings stored in correct columns
    let row = sqlx::query!(
        "SELECT code_embedding_ollama, code_embedding FROM chunks WHERE id = $1",
        scan_result.chunk_ids[0]
    )
    .fetch_one(&pool)
    .await?;

    assert!(row.code_embedding_ollama.is_some(), "Should have Ollama embedding");
    assert_eq!(row.code_embedding_ollama.unwrap().len(), 768);
    assert!(row.code_embedding.is_none(), "Should not have OpenAI embedding");

    // Step 4: Search for authentication code
    let query_emb = provider.embed(vec!["authentication password validation".to_string()]).await?;
    let results = hybrid_search(
        &pool,
        "authentication password",
        query_emb[0].clone(),
        "test-repo",
        "main",
        10,
        0.5,
        0.5,
    ).await?;

    println!("Search found {} results", results.len());
    assert!(results.len() > 0, "Should find authentication code");

    // Verify result relevance
    let top_result = &results[0];
    assert!(top_result.content.contains("authenticate"));
    assert_eq!(top_result.embedding_dimension, Some("768".to_string()));

    Ok(())
}

#[sqlx::test]
async fn e2e_google_scan_and_search(pool: PgPool) -> anyhow::Result<()> {
    if std::env::var("GOOGLE_PROJECT_ID").is_err() {
        println!("Skipping Google E2E test (GOOGLE_PROJECT_ID not set)");
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;
    let test_file = temp_dir.path().join("db.ts");
    std::fs::write(
        &test_file,
        r#"
        export class DatabaseConnection {
            async query(sql: string, params: any[]) {
                const result = await this.pool.query(sql, params);
                return result.rows;
            }
        }
        "#,
    )?;

    // Scan with Google provider
    std::env::set_var("EMBEDDING_PROVIDER", "google");
    let provider = create_provider("google")?;
    assert_eq!(provider.dimension(), 768);

    let scanner = Scanner::new(pool.clone());
    let scan_result = scanner.scan(temp_dir.path(), "test-repo", "main").await?;

    let pipeline = EmbeddingPipeline::new(provider.clone(), pool.clone());
    let stats = pipeline.process_chunks(scan_result.chunk_ids.clone()).await?;

    assert_eq!(stats.dimension, 768);

    // Search for database code
    let query_emb = provider.embed(vec!["database query".to_string()]).await?;
    let results = hybrid_search(
        &pool,
        "database query",
        query_emb[0].clone(),
        "test-repo",
        "main",
        10,
        0.5,
        0.5,
    ).await?;

    assert!(results.len() > 0);
    assert!(results[0].content.contains("query"));

    Ok(())
}

#[sqlx::test]
async fn e2e_openai_scan_and_search(pool: PgPool) -> anyhow::Result<()> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        println!("Skipping OpenAI E2E test (OPENAI_API_KEY not set)");
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;
    let test_file = temp_dir.path().join("error.ts");
    std::fs::write(
        &test_file,
        r#"
        export class ErrorHandler {
            handle(error: Error) {
                console.error(error.message);
                throw error;
            }
        }
        "#,
    )?;

    // Scan with OpenAI provider
    std::env::set_var("EMBEDDING_PROVIDER", "openai");
    let provider = create_provider("openai")?;
    assert_eq!(provider.dimension(), 1536);

    let scanner = Scanner::new(pool.clone());
    let scan_result = scanner.scan(temp_dir.path(), "test-repo", "main").await?;

    let pipeline = EmbeddingPipeline::new(provider.clone(), pool.clone());
    let stats = pipeline.process_chunks(scan_result.chunk_ids.clone()).await?;

    assert_eq!(stats.dimension, 1536);

    // Verify stored in original columns
    let row = sqlx::query!(
        "SELECT code_embedding, code_embedding_ollama FROM chunks WHERE id = $1",
        scan_result.chunk_ids[0]
    )
    .fetch_one(&pool)
    .await?;

    assert!(row.code_embedding.is_some());
    assert_eq!(row.code_embedding.unwrap().len(), 1536);
    assert!(row.code_embedding_ollama.is_none());

    // Search
    let query_emb = provider.embed(vec!["error handling".to_string()]).await?;
    let results = hybrid_search(
        &pool,
        "error handling",
        query_emb[0].clone(),
        "test-repo",
        "main",
        10,
        0.5,
        0.5,
    ).await?;

    assert!(results.len() > 0);
    assert_eq!(results[0].embedding_dimension, Some("1536".to_string()));

    Ok(())
}

#[sqlx::test]
async fn e2e_mixed_embeddings_workflow(pool: PgPool) -> anyhow::Result<()> {
    if std::env::var("TEST_OLLAMA").is_err() || std::env::var("OPENAI_API_KEY").is_err() {
        println!("Skipping mixed embeddings test (requires both Ollama and OpenAI)");
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;

    // Create file 1: will be indexed with OpenAI
    let file1 = temp_dir.path().join("openai.ts");
    std::fs::write(&file1, "export function openaiFunction() { return 'openai'; }")?;

    // Create file 2: will be indexed with Ollama
    let file2 = temp_dir.path().join("ollama.ts");
    std::fs::write(&file2, "export function ollamaFunction() { return 'ollama'; }")?;

    // Step 1: Scan and embed with OpenAI
    std::env::set_var("EMBEDDING_PROVIDER", "openai");
    let openai_provider = create_provider("openai")?;

    let scanner = Scanner::new(pool.clone());
    let scan_result = scanner.scan(temp_dir.path(), "test-repo", "main").await?;
    let openai_chunks: Vec<_> = scan_result.chunk_ids.iter()
        .take(scan_result.chunk_ids.len() / 2)
        .copied()
        .collect();

    let pipeline = EmbeddingPipeline::new(openai_provider.clone(), pool.clone());
    pipeline.process_chunks(openai_chunks).await?;

    // Step 2: Scan and embed with Ollama
    std::env::set_var("EMBEDDING_PROVIDER", "ollama");
    let ollama_provider = create_provider("ollama")?;

    let ollama_chunks: Vec<_> = scan_result.chunk_ids.iter()
        .skip(scan_result.chunk_ids.len() / 2)
        .copied()
        .collect();

    let pipeline = EmbeddingPipeline::new(ollama_provider.clone(), pool.clone());
    pipeline.process_chunks(ollama_chunks).await?;

    // Step 3: Search with Ollama query
    let query_emb = ollama_provider.embed(vec!["function".to_string()]).await?;
    let results = hybrid_search(
        &pool,
        "function",
        query_emb[0].clone(),
        "test-repo",
        "main",
        10,
        0.5,
        0.5,
    ).await?;

    // Should find results from both providers
    println!("Mixed search found {} results", results.len());
    assert!(results.len() >= 2, "Should find results from both providers");

    let has_768 = results.iter().any(|r| r.embedding_dimension == Some("768".to_string()));
    let has_1536 = results.iter().any(|r| r.embedding_dimension == Some("1536".to_string()));

    assert!(has_768, "Should find 768-dim (Ollama) results");
    assert!(has_1536, "Should find 1536-dim (OpenAI) results");

    Ok(())
}
```

## Dependencies
- MPEMBED-4004 (Embedding pipeline integration must be complete)
- MPEMBED-5003 (CLI provider flag must exist)

## Risk Assessment
- **Risk**: E2E tests may be slow due to real embedding generation
  - **Mitigation**: Use small test files, mark as #[ignore] for quick test runs
- **Risk**: Tests require multiple provider configurations
  - **Mitigation**: Skip tests gracefully if providers not configured

## Files/Packages Affected
- crates/maproom/tests/e2e_multi_provider.rs (create)
