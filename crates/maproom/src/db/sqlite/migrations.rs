use anyhow::{Context, Result};
use rusqlite::{Connection, params, OptionalExtension};

/// Represents a database migration
struct Migration {
    version: i32,
    name: &'static str,
    up: &'static str,
    #[allow(dead_code)]
    down: &'static str, // Best-effort rollback (not currently used)
}

/// Manages schema migrations for SQLite database
pub struct MigrationRunner<'a> {
    conn: &'a mut Connection,
}

impl<'a> MigrationRunner<'a> {
    /// Create a new migration runner
    pub fn new(conn: &'a mut Connection) -> Self {
        Self { conn }
    }

    /// Get the current schema version (0 if no migrations applied)
    pub fn current_version(&mut self) -> Result<i32> {
        // First ensure schema_migrations table exists
        self.ensure_migration_table()?;

        let version: Option<i32> = self.conn
            .query_row(
                "SELECT MAX(version) FROM schema_migrations",
                [],
                |row| row.get(0)
            )
            .optional()
            .context("Failed to query current schema version")?
            .flatten(); // Flatten Option<Option<i32>> to Option<i32>

        Ok(version.unwrap_or(0))
    }

    /// Check if migrations are needed
    pub fn needs_migration(&mut self) -> Result<bool> {
        let current = self.current_version()?;
        let latest = Self::migrations()
            .iter()
            .map(|m| m.version)
            .max()
            .unwrap_or(0);
        Ok(current < latest)
    }

    /// Apply all pending migrations
    pub fn migrate(&mut self) -> Result<()> {
        // Ensure migration tracking table exists
        self.ensure_migration_table()?;

        let current_version = self.current_version()?;
        let migrations = Self::migrations();

        // Filter to only pending migrations
        let pending: Vec<&Migration> = migrations
            .iter()
            .filter(|m| m.version > current_version)
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        // Apply each pending migration in a transaction
        for migration in pending {
            self.apply_migration(migration)
                .with_context(|| format!("Failed to apply migration {}: {}", migration.version, migration.name))?;
        }

        Ok(())
    }

    /// Ensure the schema_migrations table exists
    fn ensure_migration_table(&mut self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        ).context("Failed to create schema_migrations table")?;
        Ok(())
    }

    /// Apply a single migration in a transaction
    fn apply_migration(&mut self, migration: &Migration) -> Result<()> {
        // Check if already applied (idempotency check)
        let exists: bool = self.conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1",
                params![migration.version],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if exists {
            // Already applied, skip
            return Ok(());
        }

        // Apply migration in a transaction
        let tx = self.conn.transaction()
            .context("Failed to begin transaction for migration")?;

        // Execute the migration SQL
        tx.execute_batch(migration.up)
            .with_context(|| format!("Failed to execute migration SQL for version {}", migration.version))?;

        // Record the migration
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![migration.version, migration.name],
        ).context("Failed to record migration in schema_migrations table")?;

        // Commit the transaction
        tx.commit()
            .context("Failed to commit migration transaction")?;

        Ok(())
    }

    /// Get all defined migrations in version order
    fn migrations() -> Vec<Migration> {
        vec![
            Migration {
                version: 1,
                name: "initial_schema",
                up: r#"
-- Create Repositories table
CREATE TABLE IF NOT EXISTS repos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    root_path TEXT NOT NULL
);

-- Create Worktrees table
CREATE TABLE IF NOT EXISTS worktrees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    abs_path TEXT NOT NULL,
    UNIQUE(repo_id, name)
);

-- Create Commits table
CREATE TABLE IF NOT EXISTS commits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    sha TEXT NOT NULL,
    committed_at DATETIME,
    UNIQUE(repo_id, sha)
);

-- Create Files table
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    worktree_id INTEGER NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
    commit_id INTEGER NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    relpath TEXT NOT NULL,
    language TEXT,
    content_hash TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    last_modified DATETIME,
    UNIQUE(commit_id, relpath, content_hash)
);

-- Create Chunks table
CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    blob_sha TEXT NOT NULL,
    symbol_name TEXT,
    kind TEXT NOT NULL,
    signature TEXT,
    docstring TEXT,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    preview TEXT NOT NULL,
    ts_doc_text TEXT,
    recency_score REAL NOT NULL,
    churn_score REAL NOT NULL,
    metadata JSON,
    worktree_ids JSON NOT NULL,
    UNIQUE(file_id, start_line, end_line)
);

-- Create Chunk Edges table
CREATE TABLE IF NOT EXISTS chunk_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    src_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    dst_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    UNIQUE(src_chunk_id, dst_chunk_id, type)
);

-- Create Vector Table using vec0
CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
    chunk_id INTEGER PRIMARY KEY,
    code_embedding float[1536],
    text_embedding float[1536]
);

-- Create FTS5 Table for code search
CREATE VIRTUAL TABLE IF NOT EXISTS fts_chunks USING fts5(
    content,
    docstring,
    symbol_name,
    content='chunks',
    content_rowid='id'
);
                "#,
                down: r#"
-- Rollback: drop all tables in reverse order
DROP TABLE IF EXISTS fts_chunks;
DROP TABLE IF EXISTS vec_chunks;
DROP TABLE IF EXISTS chunk_edges;
DROP TABLE IF EXISTS chunks;
DROP TABLE IF EXISTS files;
DROP TABLE IF EXISTS commits;
DROP TABLE IF EXISTS worktrees;
DROP TABLE IF EXISTS repos;
                "#,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        // Register extension globally for all new connections
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                crate::db::sqlite::sqlite3_vec_init as *const ()
            )));
        }

        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");

        // Enable foreign keys
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            "#,
        ).expect("Failed to enable foreign keys");

        conn
    }

    #[test]
    fn test_migration_fresh_database() {
        let mut conn = setup_test_db();
        let mut runner = MigrationRunner::new(&mut conn);

        // Fresh database should be at version 0
        assert_eq!(runner.current_version().unwrap(), 0);
        assert!(runner.needs_migration().unwrap());

        // Apply migrations
        runner.migrate().unwrap();

        // Should now be at latest version
        assert_eq!(runner.current_version().unwrap(), 1);
        assert!(!runner.needs_migration().unwrap());

        // Verify core tables exist (vec_chunks and fts_chunks are virtual tables that may fail in tests without extensions)
        let table_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('repos', 'worktrees', 'commits', 'files', 'chunks', 'chunk_edges', 'schema_migrations')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 7, "Expected 7 core tables to be created");
    }

    #[test]
    fn test_migration_idempotent() {
        let mut conn = setup_test_db();
        let mut runner = MigrationRunner::new(&mut conn);

        // Apply migrations twice
        runner.migrate().unwrap();
        let version_after_first = runner.current_version().unwrap();

        runner.migrate().unwrap();
        let version_after_second = runner.current_version().unwrap();

        // Version should be the same
        assert_eq!(version_after_first, version_after_second);
        assert_eq!(version_after_second, 1);

        // Check migration was only recorded once
        let migration_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migration_count, 1);
    }

    #[test]
    fn test_migration_version_tracking() {
        let mut conn = setup_test_db();
        let mut runner = MigrationRunner::new(&mut conn);

        // Initial state
        assert_eq!(runner.current_version().unwrap(), 0);

        // Apply migrations
        runner.migrate().unwrap();

        // Check version tracking
        let (version, name): (i32, String) = conn
            .query_row(
                "SELECT version, name FROM schema_migrations WHERE version = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(version, 1);
        assert_eq!(name, "initial_schema");

        // Verify applied_at timestamp exists
        let applied_at: String = conn
            .query_row(
                "SELECT applied_at FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!applied_at.is_empty());
    }

    #[test]
    fn test_migration_rollback_on_failure() {
        let mut conn = setup_test_db();

        // Create a bad migration that will fail
        let bad_migration = Migration {
            version: 999,
            name: "bad_migration",
            up: "CREATE TABLE test (id INTEGER); INSERT INTO nonexistent_table VALUES (1);",
            down: "",
        };

        // Try to apply the bad migration
        let mut runner = MigrationRunner::new(&mut conn);
        runner.ensure_migration_table().unwrap();
        let result = runner.apply_migration(&bad_migration);

        // Should fail
        assert!(result.is_err());

        // Verify transaction rolled back - the test table should not exist
        let table_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='test'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(!table_exists);

        // Migration should not be recorded
        let recorded: bool = conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 999",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(!recorded);
    }
}
