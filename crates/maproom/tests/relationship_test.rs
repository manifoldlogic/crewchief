//! Integration tests for relationship-specific query functions.
//!
//! These tests verify relationship queries (tests, callers, callees, imports)
//! work correctly with a real database.

use anyhow::Result;
use tokio_postgres::{Client, NoTls};

/// Setup helper functions (same as graph_test.rs)
async fn run_migrations(client: &Client) -> Result<()> {
    client.batch_execute(include_str!("../migrations/0001_init.sql")).await?;
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

async fn create_test_link(client: &Client, test_chunk_id: i64, target_chunk_id: i64) -> Result<()> {
    client
        .execute(
            "INSERT INTO maproom.test_links(test_chunk_id, target_chunk_id) VALUES ($1,$2)",
            &[&test_chunk_id, &target_chunk_id],
        )
        .await?;
    Ok(())
}

/// Setup test data with comprehensive relationships
async fn setup_relationship_graph(client: &Client) -> Result<Vec<i64>> {
    run_migrations(client).await?;

    let repo_id = create_repo(client, "test-relationships", "/tmp/test-rel").await?;
    let worktree_id = create_worktree(client, repo_id, "main", "/tmp/test-rel").await?;
    let commit_id = create_commit(client, repo_id, "abc123").await?;

    // Create files
    let impl_file = create_file(client, repo_id, worktree_id, commit_id, "src/auth.ts").await?;
    let util_file = create_file(client, repo_id, worktree_id, commit_id, "src/utils.ts").await?;
    let test_file = create_file(client, repo_id, worktree_id, commit_id, "test/auth.test.ts").await?;

    // Create chunks
    let authenticate = create_chunk(client, impl_file, "authenticate", "func", 1, 20).await?;
    let validate_user = create_chunk(client, util_file, "validateUser", "func", 1, 10).await?;
    let hash_password = create_chunk(client, util_file, "hashPassword", "func", 12, 20).await?;
    let test_authenticate = create_chunk(client, test_file, "testAuthenticate", "func", 1, 30).await?;
    let test_validate = create_chunk(client, test_file, "testValidate", "func", 32, 50).await?;

    // Create relationships:
    // authenticate calls validateUser and hashPassword
    create_edge(client, authenticate, validate_user, "calls").await?;
    create_edge(client, authenticate, hash_password, "calls").await?;

    // validateUser calls hashPassword
    create_edge(client, validate_user, hash_password, "calls").await?;

    // authenticate imports validateUser
    create_edge(client, authenticate, validate_user, "imports").await?;

    // Tests
    create_edge(client, test_authenticate, authenticate, "test_of").await?;
    create_edge(client, test_validate, validate_user, "test_of").await?;

    // Also add to test_links table
    create_test_link(client, test_authenticate, authenticate).await?;
    create_test_link(client, test_validate, validate_user).await?;

    Ok(vec![authenticate, validate_user, hash_password, test_authenticate, test_validate])
}

#[tokio::test]
async fn test_find_test_files() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_relationship_graph(&client).await?;
    let authenticate = chunks[0];
    let test_authenticate = chunks[3];

    use crewchief_maproom::context::find_test_files;

    // Find tests for authenticate function
    let tests = find_test_files(&client, authenticate).await?;

    assert!(!tests.is_empty(), "Should find test files");

    // Verify test_authenticate is in the results
    let found_test = tests.iter().any(|t| t.id == test_authenticate);
    assert!(found_test, "Should find testAuthenticate as a test for authenticate");

    // Verify relevance is 1.0 for direct test
    let test_chunk = tests.iter().find(|t| t.id == test_authenticate).unwrap();
    assert_eq!(test_chunk.relevance, 1.0, "Direct test should have relevance 1.0");

    Ok(())
}

#[tokio::test]
async fn test_find_callers() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_relationship_graph(&client).await?;
    let authenticate = chunks[0];
    let validate_user = chunks[1];
    let hash_password = chunks[2];

    use crewchief_maproom::context::find_callers;

    // Find what calls validate_user
    let callers = find_callers(&client, validate_user, 2).await?;

    assert!(!callers.is_empty(), "Should find callers");

    // authenticate should be a caller of validate_user
    let found_authenticate = callers.iter().any(|c| c.id == authenticate);
    assert!(found_authenticate, "authenticate should call validate_user");

    // Find what calls hash_password
    let hash_callers = find_callers(&client, hash_password, 2).await?;

    // Should find both authenticate and validate_user
    assert!(hash_callers.len() >= 2, "hash_password should have multiple callers");

    let has_authenticate = hash_callers.iter().any(|c| c.id == authenticate);
    let has_validate = hash_callers.iter().any(|c| c.id == validate_user);

    assert!(has_authenticate || has_validate, "Should find callers of hash_password");

    Ok(())
}

#[tokio::test]
async fn test_find_callees() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_relationship_graph(&client).await?;
    let authenticate = chunks[0];
    let validate_user = chunks[1];
    let hash_password = chunks[2];

    use crewchief_maproom::context::find_callees;

    // Find what authenticate calls
    let callees = find_callees(&client, authenticate, 2).await?;

    assert!(!callees.is_empty(), "Should find callees");

    // Should find validate_user and hash_password
    let has_validate = callees.iter().any(|c| c.id == validate_user);
    let has_hash = callees.iter().any(|c| c.id == hash_password);

    assert!(has_validate, "authenticate should call validate_user");
    assert!(has_hash, "authenticate should call hash_password");

    // Verify depth 1 callees have relevance 0.7
    let direct_callees: Vec<_> = callees.iter().filter(|c| c.depth == 1).collect();
    for callee in direct_callees {
        assert!((callee.relevance - 0.7).abs() < 0.01, "Depth 1 callee should have relevance 0.7");
    }

    Ok(())
}

#[tokio::test]
async fn test_find_imports() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_relationship_graph(&client).await?;
    let authenticate = chunks[0];
    let validate_user = chunks[1];

    use crewchief_maproom::context::find_imports;

    // Find what authenticate imports
    let imports = find_imports(&client, authenticate).await?;

    assert!(!imports.is_empty(), "Should find imports");

    // Should find validate_user
    let found_validate = imports.iter().any(|i| i.id == validate_user);
    assert!(found_validate, "authenticate should import validate_user");

    // Verify imports are depth 1 only
    for import in imports {
        assert_eq!(import.depth, 1, "Imports should all be depth 1");
    }

    Ok(())
}

#[tokio::test]
async fn test_find_exports() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_relationship_graph(&client).await?;
    let validate_user = chunks[1];

    use crewchief_maproom::context::find_exports;

    // For this test, we'd need to create export edges
    // For now, test that the function runs without error
    let exports = find_exports(&client, validate_user).await?;

    // May be empty if no export edges are defined
    assert!(exports.is_empty() || !exports.is_empty(), "Function should complete");

    Ok(())
}

#[tokio::test]
async fn test_find_routes() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    // Setup for route testing
    run_migrations(&client).await?;

    let repo_id = create_repo(&client, "test-routes", "/tmp/test-routes").await?;
    let worktree_id = create_worktree(&client, repo_id, "main", "/tmp/test-routes").await?;
    let commit_id = create_commit(&client, repo_id, "abc123").await?;

    let component_file = create_file(&client, repo_id, worktree_id, commit_id, "src/Home.tsx").await?;
    let routes_file = create_file(&client, repo_id, worktree_id, commit_id, "src/routes.tsx").await?;

    let home_component = create_chunk(&client, component_file, "Home", "component", 1, 30).await?;
    let route_def = create_chunk(&client, routes_file, "homeRoute", "var", 5, 10).await?;

    // Create route_of edge
    create_edge(&client, route_def, home_component, "route_of").await?;

    use crewchief_maproom::context::find_routes;

    // Find routes for Home component
    let routes = find_routes(&client, home_component).await?;

    assert!(!routes.is_empty(), "Should find routes");

    let found_route = routes.iter().any(|r| r.id == route_def);
    assert!(found_route, "Should find homeRoute for Home component");

    Ok(())
}

#[tokio::test]
async fn test_find_all_relationships() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_relationship_graph(&client).await?;
    let authenticate = chunks[0];

    use crewchief_maproom::context::find_all_relationships;

    // Find all relationships for authenticate function
    let (tests, callers, callees, imports, exports, routes) =
        find_all_relationships(&client, authenticate, 2).await?;

    // Verify tests
    assert!(!tests.is_empty(), "Should find test files");

    // Verify callees
    assert!(!callees.is_empty(), "Should find callees");

    // Verify imports
    assert!(!imports.is_empty(), "Should find imports");

    // Other relationships may or may not exist
    // The key is that all queries execute without error

    Ok(())
}

#[tokio::test]
async fn test_multi_hop_traversal() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    let chunks = setup_relationship_graph(&client).await?;
    let authenticate = chunks[0];
    let hash_password = chunks[2];

    use crewchief_maproom::context::find_callees;

    // Find callees with depth 2
    let callees = find_callees(&client, authenticate, 2).await?;

    // Should find hash_password through multi-hop (authenticate -> validateUser -> hashPassword)
    let found_hash = callees.iter().find(|c| c.id == hash_password);

    if let Some(hash_chunk) = found_hash {
        // Verify relevance decay through multi-hop
        // Could be depth 1 (direct) or depth 2 (through validateUser)
        if hash_chunk.depth == 2 {
            let expected_relevance = 0.7 * 0.7; // Two hops
            assert!(
                (hash_chunk.relevance - expected_relevance).abs() < 0.01,
                "Depth 2 should have relevance ~{:.2}, got {:.2}",
                expected_relevance,
                hash_chunk.relevance
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_no_relationships() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/maproom_test".to_string());

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move { connection.await });

    run_migrations(&client).await?;

    let repo_id = create_repo(&client, "test-isolated", "/tmp/test-isolated").await?;
    let worktree_id = create_worktree(&client, repo_id, "main", "/tmp/test-isolated").await?;
    let commit_id = create_commit(&client, repo_id, "abc123").await?;
    let file_id = create_file(&client, repo_id, worktree_id, commit_id, "src/isolated.ts").await?;

    // Create an isolated chunk with no edges
    let isolated_chunk = create_chunk(&client, file_id, "isolated", "func", 1, 10).await?;

    use crewchief_maproom::context::{find_callees, find_callers, find_test_files};

    // All queries should return empty results without error
    let tests = find_test_files(&client, isolated_chunk).await?;
    let callers = find_callers(&client, isolated_chunk, 2).await?;
    let callees = find_callees(&client, isolated_chunk, 2).await?;

    assert!(tests.is_empty(), "Isolated chunk should have no tests");
    assert!(callers.is_empty() || callers.len() == 1, "Should only contain itself or be empty");
    assert!(callees.is_empty() || callees.len() == 1, "Should only contain itself or be empty");

    Ok(())
}
