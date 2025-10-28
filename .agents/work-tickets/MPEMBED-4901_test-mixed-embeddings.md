# Ticket: MPEMBED-4901: Test search with mixed embeddings

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
Create integration test with fixture containing 50 chunks with OpenAI embeddings (1536-dim) and 50 chunks with Ollama embeddings (768-dim). Verify hybrid search returns results from both embedding types and COALESCE preference order works correctly.

## Background
This ticket provides comprehensive integration testing for Phase 4 (Database and Search Integration) to ensure the COALESCE logic works correctly with real mixed-dimension embeddings. The test validates that the system can handle repositories partially indexed with OpenAI and partially indexed with Ollama, which represents a realistic migration scenario.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-4-database-search-integration.md

## Acceptance Criteria
- [ ] Fixture creates 50 chunks with OpenAI embeddings (1536-dim)
- [ ] Fixture creates 50 chunks with Ollama embeddings (768-dim)
- [ ] Fixture creates 10 chunks with BOTH embeddings (tests COALESCE preference)
- [ ] Hybrid search returns results from OpenAI-only chunks
- [ ] Hybrid search returns results from Ollama-only chunks
- [ ] Hybrid search returns results from both-embeddings chunks
- [ ] COALESCE prefers Ollama over OpenAI (when both present)
- [ ] Vector search with 768-dim query finds Ollama chunks
- [ ] Vector search with 1536-dim query finds OpenAI chunks
- [ ] FTS-only search unaffected by embedding dimension
- [ ] Test documents actual vs expected ranking

## Technical Requirements
- Use sqlx::test for database setup/teardown
- Create realistic code content (different programming languages)
- Generate actual embeddings (not random vectors) for relevance testing
- Test all search modes: hybrid, vector-only, FTS-only
- Verify cosine similarity scores are reasonable (> 0.5 for relevant results)
- Test edge cases: NULL embeddings, partial embeddings
- Measure and assert search latency (< 100ms for 110 chunks)
- Document test fixture structure for future reference

## Implementation Notes
**Test Structure:**
```rust
// crates/maproom/tests/mixed_embeddings_search.rs
#![cfg(test)]

use maproom::db::chunks::*;
use maproom::embedding::factory::create_provider;
use maproom::search::hybrid::*;
use sqlx::PgPool;
use uuid::Uuid;

/// Create test fixture with mixed embeddings
async fn setup_mixed_embeddings_fixture(pool: &PgPool) -> Result<MixedEmbeddingFixture> {
    // Create 50 chunks with OpenAI embeddings (1536-dim)
    let mut openai_chunks = Vec::new();
    let openai_provider = create_provider("openai")?;

    for i in 0..50 {
        let chunk_id = Uuid::new_v4();
        let content = format!("function processData{i}() {{ return data.map(x => x * 2); }}");

        // Insert chunk
        sqlx::query!(
            r#"
            INSERT INTO chunks (id, repo, worktree, relpath, start_line, end_line, content, language)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            chunk_id, "test", "main", format!("openai_{i}.ts"), 1, 10, content, "typescript"
        )
        .execute(pool)
        .await?;

        // Generate and insert OpenAI embedding
        let embeddings = openai_provider.embed(vec![content.clone()]).await?;
        sqlx::query!(
            "UPDATE chunks SET code_embedding = $1 WHERE id = $2",
            embeddings[0].as_slice(),
            chunk_id
        )
        .execute(pool)
        .await?;

        openai_chunks.push(chunk_id);
    }

    // Create 50 chunks with Ollama embeddings (768-dim)
    let mut ollama_chunks = Vec::new();
    let ollama_provider = create_provider("ollama")?;

    for i in 0..50 {
        let chunk_id = Uuid::new_v4();
        let content = format!("async fn handle_request{i}(req: Request) -> Response {{ process(req).await }}");

        sqlx::query!(
            r#"
            INSERT INTO chunks (id, repo, worktree, relpath, start_line, end_line, content, language)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            chunk_id, "test", "main", format!("ollama_{i}.rs"), 1, 10, content, "rust"
        )
        .execute(pool)
        .await?;

        // Generate and insert Ollama embedding
        let embeddings = ollama_provider.embed(vec![content.clone()]).await?;
        sqlx::query!(
            "UPDATE chunks SET code_embedding_ollama = $1 WHERE id = $2",
            embeddings[0].as_slice(),
            chunk_id
        )
        .execute(pool)
        .await?;

        ollama_chunks.push(chunk_id);
    }

    // Create 10 chunks with BOTH embeddings (test COALESCE preference)
    let mut both_chunks = Vec::new();

    for i in 0..10 {
        let chunk_id = Uuid::new_v4();
        let content = format!("class DataProcessor{i} {{ process() {{ return this.data; }} }}");

        sqlx::query!(
            r#"
            INSERT INTO chunks (id, repo, worktree, relpath, start_line, end_line, content, language)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            chunk_id, "test", "main", format!("both_{i}.js"), 1, 10, content, "javascript"
        )
        .execute(pool)
        .await?;

        // Generate both embeddings
        let openai_emb = openai_provider.embed(vec![content.clone()]).await?;
        let ollama_emb = ollama_provider.embed(vec![content.clone()]).await?;

        sqlx::query!(
            "UPDATE chunks SET code_embedding = $1, code_embedding_ollama = $2 WHERE id = $3",
            openai_emb[0].as_slice(),
            ollama_emb[0].as_slice(),
            chunk_id
        )
        .execute(pool)
        .await?;

        both_chunks.push(chunk_id);
    }

    Ok(MixedEmbeddingFixture {
        openai_chunks,
        ollama_chunks,
        both_chunks,
    })
}

struct MixedEmbeddingFixture {
    openai_chunks: Vec<Uuid>,
    ollama_chunks: Vec<Uuid>,
    both_chunks: Vec<Uuid>,
}

#[sqlx::test]
async fn test_hybrid_search_finds_both_embedding_types(pool: PgPool) -> Result<()> {
    let fixture = setup_mixed_embeddings_fixture(&pool).await?;

    // Search with query related to both TypeScript and Rust code
    let query_text = "process data function";
    let query_provider = create_provider("ollama")?;
    let query_emb = query_provider.embed(vec![query_text.to_string()]).await?;

    let results = hybrid_search(
        &pool,
        query_text,
        query_emb[0].clone(),
        "test",
        "main",
        50,
        0.5,
        0.5,
    )
    .await?;

    // Should find results from both OpenAI and Ollama chunks
    let openai_results: Vec<_> = results
        .iter()
        .filter(|r| fixture.openai_chunks.contains(&r.chunk_id))
        .collect();
    let ollama_results: Vec<_> = results
        .iter()
        .filter(|r| fixture.ollama_chunks.contains(&r.chunk_id))
        .collect();

    assert!(!openai_results.is_empty(), "Should find OpenAI chunks");
    assert!(!ollama_results.is_empty(), "Should find Ollama chunks");

    println!("Found {} OpenAI chunks, {} Ollama chunks", openai_results.len(), ollama_results.len());

    Ok(())
}

#[sqlx::test]
async fn test_coalesce_prefers_ollama_over_openai(pool: PgPool) -> Result<()> {
    let fixture = setup_mixed_embeddings_fixture(&pool).await?;

    // Search with Ollama query
    let query_provider = create_provider("ollama")?;
    let query_emb = query_provider.embed(vec!["DataProcessor".to_string()]).await?;

    let results = vector_search(
        &pool,
        query_emb[0].clone(),
        "test",
        "main",
        20,
    )
    .await?;

    // Find results that have both embeddings
    let both_results: Vec<_> = results
        .iter()
        .filter(|r| fixture.both_chunks.contains(&r.chunk_id))
        .collect();

    // All should indicate they used 768-dim (Ollama) embedding
    for result in both_results {
        assert_eq!(
            result.embedding_dimension,
            Some("768".to_string()),
            "Chunk {} should prefer Ollama (768-dim) embedding",
            result.chunk_id
        );
    }

    Ok(())
}

#[sqlx::test]
async fn test_vector_search_768_finds_ollama_chunks(pool: PgPool) -> Result<()> {
    let fixture = setup_mixed_embeddings_fixture(&pool).await?;

    // Create 768-dim query
    let query_provider = create_provider("ollama")?;
    let query_emb = query_provider.embed(vec!["async function handler".to_string()]).await?;

    let results = vector_search(
        &pool,
        query_emb[0].clone(),
        "test",
        "main",
        10,
    )
    .await?;

    // Should primarily find Ollama chunks (Rust code)
    let ollama_count = results
        .iter()
        .filter(|r| fixture.ollama_chunks.contains(&r.chunk_id))
        .count();

    assert!(ollama_count > 5, "Should find mostly Ollama chunks for Rust query");

    Ok(())
}

#[sqlx::test]
async fn test_vector_search_1536_finds_openai_chunks(pool: PgPool) -> Result<()> {
    let fixture = setup_mixed_embeddings_fixture(&pool).await?;

    // Create 1536-dim query
    let query_provider = create_provider("openai")?;
    let query_emb = query_provider.embed(vec!["map function data".to_string()]).await?;

    let results = vector_search(
        &pool,
        query_emb[0].clone(),
        "test",
        "main",
        10,
    )
    .await?;

    // Should primarily find OpenAI chunks (TypeScript code)
    let openai_count = results
        .iter()
        .filter(|r| fixture.openai_chunks.contains(&r.chunk_id))
        .count();

    assert!(openai_count > 5, "Should find mostly OpenAI chunks for TypeScript query");

    Ok(())
}

#[sqlx::test]
async fn test_fts_search_unaffected_by_embeddings(pool: PgPool) -> Result<()> {
    let fixture = setup_mixed_embeddings_fixture(&pool).await?;

    // FTS-only search (no vector component)
    let results = hybrid_search(
        &pool,
        "DataProcessor",
        vec![0.0; 768], // Dummy embedding (FTS weight = 1.0)
        "test",
        "main",
        20,
        1.0, // FTS weight
        0.0, // Vector weight
    )
    .await?;

    // Should find "both" chunks which contain "DataProcessor" in content
    let both_count = results
        .iter()
        .filter(|r| fixture.both_chunks.contains(&r.chunk_id))
        .count();

    assert_eq!(both_count, 10, "Should find all 10 chunks with 'DataProcessor'");

    Ok(())
}

#[sqlx::test]
async fn test_search_performance_with_mixed_embeddings(pool: PgPool) -> Result<()> {
    setup_mixed_embeddings_fixture(&pool).await?;

    let query_provider = create_provider("ollama")?;
    let query_emb = query_provider.embed(vec!["test query".to_string()]).await?;

    // Measure search latency
    let start = std::time::Instant::now();

    let results = hybrid_search(
        &pool,
        "test query",
        query_emb[0].clone(),
        "test",
        "main",
        50,
        0.5,
        0.5,
    )
    .await?;

    let latency = start.elapsed();

    println!("Search latency: {:?}", latency);
    println!("Results: {} chunks", results.len());

    // Assert reasonable performance (< 100ms for 110 chunks)
    assert!(latency.as_millis() < 100, "Search should complete in < 100ms");

    Ok(())
}
```

## Dependencies
- MPEMBED-4003 (Updated search queries with COALESCE must exist)

## Risk Assessment
- **Risk**: Test requires both OpenAI and Ollama providers configured
  - **Mitigation**: Use test fixtures with pre-generated embeddings, or mark as #[ignore] for CI
- **Risk**: Embedding generation in tests may be slow
  - **Mitigation**: Cache embeddings or use smaller test fixture (10+10+5 instead of 50+50+10)
- **Risk**: Test may be flaky due to similarity score variations
  - **Mitigation**: Use relaxed assertions (> 0, not exact matches), focus on presence not ranking

## Files/Packages Affected
- crates/maproom/tests/mixed_embeddings_search.rs (create)
- crates/maproom/tests/fixtures/mixed_embeddings.sql (create - optional SQL fixture)
