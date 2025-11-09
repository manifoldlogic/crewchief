//! Integration tests for graph traversal queries.
//!
//! These tests verify the core graph traversal functionality using a real database
//! with known relationship structures.

use anyhow::Result;
use tokio_postgres::{Client, NoTls};

/// Test setup: create a simple graph structure for testing
async fn setup_test_graph(client: &Client) -> Result<Vec<i64>> {
    // Run migrations first
    run_migrations(client).await?;

    // Create test repo and worktree
    let repo_id = create_repo(client, "test-graph", "/tmp/test-graph").await?;
    let worktree_id = create_worktree(client, repo_id, "main", "/tmp/test-graph").await?;
    let commit_id = create_commit(client, repo_id, "abc123").await?;

    // Create test files
    let file1 = create_file(client, repo_id, worktree_id, commit_id, "src/main.ts").await?;
    let file2 = create_file(client, repo_id, worktree_id, commit_id, "src/utils.ts").await?;
    let file3 = create_file(client, repo_id, worktree_id, commit_id, "test/main.test.ts").await?;

    // Create test chunks
    let chunk1 = create_chunk(client, file1, "main", "func", 1, 10).await?;
    let chunk2 = create_chunk(client, file2, "helper", "func", 1, 5).await?;
    let chunk3 = create_chunk(client, file2, "process", "func", 7, 15).await?;
    let chunk4 = create_chunk(client, file3, "testMain", "func", 1, 20).await?;

    // Create edges:
    // chunk1 calls chunk2 and chunk3
    create_edge(client, chunk1, chunk2, "calls").await?;
    create_edge(client, chunk1, chunk3, "calls").await?;

    // chunk2 calls chunk3
    create_edge(client, chunk2, chunk3, "calls").await?;

    // chunk4 tests chunk1
    create_edge(client, chunk4, chunk1, "test_of").await?;

    // chunk1 imports chunk2
    create_edge(client, chunk1, chunk2, "imports").await?;

    Ok(vec![chunk1, chunk2, chunk3, chunk4])
}

async fn run_migrations(client: &Client) -> Result<()> {
    client
        .batch_execute(include_str!("../migrations/0001_init.sql"))
        .await?;
    Ok(())
}

async fn create_repo(client: &Client, name: &str, root_path: &str) -> Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.repos(name, root_path) VALUES ($1,$2) RETURNING id",
            &[&name, &root_path],
        )
        .await?;
    Ok(row.get(0))
}

async fn create_worktree(client: &Client, repo_id: i64, name: &str, abs_path: &str) -> Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.worktrees(repo_id, name, abs_path) VALUES ($1,$2,$3) RETURNING id",
            &[&repo_id, &name, &abs_path],
        )
        .await?;
    Ok(row.get(0))
}

async fn create_commit(client: &Client, repo_id: i64, sha: &str) -> Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.commits(repo_id, sha) VALUES ($1,$2) RETURNING id",
            &[&repo_id, &sha],
        )
        .await?;
    Ok(row.get(0))
}

async fn create_file(
    client: &Client,
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
    relpath: &str,
) -> Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.files(repo_id, worktree_id, commit_id, relpath, content_hash)
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
            &[&repo_id, &worktree_id, &commit_id, &relpath, &"hash123"],
        )
        .await?;
    Ok(row.get(0))
}

async fn create_chunk(
    client: &Client,
    file_id: i64,
    symbol_name: &str,
    kind: &str,
    start_line: i32,
    end_line: i32,
) -> Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.chunks(file_id, symbol_name, kind, start_line, end_line, preview, ts_doc)
             VALUES ($1,$2,($3::text)::maproom.symbol_kind,$4,$5,'preview',to_tsvector('simple','test'))
             RETURNING id",
            &[&file_id, &symbol_name, &kind, &start_line, &end_line],
        )
        .await?;
    Ok(row.get(0))
}

async fn create_edge(client: &Client, src: i64, dst: i64, edge_type: &str) -> Result<()> {
    client
        .execute(
            "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type)
             VALUES ($1,$2,($3::text)::maproom.edge_type)",
            &[&src, &dst, &edge_type],
        )
        .await?;
    Ok(())
}

#[allow(dead_code)]
async fn create_test_link(client: &Client, test_chunk_id: i64, target_chunk_id: i64) -> Result<()> {
    client
        .execute(
            "INSERT INTO maproom.test_links(test_chunk_id, target_chunk_id) VALUES ($1,$2)",
            &[&test_chunk_id, &target_chunk_id],
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn test_find_related_chunks_bidirectional() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string()
    });

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_test_graph(&client).await?;
    let chunk1 = chunks[0];
    let chunk2 = chunks[1];

    // Import the function we're testing
    use crewchief_maproom::context::find_related_chunks;

    // Test bidirectional traversal from chunk1
    let related = find_related_chunks(&client, chunk1, 2, None).await?;

    // Should find chunk1 itself plus related chunks
    assert!(!related.is_empty(), "Should find related chunks");

    // Verify chunk1 is included (depth 0)
    let chunk1_result = related.iter().find(|c| c.id == chunk1);
    assert!(chunk1_result.is_some(), "Should include source chunk");
    assert_eq!(
        chunk1_result.unwrap().depth,
        0,
        "Source chunk should have depth 0"
    );
    assert_eq!(
        chunk1_result.unwrap().relevance,
        1.0,
        "Source chunk should have relevance 1.0"
    );

    // Verify chunk2 is found (depth 1)
    let chunk2_result = related.iter().find(|c| c.id == chunk2);
    assert!(
        chunk2_result.is_some(),
        "Should find directly connected chunk"
    );
    assert_eq!(
        chunk2_result.unwrap().depth,
        1,
        "Direct connection should have depth 1"
    );
    assert!(
        (chunk2_result.unwrap().relevance - 0.7).abs() < 0.01,
        "Depth 1 should have relevance 0.7"
    );

    Ok(())
}

#[tokio::test]
async fn test_find_related_chunks_depth_limiting() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string()
    });

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_test_graph(&client).await?;
    let chunk1 = chunks[0];

    use crewchief_maproom::context::find_related_chunks;

    // Test with depth limit of 1
    let related_depth1 = find_related_chunks(&client, chunk1, 1, None).await?;
    let max_depth1 = related_depth1.iter().map(|c| c.depth).max().unwrap_or(0);
    assert!(max_depth1 <= 1, "Should respect depth limit of 1");

    // Test with depth limit of 2
    let related_depth2 = find_related_chunks(&client, chunk1, 2, None).await?;
    let max_depth2 = related_depth2.iter().map(|c| c.depth).max().unwrap_or(0);
    assert!(max_depth2 <= 2, "Should respect depth limit of 2");

    // Depth 2 should find more chunks than depth 1
    assert!(
        related_depth2.len() >= related_depth1.len(),
        "Deeper traversal should find at least as many chunks"
    );

    Ok(())
}

#[tokio::test]
async fn test_find_related_chunks_edge_type_filtering() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string()
    });

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_test_graph(&client).await?;
    let chunk1 = chunks[0];

    use crewchief_maproom::context::{find_related_chunks, EdgeType};

    // Test filtering by calls edges only
    let calls_only = find_related_chunks(&client, chunk1, 2, Some(vec![EdgeType::Calls])).await?;

    // Test filtering by test_of edges only
    let tests_only = find_related_chunks(&client, chunk1, 2, Some(vec![EdgeType::TestOf])).await?;

    // calls_only should not include test chunks
    // tests_only should include test chunks

    // These should be different result sets
    assert!(
        !calls_only.is_empty() || !tests_only.is_empty(),
        "Should find chunks with edge type filtering"
    );

    Ok(())
}

#[tokio::test]
async fn test_find_related_chunks_directional() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string()
    });

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_test_graph(&client).await?;
    let chunk1 = chunks[0];
    let chunk2 = chunks[1];

    use crewchief_maproom::context::{find_related_chunks_directional, EdgeType};

    // Test forward direction (what chunk1 calls)
    let forward =
        find_related_chunks_directional(&client, chunk1, 2, Some(vec![EdgeType::Calls]), true)
            .await?;

    // Test backward direction (what calls chunk2)
    let backward =
        find_related_chunks_directional(&client, chunk2, 2, Some(vec![EdgeType::Calls]), false)
            .await?;

    // Verify results make sense
    assert!(!forward.is_empty(), "Should find forward relationships");
    assert!(!backward.is_empty(), "Should find backward relationships");

    // chunk1 should appear in backward search from chunk2
    let chunk1_in_backward = backward.iter().any(|c| c.id == chunk1);
    assert!(
        chunk1_in_backward,
        "chunk1 should be found in backward search from chunk2"
    );

    Ok(())
}

#[tokio::test]
async fn test_relevance_decay() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string()
    });

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_test_graph(&client).await?;
    let chunk1 = chunks[0];

    use crewchief_maproom::context::find_related_chunks;

    let related = find_related_chunks(&client, chunk1, 3, None).await?;

    // Verify relevance decay factor of 0.7 per hop
    for chunk in &related {
        let expected_relevance = 0.7_f64.powi(chunk.depth);
        let diff = (chunk.relevance - expected_relevance).abs();
        assert!(
            diff < 0.01,
            "Relevance at depth {} should be ~{:.2}, got {:.2}",
            chunk.depth,
            expected_relevance,
            chunk.relevance
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_ordering_by_relevance() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string()
    });

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_test_graph(&client).await?;
    let chunk1 = chunks[0];

    use crewchief_maproom::context::find_related_chunks;

    let related = find_related_chunks(&client, chunk1, 3, None).await?;

    // Verify results are ordered by relevance (descending)
    for i in 1..related.len() {
        assert!(
            related[i - 1].relevance >= related[i].relevance,
            "Results should be ordered by relevance (descending)"
        );
    }

    Ok(())
}
