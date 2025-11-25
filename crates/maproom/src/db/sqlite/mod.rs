pub mod schema;

use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params, OptionalExtension};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::task::spawn_blocking;

use crate::db::{ChunkRecord, FileRecord, SearchHit, VectorStore};
use schema::init_schema;

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

        Ok(Self { pool })
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
            // For JSON fields, we need to serialize to string if rusqlite doesn't support JSON directly
            let metadata_json = chunk.metadata.as_ref().map(|v| v.to_string());
            let worktree_ids_json = serde_json::json!([chunk.worktree_id]).to_string();

            // SQLite UPSERT
            conn.execute(
                "INSERT INTO chunks (
                   file_id, blob_sha, symbol_name, kind, signature, docstring, start_line, end_line, preview, recency_score, churn_score, metadata, worktree_ids
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
                   blob_sha = excluded.blob_sha,
                   symbol_name = excluded.symbol_name,
                   kind = excluded.kind,
                   signature = excluded.signature,
                   docstring = excluded.docstring,
                   preview = excluded.preview,
                   metadata = excluded.metadata,
                   worktree_ids = json_insert(chunks.worktree_ids, '$[#]', ?14) -- Append worktree_id if not exists logic needed
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
                    chunk.recency_score,
                    chunk.churn_score,
                    metadata_json,
                    worktree_ids_json,
                    chunk.worktree_id // For update logic
                ],
            )?;
            
            // Correct update logic for worktree_ids: 
            // If ID not in array, append it.
            // But first get the ID
            let id: i64 = conn.query_row(
                "SELECT id FROM chunks WHERE file_id = ?1 AND start_line = ?2 AND end_line = ?3",
                params![chunk.file_id, chunk.start_line, chunk.end_line],
                |row| row.get(0),
            )?;
            
            // Update FTS index manually
            conn.execute(
                "INSERT OR REPLACE INTO fts_chunks(rowid, content, docstring, symbol_name) VALUES (?1, ?2, ?3, ?4)",
                params![id, chunk.preview, chunk.docstring, chunk.symbol_name],
            )?;

            Ok(id)
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
                       file_id, blob_sha, symbol_name, kind, signature, docstring, start_line, end_line, preview, recency_score, churn_score, metadata, worktree_ids
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                     ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
                       blob_sha = excluded.blob_sha,
                       symbol_name = excluded.symbol_name,
                       kind = excluded.kind,
                       signature = excluded.signature,
                       docstring = excluded.docstring,
                       preview = excluded.preview,
                       metadata = excluded.metadata
                       -- worktree_ids update logic simplified for batch
                     RETURNING id"
                )?;

                let mut fts_stmt = tx.prepare(
                    "INSERT OR REPLACE INTO fts_chunks(rowid, content, docstring, symbol_name) VALUES (?1, ?2, ?3, ?4)"
                )?;

                for chunk in chunks {
                    let metadata_json = chunk.metadata.as_ref().map(|v| v.to_string());
                    let worktree_ids_json = serde_json::json!([chunk.worktree_id]).to_string();

                    let id: i64 = stmt.query_row(params![
                        chunk.file_id,
                        chunk.blob_sha,
                        chunk.symbol_name,
                        chunk.kind,
                        chunk.signature,
                        chunk.docstring,
                        chunk.start_line,
                        chunk.end_line,
                        chunk.preview,
                        chunk.recency_score,
                        chunk.churn_score,
                        metadata_json,
                        worktree_ids_json
                    ], |row| row.get(0))?;
                    
                    fts_stmt.execute(params![id, chunk.preview, chunk.docstring, chunk.symbol_name])?;
                    ids.push(id);
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

    async fn upsert_embeddings(
        &self,
        chunk_id: i64,
        code_embedding: Option<&[f32]>,
        text_embedding: Option<&[f32]>,
        dimension: usize,
    ) -> anyhow::Result<()> {
        let code = code_embedding.map(|s| s.to_vec());
        let text = text_embedding.map(|s| s.to_vec());
        self.run(move |conn| {
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
        dimension: usize,
    ) -> anyhow::Result<()> {
        let embeddings = embeddings.to_vec();
        self.run(move |conn| {
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
        debug: bool,
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

            // FTS5 query syntax: "term1 term2" matches (term1 AND term2) by default
            // We need prefix matching for terms: "term1* term2*"
            let fts_query = query
                .split_whitespace()
                .map(|t| format!("\"{}\"*", t.replace("\"", "")))
                .collect::<Vec<_>>()
                .join(" ");

            // SQL query with ranking
            // SQLite FTS5 rank is built-in function 'bm25' or 'rank'
            // We join with chunks and files
            
            let sql = r#"
                SELECT 
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind,
                    f.relpath,
                    fts.rank as score
                FROM fts_chunks fts
                JOIN chunks c ON c.id = fts.rowid
                JOIN files f ON f.id = c.file_id
                WHERE fts MATCH ?1
                  AND f.repo_id = ?2
                  AND (?3 IS NULL OR f.worktree_id = ?3)
                ORDER BY score
                LIMIT ?4
            "#;

            let mut stmt = conn.prepare(sql)?;
            let rows = stmt.query_map(params![fts_query, repo_id, worktree_id, k], |row| {
                let score: f64 = row.get(5)?;
                Ok(SearchHit {
                    start_line: row.get(0)?,
                    end_line: row.get(1)?,
                    symbol_name: row.get(2)?,
                    kind: row.get(3)?,
                    file_relpath: row.get(4)?,
                    score: -score, // FTS5 rank is lower=better, we want higher=better for consistency? 
                                   // Wait, usually rank is relevance. BM25 is negative in SQLite? 
                                   // Actually, 'rank' column value depends on function. 
                                   // Default 'rank' is negative of BM25 score? No.
                                   // Let's just use it raw for now or assume lower is better and negate it for sorting DESC.
                                   // Postgres ts_rank is higher=better.
                                   // SQLite FTS5 rank is usually lower=better (it's a distance-like metric often).
                                   // Let's verify FTS5 documentation. "The default rank function... returns a value that is less than or equal to zero."
                                   // So more negative is better? Or closer to zero?
                                   // Actually, it returns "a value <= 0.0". Smaller (more negative) is better?
                                   // "The default rank function returns a copy of its first argument." (Wait, no).
                                   // Standard `bm25()` returns negative values where more negative is better match.
                                   // So to sort by relevance descending (best first), we order by rank (which is negative).
                                   // Wait, `ORDER BY rank` sorts ascending (-10, -5, -1). -10 is better?
                                   // "The more relevant the match, the smaller the value returned."
                                   // So ASC order gives best results first.
                                   // We want to return a positive score. So let's negate it.
                    base_score: None,
                    kind_mult: None,
                    exact_mult: None,
                })
            })?;

            let mut hits = Vec::new();
            for row in rows {
                hits.push(row?);
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
            init_schema(conn)
        }).await
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
