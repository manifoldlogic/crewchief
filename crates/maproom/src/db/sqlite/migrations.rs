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
            Migration {
                version: 2,
                name: "add_chunk_worktrees",
                up: r#"
-- Create junction table for chunk-worktree many-to-many relationship
CREATE TABLE chunk_worktrees (
    chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    worktree_id INTEGER NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
    PRIMARY KEY (chunk_id, worktree_id)
);

-- Create index for queries filtering by worktree
CREATE INDEX idx_chunk_worktrees_worktree ON chunk_worktrees(worktree_id);
                "#,
                down: r#"
-- Rollback: drop the junction table and its index
DROP INDEX IF EXISTS idx_chunk_worktrees_worktree;
DROP TABLE IF EXISTS chunk_worktrees;
                "#,
            },
            Migration {
                version: 3,
                name: "add_code_embeddings",
                up: r#"
-- Create deduplicated embedding storage table
CREATE TABLE code_embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    blob_sha TEXT NOT NULL UNIQUE,
    embedding BLOB,
    embedding_dim INTEGER NOT NULL DEFAULT 1536,
    model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create index for blob_sha lookups
CREATE INDEX idx_embeddings_blob ON code_embeddings(blob_sha);
                "#,
                down: r#"
-- Rollback: drop the code_embeddings table and its index
DROP INDEX IF EXISTS idx_embeddings_blob;
DROP TABLE IF EXISTS code_embeddings;
                "#,
            },
            Migration {
                version: 4,
                name: "add_vec_code",
                up: r#"
-- Create vector index table using sqlite-vec
-- Note: requires sqlite-vec extension to be loaded
-- If extension is not available, migration will fail with "no such module: vec0"
CREATE VIRTUAL TABLE vec_code USING vec0(
    embedding float[1536]
);
                "#,
                down: r#"
-- Rollback: drop the virtual table
DROP TABLE IF EXISTS vec_code;
                "#,
            },
            Migration {
                version: 5,
                name: "drop_worktree_ids",
                up: r#"
-- Drop the deprecated worktree_ids JSON column
-- Requires SQLite 3.35.0+ for ALTER TABLE DROP COLUMN
ALTER TABLE chunks DROP COLUMN worktree_ids;
                "#,
                down: r#"
-- Rollback: add the worktree_ids column back
-- Note: data will be lost, this is a best-effort rollback
ALTER TABLE chunks ADD COLUMN worktree_ids JSON NOT NULL DEFAULT '[]';
                "#,
            },
            Migration {
                version: 6,
                name: "drop_vec_chunks",
                up: r#"
-- Drop the deprecated vec_chunks virtual table
-- Uses IF EXISTS for safety on fresh databases
DROP TABLE IF EXISTS vec_chunks;
                "#,
                down: r#"
-- Rollback: recreate vec_chunks table
-- Note: data will be lost, this is a best-effort rollback
CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
    chunk_id INTEGER PRIMARY KEY,
    code_embedding float[1536],
    text_embedding float[1536]
);
                "#,
            },
            Migration {
                version: 7,
                name: "add_vec_code_768",
                up: r#"
-- Create vector index table for 768-dimensional embeddings
-- Supports Ollama nomic-embed-text (768-dim) alongside existing 1536-dim
CREATE VIRTUAL TABLE vec_code_768 USING vec0(
    embedding float[768]
);
                "#,
                down: r#"
-- Rollback: drop the 768-dim virtual table
DROP TABLE IF EXISTS vec_code_768;
                "#,
            },
            Migration {
                version: 8,
                name: "add_index_state",
                up: r#"
-- Create index state table for tracking indexing progress
-- Matches PostgreSQL worktree_index_state table columns
CREATE TABLE IF NOT EXISTS index_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    worktree_id INTEGER NOT NULL UNIQUE REFERENCES worktrees(id) ON DELETE CASCADE,
    tree_sha TEXT NOT NULL,
    chunks_processed INTEGER NOT NULL DEFAULT 0,
    embeddings_generated INTEGER NOT NULL DEFAULT 0,
    last_indexed TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create index for worktree_id lookups
CREATE INDEX IF NOT EXISTS idx_index_state_worktree ON index_state(worktree_id);
                "#,
                down: r#"
-- Rollback: drop the index_state table and index
DROP INDEX IF EXISTS idx_index_state_worktree;
DROP TABLE IF EXISTS index_state;
                "#,
            },
            Migration {
                version: 9,
                name: "add_context_cache",
                up: r#"
-- Create context cache table for storing assembled context bundles
-- Supports TTL and LRU eviction strategies
CREATE TABLE IF NOT EXISTS context_cache (
    cache_key TEXT PRIMARY KEY,
    bundle_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    accessed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create index for expiration queries
CREATE INDEX IF NOT EXISTS idx_context_cache_expires ON context_cache(expires_at);

-- Create index for LRU eviction queries
CREATE INDEX IF NOT EXISTS idx_context_cache_accessed ON context_cache(accessed_at);
                "#,
                down: r#"
-- Rollback: drop the context_cache table and indexes
DROP INDEX IF EXISTS idx_context_cache_accessed;
DROP INDEX IF EXISTS idx_context_cache_expires;
DROP TABLE IF EXISTS context_cache;
                "#,
            },
            Migration {
                version: 10,
                name: "add_vec_code_1024",
                up: r#"
-- Create vector index table for 1024-dimensional embeddings
-- Supports mxbai-embed-large (1024-dim) alongside existing 768-dim and 1536-dim
CREATE VIRTUAL TABLE vec_code_1024 USING vec0(
    embedding float[1024]
);
                "#,
                down: r#"
-- Rollback: drop the 1024-dim virtual table
DROP TABLE IF EXISTS vec_code_1024;
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

        // Should now be at latest version (10)
        assert_eq!(runner.current_version().unwrap(), 10);
        assert!(!runner.needs_migration().unwrap());

        // Verify core tables exist (excluding virtual tables and dropped tables)
        let table_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('repos', 'worktrees', 'commits', 'files', 'chunks', 'chunk_edges', 'schema_migrations', 'chunk_worktrees', 'code_embeddings', 'index_state', 'context_cache')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 11, "Expected 11 core tables to be created");

        // Verify vec_chunks table does NOT exist (dropped by migration 6)
        let vec_chunks_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='vec_chunks'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(!vec_chunks_exists, "vec_chunks table should be dropped");

        // Verify chunk_worktrees junction table exists
        let junction_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='chunk_worktrees'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(junction_exists, "chunk_worktrees junction table should exist");

        // Verify code_embeddings table exists
        let embeddings_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='code_embeddings'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(embeddings_exists, "code_embeddings table should exist");
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
        assert_eq!(version_after_second, 10);

        // Check each migration was only recorded once
        let migration_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migration_count, 10, "Expected 10 migrations to be recorded");
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

    #[test]
    fn test_new_migrations_schema() {
        let mut conn = setup_test_db();
        let mut runner = MigrationRunner::new(&mut conn);

        // Apply all migrations
        runner.migrate().unwrap();

        // Verify chunk_worktrees junction table has correct schema
        let junction_schema: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='chunk_worktrees'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(junction_schema.contains("chunk_id"), "chunk_worktrees should have chunk_id column");
        assert!(junction_schema.contains("worktree_id"), "chunk_worktrees should have worktree_id column");
        assert!(junction_schema.contains("PRIMARY KEY"), "chunk_worktrees should have composite primary key");

        // Verify index exists on chunk_worktrees
        let index_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_chunk_worktrees_worktree'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(index_exists, "Index idx_chunk_worktrees_worktree should exist");

        // Verify code_embeddings table has correct schema
        let embeddings_schema: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='code_embeddings'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(embeddings_schema.contains("blob_sha"), "code_embeddings should have blob_sha column");
        assert!(embeddings_schema.contains("UNIQUE"), "code_embeddings should have UNIQUE constraint on blob_sha");
        assert!(embeddings_schema.contains("embedding BLOB"), "code_embeddings should have embedding BLOB column");
        assert!(embeddings_schema.contains("embedding_dim"), "code_embeddings should have embedding_dim column");

        // Verify index exists on code_embeddings
        let blob_index_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_embeddings_blob'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(blob_index_exists, "Index idx_embeddings_blob should exist");

        // Verify vec_code virtual table exists
        let vec_code_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='vec_code'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(vec_code_exists, "vec_code virtual table should exist");

        // Verify worktree_ids column does NOT exist in chunks table (dropped by migration 5)
        let chunks_schema: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='chunks'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!chunks_schema.contains("worktree_ids"), "worktree_ids column should be dropped from chunks table");
    }
}
