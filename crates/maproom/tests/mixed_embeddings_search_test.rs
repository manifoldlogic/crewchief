//! Integration tests for mixed embedding search with COALESCE pattern.
//!
//! These tests verify that vector search correctly handles chunks with different
//! embedding dimensions (768-dim from Ollama/Google, 1536-dim from OpenAI) and
//! properly prefers 768-dim embeddings when both are present.

use crewchief_maproom::search::{SearchMode, VectorExecutor};
use tokio_postgres::{Client, NoTls};

/// Helper function to establish database connection.
async fn get_test_client() -> Result<Client, Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@postgres:5432/crewchief".to_string());

    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    Ok(client)
}

/// Helper function to create test data with mixed embeddings.
async fn setup_test_data(client: &Client) -> Result<(i64, i64), Box<dyn std::error::Error>> {
    // Create test repo
    let repo_row = client
        .query_one(
            "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET root_path = EXCLUDED.root_path
             RETURNING id",
            &[&"test_mixed_embeddings", &"/tmp/test"],
        )
        .await?;
    let repo_id: i64 = repo_row.get(0);

    // Create test worktree
    let worktree_row = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3)
             ON CONFLICT (repo_id, name) DO UPDATE SET abs_path = EXCLUDED.abs_path
             RETURNING id",
            &[&repo_id, &"main", &"/tmp/test"],
        )
        .await?;
    let worktree_id: i64 = worktree_row.get(0);

    // Create test commit
    let commit_row = client
        .query_one(
            "INSERT INTO maproom.commits (repo_id, sha, committed_at) VALUES ($1, $2, NOW())
             ON CONFLICT (repo_id, sha) DO UPDATE SET committed_at = NOW()
             RETURNING id",
            &[&repo_id, &"abc123"],
        )
        .await?;
    let commit_id: i64 = commit_row.get(0);

    // Create test file
    let file_row = client
        .query_one(
            "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes, last_modified)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
             ON CONFLICT (commit_id, relpath, content_hash) DO UPDATE SET size_bytes = EXCLUDED.size_bytes
             RETURNING id",
            &[&repo_id, &worktree_id, &commit_id, &"test.rs", &"rust", &"hash123", &100],
        )
        .await?;
    let file_id: i64 = file_row.get(0);

    // Create chunks with different embedding configurations

    // Chunk 1: Only 768-dim Ollama embeddings
    let code_emb_768 = vec![0.1f32; 768];
    let text_emb_768 = vec![0.1f32; 768];
    client
        .execute(
            "INSERT INTO maproom.chunks (file_id, start_line, end_line, preview, code_embedding_ollama, text_embedding_ollama)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (file_id, start_line, end_line) DO UPDATE SET preview = EXCLUDED.preview",
            &[&file_id, &1, &10, &"chunk with 768-dim only", &code_emb_768, &text_emb_768],
        )
        .await?;

    // Chunk 2: Only 1536-dim OpenAI embeddings
    let code_emb_1536 = vec![0.2f32; 1536];
    let text_emb_1536 = vec![0.2f32; 1536];
    client
        .execute(
            "INSERT INTO maproom.chunks (file_id, start_line, end_line, preview, code_embedding, text_embedding)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (file_id, start_line, end_line) DO UPDATE SET preview = EXCLUDED.preview",
            &[&file_id, &11, &20, &"chunk with 1536-dim only", &code_emb_1536, &text_emb_1536],
        )
        .await?;

    // Chunk 3: Both 768-dim and 1536-dim embeddings (should prefer 768-dim)
    let code_emb_768_both = vec![0.3f32; 768];
    let text_emb_768_both = vec![0.3f32; 768];
    let code_emb_1536_both = vec![0.4f32; 1536];
    let text_emb_1536_both = vec![0.4f32; 1536];
    client
        .execute(
            "INSERT INTO maproom.chunks (file_id, start_line, end_line, preview, code_embedding_ollama, text_embedding_ollama, code_embedding, text_embedding)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (file_id, start_line, end_line) DO UPDATE SET preview = EXCLUDED.preview",
            &[
                &file_id,
                &21,
                &30,
                &"chunk with both dimensions",
                &code_emb_768_both,
                &text_emb_768_both,
                &code_emb_1536_both,
                &text_emb_1536_both,
            ],
        )
        .await?;

    Ok((repo_id, worktree_id))
}

/// Helper function to cleanup test data.
async fn cleanup_test_data(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "DELETE FROM maproom.repos WHERE name = $1",
            &[&"test_mixed_embeddings"],
        )
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup and migration 0015
async fn test_vector_search_with_768_dim_query() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let (repo_id, worktree_id) = setup_test_data(&client).await?;

    // Search with 768-dim query
    let query_embedding = vec![0.15f32; 768];
    let results = VectorExecutor::execute(
        &client,
        &query_embedding,
        SearchMode::Code,
        repo_id,
        Some(worktree_id),
        10,
    )
    .await?;

    // Should find chunks with 768-dim embeddings
    assert!(
        results.len() >= 2,
        "Should find at least 2 chunks (768-only and both)"
    );

    // Verify that results include embedding dimension information
    let has_dimension_info = results
        .results
        .iter()
        .any(|r| r.embedding_dimension.is_some());
    assert!(
        has_dimension_info,
        "Results should include embedding dimension information"
    );

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup and migration 0015
async fn test_vector_search_with_1536_dim_query() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let (repo_id, worktree_id) = setup_test_data(&client).await?;

    // Search with 1536-dim query
    let query_embedding = vec![0.25f32; 1536];
    let results = VectorExecutor::execute(
        &client,
        &query_embedding,
        SearchMode::Code,
        repo_id,
        Some(worktree_id),
        10,
    )
    .await?;

    // Should find chunks with embeddings (all three chunks)
    assert!(
        results.len() >= 3,
        "Should find at least 3 chunks with 1536-dim query"
    );

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup and migration 0015
async fn test_coalesce_prefers_768_dim() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let (repo_id, _worktree_id) = setup_test_data(&client).await?;

    // Query the chunk that has both 768 and 1536 dim embeddings
    let query = r#"
        SELECT
            CASE
                WHEN c.code_embedding_ollama IS NOT NULL THEN '768'
                WHEN c.code_embedding IS NOT NULL THEN '1536'
                ELSE NULL
            END as embedding_dimension
        FROM maproom.chunks c
        JOIN maproom.files f ON f.id = c.file_id
        WHERE f.repo_id = $1
          AND c.code_embedding_ollama IS NOT NULL
          AND c.code_embedding IS NOT NULL
    "#;

    let rows = client.query(query, &[&repo_id]).await?;

    // Should have at least one row (chunk 3)
    assert!(!rows.is_empty(), "Should find chunk with both embeddings");

    // Verify it reports 768 dimension (COALESCE preference)
    for row in rows {
        let dimension: Option<String> = row.get(0);
        assert_eq!(
            dimension,
            Some("768".to_string()),
            "Should prefer 768-dim embedding when both are present"
        );
    }

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup and migration 0015
async fn test_hybrid_search_with_mixed_embeddings() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let (repo_id, worktree_id) = setup_test_data(&client).await?;

    // Search with 768-dim query in hybrid mode
    let query_embedding = vec![0.15f32; 768];
    let results = VectorExecutor::execute(
        &client,
        &query_embedding,
        SearchMode::Auto,
        repo_id,
        Some(worktree_id),
        10,
    )
    .await?;

    // Should find chunks with embeddings
    assert!(
        results.len() >= 2,
        "Hybrid search should find chunks with 768-dim embeddings"
    );

    // Verify embedding dimension information
    for result in &results.results {
        assert!(
            result.embedding_dimension.is_some(),
            "Hybrid results should include embedding dimension"
        );
        let dim = result.embedding_dimension.as_ref().unwrap();
        assert!(
            dim == "768" || dim == "1536",
            "Embedding dimension should be either 768 or 1536"
        );
    }

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup and migration 0015
async fn test_text_mode_with_mixed_embeddings() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let (repo_id, worktree_id) = setup_test_data(&client).await?;

    // Search with 768-dim query in text mode
    let query_embedding = vec![0.15f32; 768];
    let results = VectorExecutor::execute(
        &client,
        &query_embedding,
        SearchMode::Text,
        repo_id,
        Some(worktree_id),
        10,
    )
    .await?;

    // Should find chunks with text embeddings
    assert!(
        results.len() >= 2,
        "Text search should find chunks with 768-dim text embeddings"
    );

    // Verify embedding dimension information
    for result in &results.results {
        assert!(
            result.embedding_dimension.is_some(),
            "Text search results should include embedding dimension"
        );
    }

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup and migration 0015
async fn test_empty_query_embedding() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let (repo_id, worktree_id) = setup_test_data(&client).await?;

    // Search with empty query embedding
    let query_embedding = vec![];
    let results = VectorExecutor::execute(
        &client,
        &query_embedding,
        SearchMode::Code,
        repo_id,
        Some(worktree_id),
        10,
    )
    .await?;

    // Should return empty results
    assert_eq!(results.len(), 0, "Empty query should return no results");

    cleanup_test_data(&client).await?;
    Ok(())
}

#[tokio::test]
#[ignore] // Requires database setup and migration 0015
async fn test_scoring_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_test_client().await?;
    let (repo_id, worktree_id) = setup_test_data(&client).await?;

    // Search with 768-dim query
    let query_embedding = vec![0.15f32; 768];
    let results = VectorExecutor::execute(
        &client,
        &query_embedding,
        SearchMode::Code,
        repo_id,
        Some(worktree_id),
        10,
    )
    .await?;

    // Verify scores are in valid range and sorted
    let mut prev_score = 1.0;
    for result in &results.results {
        assert!(
            result.score >= 0.0 && result.score <= 1.0,
            "Score should be in range [0.0, 1.0], got {}",
            result.score
        );
        assert!(
            result.score <= prev_score,
            "Results should be sorted by score descending"
        );
        prev_score = result.score;
    }

    cleanup_test_data(&client).await?;
    Ok(())
}
