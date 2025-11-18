//! IDXCLEAN-1001: Stale Worktree Detection Integration Tests
//!
//! Integration tests for stale worktree detection using a real PostgreSQL database.
//! Uses the existing PostgreSQL database at maproom-postgres:5432.
//!
//! Tests verify:
//! - Detection of worktrees with non-existent paths
//! - Preservation of worktrees with valid paths
//! - Parallel validation performance
//! - Error handling for edge cases

use anyhow::{Context, Result};
use crewchief_maproom::db::cleanup::StaleWorktreeDetector;
use serial_test::serial;
use tempfile::TempDir;
use tokio_postgres::{Client, NoTls};

const POSTGRES_USER: &str = "maproom";
const POSTGRES_PASSWORD: &str = "maproom";

/// Get postgres connection parameters from environment or defaults
fn get_postgres_params() -> (String, u16) {
    let host = std::env::var("MAPROOM_TEST_DB_HOST").unwrap_or_else(|_| "maproom-postgres".to_string());
    let port = std::env::var("MAPROOM_TEST_DB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432);
    (host, port)
}

/// Generate a unique test database name using timestamp
fn generate_test_db_name(test_name: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("maproom_test_cleanup_{}_{}", test_name, timestamp)
}

/// Setup a test database and return the database name and connection string
async fn setup_test_database(test_name: &str) -> Result<(String, String)> {
    let test_db_name = generate_test_db_name(test_name);
    let (host, port) = get_postgres_params();

    // Connect to the default 'postgres' database to create our test database
    let postgres_conn_string = format!(
        "postgresql://{}:{}@{}:{}/postgres",
        POSTGRES_USER, POSTGRES_PASSWORD, host, port
    );

    let (client, connection) = tokio_postgres::connect(&postgres_conn_string, NoTls)
        .await
        .context("Failed to connect to postgres database")?;

    // Spawn connection driver
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error: {}", e);
        }
    });

    // Create test database
    client
        .execute(&format!("CREATE DATABASE {}", test_db_name), &[])
        .await
        .with_context(|| format!("Failed to create test database {}", test_db_name))?;

    // Build connection string for the test database
    let test_conn_string = format!(
        "postgresql://{}:{}@{}:{}/{}",
        POSTGRES_USER, POSTGRES_PASSWORD, host, port, test_db_name
    );

    Ok((test_db_name, test_conn_string))
}

/// Connect to the test database
async fn connect_to_test_database(conn_string: &str) -> Result<Client> {
    let (client, connection) = tokio_postgres::connect(conn_string, NoTls)
        .await
        .context("Failed to connect to test database")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error: {}", e);
        }
    });

    Ok(client)
}

/// Drop the test database after test completion
async fn cleanup_test_database(test_db_name: &str) -> Result<()> {
    let (host, port) = get_postgres_params();
    let postgres_conn_string = format!(
        "postgresql://{}:{}@{}:{}/postgres",
        POSTGRES_USER, POSTGRES_PASSWORD, host, port
    );

    let (client, connection) = tokio_postgres::connect(&postgres_conn_string, NoTls)
        .await
        .context("Failed to connect to postgres database for cleanup")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error during cleanup: {}", e);
        }
    });

    // Terminate active connections to the test database
    client
        .execute(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()",
            &[&test_db_name],
        )
        .await
        .context("Failed to terminate active connections")?;

    // Drop the test database
    client
        .execute(&format!("DROP DATABASE IF EXISTS {}", test_db_name), &[])
        .await
        .with_context(|| format!("Failed to drop test database {}", test_db_name))?;

    Ok(())
}

/// Run migrations on the test database
async fn run_migrations(client: &Client) -> Result<()> {
    crewchief_maproom::db::queries::migrate(client)
        .await
        .context("Failed to run migrations")?;
    Ok(())
}

/// Insert test data: repo, worktree, and chunks
async fn insert_test_data(
    client: &Client,
    abs_path: &str,
    chunk_count: i32,
) -> Result<(i64, i64)> {
    // Insert repo
    let repo_id: i64 = client
        .query_one(
            "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id",
            &[&"test-repo", &"/tmp/test-repo"],
        )
        .await?
        .get(0);

    // Insert worktree
    let worktree_id: i64 = client
        .query_one(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) RETURNING id",
            &[&repo_id, &"test-branch", &abs_path],
        )
        .await?
        .get(0);

    // Insert commit
    let commit_id: i64 = client
        .query_one(
            "INSERT INTO maproom.commits (repo_id, sha, committed_at) VALUES ($1, $2, NOW()) RETURNING id",
            &[&repo_id, &"abc123"],
        )
        .await?
        .get(0);

    // Insert file
    let file_id: i64 = client
        .query_one(
            "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
            &[&repo_id, &worktree_id, &commit_id, &"test.rs", &"rust", &"hash123", &1000],
        )
        .await?
        .get(0);

    // Insert chunks with worktree_ids
    for i in 0..chunk_count {
        client
            .execute(
                "INSERT INTO maproom.chunks (file_id, symbol_name, kind, start_line, end_line, preview, worktree_ids)
                 VALUES ($1, $2, 'func', $3, $4, $5, $6)",
                &[
                    &file_id,
                    &format!("test_func_{}", i),
                    &(i * 10),
                    &((i + 1) * 10),
                    &format!("fn test_func_{}() {{}}", i),
                    &serde_json::json!([worktree_id.to_string()]),
                ],
            )
            .await?;
    }

    Ok((repo_id, worktree_id))
}

#[tokio::test]
#[serial]
async fn test_detects_stale_worktree() -> Result<()> {
    // Setup test database
    let (test_db_name, conn_string) = setup_test_database("detects_stale").await?;
    let client = connect_to_test_database(&conn_string).await?;

    // Run migrations
    run_migrations(&client).await?;

    // Insert test data with non-existent path
    let non_existent_path = "/tmp/non_existent_worktree_12345";
    insert_test_data(&client, non_existent_path, 5).await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&client);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 1 stale worktree
    assert_eq!(
        stale_worktrees.len(),
        1,
        "Expected 1 stale worktree, found {}",
        stale_worktrees.len()
    );

    let stale = &stale_worktrees[0];
    assert_eq!(stale.name, "test-branch");
    assert_eq!(stale.abs_path, non_existent_path);
    assert!(!stale.exists, "Worktree should be marked as not existing");
    assert_eq!(stale.chunk_count, 5, "Expected 5 chunks");

    // Cleanup
    cleanup_test_database(&test_db_name).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_preserves_valid_worktree() -> Result<()> {
    // Setup test database
    let (test_db_name, conn_string) = setup_test_database("preserves_valid").await?;
    let client = connect_to_test_database(&conn_string).await?;

    // Run migrations
    run_migrations(&client).await?;

    // Create a temporary directory that exists
    let temp_dir = TempDir::new()?;
    let existing_path = temp_dir.path().to_str().unwrap();

    // Insert test data with existing path
    insert_test_data(&client, existing_path, 3).await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&client);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 0 stale worktrees (the path exists)
    assert_eq!(
        stale_worktrees.len(),
        0,
        "Expected 0 stale worktrees for existing path, found {}",
        stale_worktrees.len()
    );

    // Cleanup
    drop(temp_dir);
    cleanup_test_database(&test_db_name).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_mixed_worktrees() -> Result<()> {
    // Setup test database
    let (test_db_name, conn_string) = setup_test_database("mixed").await?;
    let client = connect_to_test_database(&conn_string).await?;

    // Run migrations
    run_migrations(&client).await?;

    // Create one valid worktree
    let temp_dir = TempDir::new()?;
    let valid_path = temp_dir.path().to_str().unwrap();
    insert_test_data(&client, valid_path, 10).await?;

    // Create two stale worktrees
    insert_test_data(&client, "/tmp/stale_worktree_1_xyz", 5).await?;
    insert_test_data(&client, "/tmp/stale_worktree_2_xyz", 8).await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&client);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 2 stale worktrees
    assert_eq!(
        stale_worktrees.len(),
        2,
        "Expected 2 stale worktrees, found {}",
        stale_worktrees.len()
    );

    // Verify chunk counts are correct
    let total_chunks: i64 = stale_worktrees.iter().map(|w| w.chunk_count).sum();
    assert_eq!(total_chunks, 13, "Expected 13 total chunks in stale worktrees");

    // Cleanup
    drop(temp_dir);
    cleanup_test_database(&test_db_name).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_empty_database() -> Result<()> {
    // Setup test database
    let (test_db_name, conn_string) = setup_test_database("empty").await?;
    let client = connect_to_test_database(&conn_string).await?;

    // Run migrations
    run_migrations(&client).await?;

    // Run detection on empty database
    let detector = StaleWorktreeDetector::new(&client);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 0 stale worktrees
    assert_eq!(
        stale_worktrees.len(),
        0,
        "Expected 0 stale worktrees in empty database"
    );

    // Cleanup
    cleanup_test_database(&test_db_name).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_worktree_with_no_chunks() -> Result<()> {
    // Setup test database
    let (test_db_name, conn_string) = setup_test_database("no_chunks").await?;
    let client = connect_to_test_database(&conn_string).await?;

    // Run migrations
    run_migrations(&client).await?;

    // Insert worktree with no chunks
    let repo_id: i64 = client
        .query_one(
            "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id",
            &[&"test-repo", &"/tmp/test-repo"],
        )
        .await?
        .get(0);

    client
        .execute(
            "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3)",
            &[&repo_id, &"empty-branch", &"/tmp/non_existent_empty"],
        )
        .await?;

    // Run detection
    let detector = StaleWorktreeDetector::new(&client);
    let stale_worktrees = detector.detect_stale_worktrees().await?;

    // Verify: should find 1 stale worktree with 0 chunks
    assert_eq!(stale_worktrees.len(), 1);
    assert_eq!(stale_worktrees[0].chunk_count, 0);
    assert_eq!(stale_worktrees[0].name, "empty-branch");

    // Cleanup
    cleanup_test_database(&test_db_name).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_parallel_performance() -> Result<()> {
    // Setup test database
    let (test_db_name, conn_string) = setup_test_database("performance").await?;
    let client = connect_to_test_database(&conn_string).await?;

    // Run migrations
    run_migrations(&client).await?;

    // Insert 50 worktrees (a reasonable test size)
    for i in 0..50 {
        insert_test_data(&client, &format!("/tmp/test_worktree_{}", i), 2).await?;
    }

    // Run detection and measure time
    let start = std::time::Instant::now();
    let detector = StaleWorktreeDetector::new(&client);
    let stale_worktrees = detector.detect_stale_worktrees().await?;
    let duration = start.elapsed();

    // Verify results
    assert_eq!(stale_worktrees.len(), 50, "Expected 50 stale worktrees");

    // Verify performance: should complete in under 2 seconds for 50 worktrees
    // (Target is <1s for 100, so 50 should be well under 1s, using 2s as safe margin)
    assert!(
        duration.as_secs() < 2,
        "Detection took too long: {:?} (expected < 2s)",
        duration
    );

    println!("Performance test: detected 50 worktrees in {:?}", duration);

    // Cleanup
    cleanup_test_database(&test_db_name).await?;

    Ok(())
}
