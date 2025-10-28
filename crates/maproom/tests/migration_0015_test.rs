//! Integration tests for migration 0015: Add Ollama embedding columns
//!
//! This test validates the complete migration lifecycle:
//! 1. Load 100-chunk fixture with OpenAI embeddings
//! 2. Run forward migration (add Ollama columns)
//! 3. Verify columns and indexes exist
//! 4. Verify data preservation (no data loss)
//! 5. Run rollback migration (remove Ollama columns)
//! 6. Verify columns and indexes removed
//! 7. Verify data still preserved after rollback
//!
//! # Safety Requirements
//! - Uses isolated test database (ephemeral)
//! - Verifies zero data loss at each step
//! - Tests both forward and rollback migrations
//! - Measures execution time (should be < 5 seconds for 100 chunks)
//!
//! # Running Tests
//! ```bash
//! cargo test --test migration_0015_test -- --nocapture
//! ```

use anyhow::{Context, Result};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tokio_postgres::{Client, Config, NoTls};

// =============================================================================
// Test Configuration
// =============================================================================

/// Devcontainer PostgreSQL connection (as specified in DATABASE_ARCHITECTURE.md)
const TEST_DB_URL: &str = "postgresql://postgres:postgres@postgres:5432";
const TEST_DB_NAME: &str = "maproom_migration_test";

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates an ephemeral test database and returns a client connected to it
async fn create_test_database() -> Result<Client> {
    // Connect to postgres database to create test db
    let (client, connection) = tokio_postgres::connect(
        &format!("{}/postgres", TEST_DB_URL),
        NoTls,
    )
    .await
    .context("Failed to connect to PostgreSQL")?;

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Drop test database if it exists (cleanup from previous failed runs)
    let _ = client
        .execute(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME), &[])
        .await;

    // Create test database
    client
        .execute(&format!("CREATE DATABASE {}", TEST_DB_NAME), &[])
        .await
        .context("Failed to create test database")?;

    // Connect to test database
    let mut pg_config = Config::new();
    pg_config.host("postgres");
    pg_config.user("postgres");
    pg_config.password("postgres");
    pg_config.dbname(TEST_DB_NAME);

    let (test_client, test_connection) = pg_config
        .connect(NoTls)
        .await
        .context("Failed to connect to test database")?;

    tokio::spawn(async move {
        if let Err(e) = test_connection.await {
            eprintln!("Test connection error: {}", e);
        }
    });

    // Run initial schema migrations to match production schema
    // 0001: Create maproom schema and base tables
    run_migration(&test_client, "crates/maproom/migrations/0001_init.sql")
        .await
        .context("Failed to run migration 0001_init.sql")?;

    // 0002: Add markdown support and indexed_at column to worktrees
    run_migration(&test_client, "crates/maproom/migrations/0002_markdown_support.sql")
        .await
        .context("Failed to run migration 0002_markdown_support.sql")?;

    // 0003: Add YAML/TOML support and indexed_at column to chunks
    run_migration(&test_client, "crates/maproom/migrations/0003_yaml_toml_support.sql")
        .await
        .context("Failed to run migration 0003_yaml_toml_support.sql")?;

    Ok(test_client)
}

/// Drops the test database
async fn drop_test_database() -> Result<()> {
    let (client, connection) = tokio_postgres::connect(
        &format!("{}/postgres", TEST_DB_URL),
        NoTls,
    )
    .await
    .context("Failed to connect to PostgreSQL for cleanup")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Cleanup connection error: {}", e);
        }
    });

    // Terminate existing connections
    let _ = client
        .execute(
            &format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
                TEST_DB_NAME
            ),
            &[],
        )
        .await;

    // Drop database
    client
        .execute(&format!("DROP DATABASE IF EXISTS {}", TEST_DB_NAME), &[])
        .await
        .context("Failed to drop test database")?;

    Ok(())
}

/// Creates a test fixture with 100 chunks directly in the database
/// This avoids psql authentication issues and is more reliable for testing
async fn create_test_fixture(client: &Client) -> Result<()> {
    // Insert test repo
    client
        .execute(
            "INSERT INTO maproom.repos (id, name, root_path) VALUES (1, 'crewchief', '/workspace')",
            &[],
        )
        .await?;

    // Insert test worktree
    client
        .execute(
            "INSERT INTO maproom.worktrees (id, repo_id, name, abs_path) VALUES (1, 1, 'maproom-vamp', '/workspace')",
            &[],
        )
        .await?;

    // Insert test commit
    client
        .execute(
            "INSERT INTO maproom.commits (id, repo_id, sha) VALUES (1, 1, 'HEAD')",
            &[],
        )
        .await?;

    // Insert test file
    client
        .execute(
            "INSERT INTO maproom.files (id, repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes)
             VALUES (1, 1, 1, 1, 'test/file.ts', 'ts', 'abc123', 1000)",
            &[],
        )
        .await?;

    // Create 100 test chunks with embeddings
    // OpenAI embeddings are 1536 dimensions, we use a simple repeating pattern
    // Generate embedding SQL once: array of 1536 small floats
    let embedding_values: Vec<String> = (0..1536).map(|i| format!("{}", (i % 10) as f32 / 10.0)).collect();
    let embedding_sql = format!("ARRAY[{}]::vector", embedding_values.join(","));

    for i in 1..=100 {
        let insert_sql = format!(
            "INSERT INTO maproom.chunks
             (id, file_id, symbol_name, kind, start_line, end_line, preview, code_embedding)
             VALUES ({}, 1, 'test_function_{}', 'func', {}, {}, 'function test_function_{}() {{}}', {})",
            i,
            i,
            i * 10,
            i * 10 + 5,
            i,
            embedding_sql
        );

        client
            .execute(&insert_sql, &[])
            .await
            .with_context(|| format!("Failed to insert chunk {}", i))?;
    }

    Ok(())
}

/// Runs a migration SQL file with CONCURRENTLY support
/// Since CONCURRENTLY cannot run inside a transaction, we split the SQL into parts
async fn run_migration_with_concurrently(client: &Client, migration_path: &str) -> Result<()> {
    let workspace_root = PathBuf::from("/workspace");
    let full_path = workspace_root.join(migration_path);

    let sql = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read migration file: {}", full_path.display()))?;

    // Handle migrations with CONCURRENTLY by executing parts separately
    // Strategy: Find BEGIN...COMMIT blocks and CONCURRENTLY statements, execute in order

    let mut parts = Vec::new();
    let mut current_pos = 0;

    // Find all BEGIN...COMMIT blocks
    while let Some(begin_pos) = sql[current_pos..].find("BEGIN;") {
        let abs_begin = current_pos + begin_pos;
        if let Some(commit_offset) = sql[abs_begin..].find("COMMIT;") {
            let abs_commit = abs_begin + commit_offset + 7; // +7 for "COMMIT;"

            // Check if there's any CONCURRENTLY content before this BEGIN
            if current_pos < abs_begin {
                let before_block = &sql[current_pos..abs_begin];
                for stmt in before_block.split(';') {
                    let cleaned = stmt
                        .lines()
                        .filter(|line| {
                            let trimmed = line.trim();
                            !trimmed.starts_with("--") && !trimmed.is_empty()
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim()
                        .to_string();

                    if !cleaned.is_empty() && (cleaned.to_uppercase().contains("CREATE INDEX") || cleaned.to_uppercase().contains("DROP INDEX")) {
                        parts.push(cleaned);
                    }
                }
            }

            // Add the transaction block
            parts.push(sql[abs_begin..abs_commit].to_string());
            current_pos = abs_commit;
        } else {
            break;
        }
    }

    // Check for any remaining CONCURRENTLY statements after last COMMIT
    if current_pos < sql.len() {
        let remaining = &sql[current_pos..];
        for stmt in remaining.split(';') {
            let cleaned = stmt
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.starts_with("--") && !trimmed.is_empty()
                })
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();

            if !cleaned.is_empty() && (cleaned.to_uppercase().contains("CREATE INDEX") || cleaned.to_uppercase().contains("DROP INDEX")) {
                parts.push(cleaned);
            }
        }
    }

    // Execute all parts in order
    for part in parts {
        let part_trimmed = part.trim();
        if !part_trimmed.is_empty() {
            if part_trimmed.starts_with("BEGIN") {
                // Transaction block
                client
                    .batch_execute(part_trimmed)
                    .await
                    .with_context(|| format!("Failed to execute transaction block"))?;
            } else {
                // CONCURRENTLY statement
                client
                    .execute(part_trimmed, &[])
                    .await
                    .with_context(|| format!("Failed to execute CONCURRENTLY statement: {}", part_trimmed))?;
            }
        }
    }

    Ok(())
}

/// Runs a migration SQL file using tokio-postgres (for non-CONCURRENTLY migrations)
async fn run_migration(client: &Client, migration_path: &str) -> Result<()> {
    let workspace_root = PathBuf::from("/workspace");
    let full_path = workspace_root.join(migration_path);

    let sql = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read migration file: {}", full_path.display()))?;

    client
        .batch_execute(&sql)
        .await
        .context("Failed to execute migration SQL")?;

    Ok(())
}

/// Checks if a column exists in the chunks table
async fn column_exists(client: &Client, column_name: &str) -> Result<bool> {
    let row = client
        .query_one(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'maproom'
                  AND table_name = 'chunks'
                  AND column_name = $1
            )",
            &[&column_name],
        )
        .await?;

    Ok(row.get(0))
}

/// Checks if an index exists
async fn index_exists(client: &Client, index_name: &str) -> Result<bool> {
    let row = client
        .query_one(
            "SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE schemaname = 'maproom'
                  AND tablename = 'chunks'
                  AND indexname = $1
            )",
            &[&index_name],
        )
        .await?;

    Ok(row.get(0))
}

/// Counts non-NULL embeddings in a column
async fn count_embeddings(client: &Client, column_name: &str) -> Result<i64> {
    let row = client
        .query_one(
            &format!(
                "SELECT COUNT(*) FROM maproom.chunks WHERE {} IS NOT NULL",
                column_name
            ),
            &[],
        )
        .await?;

    Ok(row.get(0))
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test]
#[serial]
async fn test_migration_0015_forward_and_rollback() {
    println!("\n========================================");
    println!("Testing Migration 0015: Forward and Rollback");
    println!("========================================\n");

    // Create test database
    let client = create_test_database()
        .await
        .expect("Failed to create test database");
    println!("✓ Test database created");

    // Load fixture (100 chunks with OpenAI embeddings)
    println!("\n--- Loading fixture ---");
    create_test_fixture(&client)
        .await
        .expect("Failed to create test fixture");
    println!("✓ Test fixture created: 100 chunks");

    // Verify fixture loaded correctly
    let total_chunks: i64 = client
        .query_one("SELECT COUNT(*) FROM maproom.chunks", &[])
        .await
        .expect("Failed to count chunks")
        .get(0);
    println!("✓ Total chunks in fixture: {}", total_chunks);
    assert_eq!(total_chunks, 100, "Expected 100 chunks in fixture");

    // Count existing OpenAI embeddings before migration
    let before_code_count = count_embeddings(&client, "code_embedding")
        .await
        .expect("Failed to count code_embedding");
    let before_text_count = count_embeddings(&client, "text_embedding")
        .await
        .expect("Failed to count text_embedding");

    println!("✓ OpenAI code embeddings before migration: {}", before_code_count);
    println!("✓ OpenAI text embeddings before migration: {}", before_text_count);

    // The fixture should have 100 code embeddings (one per chunk)
    assert_eq!(before_code_count, 100, "Expected 100 code embeddings in fixture");

    // === FORWARD MIGRATION ===
    println!("\n--- Running forward migration ---");
    let migration_start = Instant::now();

    run_migration_with_concurrently(&client, "crates/maproom/migrations/0015_add_ollama_columns.sql")
        .await
        .expect("Failed to run forward migration");

    let migration_duration = migration_start.elapsed();
    println!("✓ Forward migration completed in {:.2?}", migration_duration);

    // Verify migration completed within 5 seconds (requirement)
    assert!(
        migration_duration.as_secs() < 5,
        "Migration took too long: {:?} (expected < 5s)",
        migration_duration
    );

    // Verify new columns exist
    println!("\n--- Verifying forward migration ---");

    let code_col_exists = column_exists(&client, "code_embedding_ollama")
        .await
        .expect("Failed to check code_embedding_ollama column");
    assert!(code_col_exists, "code_embedding_ollama column should exist");
    println!("✓ code_embedding_ollama column exists");

    let text_col_exists = column_exists(&client, "text_embedding_ollama")
        .await
        .expect("Failed to check text_embedding_ollama column");
    assert!(text_col_exists, "text_embedding_ollama column should exist");
    println!("✓ text_embedding_ollama column exists");

    // Verify new indexes exist
    let code_idx_exists = index_exists(&client, "idx_chunks_code_vec_ollama")
        .await
        .expect("Failed to check code index");
    assert!(code_idx_exists, "idx_chunks_code_vec_ollama index should exist");
    println!("✓ idx_chunks_code_vec_ollama index exists");

    let text_idx_exists = index_exists(&client, "idx_chunks_text_vec_ollama")
        .await
        .expect("Failed to check text index");
    assert!(text_idx_exists, "idx_chunks_text_vec_ollama index should exist");
    println!("✓ idx_chunks_text_vec_ollama index exists");

    // Verify existing OpenAI embeddings preserved (NO DATA LOSS)
    let after_code_count = count_embeddings(&client, "code_embedding")
        .await
        .expect("Failed to count code_embedding after migration");
    let after_text_count = count_embeddings(&client, "text_embedding")
        .await
        .expect("Failed to count text_embedding after migration");

    assert_eq!(
        after_code_count, before_code_count,
        "OpenAI code embeddings should be preserved (no data loss)"
    );
    assert_eq!(
        after_text_count, before_text_count,
        "OpenAI text embeddings should be preserved (no data loss)"
    );
    println!("✓ OpenAI embeddings preserved: {} code, {} text", after_code_count, after_text_count);

    // Verify new Ollama columns are NULL (no data yet)
    let ollama_code_count = count_embeddings(&client, "code_embedding_ollama")
        .await
        .expect("Failed to count code_embedding_ollama");
    let ollama_text_count = count_embeddings(&client, "text_embedding_ollama")
        .await
        .expect("Failed to count text_embedding_ollama");

    assert_eq!(ollama_code_count, 0, "code_embedding_ollama should be NULL initially");
    assert_eq!(ollama_text_count, 0, "text_embedding_ollama should be NULL initially");
    println!("✓ Ollama columns are NULL: {} code, {} text", ollama_code_count, ollama_text_count);

    // === ROLLBACK MIGRATION ===
    println!("\n--- Running rollback migration ---");
    let rollback_start = Instant::now();

    run_migration_with_concurrently(&client, "crates/maproom/migrations/0015_add_ollama_columns_rollback.sql")
        .await
        .expect("Failed to run rollback migration");

    let rollback_duration = rollback_start.elapsed();
    println!("✓ Rollback migration completed in {:.2?}", rollback_duration);

    // Verify rollback completed within 5 seconds
    assert!(
        rollback_duration.as_secs() < 5,
        "Rollback took too long: {:?} (expected < 5s)",
        rollback_duration
    );

    // Verify columns removed
    println!("\n--- Verifying rollback migration ---");

    let code_col_removed = !column_exists(&client, "code_embedding_ollama")
        .await
        .expect("Failed to check code_embedding_ollama column after rollback");
    assert!(code_col_removed, "code_embedding_ollama column should be removed");
    println!("✓ code_embedding_ollama column removed");

    let text_col_removed = !column_exists(&client, "text_embedding_ollama")
        .await
        .expect("Failed to check text_embedding_ollama column after rollback");
    assert!(text_col_removed, "text_embedding_ollama column should be removed");
    println!("✓ text_embedding_ollama column removed");

    // Verify indexes removed
    let code_idx_removed = !index_exists(&client, "idx_chunks_code_vec_ollama")
        .await
        .expect("Failed to check code index after rollback");
    assert!(code_idx_removed, "idx_chunks_code_vec_ollama index should be removed");
    println!("✓ idx_chunks_code_vec_ollama index removed");

    let text_idx_removed = !index_exists(&client, "idx_chunks_text_vec_ollama")
        .await
        .expect("Failed to check text index after rollback");
    assert!(text_idx_removed, "idx_chunks_text_vec_ollama index should be removed");
    println!("✓ idx_chunks_text_vec_ollama index removed");

    // Verify existing OpenAI embeddings STILL preserved after rollback (NO DATA LOSS)
    let final_code_count = count_embeddings(&client, "code_embedding")
        .await
        .expect("Failed to count code_embedding after rollback");
    let final_text_count = count_embeddings(&client, "text_embedding")
        .await
        .expect("Failed to count text_embedding after rollback");

    assert_eq!(
        final_code_count, before_code_count,
        "OpenAI code embeddings should still be preserved after rollback (no data loss)"
    );
    assert_eq!(
        final_text_count, before_text_count,
        "OpenAI text embeddings should still be preserved after rollback (no data loss)"
    );
    println!("✓ OpenAI embeddings still preserved after rollback: {} code, {} text", final_code_count, final_text_count);

    // Cleanup
    drop(client);
    drop_test_database()
        .await
        .expect("Failed to drop test database");
    println!("\n✓ Test database cleaned up");

    println!("\n========================================");
    println!("✓ All migration tests passed!");
    println!("========================================");
    println!("\nSummary:");
    println!("  - Forward migration: {:.2?}", migration_duration);
    println!("  - Rollback migration: {:.2?}", rollback_duration);
    println!("  - Total chunks: {}", total_chunks);
    println!("  - OpenAI embeddings preserved: {} code, {} text", final_code_count, final_text_count);
    println!("  - Zero data loss confirmed ✓");
    println!();
}

#[tokio::test]
#[serial]
async fn test_migration_0015_idempotency() {
    println!("\n========================================");
    println!("Testing Migration 0015: Idempotency");
    println!("========================================\n");

    // Create test database
    let client = create_test_database()
        .await
        .expect("Failed to create test database");
    println!("✓ Test database created");

    // Load fixture
    println!("\n--- Loading fixture ---");
    create_test_fixture(&client)
        .await
        .expect("Failed to create test fixture");
    println!("✓ Test fixture created");

    // Run forward migration twice (should be idempotent)
    println!("\n--- Testing forward migration idempotency ---");
    run_migration_with_concurrently(&client, "crates/maproom/migrations/0015_add_ollama_columns.sql")
        .await
        .expect("Failed to run forward migration (first time)");
    println!("✓ Forward migration (1st run) succeeded");

    run_migration_with_concurrently(&client, "crates/maproom/migrations/0015_add_ollama_columns.sql")
        .await
        .expect("Failed to run forward migration (second time)");
    println!("✓ Forward migration (2nd run) succeeded (idempotent)");

    // Verify columns still exist and only one index
    let code_col_exists = column_exists(&client, "code_embedding_ollama")
        .await
        .expect("Failed to check code_embedding_ollama column");
    assert!(code_col_exists, "code_embedding_ollama column should exist after idempotent migration");

    let code_idx_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM pg_indexes
             WHERE schemaname = 'maproom' AND tablename = 'chunks'
             AND indexname = 'idx_chunks_code_vec_ollama'",
            &[],
        )
        .await
        .expect("Failed to count indexes")
        .get(0);
    assert_eq!(code_idx_count, 1, "Should have exactly one index after idempotent migration");
    println!("✓ Idempotency verified: columns exist, indexes not duplicated");

    // Run rollback twice (should be idempotent)
    println!("\n--- Testing rollback migration idempotency ---");
    run_migration_with_concurrently(&client, "crates/maproom/migrations/0015_add_ollama_columns_rollback.sql")
        .await
        .expect("Failed to run rollback migration (first time)");
    println!("✓ Rollback migration (1st run) succeeded");

    run_migration_with_concurrently(&client, "crates/maproom/migrations/0015_add_ollama_columns_rollback.sql")
        .await
        .expect("Failed to run rollback migration (second time)");
    println!("✓ Rollback migration (2nd run) succeeded (idempotent)");

    // Verify columns removed
    let code_col_removed = !column_exists(&client, "code_embedding_ollama")
        .await
        .expect("Failed to check code_embedding_ollama column after rollback");
    assert!(code_col_removed, "code_embedding_ollama column should be removed after idempotent rollback");
    println!("✓ Idempotency verified: columns removed");

    // Cleanup
    drop(client);
    drop_test_database()
        .await
        .expect("Failed to drop test database");
    println!("\n✓ Test database cleaned up");

    println!("\n========================================");
    println!("✓ All idempotency tests passed!");
    println!("========================================\n");
}
