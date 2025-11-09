//! Common test utilities for integration tests.
//!
//! This module provides shared test infrastructure including:
//! - Database setup and teardown
//! - Test fixture loading
//! - Configuration helpers
//! - Assertion utilities

use anyhow::{Context, Result};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use std::env;
use std::path::PathBuf;
use tokio_postgres::{Config, NoTls};

/// Test database configuration.
pub struct TestDb {
    pub pool: Pool,
    pub db_name: String,
}

impl TestDb {
    /// Create a new test database with a unique name.
    pub async fn new() -> Result<Self> {
        let db_name = format!("maproom_test_{}", uuid::Uuid::new_v4().simple());

        // Connect to postgres database to create test db
        let postgres_url = env::var("MAPROOM_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/postgres".to_string());

        let (client, connection) = tokio_postgres::connect(&postgres_url, NoTls)
            .await
            .context("Failed to connect to PostgreSQL")?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Create test database
        client
            .execute(&format!("CREATE DATABASE {}", db_name), &[])
            .await
            .context("Failed to create test database")?;

        // Connect to test database
        let test_db_url = postgres_url.replace("/postgres", &format!("/{}", db_name));

        let mut pg_config = Config::new();
        pg_config.host("localhost");
        pg_config.user("postgres");
        pg_config.password("postgres");
        pg_config.dbname(&db_name);

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
        let pool = Pool::builder(mgr)
            .max_size(5)
            .build()
            .context("Failed to create connection pool")?;

        Ok(Self { pool, db_name })
    }

    /// Run migrations on the test database.
    pub async fn run_migrations(&self) -> Result<()> {
        let client = self.pool.get().await?;

        // Enable required extensions
        client
            .execute("CREATE EXTENSION IF NOT EXISTS vector", &[])
            .await
            .context("Failed to create vector extension")?;

        // Run basic schema creation
        // Note: In a real implementation, this would run proper migrations
        client
            .batch_execute(
                r#"
                CREATE TABLE IF NOT EXISTS repos (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    root_path TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS worktrees (
                    id SERIAL PRIMARY KEY,
                    repo_id INTEGER NOT NULL REFERENCES repos(id),
                    name TEXT NOT NULL,
                    path TEXT NOT NULL,
                    commit_hash TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(repo_id, name)
                );

                CREATE TABLE IF NOT EXISTS chunks (
                    id SERIAL PRIMARY KEY,
                    worktree_id INTEGER NOT NULL REFERENCES worktrees(id),
                    rel_path TEXT NOT NULL,
                    chunk_index INTEGER NOT NULL,
                    content TEXT NOT NULL,
                    start_line INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    chunk_type TEXT NOT NULL,
                    language TEXT,
                    embedding vector(1536),
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(worktree_id, rel_path, chunk_index)
                );

                CREATE INDEX IF NOT EXISTS idx_chunks_worktree
                    ON chunks(worktree_id);
                CREATE INDEX IF NOT EXISTS idx_chunks_embedding
                    ON chunks USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

                CREATE TABLE IF NOT EXISTS chunk_edges (
                    id SERIAL PRIMARY KEY,
                    from_chunk_id INTEGER NOT NULL REFERENCES chunks(id),
                    to_chunk_id INTEGER NOT NULL,
                    edge_type TEXT NOT NULL,
                    weight DOUBLE PRECISION DEFAULT 1.0,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );
                "#,
            )
            .await
            .context("Failed to create schema")?;

        Ok(())
    }

    /// Insert test data into the database.
    pub async fn insert_test_data(&self) -> Result<()> {
        let client = self.pool.get().await?;

        // Insert test repo
        let repo_id: i32 = client
            .query_one(
                "INSERT INTO repos (name, root_path) VALUES ($1, $2) RETURNING id",
                &[&"test-repo", &"/tmp/test-repo"],
            )
            .await?
            .get(0);

        // Insert test worktree
        let worktree_id: i32 = client
            .query_one(
                "INSERT INTO worktrees (repo_id, name, path, commit_hash)
                 VALUES ($1, $2, $3, $4) RETURNING id",
                &[&repo_id, &"main", &"/tmp/test-repo", &"abc123"],
            )
            .await?
            .get(0);

        // Insert test chunks
        client
            .execute(
                "INSERT INTO chunks (worktree_id, rel_path, chunk_index, content, start_line, end_line, chunk_type, language)
                 VALUES
                 ($1, 'src/auth.ts', 0, 'export function authenticate(user: User) { return user.isValid(); }', 1, 1, 'function', 'typescript'),
                 ($1, 'src/user.ts', 0, 'export class User { constructor(public name: string) {} }', 1, 1, 'class', 'typescript'),
                 ($1, 'README.md', 0, '# Test Project\n\nThis is a test project for authentication.', 1, 3, 'text', 'markdown')",
                &[&worktree_id],
            )
            .await?;

        Ok(())
    }

    /// Get the connection pool.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Clean up will be handled by the Drop implementation
        // In a real scenario, we'd use async Drop when stable
        let db_name = self.db_name.clone();
        tokio::spawn(async move {
            let postgres_url = env::var("MAPROOM_DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/postgres".to_string());

            if let Ok((client, connection)) = tokio_postgres::connect(&postgres_url, NoTls).await {
                tokio::spawn(async move { connection.await });

                // Terminate connections
                let _ = client
                    .execute(
                        &format!(
                            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
                            db_name
                        ),
                        &[],
                    )
                    .await;

                // Drop database
                let _ = client
                    .execute(&format!("DROP DATABASE IF EXISTS {}", db_name), &[])
                    .await;
            }
        });
    }
}

/// Test configuration helper.
pub struct TestConfig {
    config_dir: PathBuf,
}

impl TestConfig {
    /// Create a temporary test configuration directory.
    pub fn new() -> Result<Self> {
        let config_dir = std::env::temp_dir().join(format!("maproom_test_config_{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&config_dir)?;
        Ok(Self { config_dir })
    }

    /// Write a test configuration file.
    pub fn write_config(&self, filename: &str, content: &str) -> Result<PathBuf> {
        let path = self.config_dir.join(filename);
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// Get the configuration directory path.
    pub fn dir(&self) -> &PathBuf {
        &self.config_dir
    }
}

impl Drop for TestConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.config_dir);
    }
}

/// Assertion utilities for search results.
pub mod assertions {
    use crewchief_maproom::search::ChunkSearchResult;

    /// Assert that search results contain expected content.
    pub fn assert_contains_result(results: &[ChunkSearchResult], expected_content: &str) {
        assert!(
            results.iter().any(|r| r.content.contains(expected_content)),
            "Expected to find content '{}' in results, but it was not present",
            expected_content
        );
    }

    /// Assert that search results are ordered by score (descending).
    pub fn assert_ordered_by_score(results: &[ChunkSearchResult]) {
        for i in 1..results.len() {
            assert!(
                results[i - 1].final_score >= results[i].final_score,
                "Results are not ordered by score: {} < {}",
                results[i - 1].final_score,
                results[i].final_score
            );
        }
    }

    /// Assert that all results have a minimum score threshold.
    pub fn assert_min_score(results: &[ChunkSearchResult], min_score: f64) {
        for result in results {
            assert!(
                result.final_score >= min_score,
                "Result has score {} which is below minimum {}",
                result.final_score,
                min_score
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_creation() {
        let test_db = TestDb::new().await.expect("Failed to create test database");
        assert!(test_db.db_name.starts_with("maproom_test_"));
    }

    #[test]
    fn test_config_creation() {
        let test_config = TestConfig::new().expect("Failed to create test config");
        assert!(test_config.dir().exists());
    }
}
