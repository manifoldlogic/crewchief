pub mod embeddings;
pub mod fts;
pub mod graph;
pub mod hybrid;
pub mod migrations;
pub mod schema;
pub mod vector;

use anyhow::Context;
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::spawn_blocking;

use crate::config::{EdgeQualityWeights, SqliteConfig};
use crate::db::traits::StoreChunks;
use crate::db::traits::StoreCore;
use crate::db::traits::StoreGraph;
use crate::db::traits::StoreSearch;

use crate::db::{ChunkRecord, FileRecord, SearchHit};
use fts::sanitize_fts_term;
use migrations::MigrationRunner;

// Declare the C extension init function from sqlite-vec
// This is provided by the static link
extern "C" {
    fn sqlite3_vec_init(
        db: *mut rusqlite::ffi::sqlite3,
        pzErrMsg: *mut *mut std::os::raw::c_char,
        pApi: *const rusqlite::ffi::sqlite3_api_routines,
    ) -> std::os::raw::c_int;
}

#[derive(Clone)]
pub struct SqliteStore {
    // We use a connection manager with r2d2 for pooling
    // Since rusqlite is sync, we wrap operations in spawn_blocking
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    // Extension verification (cached after first check)
    vec_available: Arc<AtomicBool>,
    vec_checked: Arc<AtomicBool>,
    // Configuration (needed for retry logic)
    config: SqliteConfig,
}

impl SqliteStore {
    pub async fn connect(path: &str) -> anyhow::Result<Self> {
        // Load configuration from environment variables
        let config = SqliteConfig::from_env()
            .context("Failed to load SQLite configuration from environment")?;
        Self::connect_with_config(path, &config).await
    }

    pub async fn connect_with_config(path: &str, config: &SqliteConfig) -> anyhow::Result<Self> {
        let path = if path.starts_with("sqlite://") {
            &path[9..]
        } else {
            path
        };

        // Create parent directory if needed (for auto-created databases)
        // Skip for in-memory databases
        if !path.contains(":memory:") {
            let db_path = std::path::Path::new(path);
            if let Some(parent) = db_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).context(format!(
                        "Failed to create database directory: {}",
                        parent.display()
                    ))?;
                    tracing::info!("Created database directory: {}", parent.display());
                }
            }
        }

        // Register extension globally for all new connections
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite3_vec_init as *const (),
            )));
        }

        // Build PRAGMA statements from configuration
        let pragmas = format!(
            r#"
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = {};
                PRAGMA busy_timeout = {};
                PRAGMA wal_autocheckpoint = {};
                PRAGMA cache_size = {};
                PRAGMA mmap_size = {};
                PRAGMA foreign_keys = ON;
                "#,
            config.pragmas.synchronous,
            config.pragmas.busy_timeout_ms,
            config.pragmas.wal_autocheckpoint,
            -config.pragmas.cache_size_kb, // Negative for KB
            config.pragmas.mmap_size_bytes
        );

        let manager = r2d2_sqlite::SqliteConnectionManager::file(path).with_init(move |conn| {
            conn.execute_batch(&pragmas)?;
            Ok(())
        });

        let mut pool_builder = r2d2::Pool::builder().max_size(config.pool.max_size);
        if let Some(min_idle) = config.pool.min_idle {
            pool_builder = pool_builder.min_idle(Some(min_idle));
        }
        pool_builder = pool_builder.connection_timeout(std::time::Duration::from_millis(
            config.pool.connection_timeout_ms,
        ));

        let pool = pool_builder
            .build(manager)
            .context("Failed to create SQLite connection pool")?;

        tracing::info!(
            pool_size = config.pool.max_size,
            min_idle = ?config.pool.min_idle,
            connection_timeout_ms = config.pool.connection_timeout_ms,
            busy_timeout_ms = config.pragmas.busy_timeout_ms,
            wal_autocheckpoint = config.pragmas.wal_autocheckpoint,
            cache_size_kb = config.pragmas.cache_size_kb,
            mmap_size_bytes = config.pragmas.mmap_size_bytes,
            synchronous = %config.pragmas.synchronous,
            retry_attempts = config.retry.max_attempts,
            retry_base_ms = config.retry.base_delay_ms,
            retry_max_ms = config.retry.max_delay_ms,
            retry_exponential = config.retry.exponential,
            "SQLite connection pool created with configuration"
        );

        // Set secure file permissions on database file (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let db_path = std::path::Path::new(path);
            if db_path.exists() && !path.contains(":memory:") {
                std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600))
                    .context("Failed to set database file permissions")?;
            }
        }

        let store = Self {
            pool,
            vec_available: Arc::new(AtomicBool::new(false)),
            vec_checked: Arc::new(AtomicBool::new(false)),
            config: config.clone(),
        };

        // Auto-run migrations on connect to ensure schema is up to date
        // This is idempotent - migrations track applied versions and skip duplicates
        tracing::debug!("SqliteStore::connect: running migrations...");
        store
            .migrate()
            .await
            .context("Failed to run database migrations")?;
        tracing::debug!("SqliteStore::connect: migrations complete");

        Ok(store)
    }

    // Helper to run a blocking closure with a connection
    pub async fn run<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut rusqlite::Connection) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        spawn_blocking(move || {
            let mut conn = pool.get().context("Failed to get SQLite connection")?;
            f(&mut conn)
        })
        .await?
    }

    /// Execute a write operation with automatic retry on SQLITE_BUSY errors.
    ///
    /// Uses exponential backoff: 50ms → 100ms → 200ms → 400ms → 800ms
    /// Logs warnings for observability of contention issues.
    pub async fn write_with_retry<F, T>(&self, op: F) -> anyhow::Result<T>
    where
        F: FnMut(&mut Connection) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let config = self.config.retry.clone();
        let pool = self.pool.clone();
        let mut delay_ms = config.base_delay_ms;

        // Wrap the operation in an Arc<Mutex<>> so we can use it across multiple spawn_blocking calls
        let op = Arc::new(std::sync::Mutex::new(op));

        for attempt in 1..=config.max_attempts {
            // Clone references for this attempt
            let pool_clone = pool.clone();
            let op_clone = Arc::clone(&op);

            // Execute the operation in a blocking task
            let result = spawn_blocking(move || {
                let mut conn = pool_clone
                    .get()
                    .context("Failed to get SQLite connection")?;
                let mut op_guard = op_clone.lock().unwrap();
                op_guard(&mut conn)
            })
            .await?;

            match result {
                Ok(value) => {
                    if attempt > 1 {
                        tracing::info!(attempt, "Write succeeded after retry");
                    }
                    return Ok(value);
                }
                Err(e) if is_busy_error(&e) && attempt < config.max_attempts => {
                    tracing::warn!(
                        attempt,
                        total_attempts = config.max_attempts,
                        delay_ms,
                        error = %e,
                        "SQLITE_BUSY error, retrying"
                    );

                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

                    // Exponential backoff
                    if config.exponential {
                        delay_ms = (delay_ms * 2).min(config.max_delay_ms);
                    }
                }
                Err(e) => {
                    if is_busy_error(&e) {
                        tracing::error!(
                            max_attempts = config.max_attempts,
                            "Write failed after all retry attempts"
                        );
                    }
                    return Err(e);
                }
            }
        }

        anyhow::bail!("Write retry loop completed without returning (this should be unreachable)")
    }
}

/// Verify that sqlite-vec extension is loaded correctly
fn verify_vec_extension(conn: &Connection) -> bool {
    conn.query_row("SELECT vec_version()", [], |row| row.get::<_, String>(0))
        .is_ok()
}

/// Check if error is a transient lock contention error that should be retried
fn is_busy_error(error: &anyhow::Error) -> bool {
    // Check if the error chain contains a rusqlite busy/locked error
    // We need to iterate through the error chain because rusqlite::Error
    // might be wrapped inside anyhow::Error
    error.chain().any(|cause| {
        cause
            .downcast_ref::<rusqlite::Error>()
            .and_then(|rusqlite_err| match rusqlite_err {
                rusqlite::Error::SqliteFailure(err, _) => Some(matches!(
                    err.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                )),
                _ => Some(false),
            })
            .unwrap_or(false)
    })
}

/// Resolve a repo name to its ID with fuzzy matching.
///
/// Supports:
/// - Exact match: "manifoldlogic/crewchief" -> matches "manifoldlogic/crewchief"
/// - Suffix match: "crewchief" -> matches "manifoldlogic/crewchief"
///
/// Returns the repo ID if found, or an error if not found or ambiguous.
pub fn resolve_repo_id(conn: &Connection, repo: &str) -> anyhow::Result<i64> {
    // Try exact match first
    let exact_match: Option<i64> = conn
        .query_row(
            "SELECT id FROM repos WHERE name = ?1",
            params![repo],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = exact_match {
        return Ok(id);
    }

    // Try suffix match (e.g., "crewchief" matches "owner/crewchief")
    // Use LIKE with escaped pattern for suffix matching
    let pattern = format!("%/{}", repo);
    let suffix_matches: Vec<(i64, String)> = {
        let mut stmt = conn.prepare("SELECT id, name FROM repos WHERE name LIKE ?1")?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    match suffix_matches.len() {
        0 => anyhow::bail!("Repository not found: {}", repo),
        1 => Ok(suffix_matches[0].0),
        _ => {
            let names: Vec<&str> = suffix_matches.iter().map(|(_, n)| n.as_str()).collect();
            anyhow::bail!(
                "Ambiguous repository name '{}'. Matches: {}",
                repo,
                names.join(", ")
            )
        }
    }
}

// =============================================================================
// StoreCore trait implementation
// =============================================================================

#[async_trait]
impl StoreCore for SqliteStore {
    fn has_vec_extension(&self) -> bool {
        self.vec_available.load(Ordering::Relaxed)
    }

    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64> {
        let name = name.to_string();
        let root_path = root_path.to_string();
        self.write_with_retry(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO repos(name, root_path) VALUES (?1, ?2)",
                params![name, root_path],
            )?;

            // If we ignored the insert, we might want to update the root_path
            // Postgres does ON CONFLICT DO UPDATE
            conn.execute(
                "UPDATE repos SET root_path = ?2 WHERE name = ?1",
                params![name, root_path],
            )?;

            let id: i64 = conn.query_row(
                "SELECT id FROM repos WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )?;
            Ok(id)
        })
        .await
    }

    async fn get_or_create_worktree(
        &self,
        repo_id: i64,
        name: &str,
        abs_path: &str,
    ) -> anyhow::Result<i64> {
        let name = name.to_string();
        let abs_path = abs_path.to_string();
        self.write_with_retry(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO worktrees(repo_id, name, abs_path) VALUES (?1, ?2, ?3)",
                params![repo_id, name, abs_path],
            )?;

            conn.execute(
                "UPDATE worktrees SET abs_path = ?3 WHERE repo_id = ?1 AND name = ?2",
                params![repo_id, name, abs_path],
            )?;

            let id: i64 = conn.query_row(
                "SELECT id FROM worktrees WHERE repo_id = ?1 AND name = ?2",
                params![repo_id, name],
                |row| row.get(0),
            )?;
            Ok(id)
        })
        .await
    }

    async fn get_or_create_commit(
        &self,
        repo_id: i64,
        sha: &str,
        committed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64> {
        let sha = sha.to_string();
        self.write_with_retry(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO commits(repo_id, sha, committed_at) VALUES (?1, ?2, ?3)",
                params![repo_id, sha, committed_at],
            )?;

            if let Some(ca) = committed_at {
                conn.execute(
                    "UPDATE commits SET committed_at = ?3 WHERE repo_id = ?1 AND sha = ?2 AND committed_at IS NULL",
                    params![repo_id, sha, ca],
                )?;
            }

            let id: i64 = conn.query_row(
                "SELECT id FROM commits WHERE repo_id = ?1 AND sha = ?2",
                params![repo_id, sha],
                |row| row.get(0),
            )?;
            Ok(id)
        }).await
    }

    async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<crate::db::RepoInfo>> {
        let name = name.to_string();
        self.run(move |conn| {
            let result = conn
                .query_row(
                    "SELECT id, name, root_path FROM repos WHERE name = ?1",
                    params![name],
                    |row| {
                        Ok(crate::db::RepoInfo {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            root_path: row.get(2)?,
                        })
                    },
                )
                .optional()?;
            Ok(result)
        })
        .await
    }

    async fn get_worktree_by_name(
        &self,
        repo_id: i64,
        name: &str,
    ) -> anyhow::Result<Option<crate::db::WorktreeInfo>> {
        let name = name.to_string();
        self.run(move |conn| {
            let result = conn.query_row(
                "SELECT id, repo_id, name, abs_path FROM worktrees WHERE repo_id = ?1 AND name = ?2",
                params![repo_id, name],
                |row| Ok(crate::db::WorktreeInfo {
                    id: row.get(0)?,
                    repo_id: row.get(1)?,
                    name: row.get(2)?,
                    abs_path: row.get(3)?,
                }),
            ).optional()?;
            Ok(result)
        }).await
    }

    async fn list_repos(&self) -> anyhow::Result<Vec<crate::db::RepoInfo>> {
        self.run(move |conn| {
            let mut stmt = conn.prepare("SELECT id, name, root_path FROM repos ORDER BY name")?;
            let repos = stmt
                .query_map([], |row| {
                    Ok(crate::db::RepoInfo {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        root_path: row.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(repos)
        })
        .await
    }

    async fn list_worktrees(&self, repo_id: i64) -> anyhow::Result<Vec<crate::db::WorktreeInfo>> {
        self.run(move |conn| {
            let mut stmt = conn.prepare("SELECT id, repo_id, name, abs_path FROM worktrees WHERE repo_id = ?1 ORDER BY name")?;
            let worktrees = stmt.query_map(params![repo_id], |row| {
                Ok(crate::db::WorktreeInfo {
                    id: row.get(0)?,
                    repo_id: row.get(1)?,
                    name: row.get(2)?,
                    abs_path: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
            Ok(worktrees)
        }).await
    }

    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64> {
        let file = file.clone();
        self.write_with_retry(move |conn| {
            conn.execute(
                "INSERT INTO files (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes, last_modified)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(commit_id, relpath, content_hash) DO UPDATE SET
                   language = COALESCE(excluded.language, files.language),
                   size_bytes = excluded.size_bytes,
                   last_modified = excluded.last_modified",
                params![
                    file.repo_id,
                    file.worktree_id,
                    file.commit_id,
                    file.relpath,
                    file.language,
                    file.content_hash,
                    file.size_bytes,
                    file.last_modified
                ],
            )?;

            let id: i64 = conn.query_row(
                "SELECT id FROM files WHERE commit_id = ?1 AND relpath = ?2 AND content_hash = ?3",
                params![file.commit_id, file.relpath, file.content_hash],
                |row| row.get(0),
            )?;
            Ok(id)
        }).await
    }

    async fn delete_file(&self, file_id: i64) -> anyhow::Result<bool> {
        self.run(move |conn| {
            let rows_deleted = conn.execute("DELETE FROM files WHERE id = ?1", params![file_id])?;
            Ok(rows_deleted > 0)
        })
        .await
    }

    async fn get_file_id_by_relpath(
        &self,
        relpath: &str,
        worktree_id: i64,
    ) -> anyhow::Result<Option<i64>> {
        let relpath = relpath.to_string();
        self.run(move |conn| {
            let result: Option<i64> = conn
                .query_row(
                    "SELECT id FROM files WHERE relpath = ?1 AND worktree_id = ?2",
                    params![relpath, worktree_id],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(result)
        })
        .await
    }

    async fn get_worktree_chunk_count(&self, worktree_id: i64) -> anyhow::Result<i64> {
        self.run(move |conn| {
            // SQLite: count chunks via chunk_worktrees junction table
            let count: i64 = conn.query_row(
                "SELECT COUNT(DISTINCT chunk_id) FROM chunk_worktrees WHERE worktree_id = ?1",
                params![worktree_id],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
    }

    async fn get_worktree_file_count(&self, worktree_id: i64) -> anyhow::Result<i64> {
        self.run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM files WHERE worktree_id = ?1",
                params![worktree_id],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
    }

    async fn get_worktree_embedding_count(&self, worktree_id: i64) -> anyhow::Result<i64> {
        self.run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(DISTINCT cw.chunk_id)
                 FROM chunk_worktrees cw
                 JOIN chunks c ON c.id = cw.chunk_id
                 WHERE cw.worktree_id = ?1
                   AND c.blob_sha IN (SELECT blob_sha FROM code_embeddings)",
                params![worktree_id],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
    }

    async fn get_worktree_language_breakdown(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<Vec<(String, i64)>> {
        self.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT COALESCE(f.language, 'unknown') as lang, COUNT(*) as file_count
                 FROM files f
                 WHERE f.worktree_id = ?1
                 GROUP BY f.language
                 ORDER BY file_count DESC",
            )?;
            let rows = stmt.query_map(params![worktree_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            let results: Vec<(String, i64)> = rows.collect::<Result<_, _>>()?;
            Ok(results)
        })
        .await
    }

    async fn get_worktree_last_scan(&self, worktree_id: i64) -> anyhow::Result<Option<String>> {
        self.run(move |conn| {
            let result = conn
                .query_row(
                    "SELECT last_indexed FROM index_state WHERE worktree_id = ?1",
                    params![worktree_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(result)
        })
        .await
    }

    async fn get_global_chunk_count(&self) -> anyhow::Result<i64> {
        self.run(move |conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(DISTINCT blob_sha) FROM chunks", [], |row| {
                    row.get(0)
                })?;
            Ok(count)
        })
        .await
    }

    /// Get the global count of embeddings.
    async fn get_global_embedding_count(&self) -> anyhow::Result<i64> {
        self.run(move |conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM code_embeddings", [], |row| row.get(0))?;
            Ok(count)
        })
        .await
    }

    /// Get the count of distinct chunks (by blob_sha) for a specific repo.
    ///
    /// Uses chunk_worktrees junction table to filter by repo via worktrees.
    /// Returns 0 if the repo does not exist.
    async fn get_repo_chunk_count(&self, repo_name: &str) -> anyhow::Result<i64> {
        let repo_name = repo_name.to_string();
        self.run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(DISTINCT c.blob_sha)
                 FROM chunks c
                 JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                 JOIN worktrees w ON w.id = cw.worktree_id
                 JOIN repos r ON r.id = w.repo_id
                 WHERE r.name = ?1",
                params![repo_name],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
    }

    /// Get the count of embeddings for a specific repo.
    ///
    /// Counts embeddings whose blob_sha matches chunks in the specified repo.
    /// Returns 0 if the repo does not exist.
    async fn get_repo_embedding_count(&self, repo_name: &str) -> anyhow::Result<i64> {
        let repo_name = repo_name.to_string();
        self.run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(DISTINCT ce.id)
                 FROM code_embeddings ce
                 WHERE ce.blob_sha IN (
                     SELECT DISTINCT c.blob_sha
                     FROM chunks c
                     JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                     JOIN worktrees w ON w.id = cw.worktree_id
                     JOIN repos r ON r.id = w.repo_id
                     WHERE r.name = ?1
                 )",
                params![repo_name],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
    }
}

// =============================================================================
// StoreChunks trait implementation
// =============================================================================

#[async_trait]
impl StoreChunks for SqliteStore {
    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64> {
        let chunk = chunk.clone();
        self.write_with_retry(move |conn| {
            let tx = conn.transaction()?;

            // For JSON fields, we need to serialize to string if rusqlite doesn't support JSON directly
            let metadata_json = chunk.metadata.as_ref().map(|v| v.to_string());

            // SQLite UPSERT - no longer includes worktree_ids column
            tx.execute(
                "INSERT INTO chunks (
                   file_id, blob_sha, symbol_name, kind, signature, docstring,
                   start_line, end_line, preview, ts_doc_text, recency_score,
                   churn_score, metadata
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
                   blob_sha = excluded.blob_sha,
                   symbol_name = excluded.symbol_name,
                   kind = excluded.kind,
                   signature = excluded.signature,
                   docstring = excluded.docstring,
                   preview = excluded.preview,
                   ts_doc_text = excluded.ts_doc_text,
                   metadata = excluded.metadata,
                   recency_score = excluded.recency_score,
                   churn_score = excluded.churn_score
                 ",
                params![
                    chunk.file_id,
                    chunk.blob_sha,
                    chunk.symbol_name,
                    chunk.kind,
                    chunk.signature,
                    chunk.docstring,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.preview,
                    chunk.ts_doc_text,
                    chunk.recency_score,
                    chunk.churn_score,
                    metadata_json,
                ],
            )?;

            // Get the chunk ID
            let chunk_id: i64 = tx.query_row(
                "SELECT id FROM chunks WHERE file_id = ?1 AND start_line = ?2 AND end_line = ?3",
                params![chunk.file_id, chunk.start_line, chunk.end_line],
                |row| row.get(0),
            )?;

            // Insert into junction table (INSERT OR IGNORE handles duplicates)
            tx.execute(
                "INSERT OR IGNORE INTO chunk_worktrees (chunk_id, worktree_id) VALUES (?1, ?2)",
                params![chunk_id, chunk.worktree_id],
            )?;

            // Update FTS index manually
            tx.execute(
                "INSERT OR REPLACE INTO fts_chunks(rowid, content, docstring, symbol_name) VALUES (?1, ?2, ?3, ?4)",
                params![chunk_id, chunk.preview, chunk.docstring, chunk.symbol_name],
            )?;

            tx.commit()?;
            Ok(chunk_id)
        }).await
    }

    async fn insert_chunks_batch(&self, chunks: &[ChunkRecord]) -> anyhow::Result<Vec<i64>> {
        let chunks = chunks.to_vec();
        self.write_with_retry(move |conn| {
            let tx = conn.transaction()?;
            let mut ids = Vec::new();

            {
                let mut stmt = tx.prepare(
                    "INSERT INTO chunks (
                       file_id, blob_sha, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc_text, recency_score, churn_score, metadata
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                     ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
                       blob_sha = excluded.blob_sha,
                       symbol_name = excluded.symbol_name,
                       kind = excluded.kind,
                       signature = excluded.signature,
                       docstring = excluded.docstring,
                       preview = excluded.preview,
                       ts_doc_text = excluded.ts_doc_text,
                       metadata = excluded.metadata,
                       recency_score = excluded.recency_score,
                       churn_score = excluded.churn_score
                     RETURNING id"
                )?;

                let mut junction_stmt = tx.prepare(
                    "INSERT OR IGNORE INTO chunk_worktrees (chunk_id, worktree_id) VALUES (?1, ?2)"
                )?;

                let mut fts_stmt = tx.prepare(
                    "INSERT OR REPLACE INTO fts_chunks(rowid, content, docstring, symbol_name) VALUES (?1, ?2, ?3, ?4)"
                )?;

                for chunk in &chunks {
                    let metadata_json = chunk.metadata.as_ref().map(|v| v.to_string());

                    let chunk_id: i64 = stmt.query_row(params![
                        chunk.file_id,
                        chunk.blob_sha,
                        chunk.symbol_name,
                        chunk.kind,
                        chunk.signature,
                        chunk.docstring,
                        chunk.start_line,
                        chunk.end_line,
                        chunk.preview,
                        chunk.ts_doc_text,
                        chunk.recency_score,
                        chunk.churn_score,
                        metadata_json,
                    ], |row| row.get(0))?;

                    // Insert into junction table
                    junction_stmt.execute(params![chunk_id, chunk.worktree_id])?;

                    fts_stmt.execute(params![chunk_id, chunk.preview, chunk.docstring, chunk.symbol_name])?;
                    ids.push(chunk_id);
                }
            }

            tx.commit()?;
            Ok(ids)
        }).await
    }

    async fn insert_chunk_edge(
        &self,
        src_chunk_id: i64,
        dst_chunk_id: i64,
        edge_type: &str,
    ) -> anyhow::Result<()> {
        let edge_type = edge_type.to_string();
        self.write_with_retry(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES (?1, ?2, ?3)",
                params![src_chunk_id, dst_chunk_id, edge_type],
            )?;
            Ok(())
        }).await
    }

    async fn find_chunk_by_symbol(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        symbol_name: &str,
        relpath: Option<&str>,
    ) -> anyhow::Result<Option<i64>> {
        let symbol_name = symbol_name.to_string();
        let relpath = relpath.map(|s| s.to_string());
        self.run(move |conn| {
            // Use reference to avoid move of relpath
            let relpath_ref = relpath.as_deref();

            // Similar to Postgres logic
            let sql = if relpath_ref.is_some() {
                if worktree_id.is_some() {
                    "SELECT c.id FROM chunks c
                     JOIN files f ON f.id = c.file_id
                     WHERE f.repo_id = ?1 AND f.worktree_id = ?2
                       AND f.relpath = ?3 AND c.symbol_name = ?4
                     ORDER BY c.id DESC LIMIT 1"
                } else {
                    "SELECT c.id FROM chunks c
                     JOIN files f ON f.id = c.file_id
                     WHERE f.repo_id = ?1
                       AND f.relpath = ?3 AND c.symbol_name = ?4
                     ORDER BY c.id DESC LIMIT 1"
                }
            } else {
                if worktree_id.is_some() {
                    "SELECT c.id FROM chunks c
                     JOIN files f ON f.id = c.file_id
                     WHERE f.repo_id = ?1 AND f.worktree_id = ?2 AND c.symbol_name = ?4
                     ORDER BY c.id DESC LIMIT 1"
                } else {
                    "SELECT c.id FROM chunks c
                     JOIN files f ON f.id = c.file_id
                     WHERE f.repo_id = ?1 AND c.symbol_name = ?4
                     ORDER BY c.id DESC LIMIT 1"
                }
            };

            let id: Option<i64> = if let Some(path) = relpath_ref {
                if let Some(wid) = worktree_id {
                    conn.query_row(sql, params![repo_id, wid, path, symbol_name], |row| {
                        row.get(0)
                    })
                    .optional()?
                } else {
                    conn.query_row(sql, params![repo_id, path, symbol_name], |row| row.get(0))
                        .optional()?
                }
            } else {
                if let Some(wid) = worktree_id {
                    conn.query_row(sql, params![repo_id, wid, symbol_name], |row| row.get(0))
                        .optional()?
                } else {
                    conn.query_row(sql, params![repo_id, symbol_name], |row| row.get(0))
                        .optional()?
                }
            };

            Ok(id)
        })
        .await
    }

    async fn get_chunk_by_id(&self, chunk_id: i64) -> anyhow::Result<Option<crate::db::ChunkFull>> {
        self.run(move |conn| {
            let result = conn
                .query_row(
                    "SELECT c.id, c.file_id, c.blob_sha, c.symbol_name, c.kind, c.signature,
                        c.docstring, c.start_line, c.end_line, c.preview, f.relpath
                 FROM chunks c
                 JOIN files f ON f.id = c.file_id
                 WHERE c.id = ?1",
                    params![chunk_id],
                    |row| {
                        Ok(crate::db::ChunkFull {
                            id: row.get(0)?,
                            file_id: row.get(1)?,
                            blob_sha: row.get(2)?,
                            symbol_name: row.get(3)?,
                            kind: row.get(4)?,
                            signature: row.get(5)?,
                            docstring: row.get(6)?,
                            start_line: row.get(7)?,
                            end_line: row.get(8)?,
                            preview: row.get(9)?,
                            file_path: row.get(10)?,
                        })
                    },
                )
                .optional()?;
            Ok(result)
        })
        .await
    }

    async fn get_file_chunks(&self, file_id: i64) -> anyhow::Result<Vec<crate::db::ChunkSummary>> {
        self.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT c.id, c.symbol_name, c.kind, c.start_line, c.end_line, f.relpath
                 FROM chunks c
                 JOIN files f ON f.id = c.file_id
                 WHERE c.file_id = ?1
                 ORDER BY c.start_line ASC",
            )?;

            let rows = stmt.query_map(params![file_id], |row| {
                Ok(crate::db::ChunkSummary {
                    id: row.get(0)?,
                    symbol_name: row.get(1)?,
                    kind: row.get(2)?,
                    start_line: row.get(3)?,
                    end_line: row.get(4)?,
                    file_path: row.get(5)?,
                })
            })?;

            let mut chunks = Vec::new();
            for chunk_result in rows {
                chunks.push(chunk_result?);
            }
            Ok(chunks)
        })
        .await
    }

    async fn get_chunk_context(
        &self,
        chunk_id: i64,
        surrounding: usize,
    ) -> anyhow::Result<Option<crate::db::ChunkContext>> {
        self.run(move |conn| {
            // First, get the target chunk
            let chunk = conn
                .query_row(
                    "SELECT c.id, c.file_id, c.blob_sha, c.symbol_name, c.kind, c.signature,
                        c.docstring, c.start_line, c.end_line, c.preview, f.relpath
                 FROM chunks c
                 JOIN files f ON f.id = c.file_id
                 WHERE c.id = ?1",
                    params![chunk_id],
                    |row| {
                        Ok(crate::db::ChunkFull {
                            id: row.get(0)?,
                            file_id: row.get(1)?,
                            blob_sha: row.get(2)?,
                            symbol_name: row.get(3)?,
                            kind: row.get(4)?,
                            signature: row.get(5)?,
                            docstring: row.get(6)?,
                            start_line: row.get(7)?,
                            end_line: row.get(8)?,
                            preview: row.get(9)?,
                            file_path: row.get(10)?,
                        })
                    },
                )
                .optional()?;

            let chunk = match chunk {
                Some(c) => c,
                None => return Ok(None),
            };

            // Get surrounding chunks from the same file, ordered by line proximity
            let mut stmt = conn.prepare(
                "SELECT c.id, c.symbol_name, c.kind, c.start_line, c.end_line, f.relpath
                 FROM chunks c
                 JOIN files f ON f.id = c.file_id
                 WHERE c.file_id = ?1 AND c.id != ?2
                 ORDER BY ABS(c.start_line - ?3)
                 LIMIT ?4",
            )?;

            let rows = stmt.query_map(
                params![
                    chunk.file_id,
                    chunk_id,
                    chunk.start_line,
                    (surrounding as i64 * 2)
                ],
                |row| {
                    Ok(crate::db::ChunkSummary {
                        id: row.get(0)?,
                        symbol_name: row.get(1)?,
                        kind: row.get(2)?,
                        start_line: row.get(3)?,
                        end_line: row.get(4)?,
                        file_path: row.get(5)?,
                    })
                },
            )?;

            let mut surrounding_chunks = Vec::new();
            for chunk_result in rows {
                surrounding_chunks.push(chunk_result?);
            }

            Ok(Some(crate::db::ChunkContext {
                file_path: chunk.file_path.clone(),
                chunk,
                surrounding_chunks,
            }))
        })
        .await
    }

    async fn delete_chunks_by_file(&self, file_id: i64) -> anyhow::Result<u64> {
        self.write_with_retry(move |conn| {
            // Get count of chunks to delete
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM chunks WHERE file_id = ?1",
                    params![file_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            // Delete embeddings for chunks in this file
            conn.execute(
                "DELETE FROM code_embeddings WHERE blob_sha IN (
                     SELECT blob_sha FROM chunks WHERE file_id = ?1
                 )",
                params![file_id],
            )?;

            // Delete chunk_worktrees entries
            conn.execute(
                "DELETE FROM chunk_worktrees WHERE chunk_id IN (
                     SELECT id FROM chunks WHERE file_id = ?1
                 )",
                params![file_id],
            )?;

            // Delete chunk edges
            conn.execute(
                "DELETE FROM chunk_edges WHERE src_chunk_id IN (
                     SELECT id FROM chunks WHERE file_id = ?1
                 ) OR dst_chunk_id IN (
                     SELECT id FROM chunks WHERE file_id = ?1
                 )",
                params![file_id],
            )?;

            // Delete chunks
            conn.execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])?;

            Ok(count as u64)
        })
        .await
    }

    /// Get all chunks for a worktree with their file paths.
    ///
    /// Returns chunk ID and file relative path for all chunks in the specified worktree.
    /// Used by clean-ignored command to identify chunks matching ignore patterns.
    ///
    /// # Arguments
    /// * `worktree_id` - Worktree ID
    ///
    /// # Returns
    /// * `Ok(Vec<(chunk_id, file_relpath)>)` - List of chunks with their file paths
    async fn get_chunks_for_worktree(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<Vec<(i64, String)>> {
        self.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT c.id, f.relpath
                 FROM chunks c
                 JOIN files f ON c.file_id = f.id
                 WHERE f.worktree_id = ?1",
            )?;

            let chunks = stmt
                .query_map(params![worktree_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(chunks)
        })
        .await
    }

    /// Delete chunks by their IDs.
    ///
    /// Deletes specified chunks and cleans up related data (edges, embeddings, junction table).
    /// Used by clean-ignored command to remove chunks matching ignore patterns.
    ///
    /// # Arguments
    /// * `_worktree_id` - Worktree ID (reserved for future validation/context)
    /// * `chunk_ids` - List of chunk IDs to delete
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of chunks deleted
    async fn delete_chunks_by_ids(
        &self,
        _worktree_id: i64,
        chunk_ids: &[i64],
    ) -> anyhow::Result<usize> {
        if chunk_ids.is_empty() {
            return Ok(0);
        }

        // SQLite has a limit of 32766 SQL variables (SQLITE_MAX_VARIABLE_NUMBER).
        // To avoid "too many SQL variables" errors, batch deletions into chunks of 500.
        // This allows deleting large numbers of chunks (e.g., 10,000+) without hitting limits.
        const BATCH_SIZE: usize = 500;

        let chunk_ids = chunk_ids.to_vec();
        self.run(move |conn| {
            let mut total_deleted = 0;

            // Process chunks in batches
            for batch in chunk_ids.chunks(BATCH_SIZE) {
                // Create placeholders for SQL IN clause
                let placeholders = batch.iter().map(|_| "?").collect::<Vec<_>>().join(",");

                // Delete embeddings for these chunks
                let embeddings_query = format!(
                    "DELETE FROM code_embeddings WHERE blob_sha IN (
                         SELECT blob_sha FROM chunks WHERE id IN ({})
                     )",
                    placeholders
                );
                let mut stmt = conn.prepare(&embeddings_query)?;
                stmt.execute(rusqlite::params_from_iter(batch.iter()))?;

                // Delete chunk_worktrees junction entries
                let junction_query = format!(
                    "DELETE FROM chunk_worktrees WHERE chunk_id IN ({})",
                    placeholders
                );
                let mut stmt = conn.prepare(&junction_query)?;
                stmt.execute(rusqlite::params_from_iter(batch.iter()))?;

                // Delete chunk_edges
                let edges_query = format!(
                    "DELETE FROM chunk_edges WHERE src_chunk_id IN ({}) OR dst_chunk_id IN ({})",
                    placeholders, placeholders
                );
                let mut stmt = conn.prepare(&edges_query)?;
                let mut params: Vec<i64> = Vec::new();
                params.extend_from_slice(batch);
                params.extend_from_slice(batch);
                stmt.execute(rusqlite::params_from_iter(params.iter()))?;

                // Delete chunks themselves and accumulate count
                let chunks_query = format!("DELETE FROM chunks WHERE id IN ({})", placeholders);
                let mut stmt = conn.prepare(&chunks_query)?;
                let rows_affected = stmt.execute(rusqlite::params_from_iter(batch.iter()))?;
                total_deleted += rows_affected;
            }

            Ok(total_deleted)
        })
        .await
    }

    async fn get_chunks_by_blob_sha(
        &self,
        blob_sha: &str,
    ) -> anyhow::Result<Vec<crate::db::ChunkSummary>> {
        let blob_sha = blob_sha.to_string();
        self.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT c.id, c.symbol_name, c.kind, c.start_line, c.end_line, f.relpath
                 FROM chunks c
                 JOIN files f ON c.file_id = f.id
                 WHERE c.blob_sha = ?1
                 ORDER BY c.start_line",
            )?;

            let rows = stmt.query_map(params![blob_sha], |row| {
                Ok(crate::db::ChunkSummary {
                    id: row.get(0)?,
                    symbol_name: row.get(1)?,
                    kind: row.get(2)?,
                    start_line: row.get(3)?,
                    end_line: row.get(4)?,
                    file_path: row.get(5)?,
                })
            })?;

            let mut chunks = Vec::new();
            for chunk_result in rows {
                chunks.push(chunk_result?);
            }
            Ok(chunks)
        })
        .await
    }

    /// Add chunk to additional worktree
    async fn add_chunk_to_worktree(&self, chunk_id: i64, worktree_id: i64) -> anyhow::Result<()> {
        self.run(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO chunk_worktrees (chunk_id, worktree_id) VALUES (?1, ?2)",
                params![chunk_id, worktree_id],
            )?;
            Ok(())
        })
        .await
    }

    /// Get all worktrees containing this chunk
    async fn get_chunk_worktrees(&self, chunk_id: i64) -> anyhow::Result<Vec<i64>> {
        self.run(move |conn| {
            let mut stmt =
                conn.prepare("SELECT worktree_id FROM chunk_worktrees WHERE chunk_id = ?1")?;
            let rows = stmt.query_map(params![chunk_id], |row| row.get(0))?;
            let mut ids = Vec::new();
            for id in rows {
                ids.push(id?);
            }
            Ok(ids)
        })
        .await
    }
}

// =============================================================================
// StoreSearch - FTS, vector, and hybrid search
// =============================================================================

#[async_trait]
impl StoreSearch for SqliteStore {
    #[allow(clippy::too_many_arguments)]
    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        _debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let repo = repo.to_string();
        let worktree = worktree.map(|s| s.to_string());
        let query = query.to_string();
        let kind_filter: Option<Vec<String>> = kind_filter.map(|k| k.to_vec());
        let lang_filter: Option<Vec<String>> = lang_filter.map(|l| l.to_vec());
        self.run(move |conn| {
            // Resolve repo/worktree ids with fuzzy matching
            let repo_id = resolve_repo_id(conn, &repo)?;

            let worktree_id: Option<i64> = if let Some(w) = worktree {
                conn.query_row(
                    "SELECT id FROM worktrees WHERE repo_id = ?1 AND name = ?2",
                    params![repo_id, w],
                    |row| row.get(0),
                )
                .optional()?
            } else {
                None
            };

            // FTS5 query syntax: term1 term2 (implicit AND), term1 OR term2
            // Prefix matching: term* (no quotes around term!)
            // Invalid: "term"* (wildcard outside quotes is syntax error)
            let fts_query = query
                .split_whitespace()
                .filter(|t| !t.is_empty())
                .map(|t| {
                    // Sanitize: remove FTS5 special characters
                    let clean = sanitize_fts_term(t);
                    if clean.trim().is_empty() {
                        return String::new();
                    }
                    // FTS5 prefix syntax: term* (no quotes!)
                    format!("{}*", clean.trim())
                })
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" OR "); // Use OR for broader matching

            // SQL query with ranking
            // SQLite FTS5 rank is built-in function 'bm25' or 'rank'
            // We join with chunks and files

            // Build dynamic filter conditions
            // Base params: ?1 = fts_query, ?2 = repo_id
            // With worktree: ?3 = worktree_id, then filters, then LIMIT
            // Without worktree: filters start at ?3, then LIMIT
            let mut param_idx: usize = if worktree_id.is_some() { 4 } else { 3 };
            let mut filter_conditions = Vec::new();

            if let Some(ref kinds) = kind_filter {
                if !kinds.is_empty() {
                    let placeholders = (0..kinds.len())
                        .map(|i| format!("?{}", param_idx + i))
                        .collect::<Vec<_>>()
                        .join(", ");
                    filter_conditions.push(format!("c.kind IN ({})", placeholders));
                    param_idx += kinds.len();
                }
            }

            if let Some(ref langs) = lang_filter {
                if !langs.is_empty() {
                    let placeholders = (0..langs.len())
                        .map(|i| format!("?{}", param_idx + i))
                        .collect::<Vec<_>>()
                        .join(", ");
                    filter_conditions.push(format!("f.language IN ({})", placeholders));
                    param_idx += langs.len();
                }
            }

            let filter_clause = if filter_conditions.is_empty() {
                String::new()
            } else {
                format!(" AND {}", filter_conditions.join(" AND "))
            };

            let limit_placeholder = format!("?{}", param_idx);

            let sql = if worktree_id.is_some() {
                format!(
                    r#"
                SELECT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    fts_chunks.rank as score,
                    c.preview
                FROM fts_chunks
                JOIN chunks c ON c.id = fts_chunks.rowid
                JOIN files f ON f.id = c.file_id
                JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                WHERE fts_chunks MATCH ?1
                  AND f.repo_id = ?2
                  AND cw.worktree_id = ?3
                  {}
                ORDER BY score
                LIMIT {}
                "#,
                    filter_clause, limit_placeholder
                )
            } else {
                format!(
                    r#"
                SELECT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    fts_chunks.rank as score,
                    c.preview
                FROM fts_chunks
                JOIN chunks c ON c.id = fts_chunks.rowid
                JOIN files f ON f.id = c.file_id
                WHERE fts_chunks MATCH ?1
                  AND f.repo_id = ?2
                  {}
                ORDER BY score
                LIMIT {}
                "#,
                    filter_clause, limit_placeholder
                )
            };

            // Build dynamic parameter list
            let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            param_values.push(Box::new(fts_query));
            param_values.push(Box::new(repo_id));

            if let Some(wid) = worktree_id {
                param_values.push(Box::new(wid));
            }

            if let Some(ref kinds) = kind_filter {
                for kind in kinds {
                    param_values.push(Box::new(kind.clone()));
                }
            }

            if let Some(ref langs) = lang_filter {
                for lang in langs {
                    param_values.push(Box::new(lang.clone()));
                }
            }

            param_values.push(Box::new(k));

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn.prepare(&sql)?;

            let mut hits = Vec::new();
            // Columns: id(0), start_line(1), end_line(2), symbol_name(3), kind(4), relpath(5), score(6), preview(7)
            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                let score: f64 = row.get(6)?;
                Ok(SearchHit {
                    chunk_id: row.get(0)?,
                    start_line: row.get(1)?,
                    end_line: row.get(2)?,
                    symbol_name: row.get(3)?,
                    kind: row.get(4)?,
                    file_relpath: row.get(5)?,
                    score: -score, // FTS5 rank is negative, negate for positive score
                    base_score: None,
                    kind_mult: None,
                    exact_mult: None,
                    preview: Some(row.get(7)?),
                })
            })?;
            for row in rows {
                hits.push(row?);
            }
            Ok(hits)
        })
        .await
    }

    /// FTS search by repo_id and worktree_id (for use by search executors)
    async fn search_fts_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query: &str,
        normalized_query: &str,
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let query = query.to_string();
        let normalized_query = normalized_query.to_string();
        self.run(move |conn| {
            // FTS5 query syntax: term1 term2 (implicit AND), term1 OR term2
            // Prefix matching: term* (no quotes around term!)
            let fts_query = query
                .split_whitespace()
                .filter(|t| !t.is_empty())
                .map(|t| {
                    // Sanitize: remove FTS5 special characters
                    let clean = sanitize_fts_term(t);
                    if clean.trim().is_empty() {
                        return String::new();
                    }
                    // FTS5 prefix syntax: term* (no quotes!)
                    format!("{}*", clean.trim())
                })
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" OR ");

            if fts_query.is_empty() {
                return Ok(vec![]);
            }

            // Helper closure to compute exact match multiplier
            let compute_exact_mult = |symbol_name: Option<String>| -> f64 {
                if let Some(ref symbol) = symbol_name {
                    let normalized_symbol = crate::search::fts::normalize_for_exact_match(symbol);
                    if normalized_symbol.to_lowercase() == normalized_query.to_lowercase() {
                        return 3.0; // Exact match boost
                    }
                }
                1.0 // No boost
            };

            let sql = if worktree_id.is_some() {
                r#"
                SELECT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    fts_chunks.rank as score
                FROM fts_chunks
                JOIN chunks c ON c.id = fts_chunks.rowid
                JOIN files f ON f.id = c.file_id
                JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                WHERE fts_chunks MATCH ?1
                  AND f.repo_id = ?2
                  AND cw.worktree_id = ?3
                ORDER BY score
                LIMIT ?4
                "#
            } else {
                r#"
                SELECT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    fts_chunks.rank as score
                FROM fts_chunks
                JOIN chunks c ON c.id = fts_chunks.rowid
                JOIN files f ON f.id = c.file_id
                WHERE fts_chunks MATCH ?1
                  AND f.repo_id = ?2
                ORDER BY score
                LIMIT ?3
                "#
            };

            let mut stmt = conn.prepare(sql)?;
            let mut hits = Vec::new();

            if let Some(wid) = worktree_id {
                let rows = stmt.query_map(params![fts_query, repo_id, wid, k], |row| {
                    let score: f64 = row.get(6)?;
                    let symbol_name: Option<String> = row.get(3)?;
                    let exact_mult = compute_exact_mult(symbol_name.clone());
                    Ok(SearchHit {
                        chunk_id: row.get(0)?,
                        start_line: row.get(1)?,
                        end_line: row.get(2)?,
                        symbol_name,
                        kind: row.get(4)?,
                        file_relpath: row.get(5)?,
                        score: -score, // FTS5 rank is negative, negate for positive score
                        base_score: None,
                        kind_mult: None,
                        exact_mult: Some(exact_mult),
                        preview: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            } else {
                let rows = stmt.query_map(params![fts_query, repo_id, k], |row| {
                    let score: f64 = row.get(6)?;
                    let symbol_name: Option<String> = row.get(3)?;
                    let exact_mult = compute_exact_mult(symbol_name.clone());
                    Ok(SearchHit {
                        chunk_id: row.get(0)?,
                        start_line: row.get(1)?,
                        end_line: row.get(2)?,
                        symbol_name,
                        kind: row.get(4)?,
                        file_relpath: row.get(5)?,
                        score: -score, // FTS5 rank is negative, negate for positive score
                        base_score: None,
                        kind_mult: None,
                        exact_mult: Some(exact_mult),
                        preview: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            }
            Ok(hits)
        })
        .await
    }

    /// Vector search by repo_id and worktree_id (for use by search executors)
    async fn search_vector_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query_embedding: &[f32],
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // Graceful degradation if sqlite-vec not available
        if !self.has_vec_extension() {
            return Ok(vec![]);
        }

        let embedding = query_embedding.to_vec();
        let limit = k as usize;

        self.run(move |conn| {
            // Validate embedding dimension
            let dimension = embedding.len();
            let vec_table = match dimension {
                768 => "vec_code_768",
                1536 => "vec_code",
                _ => return Ok(vec![]), // Unsupported dimension, return empty
            };

            let query_blob = embeddings::vec_to_blob(&embedding);

            // Use repo_id directly in query
            let sql = if worktree_id.is_some() {
                format!(
                    r#"
                    SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, v.distance
                    FROM {} v
                    JOIN code_embeddings e ON e.id = v.rowid
                    JOIN chunks c ON c.blob_sha = e.blob_sha
                    JOIN files f ON f.id = c.file_id
                    JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                    WHERE v.embedding MATCH ?1
                      AND k = ?4
                      AND f.repo_id = ?2
                      AND cw.worktree_id = ?3
                    ORDER BY v.distance ASC
                    "#,
                    vec_table
                )
            } else {
                format!(
                    r#"
                    SELECT DISTINCT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, v.distance
                    FROM {} v
                    JOIN code_embeddings e ON e.id = v.rowid
                    JOIN chunks c ON c.blob_sha = e.blob_sha
                    JOIN files f ON f.id = c.file_id
                    WHERE v.embedding MATCH ?1
                      AND k = ?3
                      AND f.repo_id = ?2
                    ORDER BY v.distance ASC
                    "#,
                    vec_table
                )
            };

            let mut stmt = conn.prepare(&sql)?;
            let mut hits = Vec::new();

            if let Some(wid) = worktree_id {
                let rows = stmt.query_map(params![query_blob, repo_id, wid, limit as i64], |row| {
                    let distance: f64 = row.get(6)?;
                    Ok(SearchHit {
                        chunk_id: row.get(0)?,
                        start_line: row.get(1)?,
                        end_line: row.get(2)?,
                        symbol_name: row.get(3)?,
                        kind: row.get(4)?,
                        file_relpath: row.get(5)?,
                        score: vector::distance_to_similarity(distance),
                        base_score: None,
                        kind_mult: None,
                        exact_mult: None,
                        preview: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            } else {
                let rows = stmt.query_map(params![query_blob, repo_id, limit as i64], |row| {
                    let distance: f64 = row.get(6)?;
                    Ok(SearchHit {
                        chunk_id: row.get(0)?,
                        start_line: row.get(1)?,
                        end_line: row.get(2)?,
                        symbol_name: row.get(3)?,
                        kind: row.get(4)?,
                        file_relpath: row.get(5)?,
                        score: vector::distance_to_similarity(distance),
                        base_score: None,
                        kind_mult: None,
                        exact_mult: None,
                        preview: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            }
            Ok(hits)
        }).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // Graceful degradation if sqlite-vec not available
        if !self.has_vec_extension() {
            return Ok(vec![]);
        }

        let repo = repo.to_string();
        let worktree = worktree.map(|s| s.to_string());
        let embedding = embedding.to_vec();
        let limit = k as usize;
        let kind_filter: Option<Vec<String>> = kind_filter.map(|k| k.to_vec());
        let lang_filter: Option<Vec<String>> = lang_filter.map(|l| l.to_vec());

        self.run(move |conn| {
            // Resolve repo/worktree ids with fuzzy matching
            let repo_id = resolve_repo_id(conn, &repo)?;

            let worktree_id: Option<i64> = if let Some(ref w) = worktree {
                conn.query_row(
                    "SELECT id FROM worktrees WHERE repo_id = ?1 AND name = ?2",
                    params![repo_id, w],
                    |row| row.get(0),
                )
                .optional()?
            } else {
                None
            };

            // Get vector search results (chunk_id, distance, similarity)
            let vec_results = vector::search_vector(
                conn,
                &repo,
                worktree.as_deref(),
                &embedding,
                limit,
                kind_filter.as_deref(),
                lang_filter.as_deref(),
            )?;

            // Convert VectorResult to SearchHit by fetching chunk metadata
            let mut hits = Vec::new();
            for vec_result in vec_results {
                // Fetch chunk details with file relpath
                let chunk_id = vec_result.chunk_id;
                // Columns: start_line(0), end_line(1), symbol_name(2), kind(3), relpath(4), preview(5)
                let hit_result = if let Some(wid) = worktree_id {
                    conn.query_row(
                        r#"
                        SELECT c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, c.preview
                        FROM chunks c
                        JOIN files f ON f.id = c.file_id
                        JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                        WHERE c.id = ?1 AND cw.worktree_id = ?2
                        "#,
                        params![vec_result.chunk_id, wid],
                        |row| {
                            Ok(SearchHit {
                                chunk_id,
                                start_line: row.get(0)?,
                                end_line: row.get(1)?,
                                symbol_name: row.get(2)?,
                                kind: row.get(3)?,
                                file_relpath: row.get(4)?,
                                score: vec_result.similarity,
                                base_score: if debug {
                                    Some(vec_result.similarity)
                                } else {
                                    None
                                },
                                kind_mult: None, // TODO: Apply kind multipliers like PostgreSQL
                                exact_mult: None,
                                preview: Some(row.get(5)?),
                            })
                        },
                    )
                    .optional()?
                } else {
                    conn.query_row(
                        r#"
                        SELECT c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, c.preview
                        FROM chunks c
                        JOIN files f ON f.id = c.file_id
                        WHERE c.id = ?1
                        "#,
                        params![vec_result.chunk_id],
                        |row| {
                            Ok(SearchHit {
                                chunk_id,
                                start_line: row.get(0)?,
                                end_line: row.get(1)?,
                                symbol_name: row.get(2)?,
                                kind: row.get(3)?,
                                file_relpath: row.get(4)?,
                                score: vec_result.similarity,
                                base_score: if debug {
                                    Some(vec_result.similarity)
                                } else {
                                    None
                                },
                                kind_mult: None, // TODO: Apply kind multipliers like PostgreSQL
                                exact_mult: None,
                                preview: Some(row.get(5)?),
                            })
                        },
                    )
                    .optional()?
                };

                if let Some(hit) = hit_result {
                    hits.push(hit);
                }
            }

            Ok(hits)
        })
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn search_chunks_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        embedding: &[f32],
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // Check vec extension availability before entering blocking closure
        let has_vec = self.has_vec_extension();

        let repo = repo.to_string();
        let worktree = worktree.map(|s| s.to_string());
        let query = query.to_string();
        let embedding = embedding.to_vec();
        let limit = k as usize;
        let kind_filter: Option<Vec<String>> = kind_filter.map(|k| k.to_vec());
        let lang_filter: Option<Vec<String>> = lang_filter.map(|l| l.to_vec());

        self.run(move |conn| {
            // Resolve repo/worktree ids with fuzzy matching
            let repo_id = resolve_repo_id(conn, &repo)?;

            let worktree_id: Option<i64> = if let Some(ref w) = worktree {
                conn.query_row(
                    "SELECT id FROM worktrees WHERE repo_id = ?1 AND name = ?2",
                    params![repo_id, w],
                    |row| row.get(0),
                )
                .optional()?
            } else {
                None
            };

            // Run FTS and vector search in sequence (no async in blocking closure)
            let fts_results = fts::search_fts(
                conn,
                &repo,
                worktree.as_deref(),
                &query,
                limit * 3,
                kind_filter.as_deref(),
                lang_filter.as_deref(),
            )?;

            // Vector search with graceful fallback
            let vec_results = if has_vec {
                vector::search_vector(
                    conn,
                    &repo,
                    worktree.as_deref(),
                    &embedding,
                    limit * 3,
                    kind_filter.as_deref(),
                    lang_filter.as_deref(),
                )?
            } else {
                vec![]
            };

            // Combine using RRF
            let weights = hybrid::HybridWeights::default();
            let hybrid_results =
                hybrid::combine_results(&fts_results, &vec_results, &weights, limit);

            // Convert HybridResult to SearchHit by fetching chunk metadata
            let mut hits = Vec::new();
            for hybrid_result in hybrid_results {
                // Fetch chunk details with file relpath
                let chunk_id = hybrid_result.chunk_id;
                let hit_result = if let Some(wid) = worktree_id {
                    conn.query_row(
                        r#"
                        SELECT c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath
                        FROM chunks c
                        JOIN files f ON f.id = c.file_id
                        JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                        WHERE c.id = ?1 AND cw.worktree_id = ?2
                        "#,
                        params![hybrid_result.chunk_id, wid],
                        |row| {
                            Ok(SearchHit {
                                chunk_id,
                                start_line: row.get(0)?,
                                end_line: row.get(1)?,
                                symbol_name: row.get(2)?,
                                kind: row.get(3)?,
                                file_relpath: row.get(4)?,
                                score: hybrid_result.score,
                                base_score: if debug {
                                    Some(hybrid_result.score)
                                } else {
                                    None
                                },
                                kind_mult: None, // RRF score already incorporates semantic ranking
                                exact_mult: None,
                                preview: None,
                            })
                        },
                    )
                    .optional()?
                } else {
                    conn.query_row(
                        r#"
                        SELECT c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath
                        FROM chunks c
                        JOIN files f ON f.id = c.file_id
                        WHERE c.id = ?1
                        "#,
                        params![hybrid_result.chunk_id],
                        |row| {
                            Ok(SearchHit {
                                chunk_id,
                                start_line: row.get(0)?,
                                end_line: row.get(1)?,
                                symbol_name: row.get(2)?,
                                kind: row.get(3)?,
                                file_relpath: row.get(4)?,
                                score: hybrid_result.score,
                                base_score: if debug {
                                    Some(hybrid_result.score)
                                } else {
                                    None
                                },
                                kind_mult: None, // RRF score already incorporates semantic ranking
                                exact_mult: None,
                                preview: None,
                            })
                        },
                    )
                    .optional()?
                };

                if let Some(hit) = hit_result {
                    hits.push(hit);
                }
            }

            Ok(hits)
        })
        .await
    }

    /// Hybrid search combining FTS5 and vector search using Reciprocal Rank Fusion
    ///
    /// Combines keyword matching (FTS5) with semantic similarity (vectors) to provide
    /// comprehensive search results. Falls back to FTS-only when vector search is
    /// unavailable or returns no results.
    ///
    /// # Arguments
    /// * `repo` - Repository name to filter by
    /// * `worktree` - Optional worktree name to filter by
    /// * `query` - User's search query (for FTS)
    /// * `query_embedding` - Query embedding vector (for semantic search)
    /// * `limit` - Maximum number of results to return
    /// * `weights` - Weights for combining FTS and vector contributions
    async fn search_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: hybrid::HybridWeights,
    ) -> anyhow::Result<Vec<hybrid::HybridResult>> {
        // Over-fetch from each source for better fusion coverage
        let fetch_limit = limit * 3;

        // Run FTS and vector search in parallel
        let (fts_results, vec_results) = tokio::join!(
            self.search_fts(repo, worktree, query, fetch_limit),
            self.search_vector(repo, worktree, query_embedding, fetch_limit),
        );

        let fts_results = fts_results?;
        let vec_results = vec_results?;

        // Combine results using RRF
        let results = hybrid::combine_results(&fts_results, &vec_results, &weights, limit);

        Ok(results)
    }

    /// Get metadata for multiple chunks (batch query for semantic ranking)
    ///
    /// Returns a map of chunk_id -> ChunkMetadata with kind, symbol_name, and recency_score.
    async fn get_chunks_metadata(
        &self,
        chunk_ids: &[i64],
    ) -> anyhow::Result<std::collections::HashMap<i64, hybrid::ChunkMetadata>> {
        if chunk_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let chunk_ids = chunk_ids.to_vec();

        self.run(move |conn| {
            let placeholders: Vec<String> =
                (1..=chunk_ids.len()).map(|i| format!("?{}", i)).collect();
            let sql = format!(
                "SELECT id, kind, symbol_name, recency_score FROM chunks WHERE id IN ({})",
                placeholders.join(", ")
            );

            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::ToSql> = chunk_ids
                .iter()
                .map(|id| id as &dyn rusqlite::ToSql)
                .collect();

            let rows = stmt.query_map(params.as_slice(), |row| {
                let id: i64 = row.get(0)?;
                let kind: String = row.get(1)?;
                let symbol_name: Option<String> = row.get(2)?;
                let recency_score: f64 = row.get(3)?;
                Ok((
                    id,
                    hybrid::ChunkMetadata {
                        kind,
                        symbol_name,
                        recency_score,
                    },
                ))
            })?;

            let mut map = std::collections::HashMap::new();
            for result in rows {
                let (id, metadata) = result?;
                map.insert(id, metadata);
            }
            Ok(map)
        })
        .await
    }

    /// Hybrid search with semantic ranking applied
    ///
    /// Combines FTS5 and vector search using RRF, then applies semantic ranking
    /// based on chunk kind, symbol name matching, and recency.
    ///
    /// # Arguments
    /// * `repo` - Repository name to filter by
    /// * `worktree` - Optional worktree name to filter by
    /// * `query` - User's search query (for FTS and exact match detection)
    /// * `query_embedding` - Query embedding vector (for semantic search)
    /// * `limit` - Maximum number of results to return
    /// * `weights` - Weights for combining FTS and vector contributions
    /// * `ranking` - Semantic ranking configuration
    async fn search_hybrid_ranked(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: hybrid::HybridWeights,
        ranking: hybrid::SemanticRanking,
    ) -> anyhow::Result<Vec<hybrid::RankedSearchHit>> {
        // Over-fetch by 2x before ranking to ensure good results after re-ordering
        let fetch_limit = limit * 2;

        // Get base hybrid results
        let hits = self
            .search_hybrid(repo, worktree, query, query_embedding, fetch_limit, weights)
            .await?;

        if hits.is_empty() {
            return Ok(vec![]);
        }

        // Fetch chunk metadata for all results
        let chunk_ids: Vec<i64> = hits.iter().map(|h| h.chunk_id).collect();
        let metadata = self.get_chunks_metadata(&chunk_ids).await?;

        // Build ranked hits with metadata
        let mut ranked: Vec<hybrid::RankedSearchHit> = hits
            .into_iter()
            .filter_map(|h| {
                let meta = metadata.get(&h.chunk_id)?;
                Some(hybrid::RankedSearchHit {
                    chunk_id: h.chunk_id,
                    score: h.score,
                    fts_rank: h.fts_rank,
                    vector_rank: h.vector_rank,
                    kind: meta.kind.clone(),
                    symbol_name: meta.symbol_name.clone(),
                    recency_score: meta.recency_score,
                    source: h.source,
                })
            })
            .collect();

        // Apply semantic ranking
        hybrid::apply_semantic_ranking(&mut ranked, query, &ranking);

        // Take top N after re-ranking
        ranked.truncate(limit);

        Ok(ranked)
    }
}

// =============================================================================
// StoreGraph - Graph traversal and importance scoring
// =============================================================================

#[async_trait]
impl StoreGraph for SqliteStore {
    /// Find all chunks that call the target chunk (directly or transitively)
    ///
    /// # Arguments
    /// * `target_chunk_id` - The chunk to find callers for
    /// * `max_depth` - Maximum traversal depth (default 3, max 10)
    ///
    /// # Returns
    /// Vector of GraphResult ordered by depth (closest first)
    async fn find_callers(
        &self,
        target_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<graph::GraphResult>> {
        self.run(move |conn| graph::find_callers(conn, target_chunk_id, max_depth))
            .await
    }

    /// Find all chunks called by the source chunk (directly or transitively)
    ///
    /// # Arguments
    /// * `source_chunk_id` - The chunk to find callees for
    /// * `max_depth` - Maximum traversal depth (default 3, max 10)
    ///
    /// # Returns
    /// Vector of GraphResult ordered by depth (closest first)
    async fn find_callees(
        &self,
        source_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<graph::GraphResult>> {
        self.run(move |conn| graph::find_callees(conn, source_chunk_id, max_depth))
            .await
    }

    /// Find import relationships for a chunk
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk to find imports for
    /// * `direction` - Incoming (who imports this) or Outgoing (what this imports)
    /// * `max_depth` - Maximum traversal depth (default 3, max 10)
    ///
    /// # Returns
    /// Vector of GraphResult ordered by depth (closest first)
    async fn find_imports(
        &self,
        chunk_id: i64,
        direction: graph::ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<graph::GraphResult>> {
        self.run(move |conn| graph::find_imports(conn, chunk_id, direction, max_depth))
            .await
    }

    /// Find extension/inheritance relationships for a chunk
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk to find extensions for
    /// * `direction` - Incoming (what extends this) or Outgoing (what this extends)
    /// * `max_depth` - Maximum traversal depth (default 3, max 10)
    ///
    /// # Returns
    /// Vector of GraphResult ordered by depth (closest first)
    async fn find_extensions(
        &self,
        chunk_id: i64,
        direction: graph::ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<graph::GraphResult>> {
        self.run(move |conn| graph::find_extensions(conn, chunk_id, direction, max_depth))
            .await
    }

    /// Get all direct edges from or to a chunk (without recursion)
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk to find edges for
    /// * `direction` - Incoming (edges pointing to chunk) or Outgoing (edges from chunk)
    ///
    /// # Returns
    /// Vector of GraphResult with depth=1 for all direct relationships
    async fn get_direct_edges(
        &self,
        chunk_id: i64,
        direction: graph::ImportDirection,
    ) -> anyhow::Result<Vec<graph::GraphResult>> {
        self.run(move |conn| graph::get_direct_edges(conn, chunk_id, direction))
            .await
    }

    /// Calculate graph importance scores for chunks in a repo/worktree
    ///
    /// Uses edge counts (callers, importers, tests) to calculate PageRank-like scores.
    ///
    /// When `enable_quality` is false: uses legacy edge counting with hardcoded weights
    /// (calls=0.3, imports=0.2, tests=0.1, logarithmic scaling).
    ///
    /// When `enable_quality` is true: uses quality-weighted edge scoring with test detection
    /// using configurable weights from EdgeQualityWeights.
    ///
    /// # Arguments
    /// * `repo_id` - Repository ID to query
    /// * `worktree_id` - Optional worktree ID to filter by
    /// * `limit` - Maximum number of results
    /// * `enable_quality` - If true, use quality-weighted scoring; if false, use legacy scoring
    /// * `weights` - Edge quality weights (production_code, test_code, calls)
    async fn calculate_graph_importance(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        enable_quality: bool,
        weights: &EdgeQualityWeights,
    ) -> anyhow::Result<Vec<SearchHit>> {
        tracing::debug!(
            repo_id = repo_id,
            worktree_id = ?worktree_id,
            limit = limit,
            enable_quality = enable_quality,
            production_code_weight = weights.production_code,
            test_code_weight = weights.test_code,
            calls_weight = weights.calls,
            "Calculating graph importance"
        );

        if !enable_quality {
            // Call legacy implementation
            return self
                .calculate_graph_importance_legacy(repo_id, worktree_id, limit)
                .await;
        }

        // New quality-weighted implementation with configurable weights
        self.calculate_graph_importance_quality(repo_id, worktree_id, limit, weights)
            .await
    }

    /// Calculate graph importance for specific chunk IDs
    ///
    /// Uses edge counts to calculate PageRank-like scores for the given chunks.
    async fn calculate_graph_importance_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        if chunk_ids.is_empty() {
            return Ok(vec![]);
        }

        let chunk_ids = chunk_ids.to_vec();
        self.run(move |conn| {
            // Build placeholders for IN clause
            let placeholders = chunk_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

            let sql = if worktree_id.is_some() {
                format!(
                    r#"
                    WITH edge_counts AS (
                        SELECT
                            dst_chunk_id as chunk_id,
                            SUM(CASE WHEN type = 'calls' THEN 1 ELSE 0 END) as callers,
                            SUM(CASE WHEN type = 'imports' THEN 1 ELSE 0 END) as importers,
                            SUM(CASE WHEN type = 'test_of' THEN 1 ELSE 0 END) as tests
                        FROM chunk_edges
                        WHERE dst_chunk_id IN ({})
                        GROUP BY dst_chunk_id
                    )
                    SELECT
                        c.id,
                        c.start_line,
                        c.end_line,
                        c.symbol_name,
                        c.kind,
                        f.relpath,
                        COALESCE(
                            (ln(2 + COALESCE(e.callers, 0)) * 0.3 +
                             ln(2 + COALESCE(e.importers, 0)) * 0.2 +
                             ln(2 + COALESCE(e.tests, 0)) * 0.1),
                            0
                        ) as graph_score
                    FROM chunks c
                    JOIN files f ON f.id = c.file_id
                    JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                    LEFT JOIN edge_counts e ON e.chunk_id = c.id
                    WHERE c.id IN ({}) AND f.repo_id = ? AND cw.worktree_id = ?
                    ORDER BY graph_score DESC
                    "#,
                    placeholders, placeholders
                )
            } else {
                format!(
                    r#"
                    WITH edge_counts AS (
                        SELECT
                            dst_chunk_id as chunk_id,
                            SUM(CASE WHEN type = 'calls' THEN 1 ELSE 0 END) as callers,
                            SUM(CASE WHEN type = 'imports' THEN 1 ELSE 0 END) as importers,
                            SUM(CASE WHEN type = 'test_of' THEN 1 ELSE 0 END) as tests
                        FROM chunk_edges
                        WHERE dst_chunk_id IN ({})
                        GROUP BY dst_chunk_id
                    )
                    SELECT DISTINCT
                        c.id,
                        c.start_line,
                        c.end_line,
                        c.symbol_name,
                        c.kind,
                        f.relpath,
                        COALESCE(
                            (ln(2 + COALESCE(e.callers, 0)) * 0.3 +
                             ln(2 + COALESCE(e.importers, 0)) * 0.2 +
                             ln(2 + COALESCE(e.tests, 0)) * 0.1),
                            0
                        ) as graph_score
                    FROM chunks c
                    JOIN files f ON f.id = c.file_id
                    LEFT JOIN edge_counts e ON e.chunk_id = c.id
                    WHERE c.id IN ({}) AND f.repo_id = ?
                    ORDER BY graph_score DESC
                    "#,
                    placeholders, placeholders
                )
            };

            let mut stmt = conn.prepare(&sql)?;
            let mut hits = Vec::new();

            // Build parameter list
            let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = chunk_ids
                .iter()
                .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
                .collect();

            // Duplicate chunk_ids for the second IN clause
            for id in &chunk_ids {
                param_values.push(Box::new(*id));
            }

            // Add repo_id
            param_values.push(Box::new(repo_id));

            // Add worktree_id if present
            if let Some(wid) = worktree_id {
                param_values.push(Box::new(wid));
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();

            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                Ok(SearchHit {
                    chunk_id: row.get(0)?,
                    start_line: row.get(1)?,
                    end_line: row.get(2)?,
                    symbol_name: row.get(3)?,
                    kind: row.get(4)?,
                    file_relpath: row.get(5)?,
                    score: row.get(6)?,
                    base_score: None,
                    kind_mult: None,
                    exact_mult: None,
                    preview: None,
                })
            })?;
            for row in rows {
                hits.push(row?);
            }
            Ok(hits)
        })
        .await
    }

    /// Calculate signal scores (recency + churn) for chunks in a repo/worktree
    ///
    /// Combines recency_score and churn_score from chunks table with configurable weights.
    async fn calculate_signal_scores(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchHit>> {
        self.run(move |conn| {
            let sql = if worktree_id.is_some() {
                r#"
                SELECT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    (c.recency_score * ?3 + c.churn_score * ?4) as combined_signal
                FROM chunks c
                JOIN files f ON f.id = c.file_id
                JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                WHERE f.repo_id = ?1 AND cw.worktree_id = ?2
                ORDER BY combined_signal DESC
                LIMIT ?5
                "#
            } else {
                r#"
                SELECT DISTINCT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    (c.recency_score * ?2 + c.churn_score * ?3) as combined_signal
                FROM chunks c
                JOIN files f ON f.id = c.file_id
                WHERE f.repo_id = ?1
                ORDER BY combined_signal DESC
                LIMIT ?4
                "#
            };

            let mut stmt = conn.prepare(sql)?;
            let mut hits = Vec::new();

            if let Some(wid) = worktree_id {
                let rows = stmt.query_map(
                    params![repo_id, wid, recency_weight, churn_weight, limit as i64],
                    |row| {
                        Ok(SearchHit {
                            chunk_id: row.get(0)?,
                            start_line: row.get(1)?,
                            end_line: row.get(2)?,
                            symbol_name: row.get(3)?,
                            kind: row.get(4)?,
                            file_relpath: row.get(5)?,
                            score: row.get(6)?,
                            base_score: None,
                            kind_mult: None,
                            exact_mult: None,
                            preview: None,
                        })
                    },
                )?;
                for row in rows {
                    hits.push(row?);
                }
            } else {
                let rows = stmt.query_map(
                    params![repo_id, recency_weight, churn_weight, limit as i64],
                    |row| {
                        Ok(SearchHit {
                            chunk_id: row.get(0)?,
                            start_line: row.get(1)?,
                            end_line: row.get(2)?,
                            symbol_name: row.get(3)?,
                            kind: row.get(4)?,
                            file_relpath: row.get(5)?,
                            score: row.get(6)?,
                            base_score: None,
                            kind_mult: None,
                            exact_mult: None,
                            preview: None,
                        })
                    },
                )?;
                for row in rows {
                    hits.push(row?);
                }
            }
            Ok(hits)
        })
        .await
    }

    /// Calculate signal scores for specific chunk IDs
    ///
    /// Combines recency_score and churn_score for the given chunks.
    async fn calculate_signal_scores_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
    ) -> anyhow::Result<Vec<SearchHit>> {
        if chunk_ids.is_empty() {
            return Ok(vec![]);
        }

        let chunk_ids = chunk_ids.to_vec();
        self.run(move |conn| {
            let placeholders = chunk_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

            let sql = if worktree_id.is_some() {
                format!(
                    r#"
                    SELECT
                        c.id,
                        c.start_line,
                        c.end_line,
                        c.symbol_name,
                        c.kind,
                        f.relpath,
                        (c.recency_score * ? + c.churn_score * ?) as combined_signal
                    FROM chunks c
                    JOIN files f ON f.id = c.file_id
                    JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                    WHERE c.id IN ({}) AND f.repo_id = ? AND cw.worktree_id = ?
                    ORDER BY combined_signal DESC
                    "#,
                    placeholders
                )
            } else {
                format!(
                    r#"
                    SELECT DISTINCT
                        c.id,
                        c.start_line,
                        c.end_line,
                        c.symbol_name,
                        c.kind,
                        f.relpath,
                        (c.recency_score * ? + c.churn_score * ?) as combined_signal
                    FROM chunks c
                    JOIN files f ON f.id = c.file_id
                    WHERE c.id IN ({}) AND f.repo_id = ?
                    ORDER BY combined_signal DESC
                    "#,
                    placeholders
                )
            };

            let mut stmt = conn.prepare(&sql)?;
            let mut hits = Vec::new();

            // Build parameters: recency_weight, churn_weight, chunk_ids..., repo_id, [worktree_id]
            let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            param_values.push(Box::new(recency_weight));
            param_values.push(Box::new(churn_weight));
            for id in &chunk_ids {
                param_values.push(Box::new(*id));
            }
            param_values.push(Box::new(repo_id));
            if let Some(wid) = worktree_id {
                param_values.push(Box::new(wid));
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();

            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                Ok(SearchHit {
                    chunk_id: row.get(0)?,
                    start_line: row.get(1)?,
                    end_line: row.get(2)?,
                    symbol_name: row.get(3)?,
                    kind: row.get(4)?,
                    file_relpath: row.get(5)?,
                    score: row.get(6)?,
                    base_score: None,
                    kind_mult: None,
                    exact_mult: None,
                    preview: None,
                })
            })?;
            for row in rows {
                hits.push(row?);
            }
            Ok(hits)
        })
        .await
    }
}

// Database operations - remaining inherent methods
impl SqliteStore {
    pub async fn get_last_indexed_tree(&self, worktree_id: i64) -> anyhow::Result<String> {
        self.run(move |conn| {
            let result = conn.query_row(
                "SELECT tree_sha FROM index_state WHERE worktree_id = ?1",
                params![worktree_id],
                |row| row.get(0),
            );

            match result {
                Ok(sha) => Ok(sha),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok("init".to_string()),
                Err(e) => Err(e.into()),
            }
        })
        .await
    }

    pub async fn update_index_state(
        &self,
        worktree_id: i64,
        tree_sha: &str,
        stats: &crate::db::UpdateStats,
    ) -> anyhow::Result<()> {
        let tree_sha = tree_sha.to_string();
        let chunks_processed = stats.chunks_processed;
        let embeddings_generated = stats.embeddings_generated;
        self.write_with_retry(move |conn| {
            conn.execute(
                "INSERT INTO index_state (worktree_id, tree_sha, chunks_processed, embeddings_generated, last_indexed)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))
                 ON CONFLICT(worktree_id) DO UPDATE SET
                     tree_sha = excluded.tree_sha,
                     chunks_processed = excluded.chunks_processed,
                     embeddings_generated = excluded.embeddings_generated,
                     last_indexed = datetime('now')",
                params![worktree_id, tree_sha, chunks_processed, embeddings_generated],
            )?;
            Ok(())
        }).await
    }

    pub async fn detect_stale_worktrees(&self) -> anyhow::Result<Vec<crate::db::StaleWorktree>> {
        self.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT w.id, w.repo_id, w.name, w.abs_path FROM worktrees w ORDER BY w.id",
            )?;

            let worktrees: Vec<_> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            let mut stale = Vec::new();
            for (id, repo_id, name, abs_path) in worktrees {
                let path = std::path::Path::new(&abs_path);
                let exists = path.exists() && path.is_dir();

                // Count chunks for this worktree via files
                let chunk_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM chunks c
                     JOIN files f ON c.file_id = f.id
                     WHERE f.worktree_id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                if !exists {
                    stale.push(crate::db::StaleWorktree {
                        id,
                        repo_id,
                        name,
                        abs_path,
                        exists: false,
                        chunk_count,
                    });
                }
            }
            Ok(stale)
        })
        .await
    }

    pub async fn delete_worktree_data(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<crate::db::WorktreeCleanupResult> {
        self.write_with_retry(move |conn| {
            // Get count of chunks that will be deleted
            let chunks_deleted: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM chunks c
                 JOIN files f ON c.file_id = f.id
                 WHERE f.worktree_id = ?1",
                    params![worktree_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            // Get count of files that will be deleted
            let files_deleted: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM files WHERE worktree_id = ?1",
                    params![worktree_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            // Delete code_embeddings for chunks in this worktree
            let embeddings_deleted: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM code_embeddings ce
                 WHERE ce.blob_sha IN (
                     SELECT c.blob_sha FROM chunks c
                     JOIN files f ON c.file_id = f.id
                     WHERE f.worktree_id = ?1
                 )",
                    params![worktree_id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            conn.execute(
                "DELETE FROM code_embeddings WHERE blob_sha IN (
                     SELECT c.blob_sha FROM chunks c
                     JOIN files f ON c.file_id = f.id
                     WHERE f.worktree_id = ?1
                 )",
                params![worktree_id],
            )?;

            // Delete chunk_worktrees junction entries
            conn.execute(
                "DELETE FROM chunk_worktrees WHERE worktree_id = ?1",
                params![worktree_id],
            )?;

            // Delete chunk_edges for chunks in this worktree
            conn.execute(
                "DELETE FROM chunk_edges WHERE src_chunk_id IN (
                     SELECT c.id FROM chunks c
                     JOIN files f ON c.file_id = f.id
                     WHERE f.worktree_id = ?1
                 ) OR dst_chunk_id IN (
                     SELECT c.id FROM chunks c
                     JOIN files f ON c.file_id = f.id
                     WHERE f.worktree_id = ?1
                 )",
                params![worktree_id],
            )?;

            // Delete chunks
            conn.execute(
                "DELETE FROM chunks WHERE file_id IN (
                     SELECT id FROM files WHERE worktree_id = ?1
                 )",
                params![worktree_id],
            )?;

            // Delete files
            conn.execute(
                "DELETE FROM files WHERE worktree_id = ?1",
                params![worktree_id],
            )?;

            // Delete index_state
            conn.execute(
                "DELETE FROM index_state WHERE worktree_id = ?1",
                params![worktree_id],
            )?;

            Ok(crate::db::WorktreeCleanupResult {
                chunks_deleted: chunks_deleted as u64,
                files_deleted: files_deleted as u64,
                embeddings_deleted: embeddings_deleted as u64,
            })
        })
        .await
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        self.run(move |conn| {
            let mut runner = MigrationRunner::new(conn);
            runner.migrate()
        })
        .await?;

        // Check extension availability after migration
        self.run(|conn| {
            let available = verify_vec_extension(conn);
            Ok(available)
        })
        .await
        .map(|available| {
            self.vec_available.store(available, Ordering::Relaxed);
            self.vec_checked.store(true, Ordering::Relaxed);
            if !available {
                tracing::warn!("sqlite-vec extension not loaded - vector search disabled");
            }
        })?;

        Ok(())
    }

    pub async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>> {
        self.run(move |conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
                    [],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if !exists {
                return Ok(HashSet::new());
            }

            let mut stmt = conn.prepare("SELECT version FROM schema_migrations")?;
            let rows = stmt.query_map([], |row| row.get(0))?;

            let mut versions = HashSet::new();
            for version in rows {
                versions.insert(version?);
            }
            Ok(versions)
        })
        .await
    }

    /// Store or update embedding by content hash
    pub async fn upsert_embedding(
        &self,
        blob_sha: &str,
        embedding: &[f32],
        model_version: &str,
    ) -> anyhow::Result<i64> {
        let blob_sha = blob_sha.to_string();
        let embedding_vec = embedding.to_vec();
        let model_version = model_version.to_string();

        let embedding_id = self
            .run(move |conn| {
                embeddings::upsert_embedding(conn, &blob_sha, &embedding_vec, &model_version)
            })
            .await?;

        // Sync to vec_code table
        self.sync_embedding_to_vec(embedding_id, embedding).await?;

        Ok(embedding_id)
    }

    /// Batch upsert embeddings with deduplication
    pub async fn upsert_embeddings_batch_new(
        &self,
        embeddings_vec: &[embeddings::EmbeddingRecord],
    ) -> anyhow::Result<()> {
        let embeddings_vec = embeddings_vec.to_vec();

        let id_embedding_pairs = self
            .run(move |conn| embeddings::upsert_embeddings_batch(conn, &embeddings_vec))
            .await?;

        // Sync all embeddings to vec_code
        if self.has_vec_extension() {
            for (embedding_id, embedding) in id_embedding_pairs {
                self.sync_embedding_to_vec(embedding_id, &embedding).await?;
            }
        }

        Ok(())
    }

    /// Check if embedding exists for blob_sha
    pub async fn has_embedding(&self, blob_sha: &str) -> anyhow::Result<bool> {
        let blob_sha = blob_sha.to_string();

        self.run(move |conn| embeddings::has_embedding(conn, &blob_sha))
            .await
    }

    /// Get embedding by blob_sha
    pub async fn get_embedding(&self, blob_sha: &str) -> anyhow::Result<Option<Vec<f32>>> {
        let blob_sha = blob_sha.to_string();

        self.run(move |conn| embeddings::get_embedding(conn, &blob_sha))
            .await
    }

    /// Sync embedding to vec_code table (skips if extension not available)
    ///
    /// This method syncs a single embedding from code_embeddings to the vec_code virtual table.
    /// The rowid in vec_code matches the embedding_id to enable joining search results.
    pub async fn sync_embedding_to_vec(
        &self,
        embedding_id: i64,
        embedding: &[f32],
    ) -> anyhow::Result<()> {
        if !self.has_vec_extension() {
            return Ok(()); // Skip silently if extension not available
        }

        let embedding = embedding.to_vec();
        self.run(move |conn| embeddings::sync_embedding_to_vec(conn, embedding_id, &embedding))
            .await
    }

    /// Sync all embeddings to vec_code table
    ///
    /// This method finds all embeddings in code_embeddings that don't have a corresponding
    /// entry in vec_code and syncs them. Returns the number of embeddings synced.
    pub async fn sync_all_embeddings_to_vec(&self) -> anyhow::Result<usize> {
        if !self.has_vec_extension() {
            return Ok(0); // Skip if extension not available
        }

        self.run(move |conn| embeddings::sync_all_embeddings_to_vec(conn))
            .await
    }

    /// Search for similar chunks by embedding (SQLite-specific)
    ///
    /// Returns empty Vec (not error) when extension is not available.
    pub async fn search_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query_embedding: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<vector::VectorResult>> {
        if !self.has_vec_extension() {
            return Ok(vec![]);
        }

        let repo = repo.to_string();
        let worktree = worktree.map(|s| s.to_string());
        let query_embedding = query_embedding.to_vec();

        self.run(move |conn| {
            vector::search_vector(
                conn,
                &repo,
                worktree.as_deref(),
                &query_embedding,
                limit,
                None,
                None,
            )
        })
        .await
    }

    /// Search for chunks using FTS5 full-text search (SQLite-specific)
    ///
    /// Returns FtsResult with chunk_id, rank, normalized_rank (0-1), and position.
    /// The normalized_rank is suitable for RRF fusion with vector search results.
    pub async fn search_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<fts::FtsResult>> {
        let repo = repo.to_string();
        let worktree = worktree.map(|s| s.to_string());
        let query = query.to_string();

        self.run(move |conn| {
            fts::search_fts(conn, &repo, worktree.as_deref(), &query, limit, None, None)
        })
        .await
    }

    /// Legacy graph importance calculation (pre-quality weights)
    ///
    /// Uses simple edge counting with hardcoded type weights.
    /// Preserved for backward compatibility.
    async fn calculate_graph_importance_legacy(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchHit>> {
        self.run(move |conn| {
            // SQLite doesn't have FILTER, so use CASE for conditional counts
            let sql = if worktree_id.is_some() {
                r#"
                WITH edge_counts AS (
                    SELECT
                        dst_chunk_id as chunk_id,
                        SUM(CASE WHEN type = 'calls' THEN 1 ELSE 0 END) as callers,
                        SUM(CASE WHEN type = 'imports' THEN 1 ELSE 0 END) as importers,
                        SUM(CASE WHEN type = 'test_of' THEN 1 ELSE 0 END) as tests
                    FROM chunk_edges
                    GROUP BY dst_chunk_id
                )
                SELECT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    COALESCE(
                        (ln(2 + COALESCE(e.callers, 0)) * 0.3 +
                         ln(2 + COALESCE(e.importers, 0)) * 0.2 +
                         ln(2 + COALESCE(e.tests, 0)) * 0.1),
                        0
                    ) as graph_score
                FROM chunks c
                JOIN files f ON f.id = c.file_id
                JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                LEFT JOIN edge_counts e ON e.chunk_id = c.id
                WHERE f.repo_id = ?1 AND cw.worktree_id = ?2
                ORDER BY graph_score DESC
                LIMIT ?3
                "#
            } else {
                r#"
                WITH edge_counts AS (
                    SELECT
                        dst_chunk_id as chunk_id,
                        SUM(CASE WHEN type = 'calls' THEN 1 ELSE 0 END) as callers,
                        SUM(CASE WHEN type = 'imports' THEN 1 ELSE 0 END) as importers,
                        SUM(CASE WHEN type = 'test_of' THEN 1 ELSE 0 END) as tests
                    FROM chunk_edges
                    GROUP BY dst_chunk_id
                )
                SELECT DISTINCT
                    c.id,
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    COALESCE(
                        (ln(2 + COALESCE(e.callers, 0)) * 0.3 +
                         ln(2 + COALESCE(e.importers, 0)) * 0.2 +
                         ln(2 + COALESCE(e.tests, 0)) * 0.1),
                        0
                    ) as graph_score
                FROM chunks c
                JOIN files f ON f.id = c.file_id
                LEFT JOIN edge_counts e ON e.chunk_id = c.id
                WHERE f.repo_id = ?1
                ORDER BY graph_score DESC
                LIMIT ?2
                "#
            };

            let mut stmt = conn.prepare(sql)?;
            let mut hits = Vec::new();

            if let Some(wid) = worktree_id {
                let rows = stmt.query_map(params![repo_id, wid, limit as i64], |row| {
                    Ok(SearchHit {
                        chunk_id: row.get(0)?,
                        start_line: row.get(1)?,
                        end_line: row.get(2)?,
                        symbol_name: row.get(3)?,
                        kind: row.get(4)?,
                        file_relpath: row.get(5)?,
                        score: row.get(6)?,
                        base_score: None,
                        kind_mult: None,
                        exact_mult: None,
                        preview: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            } else {
                let rows = stmt.query_map(params![repo_id, limit as i64], |row| {
                    Ok(SearchHit {
                        chunk_id: row.get(0)?,
                        start_line: row.get(1)?,
                        end_line: row.get(2)?,
                        symbol_name: row.get(3)?,
                        kind: row.get(4)?,
                        file_relpath: row.get(5)?,
                        score: row.get(6)?,
                        base_score: None,
                        kind_mult: None,
                        exact_mult: None,
                        preview: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            }
            Ok(hits)
        })
        .await
    }

    /// Quality-weighted graph importance calculation
    ///
    /// Applies quality weights based on:
    /// - Edge type (calls weight from config)
    /// - Source code type (production weight vs test weight from config)
    ///
    /// Test detection uses file path patterns and chunk kind.
    /// Uses LOG scaling on quality-weighted sum.
    ///
    /// # Parameters
    /// * `repo_id` - Repository ID to query
    /// * `worktree_id` - Optional worktree ID to filter by
    /// * `limit` - Maximum number of results
    /// * `weights` - Configurable edge quality weights (SRCHREL-2002)
    async fn calculate_graph_importance_quality(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        weights: &EdgeQualityWeights,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // Clone weights for move into closure
        let calls_weight = weights.calls as f64;
        let test_code_weight = weights.test_code as f64;
        let production_code_weight = weights.production_code as f64;

        self.run(move |conn| {
            // Quality-weighted SQL query with parameterized weights (SRCHREL-2002)
            // Parameters:
            // - ?1: repo_id
            // - ?2: worktree_id (when present)
            // - ?3: limit (position varies based on worktree)
            // - ?4: calls_weight
            // - ?5: test_code_weight
            // - ?6: production_code_weight
            let sql = if worktree_id.is_some() {
                r#"
                WITH quality_edges AS (
                  SELECT
                    ce.dst_chunk_id as chunk_id,
                    CASE ce.type
                      WHEN 'calls' THEN ?4
                      ELSE 1.0
                    END *
                    CASE
                      WHEN src_file.relpath LIKE '%/test/%'
                        OR src_file.relpath LIKE '%/tests/%'
                        OR src_file.relpath LIKE '%/__tests__/%'
                        OR src_file.relpath LIKE '%.test.ts%'
                        OR src_file.relpath LIKE '%.test.js%'
                        OR src_file.relpath LIKE '%.test.tsx%'
                        OR src_file.relpath LIKE '%.test.jsx%'
                        OR src_file.relpath LIKE '%.spec.ts%'
                        OR src_file.relpath LIKE '%.spec.js%'
                        OR src_file.relpath LIKE '%_test.rs%'
                        OR src_file.relpath LIKE '%_test.py%'
                        OR src_chunk.kind LIKE '%test%'
                      THEN ?5
                      ELSE ?6
                    END as edge_quality
                  FROM chunk_edges ce
                  JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
                  JOIN files src_file ON src_file.id = src_chunk.file_id
                  WHERE ce.dst_chunk_id IN (
                    SELECT c.id FROM chunks c
                    JOIN files f ON f.id = c.file_id
                    JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                    WHERE f.repo_id = ?1 AND cw.worktree_id = ?2
                  )
                ),
                importance_scores AS (
                  SELECT
                    chunk_id,
                    SUM(edge_quality) as quality_weighted_sum
                  FROM quality_edges
                  GROUP BY chunk_id
                )
                SELECT
                  c.id,
                  c.start_line,
                  c.end_line,
                  c.symbol_name,
                  c.kind,
                  f.relpath,
                  COALESCE(ln(2.0 + COALESCE(i.quality_weighted_sum, 0.0)), 0.0) as graph_score
                FROM chunks c
                JOIN files f ON f.id = c.file_id
                JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                LEFT JOIN importance_scores i ON i.chunk_id = c.id
                WHERE f.repo_id = ?1 AND cw.worktree_id = ?2
                ORDER BY graph_score DESC
                LIMIT ?3
                "#
            } else {
                r#"
                WITH quality_edges AS (
                  SELECT
                    ce.dst_chunk_id as chunk_id,
                    CASE ce.type
                      WHEN 'calls' THEN ?3
                      ELSE 1.0
                    END *
                    CASE
                      WHEN src_file.relpath LIKE '%/test/%'
                        OR src_file.relpath LIKE '%/tests/%'
                        OR src_file.relpath LIKE '%/__tests__/%'
                        OR src_file.relpath LIKE '%.test.ts%'
                        OR src_file.relpath LIKE '%.test.js%'
                        OR src_file.relpath LIKE '%.test.tsx%'
                        OR src_file.relpath LIKE '%.test.jsx%'
                        OR src_file.relpath LIKE '%.spec.ts%'
                        OR src_file.relpath LIKE '%.spec.js%'
                        OR src_file.relpath LIKE '%_test.rs%'
                        OR src_file.relpath LIKE '%_test.py%'
                        OR src_chunk.kind LIKE '%test%'
                      THEN ?4
                      ELSE ?5
                    END as edge_quality
                  FROM chunk_edges ce
                  JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
                  JOIN files src_file ON src_file.id = src_chunk.file_id
                  WHERE ce.dst_chunk_id IN (
                    SELECT c.id FROM chunks c
                    JOIN files f ON f.id = c.file_id
                    WHERE f.repo_id = ?1
                  )
                ),
                importance_scores AS (
                  SELECT
                    chunk_id,
                    SUM(edge_quality) as quality_weighted_sum
                  FROM quality_edges
                  GROUP BY chunk_id
                )
                SELECT DISTINCT
                  c.id,
                  c.start_line,
                  c.end_line,
                  c.symbol_name,
                  c.kind,
                  f.relpath,
                  COALESCE(ln(2.0 + COALESCE(i.quality_weighted_sum, 0.0)), 0.0) as graph_score
                FROM chunks c
                JOIN files f ON f.id = c.file_id
                LEFT JOIN importance_scores i ON i.chunk_id = c.id
                WHERE f.repo_id = ?1
                ORDER BY graph_score DESC
                LIMIT ?2
                "#
            };

            let mut stmt = conn.prepare(sql)?;
            let mut hits = Vec::new();

            if let Some(wid) = worktree_id {
                // With worktree: ?1=repo_id, ?2=worktree_id, ?3=limit, ?4=calls, ?5=test, ?6=prod
                let rows = stmt.query_map(
                    params![
                        repo_id,
                        wid,
                        limit as i64,
                        calls_weight,
                        test_code_weight,
                        production_code_weight
                    ],
                    |row| {
                        Ok(SearchHit {
                            chunk_id: row.get(0)?,
                            start_line: row.get(1)?,
                            end_line: row.get(2)?,
                            symbol_name: row.get(3)?,
                            kind: row.get(4)?,
                            file_relpath: row.get(5)?,
                            score: row.get(6)?,
                            base_score: None,
                            kind_mult: None,
                            exact_mult: None,
                            preview: None,
                        })
                    },
                )?;
                for row in rows {
                    hits.push(row?);
                }
            } else {
                // Without worktree: ?1=repo_id, ?2=limit, ?3=calls, ?4=test, ?5=prod
                let rows = stmt.query_map(
                    params![
                        repo_id,
                        limit as i64,
                        calls_weight,
                        test_code_weight,
                        production_code_weight
                    ],
                    |row| {
                        Ok(SearchHit {
                            chunk_id: row.get(0)?,
                            start_line: row.get(1)?,
                            end_line: row.get(2)?,
                            symbol_name: row.get(3)?,
                            kind: row.get(4)?,
                            file_relpath: row.get(5)?,
                            score: row.get(6)?,
                            base_score: None,
                            kind_mult: None,
                            exact_mult: None,
                            preview: None,
                        })
                    },
                )?;
                for row in rows {
                    hits.push(row?);
                }
            }

            tracing::debug!(
                hits_count = hits.len(),
                calls_weight = calls_weight,
                test_code_weight = test_code_weight,
                production_code_weight = production_code_weight,
                "Quality-weighted graph importance query completed"
            );

            Ok(hits)
        })
        .await
    }

    /// Count chunks where blob_sha is NOT in code_embeddings table
    ///
    /// This returns the count of chunks that need embeddings generated.
    /// In SQLite, embeddings are stored by blob_sha in the code_embeddings table,
    /// so a chunk needs embedding if its blob_sha doesn't exist in that table.
    pub async fn get_chunks_needing_embeddings_count(&self) -> anyhow::Result<i64> {
        self.run(move |conn| {
            let count: i64 = conn.query_row(
                r#"
                SELECT COUNT(DISTINCT c.blob_sha)
                FROM chunks c
                WHERE c.blob_sha NOT IN (SELECT blob_sha FROM code_embeddings)
                "#,
                [],
                |row| row.get(0),
            )?;
            Ok(count)
        })
        .await
    }

    /// Copy embeddings from code_embeddings cache table to chunks
    ///
    /// This is a no-op for SQLite since embeddings are already accessible
    /// via the blob_sha key. This method exists for API compatibility with
    /// the PostgreSQL implementation but doesn't need to do anything in SQLite.
    ///
    /// Returns the count of chunks that would have been updated (0).
    pub async fn copy_existing_embeddings_from_cache(&self) -> anyhow::Result<i64> {
        // In SQLite, embeddings are stored by blob_sha and accessed via joins.
        // There's no "copying" needed - chunks reference embeddings via blob_sha.
        // This method exists for API compatibility but is effectively a no-op.
        Ok(0)
    }

    /// Fetch chunks that need embeddings generated
    ///
    /// Returns chunks where the blob_sha is not in the code_embeddings table.
    ///
    /// # Arguments
    /// * `incremental` - If true, only return chunks without embeddings (always respected)
    /// * `sample_size` - Optional limit on number of chunks to return
    pub async fn fetch_chunks_needing_embeddings(
        &self,
        incremental: bool,
        sample_size: Option<usize>,
    ) -> anyhow::Result<Vec<crate::db::ChunkForEmbedding>> {
        self.run(move |conn| {
            // Build query based on parameters
            let mut query = String::from(
                r#"
                SELECT c.id, c.blob_sha, c.signature, c.docstring, c.preview
                FROM chunks c
                "#,
            );

            // In SQLite, we only fetch chunks where blob_sha is not in code_embeddings
            // The incremental parameter doesn't change this since we always check blob_sha
            if incremental {
                query.push_str("WHERE c.blob_sha NOT IN (SELECT blob_sha FROM code_embeddings)\n");
            }

            query.push_str("ORDER BY c.id\n");

            // Add LIMIT if sample_size is specified
            if let Some(limit) = sample_size {
                query.push_str(&format!("LIMIT {}", limit));
            }

            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map([], |row| {
                Ok(crate::db::ChunkForEmbedding {
                    id: row.get(0)?,
                    blob_sha: row.get(1)?,
                    signature: row.get(2)?,
                    docstring: row.get(3)?,
                    preview: row.get(4)?,
                })
            })?;

            let chunks: Result<Vec<_>, _> = rows.collect();
            Ok(chunks?)
        })
        .await
    }

    // ==================== Encoding Progress Methods ====================

    /// Create a new encoding run record. Returns the run ID.
    pub async fn create_encoding_run(
        &self,
        total_chunks: i64,
        provider: Option<&str>,
        dimension: Option<i32>,
    ) -> anyhow::Result<i64> {
        let provider = provider.map(|s| s.to_string());
        self.write_with_retry(move |conn| {
            conn.execute(
                "INSERT INTO encoding_runs (total_chunks, provider, dimension)
                 VALUES (?1, ?2, ?3)",
                params![total_chunks, provider, dimension],
            )?;
            let id: i64 = conn.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;
            Ok(id)
        })
        .await
    }

    /// Update the progress of an encoding run.
    pub async fn update_encoding_run_progress(
        &self,
        run_id: i64,
        chunks_completed: i64,
        chunks_per_second: Option<f64>,
    ) -> anyhow::Result<()> {
        self.write_with_retry(move |conn| {
            conn.execute(
                "UPDATE encoding_runs
                 SET chunks_completed = ?1,
                     chunks_per_second = ?2,
                     last_batch_at = datetime('now')
                 WHERE id = ?3",
                params![chunks_completed, chunks_per_second, run_id],
            )?;
            Ok(())
        })
        .await
    }

    /// Complete an encoding run, setting its final status and finished_at timestamp.
    pub async fn complete_encoding_run(&self, run_id: i64, status: &str) -> anyhow::Result<()> {
        let status = status.to_string();
        self.write_with_retry(move |conn| {
            conn.execute(
                "UPDATE encoding_runs
                 SET status = ?1,
                     finished_at = datetime('now')
                 WHERE id = ?2",
                params![status, run_id],
            )?;
            Ok(())
        })
        .await
    }

    /// Mark all encoding runs with status='running' as 'failed'.
    ///
    /// This is used on startup to clean up stale runs from previous crashes.
    pub async fn mark_stale_runs_as_failed(&self) -> anyhow::Result<()> {
        self.write_with_retry(move |conn| {
            conn.execute(
                "UPDATE encoding_runs SET status = ?1, finished_at = datetime('now') WHERE status = ?2",
                params!["failed", "running"],
            )?;
            Ok(())
        })
        .await
    }

    /// Get the currently active (running) encoding run, if any.
    ///
    /// Returns the most recently started running run.
    pub async fn get_active_encoding_run(&self) -> anyhow::Result<Option<EncodingRunRow>> {
        self.run(move |conn| {
            let result = conn
                .query_row(
                    "SELECT id, started_at, finished_at, status, total_chunks,
                            chunks_completed, chunks_per_second, last_batch_at,
                            provider, dimension
                     FROM encoding_runs
                     WHERE status = 'running'
                     ORDER BY id DESC
                     LIMIT 1",
                    [],
                    |row| {
                        Ok(EncodingRunRow {
                            id: row.get(0)?,
                            started_at: row.get(1)?,
                            finished_at: row.get(2)?,
                            status: row.get(3)?,
                            total_chunks: row.get(4)?,
                            chunks_completed: row.get(5)?,
                            chunks_per_second: row.get(6)?,
                            last_batch_at: row.get(7)?,
                            provider: row.get(8)?,
                            dimension: row.get(9)?,
                        })
                    },
                )
                .optional()?;
            Ok(result)
        })
        .await
    }
}

pub use crate::db::types::EncodingRunRow;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    // Counter for unique test database names
    static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    async fn setup_test_store() -> SqliteStore {
        // Use file:memdb?mode=memory&cache=shared for shared in-memory database
        // Each test gets a unique name to avoid interference
        let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("file:memdb_test_{}?mode=memory&cache=shared", counter);
        let store = SqliteStore::connect(&db_name)
            .await
            .expect("Failed to create test store");
        store.migrate().await.expect("Failed to run migrations");
        store
    }

    #[tokio::test]
    async fn test_junction_table_operations() {
        let store = setup_test_store().await;

        // Create a test repo
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();

        // Create two worktrees
        let worktree1_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let worktree2_id = store
            .get_or_create_worktree(repo_id, "feature", "/test/path/feature")
            .await
            .unwrap();

        // Create a commit
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create a file for worktree1
        let file = FileRecord {
            repo_id,
            worktree_id: worktree1_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash123".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create a chunk associated with worktree1
        let chunk = ChunkRecord {
            file_id,
            worktree_id: worktree1_id,
            blob_sha: "blob123".to_string(),
            symbol_name: Some("test_function".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn test_function() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        let chunk_id = store.insert_chunk(&chunk).await.unwrap();

        // Verify chunk is associated with worktree1
        let worktrees = store.get_chunk_worktrees(chunk_id).await.unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0], worktree1_id);

        // Add chunk to worktree2
        store
            .add_chunk_to_worktree(chunk_id, worktree2_id)
            .await
            .unwrap();

        // Verify chunk is now associated with both worktrees
        let worktrees = store.get_chunk_worktrees(chunk_id).await.unwrap();
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees.contains(&worktree1_id));
        assert!(worktrees.contains(&worktree2_id));

        // Try adding same worktree again (should be idempotent)
        store
            .add_chunk_to_worktree(chunk_id, worktree2_id)
            .await
            .unwrap();
        let worktrees = store.get_chunk_worktrees(chunk_id).await.unwrap();
        assert_eq!(worktrees.len(), 2); // Still only 2, not 3
    }

    #[tokio::test]
    async fn test_embedding_deduplication() {
        let store = setup_test_store().await;

        // Create a 1536-dimensional embedding
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();

        // Insert first embedding for blob_sha "hash1"
        let id1 = store
            .upsert_embedding("hash1", &embedding1, "model-v1")
            .await
            .unwrap();

        // Verify embedding exists
        assert!(store.has_embedding("hash1").await.unwrap());

        // Retrieve and verify
        let retrieved1 = store.get_embedding("hash1").await.unwrap();
        assert!(retrieved1.is_some());
        let retrieved1 = retrieved1.unwrap();
        assert_eq!(retrieved1.len(), 1536);
        // Check a few values
        assert!((retrieved1[0] - embedding1[0]).abs() < 1e-6);
        assert!((retrieved1[100] - embedding1[100]).abs() < 1e-6);

        // Update the same blob_sha with a different embedding
        let id2 = store
            .upsert_embedding("hash1", &embedding2, "model-v2")
            .await
            .unwrap();

        // Should return the same id (upsert)
        assert_eq!(id1, id2);

        // Retrieve updated embedding
        let retrieved2 = store.get_embedding("hash1").await.unwrap().unwrap();
        assert_eq!(retrieved2.len(), 1536);
        // Verify it's the new embedding
        assert!((retrieved2[0] - embedding2[0]).abs() < 1e-6);
        assert!((retrieved2[100] - embedding2[100]).abs() < 1e-6);

        // Insert a different blob_sha
        let id3 = store
            .upsert_embedding("hash2", &embedding1, "model-v1")
            .await
            .unwrap();

        // Should be a different id
        assert_ne!(id1, id3);

        // Both should exist
        assert!(store.has_embedding("hash1").await.unwrap());
        assert!(store.has_embedding("hash2").await.unwrap());
        assert!(!store.has_embedding("hash3").await.unwrap());

        // Test batch upsert
        let batch = vec![
            embeddings::EmbeddingRecord {
                blob_sha: "batch1".to_string(),
                embedding: embedding1.clone(),
                model_version: "model-v1".to_string(),
            },
            embeddings::EmbeddingRecord {
                blob_sha: "batch2".to_string(),
                embedding: embedding2.clone(),
                model_version: "model-v1".to_string(),
            },
        ];

        store.upsert_embeddings_batch_new(&batch).await.unwrap();

        // Verify batch inserts
        assert!(store.has_embedding("batch1").await.unwrap());
        assert!(store.has_embedding("batch2").await.unwrap());

        let batch1_emb = store.get_embedding("batch1").await.unwrap().unwrap();
        assert_eq!(batch1_emb.len(), 1536);
        assert!((batch1_emb[0] - embedding1[0]).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_batch_insert_with_junction() {
        let store = setup_test_store().await;

        // Create test data
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash123".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create multiple chunks
        let chunks = vec![
            ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: "blob1".to_string(),
                symbol_name: Some("fn1".to_string()),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
                start_line: 1,
                end_line: 5,
                preview: "fn fn1() {}".to_string(),
                ts_doc_text: String::new(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
            },
            ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: "blob2".to_string(),
                symbol_name: Some("fn2".to_string()),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
                start_line: 6,
                end_line: 10,
                preview: "fn fn2() {}".to_string(),
                ts_doc_text: String::new(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
            },
        ];

        let chunk_ids = store.insert_chunks_batch(&chunks).await.unwrap();
        assert_eq!(chunk_ids.len(), 2);

        // Verify both chunks are in junction table
        for chunk_id in chunk_ids {
            let worktrees = store.get_chunk_worktrees(chunk_id).await.unwrap();
            assert_eq!(worktrees.len(), 1);
            assert_eq!(worktrees[0], worktree_id);
        }
    }

    #[tokio::test]
    async fn test_vector_table_sync_integration() {
        let store = setup_test_store().await;

        // Create embeddings
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();

        // Upsert embeddings (should auto-sync to vec_code)
        let id1 = store
            .upsert_embedding("blob1", &embedding1, "model-v1")
            .await
            .unwrap();
        let id2 = store
            .upsert_embedding("blob2", &embedding2, "model-v1")
            .await
            .unwrap();

        assert_ne!(id1, id2);

        // Verify embeddings are in vec_code
        let count_synced = store
            .run(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM vec_code WHERE rowid IN (?1, ?2)",
                    params![id1, id2],
                    |row| row.get(0),
                )?;
                Ok(count)
            })
            .await
            .unwrap();

        assert_eq!(
            count_synced, 2,
            "Both embeddings should be synced to vec_code"
        );

        // Test update
        let embedding1_updated: Vec<f32> = (0..1536).map(|i| (i as f32 + 10.0) / 1536.0).collect();
        let id1_updated = store
            .upsert_embedding("blob1", &embedding1_updated, "model-v2")
            .await
            .unwrap();

        assert_eq!(id1, id1_updated, "ID should remain the same on update");

        // Verify still only 2 entries in vec_code
        let count_after_update = store
            .run(move |conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))?;
                Ok(count)
            })
            .await
            .unwrap();

        assert_eq!(
            count_after_update, 2,
            "Update should not create duplicate vec_code entries"
        );
    }

    #[tokio::test]
    async fn test_sync_all_embeddings_integration() {
        let store = setup_test_store().await;

        // Create embeddings directly without auto-sync
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();

        // Insert directly into code_embeddings without syncing
        let id1 = store
            .run(move |conn| embeddings::upsert_embedding(conn, "batch1", &embedding1, "model-v1"))
            .await
            .unwrap();

        let id2 = store
            .run(move |conn| embeddings::upsert_embedding(conn, "batch2", &embedding2, "model-v1"))
            .await
            .unwrap();

        // Verify vec_code is empty
        let count_before = store
            .run(|conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))?;
                Ok(count)
            })
            .await
            .unwrap();

        assert_eq!(count_before, 0, "vec_code should be empty before sync");

        // Sync all embeddings
        let synced_count = store.sync_all_embeddings_to_vec().await.unwrap();
        assert_eq!(synced_count, 2, "Should have synced 2 embeddings");

        // Verify vec_code now has both
        let count_after = store
            .run(|conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))?;
                Ok(count)
            })
            .await
            .unwrap();

        assert_eq!(
            count_after, 2,
            "vec_code should have 2 embeddings after sync"
        );

        // Verify rowid mapping
        let rowids_match = store
            .run(move |conn| {
                let match1: bool = conn
                    .query_row(
                        "SELECT 1 FROM vec_code WHERE rowid = ?1",
                        params![id1],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                let match2: bool = conn
                    .query_row(
                        "SELECT 1 FROM vec_code WHERE rowid = ?1",
                        params![id2],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                Ok(match1 && match2)
            })
            .await
            .unwrap();

        assert!(
            rowids_match,
            "Rowids in vec_code should match code_embeddings IDs"
        );

        // Sync again - should be idempotent
        let synced_again = store.sync_all_embeddings_to_vec().await.unwrap();
        assert_eq!(synced_again, 0, "Second sync should find nothing new");
    }

    #[tokio::test]
    async fn test_vector_search_integration() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree1_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let worktree2_id = store
            .get_or_create_worktree(repo_id, "feature", "/test/path/feature")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create files
        let file1 = FileRecord {
            repo_id,
            worktree_id: worktree1_id,
            commit_id,
            relpath: "test1.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file1_id = store.upsert_file(&file1).await.unwrap();

        let file2 = FileRecord {
            repo_id,
            worktree_id: worktree2_id,
            commit_id,
            relpath: "test2.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash2".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file2_id = store.upsert_file(&file2).await.unwrap();

        // Create chunks with different blob_sha values
        let chunk1 = ChunkRecord {
            file_id: file1_id,
            worktree_id: worktree1_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("fn1".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn fn1() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        let chunk1_id = store.insert_chunk(&chunk1).await.unwrap();

        let chunk2 = ChunkRecord {
            file_id: file2_id,
            worktree_id: worktree2_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("fn2".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn fn2() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        let chunk2_id = store.insert_chunk(&chunk2).await.unwrap();

        // Create embeddings (similar vectors)
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 0.1) / 1536.0).collect(); // Slightly different

        // Insert embeddings for both chunks
        store
            .upsert_embedding("blob1", &embedding1, "model-v1")
            .await
            .unwrap();
        store
            .upsert_embedding("blob2", &embedding2, "model-v1")
            .await
            .unwrap();

        // Query with a vector similar to embedding1
        let query_embedding: Vec<f32> = (0..1536).map(|i| (i as f32 + 0.05) / 1536.0).collect();

        // Search across all worktrees
        let results = store
            .search_vector("test-repo", None, &query_embedding, 10)
            .await
            .unwrap();

        assert!(!results.is_empty(), "Should find at least one result");
        assert!(results.len() <= 2, "Should find at most 2 results");

        // Verify results are sorted by similarity (best first = lowest distance)
        for i in 1..results.len() {
            assert!(
                results[i - 1].distance <= results[i].distance,
                "Results should be sorted by distance (ascending)"
            );
        }

        // Verify similarity scores are in range (0, 1]
        for result in &results {
            assert!(
                result.similarity > 0.0 && result.similarity <= 1.0,
                "Similarity should be in range (0, 1], got {}",
                result.similarity
            );
        }

        // Search with worktree filter (only worktree1)
        let results_wt1 = store
            .search_vector("test-repo", Some("main"), &query_embedding, 10)
            .await
            .unwrap();

        assert_eq!(
            results_wt1.len(),
            1,
            "Should find exactly 1 result in main worktree"
        );
        assert_eq!(
            results_wt1[0].chunk_id, chunk1_id,
            "Should find chunk1 in main worktree"
        );

        // Search with different worktree filter (only worktree2)
        let results_wt2 = store
            .search_vector("test-repo", Some("feature"), &query_embedding, 10)
            .await
            .unwrap();

        assert_eq!(
            results_wt2.len(),
            1,
            "Should find exactly 1 result in feature worktree"
        );
        assert_eq!(
            results_wt2[0].chunk_id, chunk2_id,
            "Should find chunk2 in feature worktree"
        );

        // Search with non-existent repo (should return empty)
        let results_no_repo = store
            .search_vector("non-existent", None, &query_embedding, 10)
            .await
            .unwrap();
        assert!(
            results_no_repo.is_empty(),
            "Should return empty for non-existent repo"
        );

        // Search with non-existent worktree (should return empty)
        let results_no_wt = store
            .search_vector("test-repo", Some("non-existent"), &query_embedding, 10)
            .await
            .unwrap();
        assert!(
            results_no_wt.is_empty(),
            "Should return empty for non-existent worktree"
        );
    }

    #[tokio::test]
    async fn test_vector_search_no_embeddings() {
        let store = setup_test_store().await;

        // Create test data but no embeddings
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob_no_embedding".to_string(),
            symbol_name: Some("fn1".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn fn1() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk).await.unwrap();

        // Search without any embeddings indexed
        let query_embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let results = store
            .search_vector("test-repo", None, &query_embedding, 10)
            .await
            .unwrap();

        assert!(
            results.is_empty(),
            "Should return empty when no embeddings indexed"
        );
    }

    #[tokio::test]
    async fn test_vector_search_dimension_validation() {
        let store = setup_test_store().await;

        // Create test repo
        store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();

        // Query with wrong dimension
        let query_embedding: Vec<f32> = vec![1.0, 2.0, 3.0]; // Only 3 dimensions instead of 1536

        let result = store
            .search_vector("test-repo", None, &query_embedding, 10)
            .await;

        assert!(
            result.is_err(),
            "Should return error for wrong embedding dimension"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Unsupported embedding dimension"),
            "Error should mention unsupported dimension"
        );
    }

    #[tokio::test]
    async fn test_vector_search_extension_not_available() {
        let store = setup_test_store().await;

        // Create test repo
        store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();

        // Manually disable vec extension availability (simulating missing extension)
        store.vec_available.store(false, Ordering::Relaxed);
        store.vec_checked.store(true, Ordering::Relaxed);

        // Query should return empty results, not error
        let query_embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let results = store
            .search_vector("test-repo", None, &query_embedding, 10)
            .await
            .unwrap();

        assert!(
            results.is_empty(),
            "Should return empty results when extension not available"
        );

        // has_vec_extension should return false
        assert!(
            !store.has_vec_extension(),
            "has_vec_extension should return false"
        );
    }

    #[tokio::test]
    async fn test_vector_search_similarity_ordering() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create 3 chunks with embeddings at different distances from query
        // Query will be [0.5, 0.5, 0.5, ...]
        // embed1 = [0.5, 0.5, 0.5, ...] -> distance = 0 (identical)
        // embed2 = [0.6, 0.6, 0.6, ...] -> distance = small
        // embed3 = [0.9, 0.9, 0.9, ...] -> distance = larger

        for (i, val) in [(1, 0.5f32), (2, 0.6f32), (3, 0.9f32)] {
            let chunk = ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: format!("blob{}", i),
                symbol_name: Some(format!("fn{}", i)),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
                start_line: i as i32,
                end_line: i as i32 + 10,
                preview: format!("fn fn{}() {{}}", i),
                ts_doc_text: String::new(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
            };
            store.insert_chunk(&chunk).await.unwrap();

            let embedding: Vec<f32> = vec![val; 1536];
            store
                .upsert_embedding(&format!("blob{}", i), &embedding, "model-v1")
                .await
                .unwrap();
        }

        // Query with [0.5, 0.5, 0.5, ...]
        let query_embedding: Vec<f32> = vec![0.5f32; 1536];
        let results = store
            .search_vector("test-repo", None, &query_embedding, 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 3, "Should find all 3 chunks");

        // Verify ordering: first result should be most similar (embed1, then embed2, then embed3)
        // Check that distances are in ascending order
        for i in 1..results.len() {
            assert!(
                results[i - 1].distance <= results[i].distance,
                "Results should be sorted by distance (ascending): {} <= {}",
                results[i - 1].distance,
                results[i].distance
            );
        }

        // First result should have similarity close to 1.0 (identical vector)
        assert!(
            results[0].similarity > 0.9,
            "First result should have high similarity, got {}",
            results[0].similarity
        );

        // Last result should have lower similarity
        assert!(
            results[2].similarity < results[0].similarity,
            "Last result should have lower similarity"
        );
    }

    #[tokio::test]
    async fn test_fts_search_integration() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks with searchable content
        let chunk1 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("process_authentication".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Handle user authentication and login".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn process_authentication(user: &User) -> AuthResult {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk1).await.unwrap();

        let chunk2 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("validate_token".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Validate JWT token for authentication".to_string()),
            start_line: 11,
            end_line: 20,
            preview: "fn validate_token(token: &str) -> bool {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk2).await.unwrap();

        // Search for "authentication"
        let results = store
            .search_fts("test-repo", None, "authentication", 10)
            .await
            .unwrap();

        assert!(
            !results.is_empty(),
            "Should find results for 'authentication'"
        );
        assert!(
            results.len() >= 1,
            "Should find at least 1 chunk with 'authentication'"
        );

        // Verify results have normalized rank in valid range
        for result in &results {
            assert!(
                result.normalized_rank > 0.0 && result.normalized_rank <= 1.0,
                "Normalized rank should be in (0, 1], got {}",
                result.normalized_rank
            );
        }

        // Verify position is 0-indexed
        for (i, result) in results.iter().enumerate() {
            assert_eq!(
                result.position, i,
                "Position should be 0-indexed, expected {}, got {}",
                i, result.position
            );
        }

        // Search for "token" - should find validate_token
        let results_token = store
            .search_fts("test-repo", None, "token", 10)
            .await
            .unwrap();
        assert!(!results_token.is_empty(), "Should find results for 'token'");

        // Search with empty query should return empty
        let results_empty = store.search_fts("test-repo", None, "", 10).await.unwrap();
        assert!(
            results_empty.is_empty(),
            "Empty query should return empty results"
        );

        // Search for non-existent term should return empty
        let results_none = store
            .search_fts("test-repo", None, "xyznonexistent", 10)
            .await
            .unwrap();
        assert!(
            results_none.is_empty(),
            "Non-existent term should return empty results"
        );
    }

    #[tokio::test]
    async fn test_fts_search_worktree_filter() {
        let store = setup_test_store().await;

        // Create test repo with two worktrees
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree1_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let worktree2_id = store
            .get_or_create_worktree(repo_id, "feature", "/test/path/feature")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create files in each worktree
        let file1 = FileRecord {
            repo_id,
            worktree_id: worktree1_id,
            commit_id,
            relpath: "test1.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file1_id = store.upsert_file(&file1).await.unwrap();

        let file2 = FileRecord {
            repo_id,
            worktree_id: worktree2_id,
            commit_id,
            relpath: "test2.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash2".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file2_id = store.upsert_file(&file2).await.unwrap();

        // Create chunk in worktree1
        let chunk1 = ChunkRecord {
            file_id: file1_id,
            worktree_id: worktree1_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("main_handler".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Main worktree handler function".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn main_handler() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk1).await.unwrap();

        // Create chunk in worktree2
        let chunk2 = ChunkRecord {
            file_id: file2_id,
            worktree_id: worktree2_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("feature_handler".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Feature worktree handler function".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn feature_handler() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk2).await.unwrap();

        // Search across all worktrees for "handler"
        let results_all = store
            .search_fts("test-repo", None, "handler", 10)
            .await
            .unwrap();
        assert_eq!(
            results_all.len(),
            2,
            "Should find 2 handlers across all worktrees"
        );

        // Search only in main worktree
        let results_main = store
            .search_fts("test-repo", Some("main"), "handler", 10)
            .await
            .unwrap();
        assert_eq!(
            results_main.len(),
            1,
            "Should find 1 handler in main worktree"
        );

        // Search only in feature worktree
        let results_feature = store
            .search_fts("test-repo", Some("feature"), "handler", 10)
            .await
            .unwrap();
        assert_eq!(
            results_feature.len(),
            1,
            "Should find 1 handler in feature worktree"
        );
    }

    #[tokio::test]
    async fn test_hybrid_search_integration() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks with both FTS content and embeddings
        // Chunk 1: Good for both FTS ("authentication") and vector (similar to query)
        let chunk1 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("process_authentication".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Handle user authentication".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn process_authentication() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk1).await.unwrap();

        // Chunk 2: Good for FTS only ("authentication" in content)
        let chunk2 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("validate_auth".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Validate authentication token".to_string()),
            start_line: 11,
            end_line: 20,
            preview: "fn validate_auth() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk2).await.unwrap();

        // Chunk 3: Good for vector only (semantically similar embedding)
        let chunk3 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob3".to_string(),
            symbol_name: Some("login_handler".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Handle login requests".to_string()),
            start_line: 21,
            end_line: 30,
            preview: "fn login_handler() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk3).await.unwrap();

        // Create embeddings (chunk1 and chunk3 similar to query, chunk2 different)
        let query_embedding: Vec<f32> = vec![0.5f32; 1536];
        let embedding1: Vec<f32> = vec![0.5f32; 1536]; // Similar to query
        let embedding2: Vec<f32> = vec![0.9f32; 1536]; // Different from query
        let embedding3: Vec<f32> = vec![0.51f32; 1536]; // Similar to query

        store
            .upsert_embedding("blob1", &embedding1, "model-v1")
            .await
            .unwrap();
        store
            .upsert_embedding("blob2", &embedding2, "model-v1")
            .await
            .unwrap();
        store
            .upsert_embedding("blob3", &embedding3, "model-v1")
            .await
            .unwrap();

        // Perform hybrid search for "authentication"
        let weights = hybrid::HybridWeights::default();
        let results = store
            .search_hybrid(
                "test-repo",
                None,
                "authentication",
                &query_embedding,
                10,
                weights,
            )
            .await
            .unwrap();

        // Should find results from both sources
        assert!(!results.is_empty(), "Hybrid search should return results");

        // Chunk 1 should be ranked highly (appears in both FTS and vector)
        let chunk1_result = results.iter().find(|r| r.source == "both");
        assert!(
            chunk1_result.is_some(),
            "Should have at least one result from both sources"
        );

        // Results should be sorted by score (descending)
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be sorted by score descending"
            );
        }

        // All scores should be positive
        for result in &results {
            assert!(result.score > 0.0, "All scores should be positive");
        }
    }

    #[tokio::test]
    async fn test_hybrid_search_fallback_to_fts() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file and chunk with FTS content but NO embedding
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob_no_embedding".to_string(),
            symbol_name: Some("test_function".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Test function for search".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn test_function() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk).await.unwrap();

        // Perform hybrid search - should fall back to FTS since no embeddings
        let query_embedding: Vec<f32> = vec![0.5f32; 1536];
        let weights = hybrid::HybridWeights::default();
        let results = store
            .search_hybrid("test-repo", None, "test", &query_embedding, 10, weights)
            .await
            .unwrap();

        // Should find FTS results even without vector results
        assert!(
            !results.is_empty(),
            "Should find FTS results when no embeddings"
        );
        assert!(
            results.iter().all(|r| r.source == "fts"),
            "All results should be FTS-only"
        );
    }

    #[tokio::test]
    async fn test_get_chunks_metadata() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks with different metadata
        let chunk1 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("my_function".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn my_function() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 0.9,
            churn_score: 0.5,
            metadata: None,
        };
        let chunk1_id = store.insert_chunk(&chunk1).await.unwrap();

        let chunk2 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("my_variable".to_string()),
            kind: "variable".to_string(),
            signature: None,
            docstring: None,
            start_line: 11,
            end_line: 15,
            preview: "let my_variable = 42;".to_string(),
            ts_doc_text: String::new(),
            recency_score: 0.5,
            churn_score: 0.3,
            metadata: None,
        };
        let chunk2_id = store.insert_chunk(&chunk2).await.unwrap();

        // Get metadata for both chunks
        let metadata = store
            .get_chunks_metadata(&[chunk1_id, chunk2_id])
            .await
            .unwrap();

        assert_eq!(metadata.len(), 2, "Should get metadata for both chunks");

        let meta1 = metadata
            .get(&chunk1_id)
            .expect("Should have chunk1 metadata");
        assert_eq!(meta1.kind, "function");
        assert_eq!(meta1.symbol_name, Some("my_function".to_string()));
        assert!((meta1.recency_score - 0.9).abs() < 1e-6);

        let meta2 = metadata
            .get(&chunk2_id)
            .expect("Should have chunk2 metadata");
        assert_eq!(meta2.kind, "variable");
        assert_eq!(meta2.symbol_name, Some("my_variable".to_string()));
        assert!((meta2.recency_score - 0.5).abs() < 1e-6);

        // Test empty input
        let empty_metadata = store.get_chunks_metadata(&[]).await.unwrap();
        assert!(
            empty_metadata.is_empty(),
            "Empty input should return empty map"
        );

        // Test non-existent chunk ID
        let missing_metadata = store.get_chunks_metadata(&[99999]).await.unwrap();
        assert!(
            missing_metadata.is_empty(),
            "Non-existent ID should return empty map"
        );
    }

    #[tokio::test]
    async fn test_search_hybrid_ranked_integration() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks with different kinds and symbol names
        // Chunk 1: function with matching symbol name
        let chunk1 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("validate_user".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Validate user credentials".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn validate_user() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0, // Most recent
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk1).await.unwrap();

        // Chunk 2: variable (lower rank multiplier)
        let chunk2 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("validation_flag".to_string()),
            kind: "variable".to_string(),
            signature: None,
            docstring: Some("User validation status flag".to_string()),
            start_line: 11,
            end_line: 15,
            preview: "let validation_flag = true;".to_string(),
            ts_doc_text: String::new(),
            recency_score: 0.0, // Oldest
            churn_score: 0.3,
            metadata: None,
        };
        store.insert_chunk(&chunk2).await.unwrap();

        // Chunk 3: function but no symbol match
        let chunk3 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob3".to_string(),
            symbol_name: Some("process_data".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("User data processor".to_string()),
            start_line: 16,
            end_line: 25,
            preview: "fn process_data() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 0.5,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk3).await.unwrap();

        // Create embeddings
        let query_embedding: Vec<f32> = vec![0.5f32; 1536];
        let embedding1: Vec<f32> = vec![0.5f32; 1536];
        let embedding2: Vec<f32> = vec![0.5f32; 1536];
        let embedding3: Vec<f32> = vec![0.5f32; 1536];

        store
            .upsert_embedding("blob1", &embedding1, "model-v1")
            .await
            .unwrap();
        store
            .upsert_embedding("blob2", &embedding2, "model-v1")
            .await
            .unwrap();
        store
            .upsert_embedding("blob3", &embedding3, "model-v1")
            .await
            .unwrap();

        // Perform ranked hybrid search for "validate"
        let weights = hybrid::HybridWeights::default();
        let ranking = hybrid::SemanticRanking::default();
        let results = store
            .search_hybrid_ranked(
                "test-repo",
                None,
                "validate",
                &query_embedding,
                10,
                weights,
                ranking,
            )
            .await
            .unwrap();

        // Should find results
        assert!(!results.is_empty(), "Should find results");

        // Results should include chunk metadata
        for result in &results {
            assert!(!result.kind.is_empty(), "Kind should be populated");
        }

        // validate_user should rank highest:
        // - function multiplier (1.2)
        // - exact match boost (1.5) since "validate" matches "validate_user"
        // - recency boost (1.0 * 0.1 + 1.0 = 1.1)
        // This combination should beat other chunks
        if results.len() >= 2 {
            let top_result = &results[0];
            assert_eq!(
                top_result.symbol_name,
                Some("validate_user".to_string()),
                "validate_user should rank first due to function multiplier + exact match + recency"
            );
        }

        // Results should be sorted by score descending
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[tokio::test]
    async fn test_search_hybrid_ranked_identity_ranking() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create a simple chunk
        let chunk = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("test_fn".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: Some("Test function".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn test_fn() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk).await.unwrap();

        // Create embedding
        let query_embedding: Vec<f32> = vec![0.5f32; 1536];
        let embedding: Vec<f32> = vec![0.5f32; 1536];
        store
            .upsert_embedding("blob1", &embedding, "model-v1")
            .await
            .unwrap();

        // Compare results with identity ranking vs default ranking
        let weights = hybrid::HybridWeights::default();

        let results_identity = store
            .search_hybrid_ranked(
                "test-repo",
                None,
                "test",
                &query_embedding,
                10,
                weights.clone(),
                hybrid::SemanticRanking::identity(),
            )
            .await
            .unwrap();

        let results_default = store
            .search_hybrid_ranked(
                "test-repo",
                None,
                "test",
                &query_embedding,
                10,
                weights,
                hybrid::SemanticRanking::default(),
            )
            .await
            .unwrap();

        // Both should return results
        assert!(
            !results_identity.is_empty(),
            "Identity ranking should find results"
        );
        assert!(
            !results_default.is_empty(),
            "Default ranking should find results"
        );

        // Default ranking should boost the score (function=1.2, exact match=1.5, recency=1.1)
        // Identity ranking should keep original score
        assert!(
            results_default[0].score > results_identity[0].score,
            "Default ranking should boost score compared to identity"
        );
    }

    // ========================================================================
    // Graph Traversal Integration Tests
    // ========================================================================

    /// Helper to create a simple chunk for graph testing
    async fn create_test_chunk(
        store: &SqliteStore,
        file_id: i64,
        worktree_id: i64,
        name: &str,
        line_start: i32,
    ) -> i64 {
        let chunk = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: format!("blob_{}", name),
            symbol_name: Some(name.to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: line_start,
            end_line: line_start + 10,
            preview: format!("fn {}() {{}}", name),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk).await.unwrap()
    }

    #[tokio::test]
    async fn test_graph_find_callers_direct() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks A -> B (A calls B)
        let a = create_test_chunk(&store, file_id, worktree_id, "func_a", 1).await;
        let b = create_test_chunk(&store, file_id, worktree_id, "func_b", 20).await;

        // A calls B
        store.insert_chunk_edge(a, b, "calls").await.unwrap();

        // Find callers of B
        let callers = store.find_callers(b, Some(1)).await.unwrap();

        assert_eq!(callers.len(), 1, "Should find 1 direct caller");
        assert_eq!(callers[0].chunk_id, a);
        assert_eq!(callers[0].depth, 1);
    }

    #[tokio::test]
    async fn test_graph_find_callees_direct() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks A -> B (A calls B)
        let a = create_test_chunk(&store, file_id, worktree_id, "func_a", 1).await;
        let b = create_test_chunk(&store, file_id, worktree_id, "func_b", 20).await;

        // A calls B
        store.insert_chunk_edge(a, b, "calls").await.unwrap();

        // Find callees of A
        let callees = store.find_callees(a, Some(1)).await.unwrap();

        assert_eq!(callees.len(), 1, "Should find 1 direct callee");
        assert_eq!(callees[0].chunk_id, b);
        assert_eq!(callees[0].depth, 1);
    }

    #[tokio::test]
    async fn test_graph_transitive_callers() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create linear chain: A -> B -> C
        let a = create_test_chunk(&store, file_id, worktree_id, "func_a", 1).await;
        let b = create_test_chunk(&store, file_id, worktree_id, "func_b", 20).await;
        let c = create_test_chunk(&store, file_id, worktree_id, "func_c", 40).await;

        store.insert_chunk_edge(a, b, "calls").await.unwrap();
        store.insert_chunk_edge(b, c, "calls").await.unwrap();

        // Find all callers of C (should be B at depth 1, A at depth 2)
        let callers = store.find_callers(c, Some(3)).await.unwrap();

        assert_eq!(callers.len(), 2, "Should find 2 callers (transitive)");
        assert!(
            callers.iter().any(|r| r.chunk_id == b && r.depth == 1),
            "B should be at depth 1"
        );
        assert!(
            callers.iter().any(|r| r.chunk_id == a && r.depth == 2),
            "A should be at depth 2"
        );
    }

    #[tokio::test]
    async fn test_graph_cycle_handling() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create cycle: A -> B -> C -> A
        let a = create_test_chunk(&store, file_id, worktree_id, "func_a", 1).await;
        let b = create_test_chunk(&store, file_id, worktree_id, "func_b", 20).await;
        let c = create_test_chunk(&store, file_id, worktree_id, "func_c", 40).await;

        store.insert_chunk_edge(a, b, "calls").await.unwrap();
        store.insert_chunk_edge(b, c, "calls").await.unwrap();
        store.insert_chunk_edge(c, a, "calls").await.unwrap(); // Cycle!

        // Should not hang and should not have duplicates
        let callers = store.find_callers(a, Some(10)).await.unwrap();

        // Each chunk should appear at most once
        let unique_chunks: std::collections::HashSet<i64> =
            callers.iter().map(|r| r.chunk_id).collect();
        assert_eq!(
            unique_chunks.len(),
            callers.len(),
            "Should have no duplicate chunks"
        );
    }

    #[tokio::test]
    async fn test_graph_depth_limiting() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chain: A -> B -> C
        let a = create_test_chunk(&store, file_id, worktree_id, "func_a", 1).await;
        let b = create_test_chunk(&store, file_id, worktree_id, "func_b", 20).await;
        let c = create_test_chunk(&store, file_id, worktree_id, "func_c", 40).await;

        store.insert_chunk_edge(a, b, "calls").await.unwrap();
        store.insert_chunk_edge(b, c, "calls").await.unwrap();

        // With depth 1, should only find B (direct caller of C)
        let callers = store.find_callers(c, Some(1)).await.unwrap();

        assert_eq!(callers.len(), 1, "Should find only 1 caller at depth 1");
        assert_eq!(callers[0].chunk_id, b);
        assert!(
            !callers.iter().any(|r| r.chunk_id == a),
            "A should not be found at depth 1"
        );
    }

    #[tokio::test]
    async fn test_graph_empty_results() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create isolated chunk with no edges
        let chunk = create_test_chunk(&store, file_id, worktree_id, "isolated", 1).await;

        let callers = store.find_callers(chunk, None).await.unwrap();
        let callees = store.find_callees(chunk, None).await.unwrap();

        assert!(callers.is_empty(), "Isolated chunk should have no callers");
        assert!(callees.is_empty(), "Isolated chunk should have no callees");
    }

    #[tokio::test]
    async fn test_graph_imports() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks: A imports B
        let a = create_test_chunk(&store, file_id, worktree_id, "module_a", 1).await;
        let b = create_test_chunk(&store, file_id, worktree_id, "module_b", 20).await;

        store.insert_chunk_edge(a, b, "imports").await.unwrap();

        // Find what A imports (outgoing)
        let imports_out = store
            .find_imports(a, graph::ImportDirection::Outgoing, Some(1))
            .await
            .unwrap();
        assert_eq!(imports_out.len(), 1, "A should import 1 module");
        assert_eq!(imports_out[0].chunk_id, b);

        // Find what imports B (incoming)
        let imports_in = store
            .find_imports(b, graph::ImportDirection::Incoming, Some(1))
            .await
            .unwrap();
        assert_eq!(imports_in.len(), 1, "B should be imported by 1 module");
        assert_eq!(imports_in[0].chunk_id, a);
    }

    #[tokio::test]
    async fn test_graph_direct_edges() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks with multiple edge types: A calls B, A imports C
        let a = create_test_chunk(&store, file_id, worktree_id, "func_a", 1).await;
        let b = create_test_chunk(&store, file_id, worktree_id, "func_b", 20).await;
        let c = create_test_chunk(&store, file_id, worktree_id, "module_c", 40).await;

        store.insert_chunk_edge(a, b, "calls").await.unwrap();
        store.insert_chunk_edge(a, c, "imports").await.unwrap();

        // Get all outgoing edges from A
        let edges = store
            .get_direct_edges(a, graph::ImportDirection::Outgoing)
            .await
            .unwrap();

        assert_eq!(edges.len(), 2, "A should have 2 outgoing edges");
        assert!(edges
            .iter()
            .any(|e| e.chunk_id == b && e.edge_type == "calls"));
        assert!(edges
            .iter()
            .any(|e| e.chunk_id == c && e.edge_type == "imports"));
    }

    #[tokio::test]
    async fn test_graph_large_chain() {
        let store = setup_test_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chain of 20 nodes
        let mut chunks = Vec::new();
        for i in 0..20 {
            let chunk = create_test_chunk(
                &store,
                file_id,
                worktree_id,
                &format!("func_{}", i),
                (i * 15) as i32,
            )
            .await;
            chunks.push(chunk);
        }
        for i in 0..19 {
            store
                .insert_chunk_edge(chunks[i], chunks[i + 1], "calls")
                .await
                .unwrap();
        }

        let start = std::time::Instant::now();
        let callers = store.find_callers(chunks[19], Some(10)).await.unwrap();
        let elapsed = start.elapsed();

        // Should complete quickly
        assert!(
            elapsed.as_millis() < 1000,
            "Graph traversal took {:?}",
            elapsed
        );

        // Should find up to 10 callers (limited by depth)
        assert_eq!(
            callers.len(),
            10,
            "Should find 10 callers (limited by depth)"
        );

        // Verify results are ordered by depth
        for i in 1..callers.len() {
            assert!(
                callers[i - 1].depth <= callers[i].depth,
                "Results should be ordered by depth"
            );
        }
    }

    #[tokio::test]
    async fn test_delete_chunks_by_ids() {
        let store = setup_test_store().await;

        // Setup: Create test data
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash123".to_string(),
            size_bytes: 1000,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create multiple chunks
        let chunk1 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("func1".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "test chunk 1".to_string(),
            ts_doc_text: "test function 1".to_string(),
            recency_score: 1.0,
            churn_score: 0.2,
            metadata: None,
        };
        let chunk1_id = store.insert_chunk(&chunk1).await.unwrap();

        let chunk2 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("func2".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 11,
            end_line: 20,
            preview: "test chunk 2".to_string(),
            ts_doc_text: "test function 2".to_string(),
            recency_score: 1.0,
            churn_score: 0.2,
            metadata: None,
        };
        let chunk2_id = store.insert_chunk(&chunk2).await.unwrap();

        let chunk3 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob3".to_string(),
            symbol_name: Some("func3".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 21,
            end_line: 30,
            preview: "test chunk 3".to_string(),
            ts_doc_text: "test function 3".to_string(),
            recency_score: 1.0,
            churn_score: 0.2,
            metadata: None,
        };
        let chunk3_id = store.insert_chunk(&chunk3).await.unwrap();

        // Create edges between chunks
        store
            .insert_chunk_edge(chunk1_id, chunk2_id, "calls")
            .await
            .unwrap();
        store
            .insert_chunk_edge(chunk2_id, chunk3_id, "calls")
            .await
            .unwrap();

        // Create embeddings for chunks
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();
        store
            .upsert_embedding("blob1", &embedding1, "model-v1")
            .await
            .unwrap();
        store
            .upsert_embedding("blob2", &embedding2, "model-v1")
            .await
            .unwrap();

        // Verify all chunks exist
        let chunk_exists = |id: i64| {
            store.run(move |conn| {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM chunks WHERE id = ?1)",
                        params![id],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                Ok(exists)
            })
        };

        assert!(chunk_exists(chunk1_id).await.unwrap());
        assert!(chunk_exists(chunk2_id).await.unwrap());
        assert!(chunk_exists(chunk3_id).await.unwrap());

        // Verify edges exist
        let edges_count = store
            .run(|conn| {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM chunk_edges", [], |row| row.get(0))
                    .unwrap();
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(edges_count, 2);

        // Delete chunks 1 and 2
        let deleted_count = store
            .delete_chunks_by_ids(worktree_id, &[chunk1_id, chunk2_id])
            .await
            .unwrap();
        assert_eq!(deleted_count, 2, "Should have deleted 2 chunks");

        // Verify chunks 1 and 2 are deleted, chunk 3 remains
        assert!(!chunk_exists(chunk1_id).await.unwrap());
        assert!(!chunk_exists(chunk2_id).await.unwrap());
        assert!(chunk_exists(chunk3_id).await.unwrap());

        // Verify edges involving deleted chunks are removed
        let edges_after = store
            .run(|conn| {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM chunk_edges", [], |row| row.get(0))
                    .unwrap();
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(edges_after, 0, "All edges should be deleted");

        // Verify embeddings for deleted chunks are removed
        let embeddings_count = store
            .run(|conn| {
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM code_embeddings WHERE blob_sha IN ('blob1', 'blob2')",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap();
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(
            embeddings_count, 0,
            "Embeddings for deleted chunks should be removed"
        );

        // Verify junction table entries are removed
        let junction_count = store
            .run(move |conn| {
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM chunk_worktrees WHERE chunk_id IN (?1, ?2)",
                        params![chunk1_id, chunk2_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(
            junction_count, 0,
            "Junction table entries should be removed"
        );
    }

    #[tokio::test]
    async fn test_delete_chunks_by_ids_empty() {
        let store = setup_test_store().await;

        // Setup: Create test worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();

        // Delete with empty list should return 0
        let deleted_count = store.delete_chunks_by_ids(worktree_id, &[]).await.unwrap();
        assert_eq!(deleted_count, 0, "Deleting empty list should return 0");
    }

    #[tokio::test]
    async fn test_delete_chunks_by_ids_batching() {
        let store = setup_test_store().await;

        // Setup: Create test data
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create 1200 chunks to test batching (batch size is 500)
        // This ensures we test multiple batches
        let mut chunk_ids = Vec::new();
        for i in 0..1200 {
            // Create file using proper API
            let file = FileRecord {
                repo_id,
                worktree_id,
                commit_id,
                relpath: format!("file{}.rs", i),
                language: Some("rust".to_string()),
                content_hash: format!("hash{}", i),
                size_bytes: 100,
                last_modified: None,
            };
            let file_id = store.upsert_file(&file).await.unwrap();

            // Create chunk using proper API
            let chunk = ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: format!("blob{}", i),
                symbol_name: Some(format!("function{}", i)),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
                start_line: 1,
                end_line: 10,
                preview: "fn test() {}".to_string(),
                ts_doc_text: String::new(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
            };
            let chunk_id = store.insert_chunk(&chunk).await.unwrap();
            chunk_ids.push(chunk_id);
        }

        // Verify all chunks were created
        let initial_count: i64 = store
            .run(move |conn| {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))
                    .unwrap();
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(initial_count, 1200, "Should have created 1200 chunks");

        // Delete all chunks using batching
        let deleted_count = store
            .delete_chunks_by_ids(worktree_id, &chunk_ids)
            .await
            .unwrap();

        // Verify deletion count
        assert_eq!(deleted_count, 1200, "Should have deleted all 1200 chunks");

        // Verify chunks are actually deleted
        let remaining_count: i64 = store
            .run(move |conn| {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))
                    .unwrap();
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(remaining_count, 0, "All chunks should be deleted");
    }

    #[tokio::test]
    async fn test_write_retry_with_eventual_success() {
        use std::sync::{Arc, Mutex};

        let store = setup_test_store().await;
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = store
            .write_with_retry(move |_conn| {
                let mut count = attempt_count_clone.lock().unwrap();
                *count += 1;

                // Fail first 2 attempts, succeed on 3rd
                if *count < 3 {
                    Err(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(5), // SQLITE_BUSY
                        Some("simulated busy".into()),
                    )
                    .into())
                } else {
                    Ok(42)
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_write_retry_fails_after_max_attempts() {
        use std::sync::{Arc, Mutex};

        let store = setup_test_store().await;
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result: anyhow::Result<()> = store
            .write_with_retry(move |_conn| {
                let mut count = attempt_count_clone.lock().unwrap();
                *count += 1;

                // Always fail with SQLITE_BUSY
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(5), // SQLITE_BUSY
                    Some("always busy".into()),
                )
                .into())
            })
            .await;

        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 5); // Max attempts (default config)
    }

    #[tokio::test]
    async fn test_exponential_backoff_timing() {
        use std::sync::{Arc, Mutex};

        let store = setup_test_store().await;
        let start = std::time::Instant::now();
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let _: anyhow::Result<()> = store
            .write_with_retry(move |_conn| {
                let mut count = attempt_count_clone.lock().unwrap();
                *count += 1;

                // Always fail to test full backoff sequence
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(5), // SQLITE_BUSY
                    None,
                )
                .into())
            })
            .await;

        let elapsed = start.elapsed();
        // With default config: 50 + 100 + 200 + 400 = 750ms minimum
        // (5 attempts means 4 delays between them)
        assert!(
            elapsed >= std::time::Duration::from_millis(750),
            "Expected at least 750ms, got {:?}",
            elapsed
        );
        // Allow some overhead but should be less than 2 seconds
        assert!(
            elapsed < std::time::Duration::from_millis(2000),
            "Expected less than 2000ms, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_is_busy_error_detection() {
        let busy = anyhow::Error::from(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(5), // SQLITE_BUSY
            None,
        ));
        assert!(is_busy_error(&busy));

        let locked = anyhow::Error::from(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(6), // SQLITE_LOCKED
            None,
        ));
        assert!(is_busy_error(&locked));

        let other = anyhow::Error::from(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(19), // SQLITE_CONSTRAINT
            None,
        ));
        assert!(!is_busy_error(&other));

        // Test non-rusqlite error
        let generic_error = anyhow::anyhow!("some other error");
        assert!(!is_busy_error(&generic_error));
    }

    #[tokio::test]
    async fn test_write_retry_non_busy_error_fails_immediately() {
        use std::sync::{Arc, Mutex};

        let store = setup_test_store().await;
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result: anyhow::Result<()> = store
            .write_with_retry(move |_conn| {
                let mut count = attempt_count_clone.lock().unwrap();
                *count += 1;

                // Fail with a non-BUSY error
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ErrorCode::ConstraintViolation as i32),
                    Some("constraint violation".into()),
                )
                .into())
            })
            .await;

        assert!(result.is_err());
        // Should fail immediately without retries
        assert_eq!(*attempt_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_get_worktree_embedding_count() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Test with no chunks - should return 0
        let count = store
            .get_worktree_embedding_count(worktree_id)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // Create file and chunks
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create chunks without embeddings
        let chunk1 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("fn1".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn fn1() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        let _chunk1_id = store.insert_chunk(&chunk1).await.unwrap();

        // Still no embeddings - should return 0
        let count = store
            .get_worktree_embedding_count(worktree_id)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // Add an embedding
        store
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
                     VALUES (?1, ?2, ?3, ?4)",
                    params!["blob1", vec![0u8; 4096], 1024, "test-model"],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        // Now should return 1
        let count = store
            .get_worktree_embedding_count(worktree_id)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Add another chunk with embedding
        let chunk2 = ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("fn2".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 11,
            end_line: 20,
            preview: "fn fn2() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        let _chunk2_id = store.insert_chunk(&chunk2).await.unwrap();

        store
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
                     VALUES (?1, ?2, ?3, ?4)",
                    params!["blob2", vec![0u8; 4096], 1024, "test-model"],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        // Should return 2
        let count = store
            .get_worktree_embedding_count(worktree_id)
            .await
            .unwrap();
        assert_eq!(count, 2);

        // Test with non-existent worktree ID - should return 0
        let count = store.get_worktree_embedding_count(99999).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_get_worktree_language_breakdown() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Test with no files - should return empty vec
        let breakdown = store
            .get_worktree_language_breakdown(worktree_id)
            .await
            .unwrap();
        assert_eq!(breakdown.len(), 0);

        // Add files with various languages
        let files = vec![
            ("file1.rs", Some("rust")),
            ("file2.rs", Some("rust")),
            ("file3.rs", Some("rust")),
            ("file1.ts", Some("typescript")),
            ("file2.ts", Some("typescript")),
            ("file1.py", Some("python")),
            ("unknown.txt", None), // NULL language
        ];

        for (idx, (relpath, language)) in files.iter().enumerate() {
            let file = FileRecord {
                repo_id,
                worktree_id,
                commit_id,
                relpath: relpath.to_string(),
                language: language.map(|s| s.to_string()),
                content_hash: format!("hash{}", idx),
                size_bytes: 100,
                last_modified: None,
            };
            store.upsert_file(&file).await.unwrap();
        }

        // Get breakdown
        let breakdown = store
            .get_worktree_language_breakdown(worktree_id)
            .await
            .unwrap();

        // Should have 4 entries (rust, typescript, python, unknown)
        assert_eq!(breakdown.len(), 4);

        // Check order (should be sorted by file_count DESC)
        assert_eq!(breakdown[0].0, "rust");
        assert_eq!(breakdown[0].1, 3);
        assert_eq!(breakdown[1].0, "typescript");
        assert_eq!(breakdown[1].1, 2);
        assert_eq!(breakdown[2].0, "python");
        assert_eq!(breakdown[2].1, 1);
        assert_eq!(breakdown[3].0, "unknown");
        assert_eq!(breakdown[3].1, 1);

        // Test with non-existent worktree ID - should return empty vec
        let breakdown = store.get_worktree_language_breakdown(99999).await.unwrap();
        assert_eq!(breakdown.len(), 0);
    }

    #[tokio::test]
    async fn test_get_worktree_last_scan() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();

        // Test with no index_state - should return None
        let last_scan = store.get_worktree_last_scan(worktree_id).await.unwrap();
        assert_eq!(last_scan, None);

        // Insert index_state record
        let test_timestamp = "2024-01-15 10:30:45";
        store
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO index_state (worktree_id, tree_sha, chunks_processed, embeddings_generated, last_indexed)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![worktree_id, "tree123", 100, 50, test_timestamp],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        // Should return the timestamp
        let last_scan = store.get_worktree_last_scan(worktree_id).await.unwrap();
        assert_eq!(last_scan, Some(test_timestamp.to_string()));

        // Test with non-existent worktree ID - should return None
        let last_scan = store.get_worktree_last_scan(99999).await.unwrap();
        assert_eq!(last_scan, None);
    }
}
