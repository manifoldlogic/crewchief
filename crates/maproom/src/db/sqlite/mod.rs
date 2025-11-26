pub mod schema;
pub mod migrations;
pub mod embeddings;
pub mod vector;

use anyhow::Context;
use async_trait::async_trait;
use rusqlite::{Connection, params, OptionalExtension};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::spawn_blocking;

use crate::db::{ChunkRecord, FileRecord, SearchHit, VectorStore};
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
}

impl SqliteStore {
    pub async fn connect(path: &str) -> anyhow::Result<Self> {
        let path = if path.starts_with("sqlite://") {
            &path[9..]
        } else {
            path
        };

        // Register extension globally for all new connections
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        let manager = r2d2_sqlite::SqliteConnectionManager::file(path)
            .with_init(|conn| {
                // Enable WAL mode for concurrency
                conn.execute_batch(
                    r#"
                    PRAGMA journal_mode = WAL;
                    PRAGMA synchronous = NORMAL;
                    PRAGMA foreign_keys = ON;
                    PRAGMA busy_timeout = 5000;
                    "#,
                )?;
                Ok(())
            });

        let pool = r2d2::Pool::builder()
            .max_size(10) // Configurable?
            .build(manager)
            .context("Failed to create SQLite connection pool")?;

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

        Ok(Self {
            pool,
            vec_available: Arc::new(AtomicBool::new(false)),
            vec_checked: Arc::new(AtomicBool::new(false)),
        })
    }

    // Helper to run a blocking closure with a connection
    async fn run<F, T>(&self, f: F) -> anyhow::Result<T>
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

    /// Check if sqlite-vec extension is available
    pub fn has_vec_extension(&self) -> bool {
        self.vec_available.load(Ordering::Relaxed)
    }
}

/// Verify that sqlite-vec extension is loaded correctly
fn verify_vec_extension(conn: &Connection) -> bool {
    conn.query_row("SELECT vec_version()", [], |row| row.get::<_, String>(0))
        .is_ok()
}

#[async_trait]
impl VectorStore for SqliteStore {
    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64> {
        let name = name.to_string();
        let root_path = root_path.to_string();
        self.run(move |conn| {
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
        }).await
    }

    async fn get_or_create_worktree(
        &self,
        repo_id: i64,
        name: &str,
        abs_path: &str,
    ) -> anyhow::Result<i64> {
        let name = name.to_string();
        let abs_path = abs_path.to_string();
        self.run(move |conn| {
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
        }).await
    }

    async fn get_or_create_commit(
        &self,
        repo_id: i64,
        sha: &str,
        committed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64> {
        let sha = sha.to_string();
        self.run(move |conn| {
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

    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64> {
        let file = file.clone();
        self.run(move |conn| {
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

    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64> {
        let chunk = chunk.clone();
        self.run(move |conn| {
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
        self.run(move |conn| {
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

                for chunk in chunks {
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
        self.run(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES (?1, ?2, ?3)",
                params![src_chunk_id, dst_chunk_id, edge_type],
            )?;
            Ok(())
        }).await
    }

    // NOTE: This method is deprecated. Use SqliteStore::upsert_embedding() instead for content-based deduplication.
    // Cannot use #[deprecated] attribute on trait method implementations.
    async fn upsert_embeddings(
        &self,
        chunk_id: i64,
        code_embedding: Option<&[f32]>,
        text_embedding: Option<&[f32]>,
        _dimension: usize,
    ) -> anyhow::Result<()> {
        let code = code_embedding.map(|s| s.to_vec());
        let text = text_embedding.map(|s| s.to_vec());

        // Clone Arc pointers for the closure
        let vec_available = self.vec_available.clone();
        let vec_checked = self.vec_checked.clone();

        self.run(move |conn| {
            // Check extension availability (cached after first check)
            if !vec_checked.load(Ordering::Relaxed) {
                let available = verify_vec_extension(conn);
                vec_available.store(available, Ordering::Relaxed);
                vec_checked.store(true, Ordering::Relaxed);
                if !available {
                    tracing::warn!("sqlite-vec extension not loaded - vector search disabled");
                }
            }

            // Skip vec_chunks operations if extension not available
            if !vec_available.load(Ordering::Relaxed) {
                tracing::debug!("Skipping vec_chunks upsert - extension not available");
                return Ok(());
            }

            // Upsert into vec_chunks
            // Helper to convert Vec<f32> to Vec<u8>
            let to_blob = |v: &Vec<f32>| -> Vec<u8> {
                let mut blob = Vec::with_capacity(v.len() * 4);
                for val in v {
                    blob.extend_from_slice(&val.to_le_bytes());
                }
                blob
            };

            let code_blob = code.as_ref().map(to_blob);
            let text_blob = text.as_ref().map(to_blob);

            // Check if exists
            let exists: bool = conn.query_row(
                "SELECT 1 FROM vec_chunks WHERE chunk_id = ?1",
                params![chunk_id],
                |_| Ok(true),
            ).unwrap_or(false);

            if exists {
                // Update
                if let Some(blob) = &code_blob {
                    conn.execute("UPDATE vec_chunks SET code_embedding = ?1 WHERE chunk_id = ?2", params![blob, chunk_id])?;
                }
                if let Some(blob) = &text_blob {
                    conn.execute("UPDATE vec_chunks SET text_embedding = ?1 WHERE chunk_id = ?2", params![blob, chunk_id])?;
                }
            } else {
                // Insert
                if code_blob.is_some() || text_blob.is_some() {
                     conn.execute(
                        "INSERT INTO vec_chunks(chunk_id, code_embedding, text_embedding) VALUES (?1, ?2, ?3)",
                        params![chunk_id, code_blob, text_blob],
                    )?;
                }
            }
            Ok(())
        }).await
    }

    async fn batch_upsert_embeddings(
        &self,
        embeddings: &[(i64, Option<Vec<f32>>, Option<Vec<f32>>)],
        _dimension: usize,
    ) -> anyhow::Result<()> {
        let embeddings = embeddings.to_vec();

        // Clone Arc pointers for the closure
        let vec_available = self.vec_available.clone();
        let vec_checked = self.vec_checked.clone();

        self.run(move |conn| {
            // Check extension availability (cached after first check)
            if !vec_checked.load(Ordering::Relaxed) {
                let available = verify_vec_extension(conn);
                vec_available.store(available, Ordering::Relaxed);
                vec_checked.store(true, Ordering::Relaxed);
                if !available {
                    tracing::warn!("sqlite-vec extension not loaded - vector search disabled");
                }
            }

            // Skip vec_chunks operations if extension not available
            if !vec_available.load(Ordering::Relaxed) {
                tracing::debug!("Skipping batch vec_chunks upsert - extension not available");
                return Ok(());
            }

            let tx = conn.transaction()?;

            {
                let mut update_code = tx.prepare("UPDATE vec_chunks SET code_embedding = ?1 WHERE chunk_id = ?2")?;
                let mut update_text = tx.prepare("UPDATE vec_chunks SET text_embedding = ?1 WHERE chunk_id = ?2")?;
                let mut insert = tx.prepare("INSERT INTO vec_chunks(chunk_id, code_embedding, text_embedding) VALUES (?1, ?2, ?3)")?;
                let mut check = tx.prepare("SELECT 1 FROM vec_chunks WHERE chunk_id = ?1")?;

                for (chunk_id, code, text) in embeddings {
                    // Helper to convert Vec<f32> to Vec<u8>
                    let to_blob = |v: &Vec<f32>| -> Vec<u8> {
                        let mut blob = Vec::with_capacity(v.len() * 4);
                        for val in v {
                            blob.extend_from_slice(&val.to_le_bytes());
                        }
                        blob
                    };

                    let code_blob = code.as_ref().map(to_blob);
                    let text_blob = text.as_ref().map(to_blob);

                    let exists = check.exists(params![chunk_id])?;

                    if exists {
                        if let Some(blob) = &code_blob {
                            update_code.execute(params![blob, chunk_id])?;
                        }
                        if let Some(blob) = &text_blob {
                            update_text.execute(params![blob, chunk_id])?;
                        }
                    } else if code_blob.is_some() || text_blob.is_some() {
                        insert.execute(params![chunk_id, code_blob, text_blob])?;
                    }
                }
            }

            tx.commit()?;
            Ok(())
        }).await
    }

    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        _debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let repo = repo.to_string();
        let worktree = worktree.map(|s| s.to_string());
        let query = query.to_string();
        self.run(move |conn| {
            // Resolve repo/worktree ids
            let repo_id: i64 = conn.query_row(
                "SELECT id FROM repos WHERE name = ?1",
                params![repo],
                |row| row.get(0),
            )?;

            let worktree_id: Option<i64> = if let Some(w) = worktree {
                conn.query_row(
                    "SELECT id FROM worktrees WHERE repo_id = ?1 AND name = ?2",
                    params![repo_id, w],
                    |row| row.get(0),
                ).optional()?
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
                    // Sanitize: remove quotes and special FTS characters
                    let clean = t
                        .replace('"', "")
                        .replace('\'', "")
                        .replace('*', "")
                        .replace('(', "")
                        .replace(')', "");
                    if clean.is_empty() {
                        return String::new();
                    }
                    // FTS5 prefix syntax: term* (no quotes!)
                    format!("{}*", clean)
                })
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" OR ");  // Use OR for broader matching

            // SQL query with ranking
            // SQLite FTS5 rank is built-in function 'bm25' or 'rank'
            // We join with chunks and files
            
            let sql = if worktree_id.is_some() {
                r#"
                SELECT
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
                    let score: f64 = row.get(5)?;
                    Ok(SearchHit {
                        start_line: row.get(0)?,
                        end_line: row.get(1)?,
                        symbol_name: row.get(2)?,
                        kind: row.get(3)?,
                        file_relpath: row.get(4)?,
                        score: -score, // FTS5 rank is negative, negate for positive score
                        base_score: None,
                        kind_mult: None,
                        exact_mult: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            } else {
                let rows = stmt.query_map(params![fts_query, repo_id, k], |row| {
                    let score: f64 = row.get(5)?;
                    Ok(SearchHit {
                        start_line: row.get(0)?,
                        end_line: row.get(1)?,
                        symbol_name: row.get(2)?,
                        kind: row.get(3)?,
                        file_relpath: row.get(4)?,
                        score: -score, // FTS5 rank is negative, negate for positive score
                        base_score: None,
                        kind_mult: None,
                        exact_mult: None,
                    })
                })?;
                for row in rows {
                    hits.push(row?);
                }
            }
            Ok(hits)
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
                    conn.query_row(sql, params![repo_id, wid, path, symbol_name], |row| row.get(0)).optional()?
                } else {
                    conn.query_row(sql, params![repo_id, path, symbol_name], |row| row.get(0)).optional()?
                }
            } else {
                if let Some(wid) = worktree_id {
                    conn.query_row(sql, params![repo_id, wid, symbol_name], |row| row.get(0)).optional()?
                } else {
                    conn.query_row(sql, params![repo_id, symbol_name], |row| row.get(0)).optional()?
                }
            };

            Ok(id)
        }).await
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        self.run(move |conn| {
            let mut runner = MigrationRunner::new(conn);
            runner.migrate()
        }).await?;

        // Check extension availability after migration
        self.run(|conn| {
            let available = verify_vec_extension(conn);
            Ok(available)
        }).await.map(|available| {
            self.vec_available.store(available, Ordering::Relaxed);
            self.vec_checked.store(true, Ordering::Relaxed);
            if !available {
                tracing::warn!("sqlite-vec extension not loaded - vector search disabled");
            }
        })?;

        Ok(())
    }

    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>> {
        self.run(move |conn| {
            let exists: bool = conn.query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
                [],
                |_| Ok(true)
            ).unwrap_or(false);

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
        }).await
    }
}

// Additional SQLite-specific methods not in VectorStore trait
impl SqliteStore {
    /// Add chunk to additional worktree
    pub async fn add_chunk_to_worktree(&self, chunk_id: i64, worktree_id: i64) -> anyhow::Result<()> {
        self.run(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO chunk_worktrees (chunk_id, worktree_id) VALUES (?1, ?2)",
                params![chunk_id, worktree_id],
            )?;
            Ok(())
        }).await
    }

    /// Get all worktrees containing this chunk
    pub async fn get_chunk_worktrees(&self, chunk_id: i64) -> anyhow::Result<Vec<i64>> {
        self.run(move |conn| {
            let mut stmt = conn.prepare("SELECT worktree_id FROM chunk_worktrees WHERE chunk_id = ?1")?;
            let rows = stmt.query_map(params![chunk_id], |row| row.get(0))?;
            let mut ids = Vec::new();
            for id in rows {
                ids.push(id?);
            }
            Ok(ids)
        }).await
    }

    /// Store or update embedding by content hash (SQLite-specific, not in VectorStore trait)
    pub async fn upsert_embedding(
        &self,
        blob_sha: &str,
        embedding: &[f32],
        model_version: &str,
    ) -> anyhow::Result<i64> {
        let blob_sha = blob_sha.to_string();
        let embedding_vec = embedding.to_vec();
        let model_version = model_version.to_string();

        let embedding_id = self.run(move |conn| {
            embeddings::upsert_embedding(conn, &blob_sha, &embedding_vec, &model_version)
        }).await?;

        // Sync to vec_code table
        self.sync_embedding_to_vec(embedding_id, embedding).await?;

        Ok(embedding_id)
    }

    /// Batch upsert embeddings with deduplication (SQLite-specific, not in VectorStore trait)
    pub async fn upsert_embeddings_batch_new(
        &self,
        embeddings_vec: &[embeddings::EmbeddingRecord],
    ) -> anyhow::Result<()> {
        let embeddings_vec = embeddings_vec.to_vec();

        let id_embedding_pairs = self.run(move |conn| {
            embeddings::upsert_embeddings_batch(conn, &embeddings_vec)
        }).await?;

        // Sync all embeddings to vec_code
        if self.has_vec_extension() {
            for (embedding_id, embedding) in id_embedding_pairs {
                self.sync_embedding_to_vec(embedding_id, &embedding).await?;
            }
        }

        Ok(())
    }

    /// Check if embedding exists for blob_sha (SQLite-specific, not in VectorStore trait)
    pub async fn has_embedding(&self, blob_sha: &str) -> anyhow::Result<bool> {
        let blob_sha = blob_sha.to_string();

        self.run(move |conn| {
            embeddings::has_embedding(conn, &blob_sha)
        }).await
    }

    /// Get embedding by blob_sha (SQLite-specific, not in VectorStore trait)
    pub async fn get_embedding(&self, blob_sha: &str) -> anyhow::Result<Option<Vec<f32>>> {
        let blob_sha = blob_sha.to_string();

        self.run(move |conn| {
            embeddings::get_embedding(conn, &blob_sha)
        }).await
    }

    /// Sync embedding to vec_code table (skips if extension not available)
    ///
    /// This method syncs a single embedding from code_embeddings to the vec_code virtual table.
    /// The rowid in vec_code matches the embedding_id to enable joining search results.
    pub async fn sync_embedding_to_vec(&self, embedding_id: i64, embedding: &[f32]) -> anyhow::Result<()> {
        if !self.has_vec_extension() {
            return Ok(());  // Skip silently if extension not available
        }

        let embedding = embedding.to_vec();
        self.run(move |conn| {
            embeddings::sync_embedding_to_vec(conn, embedding_id, &embedding)
        }).await
    }

    /// Sync all embeddings to vec_code table
    ///
    /// This method finds all embeddings in code_embeddings that don't have a corresponding
    /// entry in vec_code and syncs them. Returns the number of embeddings synced.
    pub async fn sync_all_embeddings_to_vec(&self) -> anyhow::Result<usize> {
        if !self.has_vec_extension() {
            return Ok(0);  // Skip if extension not available
        }

        self.run(move |conn| {
            embeddings::sync_all_embeddings_to_vec(conn)
        }).await
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
            vector::search_vector(conn, &repo, worktree.as_deref(), &query_embedding, limit)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::VectorStore;

    async fn setup_test_store() -> SqliteStore {
        let store = SqliteStore::connect(":memory:").await.expect("Failed to create test store");
        store.migrate().await.expect("Failed to run migrations");
        store
    }

    #[tokio::test]
    async fn test_junction_table_operations() {
        let store = setup_test_store().await;

        // Create a test repo
        let repo_id = store.get_or_create_repo("test-repo", "/test/path").await.unwrap();

        // Create two worktrees
        let worktree1_id = store.get_or_create_worktree(repo_id, "main", "/test/path").await.unwrap();
        let worktree2_id = store.get_or_create_worktree(repo_id, "feature", "/test/path/feature").await.unwrap();

        // Create a commit
        let commit_id = store.get_or_create_commit(repo_id, "abc123", None).await.unwrap();

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
        store.add_chunk_to_worktree(chunk_id, worktree2_id).await.unwrap();

        // Verify chunk is now associated with both worktrees
        let worktrees = store.get_chunk_worktrees(chunk_id).await.unwrap();
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees.contains(&worktree1_id));
        assert!(worktrees.contains(&worktree2_id));

        // Try adding same worktree again (should be idempotent)
        store.add_chunk_to_worktree(chunk_id, worktree2_id).await.unwrap();
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
        let repo_id = store.get_or_create_repo("test-repo", "/test/path").await.unwrap();
        let worktree_id = store.get_or_create_worktree(repo_id, "main", "/test/path").await.unwrap();
        let commit_id = store.get_or_create_commit(repo_id, "abc123", None).await.unwrap();

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
        let id1 = store.upsert_embedding("blob1", &embedding1, "model-v1").await.unwrap();
        let id2 = store.upsert_embedding("blob2", &embedding2, "model-v1").await.unwrap();

        assert_ne!(id1, id2);

        // Verify embeddings are in vec_code
        let count_synced = store.run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM vec_code WHERE rowid IN (?1, ?2)",
                params![id1, id2],
                |row| row.get(0),
            )?;
            Ok(count)
        }).await.unwrap();

        assert_eq!(count_synced, 2, "Both embeddings should be synced to vec_code");

        // Test update
        let embedding1_updated: Vec<f32> = (0..1536).map(|i| (i as f32 + 10.0) / 1536.0).collect();
        let id1_updated = store.upsert_embedding("blob1", &embedding1_updated, "model-v2").await.unwrap();

        assert_eq!(id1, id1_updated, "ID should remain the same on update");

        // Verify still only 2 entries in vec_code
        let count_after_update = store.run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM vec_code",
                [],
                |row| row.get(0),
            )?;
            Ok(count)
        }).await.unwrap();

        assert_eq!(count_after_update, 2, "Update should not create duplicate vec_code entries");
    }

    #[tokio::test]
    async fn test_sync_all_embeddings_integration() {
        let store = setup_test_store().await;

        // Create embeddings directly without auto-sync
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();

        // Insert directly into code_embeddings without syncing
        let id1 = store.run(move |conn| {
            embeddings::upsert_embedding(conn, "batch1", &embedding1, "model-v1")
        }).await.unwrap();

        let id2 = store.run(move |conn| {
            embeddings::upsert_embedding(conn, "batch2", &embedding2, "model-v1")
        }).await.unwrap();

        // Verify vec_code is empty
        let count_before = store.run(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))?;
            Ok(count)
        }).await.unwrap();

        assert_eq!(count_before, 0, "vec_code should be empty before sync");

        // Sync all embeddings
        let synced_count = store.sync_all_embeddings_to_vec().await.unwrap();
        assert_eq!(synced_count, 2, "Should have synced 2 embeddings");

        // Verify vec_code now has both
        let count_after = store.run(|conn| {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))?;
            Ok(count)
        }).await.unwrap();

        assert_eq!(count_after, 2, "vec_code should have 2 embeddings after sync");

        // Verify rowid mapping
        let rowids_match = store.run(move |conn| {
            let match1: bool = conn.query_row(
                "SELECT 1 FROM vec_code WHERE rowid = ?1",
                params![id1],
                |_| Ok(true),
            ).unwrap_or(false);

            let match2: bool = conn.query_row(
                "SELECT 1 FROM vec_code WHERE rowid = ?1",
                params![id2],
                |_| Ok(true),
            ).unwrap_or(false);

            Ok(match1 && match2)
        }).await.unwrap();

        assert!(rowids_match, "Rowids in vec_code should match code_embeddings IDs");

        // Sync again - should be idempotent
        let synced_again = store.sync_all_embeddings_to_vec().await.unwrap();
        assert_eq!(synced_again, 0, "Second sync should find nothing new");
    }

    #[tokio::test]
    async fn test_vector_search_integration() {
        let store = setup_test_store().await;

        // Create test repo and worktree
        let repo_id = store.get_or_create_repo("test-repo", "/test/path").await.unwrap();
        let worktree1_id = store.get_or_create_worktree(repo_id, "main", "/test/path").await.unwrap();
        let worktree2_id = store.get_or_create_worktree(repo_id, "feature", "/test/path/feature").await.unwrap();
        let commit_id = store.get_or_create_commit(repo_id, "abc123", None).await.unwrap();

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
        store.upsert_embedding("blob1", &embedding1, "model-v1").await.unwrap();
        store.upsert_embedding("blob2", &embedding2, "model-v1").await.unwrap();

        // Query with a vector similar to embedding1
        let query_embedding: Vec<f32> = (0..1536).map(|i| (i as f32 + 0.05) / 1536.0).collect();

        // Search across all worktrees
        let results = store.search_vector("test-repo", None, &query_embedding, 10).await.unwrap();

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
            assert!(result.similarity > 0.0 && result.similarity <= 1.0,
                "Similarity should be in range (0, 1], got {}", result.similarity);
        }

        // Search with worktree filter (only worktree1)
        let results_wt1 = store.search_vector("test-repo", Some("main"), &query_embedding, 10).await.unwrap();

        assert_eq!(results_wt1.len(), 1, "Should find exactly 1 result in main worktree");
        assert_eq!(results_wt1[0].chunk_id, chunk1_id, "Should find chunk1 in main worktree");

        // Search with different worktree filter (only worktree2)
        let results_wt2 = store.search_vector("test-repo", Some("feature"), &query_embedding, 10).await.unwrap();

        assert_eq!(results_wt2.len(), 1, "Should find exactly 1 result in feature worktree");
        assert_eq!(results_wt2[0].chunk_id, chunk2_id, "Should find chunk2 in feature worktree");

        // Search with non-existent repo (should return empty)
        let results_no_repo = store.search_vector("non-existent", None, &query_embedding, 10).await.unwrap();
        assert!(results_no_repo.is_empty(), "Should return empty for non-existent repo");

        // Search with non-existent worktree (should return empty)
        let results_no_wt = store.search_vector("test-repo", Some("non-existent"), &query_embedding, 10).await.unwrap();
        assert!(results_no_wt.is_empty(), "Should return empty for non-existent worktree");
    }

    #[tokio::test]
    async fn test_vector_search_no_embeddings() {
        let store = setup_test_store().await;

        // Create test data but no embeddings
        let repo_id = store.get_or_create_repo("test-repo", "/test/path").await.unwrap();
        let worktree_id = store.get_or_create_worktree(repo_id, "main", "/test/path").await.unwrap();
        let commit_id = store.get_or_create_commit(repo_id, "abc123", None).await.unwrap();

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
        let results = store.search_vector("test-repo", None, &query_embedding, 10).await.unwrap();

        assert!(results.is_empty(), "Should return empty when no embeddings indexed");
    }

    #[tokio::test]
    async fn test_vector_search_dimension_validation() {
        let store = setup_test_store().await;

        // Create test repo
        store.get_or_create_repo("test-repo", "/test/path").await.unwrap();

        // Query with wrong dimension
        let query_embedding: Vec<f32> = vec![1.0, 2.0, 3.0]; // Only 3 dimensions instead of 1536

        let result = store.search_vector("test-repo", None, &query_embedding, 10).await;

        assert!(result.is_err(), "Should return error for wrong embedding dimension");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("dimension mismatch"), "Error should mention dimension mismatch");
    }
}
