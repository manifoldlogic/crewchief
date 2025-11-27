//! Integration tests for multi-dimension embedding upsert functions.
//!
//! These tests verify that the upsert_embeddings and batch_upsert_embeddings
//! functions correctly route embeddings to the appropriate database columns
//! based on dimension (768 -> *_ollama columns, 1536 -> original columns).

use anyhow::Result;

/// Test upserting 768-dimensional embeddings to *_ollama columns.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_upsert_768_dimension_embeddings() -> Result<()> {
    use crewchief_maproom::db::queries::{connect, migrate, upsert_embeddings};

    let client = connect().await?;
    migrate(&client).await?;

    // Create a test chunk
    let chunk_id: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                (SELECT id FROM maproom.files LIMIT 1),
                'test_function_768',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[],
        )
        .await?
        .get(0);

    // Create 768-dimensional embeddings
    let code_emb = vec![0.1f32; 768];
    let text_emb = vec![0.2f32; 768];

    // Upsert with dimension 768
    upsert_embeddings(&client, chunk_id, Some(&code_emb), Some(&text_emb), 768).await?;

    // Verify stored in ollama columns
    let row = client
        .query_one(
            "SELECT
                code_embedding_ollama IS NOT NULL as has_code_ollama,
                text_embedding_ollama IS NOT NULL as has_text_ollama,
                code_embedding IS NULL as code_null,
                text_embedding IS NULL as text_null,
                array_length(code_embedding_ollama, 1) as code_dim,
                array_length(text_embedding_ollama, 1) as text_dim
             FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await?;

    assert!(
        row.get::<_, bool>("has_code_ollama"),
        "code_embedding_ollama should not be NULL"
    );
    assert!(
        row.get::<_, bool>("has_text_ollama"),
        "text_embedding_ollama should not be NULL"
    );
    assert!(
        row.get::<_, bool>("code_null"),
        "code_embedding should be NULL"
    );
    assert!(
        row.get::<_, bool>("text_null"),
        "text_embedding should be NULL"
    );
    assert_eq!(
        row.get::<_, i32>("code_dim"),
        768,
        "code_embedding_ollama should have 768 dimensions"
    );
    assert_eq!(
        row.get::<_, i32>("text_dim"),
        768,
        "text_embedding_ollama should have 768 dimensions"
    );

    Ok(())
}

/// Test upserting 1536-dimensional embeddings to original columns.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_upsert_1536_dimension_embeddings() -> Result<()> {
    use crewchief_maproom::db::queries::{connect, migrate, upsert_embeddings};

    let client = connect().await?;
    migrate(&client).await?;

    // Create a test chunk
    let chunk_id: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                (SELECT id FROM maproom.files LIMIT 1),
                'test_function_1536',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[],
        )
        .await?
        .get(0);

    // Create 1536-dimensional embeddings
    let code_emb = vec![0.3f32; 1536];
    let text_emb = vec![0.4f32; 1536];

    // Upsert with dimension 1536
    upsert_embeddings(&client, chunk_id, Some(&code_emb), Some(&text_emb), 1536).await?;

    // Verify stored in original columns
    let row = client
        .query_one(
            "SELECT
                code_embedding IS NOT NULL as has_code,
                text_embedding IS NOT NULL as has_text,
                code_embedding_ollama IS NULL as code_ollama_null,
                text_embedding_ollama IS NULL as text_ollama_null,
                array_length(code_embedding, 1) as code_dim,
                array_length(text_embedding, 1) as text_dim
             FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await?;

    assert!(
        row.get::<_, bool>("has_code"),
        "code_embedding should not be NULL"
    );
    assert!(
        row.get::<_, bool>("has_text"),
        "text_embedding should not be NULL"
    );
    assert!(
        row.get::<_, bool>("code_ollama_null"),
        "code_embedding_ollama should be NULL"
    );
    assert!(
        row.get::<_, bool>("text_ollama_null"),
        "text_embedding_ollama should be NULL"
    );
    assert_eq!(
        row.get::<_, i32>("code_dim"),
        1536,
        "code_embedding should have 1536 dimensions"
    );
    assert_eq!(
        row.get::<_, i32>("text_dim"),
        1536,
        "text_embedding should have 1536 dimensions"
    );

    Ok(())
}

/// Test that dimension mismatch returns an error.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_dimension_mismatch_error() -> Result<()> {
    use crewchief_maproom::db::queries::{connect, migrate, upsert_embeddings};

    let client = connect().await?;
    migrate(&client).await?;

    // Create a test chunk
    let chunk_id: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                (SELECT id FROM maproom.files LIMIT 1),
                'test_mismatch',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[],
        )
        .await?
        .get(0);

    // Try to upsert 1536-dimensional embedding with dimension=768
    let code_emb = vec![0.1f32; 1536];
    let result = upsert_embeddings(&client, chunk_id, Some(&code_emb), None, 768).await;

    assert!(
        result.is_err(),
        "Should return error for dimension mismatch"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("does not match dimension"),
        "Error message should mention dimension mismatch: {}",
        err_msg
    );

    Ok(())
}

/// Test batch upserting 768-dimensional embeddings.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_batch_upsert_768_dimension() -> Result<()> {
    use crewchief_maproom::db::queries::{batch_upsert_embeddings, connect, migrate};

    let mut client = connect().await?;
    migrate(&client).await?;

    // Create test chunks
    let mut chunk_ids = Vec::new();
    for i in 0..3 {
        let chunk_id: i64 = client
            .query_one(
                "INSERT INTO maproom.chunks (
                    file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
                 ) VALUES (
                    (SELECT id FROM maproom.files LIMIT 1),
                    $1,
                    'func',
                    1, 10,
                    'test content',
                    to_tsvector('simple', 'test'),
                    0.5, 0.5
                 )
                 RETURNING id",
                &[&format!("test_batch_768_{}", i)],
            )
            .await?
            .get(0);
        chunk_ids.push(chunk_id);
    }

    // Create batch embeddings
    let embeddings: Vec<(i64, Option<Vec<f32>>, Option<Vec<f32>>)> = chunk_ids
        .iter()
        .map(|&id| (id, Some(vec![0.1f32; 768]), Some(vec![0.2f32; 768])))
        .collect();

    // Batch upsert
    batch_upsert_embeddings(&mut client, &embeddings, 768).await?;

    // Verify all chunks have embeddings in ollama columns
    for chunk_id in chunk_ids {
        let row = client
            .query_one(
                "SELECT
                    code_embedding_ollama IS NOT NULL as has_code_ollama,
                    text_embedding_ollama IS NOT NULL as has_text_ollama,
                    array_length(code_embedding_ollama, 1) as code_dim
                 FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await?;

        assert!(row.get::<_, bool>("has_code_ollama"));
        assert!(row.get::<_, bool>("has_text_ollama"));
        assert_eq!(row.get::<_, i32>("code_dim"), 768);
    }

    Ok(())
}

/// Test batch upserting 1536-dimensional embeddings.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_batch_upsert_1536_dimension() -> Result<()> {
    use crewchief_maproom::db::queries::{batch_upsert_embeddings, connect, migrate};

    let mut client = connect().await?;
    migrate(&client).await?;

    // Create test chunks
    let mut chunk_ids = Vec::new();
    for i in 0..3 {
        let chunk_id: i64 = client
            .query_one(
                "INSERT INTO maproom.chunks (
                    file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
                 ) VALUES (
                    (SELECT id FROM maproom.files LIMIT 1),
                    $1,
                    'func',
                    1, 10,
                    'test content',
                    to_tsvector('simple', 'test'),
                    0.5, 0.5
                 )
                 RETURNING id",
                &[&format!("test_batch_1536_{}", i)],
            )
            .await?
            .get(0);
        chunk_ids.push(chunk_id);
    }

    // Create batch embeddings
    let embeddings: Vec<(i64, Option<Vec<f32>>, Option<Vec<f32>>)> = chunk_ids
        .iter()
        .map(|&id| (id, Some(vec![0.3f32; 1536]), Some(vec![0.4f32; 1536])))
        .collect();

    // Batch upsert
    batch_upsert_embeddings(&mut client, &embeddings, 1536).await?;

    // Verify all chunks have embeddings in original columns
    for chunk_id in chunk_ids {
        let row = client
            .query_one(
                "SELECT
                    code_embedding IS NOT NULL as has_code,
                    text_embedding IS NOT NULL as has_text,
                    array_length(code_embedding, 1) as code_dim
                 FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await?;

        assert!(row.get::<_, bool>("has_code"));
        assert!(row.get::<_, bool>("has_text"));
        assert_eq!(row.get::<_, i32>("code_dim"), 1536);
    }

    Ok(())
}

/// Test that batch upsert handles dimension mismatch correctly.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_batch_dimension_mismatch_error() -> Result<()> {
    use crewchief_maproom::db::queries::{batch_upsert_embeddings, connect, migrate};

    let mut client = connect().await?;
    migrate(&client).await?;

    // Create a test chunk
    let chunk_id: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                (SELECT id FROM maproom.files LIMIT 1),
                'test_batch_mismatch',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[],
        )
        .await?
        .get(0);

    // Try to batch upsert with wrong dimension
    let embeddings = vec![(chunk_id, Some(vec![0.1f32; 1536]), None)];
    let result = batch_upsert_embeddings(&mut client, &embeddings, 768).await;

    assert!(
        result.is_err(),
        "Should return error for dimension mismatch"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("dimension mismatch") || err_msg.contains("does not match"),
        "Error message should mention dimension mismatch: {}",
        err_msg
    );

    Ok(())
}

/// Test that batch upsert maintains transaction safety.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_batch_transaction_rollback() -> Result<()> {
    use crewchief_maproom::db::queries::{batch_upsert_embeddings, connect, migrate};

    let mut client = connect().await?;
    migrate(&client).await?;

    // Create test chunks
    let chunk_id1: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                (SELECT id FROM maproom.files LIMIT 1),
                'test_tx_1',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[],
        )
        .await?
        .get(0);

    let chunk_id2: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                (SELECT id FROM maproom.files LIMIT 1),
                'test_tx_2',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[],
        )
        .await?
        .get(0);

    // Create batch with one valid and one invalid embedding (dimension mismatch)
    let embeddings = vec![
        (chunk_id1, Some(vec![0.1f32; 768]), Some(vec![0.2f32; 768])),
        (chunk_id2, Some(vec![0.3f32; 1536]), Some(vec![0.4f32; 768])), // Mismatch!
    ];

    // Batch upsert should fail due to second chunk
    let result = batch_upsert_embeddings(&mut client, &embeddings, 768).await;
    assert!(
        result.is_err(),
        "Batch should fail due to dimension mismatch"
    );

    // Verify that NEITHER chunk was updated (transaction rollback)
    let row1 = client
        .query_one(
            "SELECT code_embedding_ollama IS NULL as is_null FROM maproom.chunks WHERE id = $1",
            &[&chunk_id1],
        )
        .await?;
    assert!(
        row1.get::<_, bool>("is_null"),
        "First chunk should not be updated due to transaction rollback"
    );

    let row2 = client
        .query_one(
            "SELECT code_embedding_ollama IS NULL as is_null FROM maproom.chunks WHERE id = $1",
            &[&chunk_id2],
        )
        .await?;
    assert!(
        row2.get::<_, bool>("is_null"),
        "Second chunk should not be updated"
    );

    Ok(())
}

/// Test upserting with unsupported dimension.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_unsupported_dimension_error() -> Result<()> {
    use crewchief_maproom::db::queries::{connect, migrate, upsert_embeddings};

    let client = connect().await?;
    migrate(&client).await?;

    // Create a test chunk
    let chunk_id: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                (SELECT id FROM maproom.files LIMIT 1),
                'test_unsupported',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[],
        )
        .await?
        .get(0);

    // Try to upsert with unsupported dimension
    let code_emb = vec![0.1f32; 384];
    let result = upsert_embeddings(&client, chunk_id, Some(&code_emb), None, 384).await;

    assert!(
        result.is_err(),
        "Should return error for unsupported dimension"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Unsupported") && err_msg.contains("384"),
        "Error message should mention unsupported dimension: {}",
        err_msg
    );

    Ok(())
}

/// Test that text embeddings are written to text_embedding column, not doc_embedding.
///
/// This test verifies the bug fix for MCP-009 where text embeddings were incorrectly
/// being written to doc_embedding columns instead of text_embedding columns.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_text_embedding_column_correct_768() -> Result<()> {
    use crewchief_maproom::db::queries::{connect, migrate, upsert_embeddings};

    let client = connect().await?;
    migrate(&client).await?;

    // Ensure required columns exist for test
    client
        .batch_execute(
            "ALTER TABLE maproom.chunks
             ADD COLUMN IF NOT EXISTS code_embedding_ollama vector(768),
             ADD COLUMN IF NOT EXISTS text_embedding_ollama vector(768),
             ADD COLUMN IF NOT EXISTS doc_embedding_ollama vector(768),
             ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();",
        )
        .await?;

    // Create test hierarchy: repo -> worktree -> commit -> file -> chunk
    let test_name = format!("test-repo-{}", uuid::Uuid::new_v4());
    let repo_id: i64 = client
        .query_one(
            "INSERT INTO maproom.repos (name, root_path) VALUES ($1, '/test') RETURNING id",
            &[&test_name],
        )
        .await?
        .get(0);

    let worktree_id: i64 = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, 'main', '/test/worktree') RETURNING id",
            &[&repo_id],
        )
        .await?
        .get(0);

    let commit_id: i64 = client
        .query_one(
            "INSERT INTO maproom.commits (repo_id, sha) VALUES ($1, 'abc123') RETURNING id",
            &[&repo_id],
        )
        .await?
        .get(0);

    let file_id: i64 = client
        .query_one(
            "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash)
             VALUES ($1, $2, $3, 'test.ts', 'typescript', 'testhash')
             RETURNING id",
            &[&repo_id, &worktree_id, &commit_id],
        )
        .await?
        .get(0);

    // Create a test chunk
    let chunk_id: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                $1,
                'test_text_embedding_768',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[&file_id],
        )
        .await?
        .get(0);

    // Create 768-dimensional embeddings
    let code_emb = vec![0.1f32; 768];
    let text_emb = vec![0.2f32; 768];

    // Upsert with dimension 768
    upsert_embeddings(&client, chunk_id, Some(&code_emb), Some(&text_emb), 768).await?;

    // Verify text embeddings are in text_embedding_ollama column
    let row = client
        .query_one(
            "SELECT
                code_embedding_ollama IS NOT NULL as has_code_ollama,
                text_embedding_ollama IS NOT NULL as has_text_ollama,
                array_length(code_embedding_ollama::real[], 1) as code_dim,
                array_length(text_embedding_ollama::real[], 1) as text_dim
             FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await?;

    assert!(
        row.get::<_, bool>("has_code_ollama"),
        "code_embedding_ollama should not be NULL"
    );
    assert!(
        row.get::<_, bool>("has_text_ollama"),
        "text_embedding_ollama should not be NULL - embeddings should be in text_embedding column"
    );
    assert_eq!(
        row.get::<_, i32>("code_dim"),
        768,
        "code_embedding_ollama should have 768 dimensions"
    );
    assert_eq!(
        row.get::<_, i32>("text_dim"),
        768,
        "text_embedding_ollama should have 768 dimensions"
    );

    Ok(())
}

/// Test that text embeddings are written to text_embedding column for 1536 dimensions.
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_text_embedding_column_correct_1536() -> Result<()> {
    use crewchief_maproom::db::queries::{connect, migrate, upsert_embeddings};

    let client = connect().await?;
    migrate(&client).await?;

    // Ensure required columns exist for test
    client
        .batch_execute(
            "ALTER TABLE maproom.chunks
             ADD COLUMN IF NOT EXISTS code_embedding_ollama vector(768),
             ADD COLUMN IF NOT EXISTS text_embedding_ollama vector(768),
             ADD COLUMN IF NOT EXISTS doc_embedding_ollama vector(768),
             ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();",
        )
        .await?;

    // Create test hierarchy: repo -> worktree -> commit -> file -> chunk
    let test_name = format!("test-repo-1536-{}", uuid::Uuid::new_v4());
    let repo_id: i64 = client
        .query_one(
            "INSERT INTO maproom.repos (name, root_path) VALUES ($1, '/test1536') RETURNING id",
            &[&test_name],
        )
        .await?
        .get(0);

    let worktree_id: i64 = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, 'main', '/test/worktree') RETURNING id",
            &[&repo_id],
        )
        .await?
        .get(0);

    let commit_id: i64 = client
        .query_one(
            "INSERT INTO maproom.commits (repo_id, sha) VALUES ($1, 'def456') RETURNING id",
            &[&repo_id],
        )
        .await?
        .get(0);

    let file_id: i64 = client
        .query_one(
            "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash)
             VALUES ($1, $2, $3, 'test1536.ts', 'typescript', 'testhash1536')
             RETURNING id",
            &[&repo_id, &worktree_id, &commit_id],
        )
        .await?
        .get(0);

    // Create a test chunk
    let chunk_id: i64 = client
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, start_line, end_line, preview, ts_doc, recency_score, churn_score
             ) VALUES (
                $1,
                'test_text_embedding_1536',
                'func',
                1, 10,
                'test content',
                to_tsvector('simple', 'test'),
                0.5, 0.5
             )
             RETURNING id",
            &[&file_id],
        )
        .await?
        .get(0);

    // Create 1536-dimensional embeddings
    let code_emb = vec![0.3f32; 1536];
    let text_emb = vec![0.4f32; 1536];

    // Upsert with dimension 1536
    upsert_embeddings(&client, chunk_id, Some(&code_emb), Some(&text_emb), 1536).await?;

    // Verify text embeddings are in text_embedding column
    let row = client
        .query_one(
            "SELECT
                code_embedding IS NOT NULL as has_code,
                text_embedding IS NOT NULL as has_text,
                array_length(code_embedding::real[], 1) as code_dim,
                array_length(text_embedding::real[], 1) as text_dim
             FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await?;

    assert!(
        row.get::<_, bool>("has_code"),
        "code_embedding should not be NULL"
    );
    assert!(
        row.get::<_, bool>("has_text"),
        "text_embedding should not be NULL - embeddings should be in text_embedding column"
    );
    assert_eq!(
        row.get::<_, i32>("code_dim"),
        1536,
        "code_embedding should have 1536 dimensions"
    );
    assert_eq!(
        row.get::<_, i32>("text_dim"),
        1536,
        "text_embedding should have 1536 dimensions"
    );

    Ok(())
}
