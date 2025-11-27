//! SCHMAFIX-3001: Migration Integration Tests
//!
//! Integration tests for database migrations using existing PostgreSQL database.
//! Uses the existing PostgreSQL database at maproom-postgres:5432.
//!
//! Tests verify:
//! - Fresh database migrations (0001-0020) - all 20 migrations
//! - Migration idempotency (running migrations twice)
//! - Schema validation (including migrations 0018-0020 additions)
//!
//! NOTE: Migrations 18-20 have been fixed and are now included in all tests:
//! - Migration 18: Added pgcrypto extension for digest() function
//! - Migration 19: Fixed to handle fresh databases (no existing embeddings)
//! - Migration 20: Idempotent worktree tracking schema

use anyhow::{Context, Result};
use serial_test::serial;
use tokio_postgres::{Client, NoTls};

const POSTGRES_HOST: &str = "maproom-postgres";
const POSTGRES_PORT: u16 = 5432;
const POSTGRES_USER: &str = "maproom";
const POSTGRES_PASSWORD: &str = "maproom";

/// Generate a unique test database name using timestamp.
fn generate_test_db_name(test_name: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("maproom_test_{}_{}", test_name, timestamp)
}

/// Setup a test database and return the database name and connection string.
async fn setup_test_database(test_name: &str) -> Result<(String, String)> {
    let test_db_name = generate_test_db_name(test_name);

    // Connect to the default 'postgres' database to create our test database
    let postgres_conn_string = format!(
        "postgresql://{}:{}@{}:{}/postgres",
        POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_HOST, POSTGRES_PORT
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
        POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_HOST, POSTGRES_PORT, test_db_name
    );

    Ok((test_db_name, test_conn_string))
}

/// Connect to the test database and enable required extensions.
async fn connect_to_test_database(conn_string: &str) -> Result<Client> {
    let (client, connection) = tokio_postgres::connect(conn_string, NoTls)
        .await
        .context("Failed to connect to test database")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error: {}", e);
        }
    });

    // Enable pgvector and unaccent extensions
    client
        .batch_execute(
            "CREATE EXTENSION IF NOT EXISTS vector; CREATE EXTENSION IF NOT EXISTS unaccent;",
        )
        .await
        .context("Failed to create extensions")?;

    Ok(client)
}

/// Drop the test database after test completion.
async fn cleanup_test_database(test_db_name: &str) -> Result<()> {
    // Connect to the default 'postgres' database to drop our test database
    let postgres_conn_string = format!(
        "postgresql://{}:{}@{}:{}/postgres",
        POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_HOST, POSTGRES_PORT
    );

    let (client, connection) = tokio_postgres::connect(&postgres_conn_string, NoTls)
        .await
        .context("Failed to connect to postgres database for cleanup")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error: {}", e);
        }
    });

    // Terminate existing connections to the test database
    client
        .execute(
            &format!(
                "SELECT pg_terminate_backend(pg_stat_activity.pid)
                 FROM pg_stat_activity
                 WHERE pg_stat_activity.datname = '{}'
                   AND pid <> pg_backend_pid()",
                test_db_name
            ),
            &[],
        )
        .await
        .ok(); // Ignore errors if no connections exist

    // Drop test database
    client
        .execute(&format!("DROP DATABASE IF EXISTS {}", test_db_name), &[])
        .await
        .with_context(|| format!("Failed to drop test database {}", test_db_name))?;

    Ok(())
}

/// Run all database migrations (versions 1-20).
/// Includes migrations 18-20 which were fixed during SCHMAFIX-3001.
async fn run_migrations_up_to_17(client: &Client) -> Result<()> {
    // Run the full migration runner (applies all 20 migrations defined in queries.rs)
    crewchief_maproom::db::queries::migrate(client)
        .await
        .context("Migration runner failed")
}

/// Verify schema_migrations table shows correct version.
async fn verify_migration_version(client: &Client, expected_version: i32) -> Result<()> {
    let row = client
        .query_one("SELECT MAX(version) FROM maproom.schema_migrations", &[])
        .await
        .context("Failed to query schema_migrations")?;

    let actual_version: i32 = row.get(0);

    if actual_version != expected_version {
        anyhow::bail!(
            "Expected migration version {}, got {}",
            expected_version,
            actual_version
        );
    }

    Ok(())
}

/// Verify a table exists in the maproom schema.
async fn verify_table_exists(client: &Client, table_name: &str) -> Result<()> {
    let row = client
        .query_opt(
            "SELECT 1 FROM information_schema.tables WHERE table_schema = 'maproom' AND table_name = $1",
            &[&table_name],
        )
        .await
        .context("Failed to query information_schema.tables")?;

    if row.is_none() {
        anyhow::bail!("Table maproom.{} does not exist", table_name);
    }

    Ok(())
}

/// Verify a column exists with specific properties.
async fn verify_column(
    client: &Client,
    table_name: &str,
    column_name: &str,
    expected_type: &str,
    expected_nullable: bool,
) -> Result<()> {
    let row = client
        .query_opt(
            "SELECT data_type, is_nullable
             FROM information_schema.columns
             WHERE table_schema = 'maproom'
               AND table_name = $1
               AND column_name = $2",
            &[&table_name, &column_name],
        )
        .await
        .context("Failed to query column information")?;

    let row =
        row.ok_or_else(|| anyhow::anyhow!("Column {}.{} does not exist", table_name, column_name))?;

    let data_type: String = row.get(0);
    let is_nullable: String = row.get(1);
    let is_nullable_bool = is_nullable == "YES";

    if data_type != expected_type {
        anyhow::bail!(
            "Column {}.{} has type '{}', expected '{}'",
            table_name,
            column_name,
            data_type,
            expected_type
        );
    }

    if is_nullable_bool != expected_nullable {
        anyhow::bail!(
            "Column {}.{} has is_nullable={}, expected {}",
            table_name,
            column_name,
            is_nullable_bool,
            expected_nullable
        );
    }

    Ok(())
}

/// Verify an index exists.
async fn verify_index_exists(client: &Client, index_name: &str) -> Result<()> {
    let row = client
        .query_opt(
            "SELECT 1 FROM pg_indexes WHERE schemaname = 'maproom' AND indexname = $1",
            &[&index_name],
        )
        .await
        .context("Failed to query pg_indexes")?;

    if row.is_none() {
        anyhow::bail!("Index {} does not exist", index_name);
    }

    Ok(())
}

/// Test 1: Fresh database migrations (0000-0017)
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
#[serial]
async fn test_fresh_database_migrations() -> Result<()> {
    let (test_db_name, conn_string) = setup_test_database("fresh").await?;

    let result = async {
        let client = connect_to_test_database(&conn_string).await?;

        // Run all migrations (1-20)
        run_migrations_up_to_17(&client).await?;

        // Verify schema_migrations shows version 20
        verify_migration_version(&client, 20).await?;

        // Verify key tables exist
        verify_table_exists(&client, "chunks").await?;
        verify_table_exists(&client, "repos").await?;
        verify_table_exists(&client, "worktrees").await?;
        verify_table_exists(&client, "files").await?;
        verify_table_exists(&client, "commits").await?;
        verify_table_exists(&client, "code_embeddings").await?; // Migration 19

        println!("✅ Test 1 passed: Fresh database migrations (all 20 migrations)");
        Ok(())
    }
    .await;

    cleanup_test_database(&test_db_name).await?;
    result
}

/// Test 2: Migration idempotency
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
#[serial]
async fn test_migration_idempotency() -> Result<()> {
    let (test_db_name, conn_string) = setup_test_database("idempotency").await?;

    let result = async {
        let client = connect_to_test_database(&conn_string).await?;

        // Run migrations first time
        run_migrations_up_to_17(&client).await?;

        let version_after_first: i32 = client
            .query_one("SELECT MAX(version) FROM maproom.schema_migrations", &[])
            .await?
            .get(0);

        let migration_count_first: i64 = client
            .query_one("SELECT COUNT(*) FROM maproom.schema_migrations", &[])
            .await?
            .get(0);

        // Run migrations second time (should be idempotent)
        run_migrations_up_to_17(&client).await?;

        let version_after_second: i32 = client
            .query_one("SELECT MAX(version) FROM maproom.schema_migrations", &[])
            .await?
            .get(0);

        let migration_count_second: i64 = client
            .query_one("SELECT COUNT(*) FROM maproom.schema_migrations", &[])
            .await?
            .get(0);

        // Verify no changes after second run
        if version_after_first != version_after_second {
            anyhow::bail!(
                "Version changed after second migration: {} -> {}",
                version_after_first,
                version_after_second
            );
        }

        if migration_count_first != migration_count_second {
            anyhow::bail!(
                "Migration count changed after second run: {} -> {}",
                migration_count_first,
                migration_count_second
            );
        }

        println!("✅ Test 2 passed: Migration idempotency verified");
        Ok(())
    }
    .await;

    cleanup_test_database(&test_db_name).await?;
    result
}

/// Test 3: Schema validation
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
#[serial]
async fn test_schema_validation() -> Result<()> {
    let (test_db_name, conn_string) = setup_test_database("schema").await?;

    let result = async {
        let client = connect_to_test_database(&conn_string).await?;

        // Run migrations up to v17
        run_migrations_up_to_17(&client).await?;

        // Validate core tables exist
        verify_table_exists(&client, "chunks").await?;
        verify_table_exists(&client, "repos").await?;
        verify_table_exists(&client, "worktrees").await?;
        verify_table_exists(&client, "files").await?;
        verify_table_exists(&client, "commits").await?;

        // Validate chunks table columns (pre-migration-18 schema)
        verify_column(&client, "chunks", "id", "bigint", false).await?;
        verify_column(&client, "chunks", "file_id", "bigint", false).await?;
        verify_column(&client, "chunks", "symbol_name", "text", true).await?; // nullable
        verify_column(&client, "chunks", "kind", "USER-DEFINED", true).await?; // maproom.symbol_kind ENUM
        verify_column(&client, "chunks", "preview", "text", true).await?; // nullable

        // Validate repos table
        verify_column(&client, "repos", "id", "bigint", false).await?;
        verify_column(&client, "repos", "name", "text", false).await?;

        // Validate worktrees table
        verify_column(&client, "worktrees", "id", "bigint", false).await?;
        verify_column(&client, "worktrees", "repo_id", "bigint", false).await?;
        verify_column(&client, "worktrees", "name", "text", false).await?;

        // Validate migrations 0018-0020 schema additions

        // Migration 0018: blob_sha column
        verify_column(&client, "chunks", "blob_sha", "text", false).await?; // NOT NULL
        verify_index_exists(&client, "idx_chunks_blob_sha").await?;

        // Migration 0019: code_embeddings table
        verify_table_exists(&client, "code_embeddings").await?;
        verify_column(&client, "code_embeddings", "blob_sha", "text", false).await?;
        verify_column(
            &client,
            "code_embeddings",
            "embedding",
            "USER-DEFINED",
            false,
        )
        .await?; // vector(1536)
        verify_index_exists(&client, "idx_embeddings_vector").await?; // HNSW index

        // Migration 0020: worktree tracking
        verify_column(&client, "chunks", "worktree_ids", "jsonb", false).await?;
        verify_table_exists(&client, "worktree_index_state").await?;

        // Validate some key indexes exist
        verify_index_exists(&client, "idx_chunks_tsv").await?; // GIN index for text search

        println!("✅ Test 3 passed: Schema validation complete (all 20 migrations)");
        Ok(())
    }
    .await;

    cleanup_test_database(&test_db_name).await?;
    result
}
