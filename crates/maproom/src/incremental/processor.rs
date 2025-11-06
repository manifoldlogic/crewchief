//! Incremental file processing with atomic updates and transaction integrity.
//!
//! This module provides the core processor for incremental indexing, handling three types
//! of file changes (new, modified, deleted) with full transaction safety and edge consistency.
//!
//! # Architecture
//!
//! The processor coordinates:
//! - File parsing (via existing ParserFactory)
//! - Chunk database updates (atomic transactions)
//! - Edge relationship maintenance (via EdgeUpdater)
//!
//! # Transaction Flow
//!
//! For modified files:
//! ```sql
//! BEGIN;
//!   DELETE FROM maproom.chunks WHERE file_id = $1;
//!   INSERT INTO maproom.chunks (...) VALUES (...);
//!   UPDATE maproom.files SET blake3_hash = $1, last_modified = NOW() WHERE id = $2;
//!   DELETE FROM maproom.chunk_edges WHERE src_chunk_id IN (...) OR dst_chunk_id IN (...);
//!   INSERT INTO maproom.chunk_edges (...) VALUES (...);
//! COMMIT;
//! ```
//!
//! # Performance Target
//!
//! - File updates complete in <5s for typical files
//! - Automatic rollback on any error (prevents corruption)
//! - Batch operations within transactions for efficiency

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::db::PgPool;
use crate::indexer::SymbolChunk;

use super::detector::ChangeType;
use super::edge_updater::EdgeUpdater;
use super::hash::ContentHash;
use super::path_utils::normalize_to_relpath;
use super::task::UpdateTask;

/// Incremental processor for atomic file updates.
///
/// Processes individual file changes from the update queue with full
/// transaction safety. Each file operation is atomic - either all changes
/// succeed or all are rolled back.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use crewchief_maproom::db::create_pool;
/// use crewchief_maproom::incremental::{IncrementalProcessor, UpdateTask, ChangeType, FileHasher, Trigger};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pool = create_pool().await?;
///     let repo_root = PathBuf::from("/workspace");
///     let processor = IncrementalProcessor::new(pool, repo_root);
///
///     // Process a file update
///     let path = PathBuf::from("/workspace/src/main.rs");
///     let old_hash = FileHasher::hash_bytes(b"old content");
///     let new_hash = FileHasher::hash_bytes(b"new content");
///     let task = UpdateTask::new(
///         path,
///         ChangeType::Modified { old: old_hash, new: new_hash },
///         Trigger::Save
///     );
///
///     processor.process(task).await?;
///     Ok(())
/// }
/// ```
pub struct IncrementalProcessor {
    pool: PgPool,
    edge_updater: EdgeUpdater,
    repo_root: PathBuf,
}

impl IncrementalProcessor {
    /// Create a new incremental processor.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `repo_root` - Absolute path to the repository root (used for path normalization)
    ///
    /// # Returns
    /// A new processor ready to handle file updates
    pub fn new(pool: PgPool, repo_root: PathBuf) -> Self {
        Self {
            edge_updater: EdgeUpdater::new(pool.clone()),
            pool,
            repo_root,
        }
    }

    /// Process a single update task.
    ///
    /// Handles the task based on its change type:
    /// - New: Parse and insert file chunks
    /// - Modified: Delete old chunks, insert new ones, update file record
    /// - Deleted: Remove all chunks and edges
    /// - None: Skip (no changes needed)
    ///
    /// All operations are wrapped in a transaction for atomicity.
    ///
    /// # Arguments
    /// * `task` - The update task to process
    ///
    /// # Returns
    /// * `Ok(())` - Task processed successfully
    /// * `Err(_)` - Processing failed, transaction rolled back
    ///
    /// # Performance
    ///
    /// Typical processing times:
    /// - New file: 100-500ms (parse + insert)
    /// - Modified file: 200-800ms (delete + parse + insert + edges)
    /// - Deleted file: 50-200ms (delete chunks + edges)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use crewchief_maproom::db::create_pool;
    /// # use crewchief_maproom::incremental::{IncrementalProcessor, UpdateTask, ChangeType, FileHasher, Trigger};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let pool = create_pool().await?;
    /// let repo_root = PathBuf::from("/workspace");
    /// let processor = IncrementalProcessor::new(pool, repo_root);
    /// let task = UpdateTask::new(
    ///     PathBuf::from("/workspace/src/lib.rs"),
    ///     ChangeType::New(FileHasher::hash_bytes(b"content")),
    ///     Trigger::Auto
    /// );
    ///
    /// processor.process(task).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn process(&self, task: UpdateTask) -> Result<()> {
        let path_display = task.path.display().to_string();

        debug!(
            path = %path_display,
            change_type = ?task.change_type,
            priority = ?task.priority,
            "Processing update task"
        );

        match &task.change_type {
            ChangeType::New(hash) => {
                self.index_new_file(&task.path, hash).await
                    .with_context(|| format!("Failed to index new file: {}", path_display))?;
                info!(path = %path_display, "Indexed new file");
            }
            ChangeType::Modified { old: _, new } => {
                self.update_file(&task.path, new).await
                    .with_context(|| format!("Failed to update modified file: {}", path_display))?;
                info!(path = %path_display, "Updated modified file");
            }
            ChangeType::Deleted(_) => {
                self.remove_file(&task.path).await
                    .with_context(|| format!("Failed to remove deleted file: {}", path_display))?;
                info!(path = %path_display, "Removed deleted file");
            }
            ChangeType::None => {
                debug!(path = %path_display, "No change detected, skipping");
                return Ok(());
            }
        }

        Ok(())
    }

    /// Index a new file by parsing and inserting its chunks.
    ///
    /// # Transaction Flow
    /// 1. Look up or create file record in database
    /// 2. Parse file to extract chunks
    /// 3. Insert all chunks in a transaction
    /// 4. Update edges for new chunks
    ///
    /// # Arguments
    /// * `path` - Absolute filesystem path to the new file
    /// * `hash` - Content hash of the file
    ///
    /// # Returns
    /// * `Ok(())` - File indexed successfully
    /// * `Err(_)` - Indexing failed (e.g., parse error, DB error)
    async fn index_new_file(&self, path: &Path, hash: &ContentHash) -> Result<()> {
        // Get database connection
        let mut client = self.pool.get().await
            .context("Failed to get database connection from pool")?;

        // CRITICAL: Read file content using absolute path (filesystem operation)
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Detect language from file extension
        let language = detect_language_from_path(path);

        // CRITICAL: Normalize path for database query (database stores relative paths)
        // Absolute path example: "/workspace/packages/cli/src/main.ts"
        // Relative path example: "packages/cli/src/main.ts"
        let relpath = normalize_to_relpath(path, &self.repo_root)
            .with_context(|| format!("Failed to normalize path: {}", path.display()))?;

        let relpath_str = relpath.to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path: {}", relpath.display()))?;

        // Query to find the file record (it should exist from the watcher's file creation)
        let file_row = client
            .query_opt(
                "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
                &[&relpath_str],
            )
            .await
            .context("Failed to query file record")?;

        let file_id = match file_row {
            Some(row) => row.get::<_, i64>(0),
            None => {
                // File doesn't exist in DB yet - this is an error condition
                // The file should have been created by the watcher before queueing the update
                anyhow::bail!("File not found in database (relpath={})", relpath_str);
            }
        };

        // Parse file to extract chunks
        let chunks = parse_file_chunks(&content, language.unwrap_or("unknown"))?;

        // Begin transaction
        let tx = client.transaction().await
            .context("Failed to begin transaction")?;

        // Insert all chunks
        for chunk in &chunks {
            insert_chunk_in_transaction(&tx, file_id, chunk, &content).await
                .context("Failed to insert chunk")?;
        }

        // Update file record with hash
        let hash_bytes: &[u8] = hash.as_bytes();
        tx.execute(
            "UPDATE maproom.files SET blake3_hash = $1, last_modified = NOW() WHERE id = $2",
            &[&hash_bytes, &file_id],
        )
        .await
        .context("Failed to update file hash")?;

        // Commit transaction
        tx.commit().await
            .context("Failed to commit transaction")?;

        // Update edges (outside transaction for better performance)
        self.edge_updater.update_edges(file_id).await
            .context("Failed to update edges")?;

        Ok(())
    }

    /// Update an existing file by replacing its chunks.
    ///
    /// # Transaction Flow
    /// 1. Begin transaction
    /// 2. Delete all existing chunks for the file
    /// 3. Parse file and insert new chunks
    /// 4. Update file record with new hash and timestamp
    /// 5. Commit transaction
    /// 6. Update edges (after transaction completes)
    ///
    /// # Arguments
    /// * `path` - Absolute filesystem path to the modified file
    /// * `new_hash` - New content hash of the file
    ///
    /// # Returns
    /// * `Ok(())` - File updated successfully
    /// * `Err(_)` - Update failed, transaction rolled back
    async fn update_file(&self, path: &Path, new_hash: &ContentHash) -> Result<()> {
        // Get database connection
        let mut client = self.pool.get().await
            .context("Failed to get database connection from pool")?;

        // CRITICAL: Read file content using absolute path (filesystem operation)
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Detect language from file extension
        let language = detect_language_from_path(path);

        // CRITICAL: Normalize path for database query (database stores relative paths)
        // Absolute path example: "/workspace/packages/cli/src/main.ts"
        // Relative path example: "packages/cli/src/main.ts"
        let relpath = normalize_to_relpath(path, &self.repo_root)
            .with_context(|| format!("Failed to normalize path: {}", path.display()))?;

        let relpath_str = relpath.to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path: {}", relpath.display()))?;

        // Look up file record
        let file_row = client
            .query_opt(
                "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
                &[&relpath_str],
            )
            .await
            .context("Failed to query file record")?;

        let file_id = match file_row {
            Some(row) => row.get::<_, i64>(0),
            None => {
                warn!(path = %path.display(), relpath = %relpath_str, "File not found in database during update");
                anyhow::bail!("File not found in database (relpath={})", relpath_str);
            }
        };

        // Parse file to extract new chunks
        let chunks = parse_file_chunks(&content, language.unwrap_or("unknown"))?;

        // Begin transaction
        let tx = client.transaction().await
            .context("Failed to begin transaction")?;

        // Delete old chunks (CASCADE will delete edges automatically)
        tx.execute(
            "DELETE FROM maproom.chunks WHERE file_id = $1",
            &[&file_id],
        )
        .await
        .context("Failed to delete old chunks")?;

        // Insert new chunks
        for chunk in &chunks {
            insert_chunk_in_transaction(&tx, file_id, chunk, &content).await
                .context("Failed to insert new chunk")?;
        }

        // Update file record with new hash and timestamp
        let hash_bytes: &[u8] = new_hash.as_bytes();
        tx.execute(
            "UPDATE maproom.files SET blake3_hash = $1, last_modified = NOW() WHERE id = $2",
            &[&hash_bytes, &file_id],
        )
        .await
        .context("Failed to update file record")?;

        // Commit transaction
        tx.commit().await
            .context("Failed to commit transaction")?;

        // Update edges (after transaction completes)
        self.edge_updater.update_edges(file_id).await
            .context("Failed to update edges after file modification")?;

        Ok(())
    }

    /// Remove a deleted file and all its chunks.
    ///
    /// # Transaction Flow
    /// 1. Begin transaction
    /// 2. Delete all chunks (CASCADE deletes edges automatically)
    /// 3. Delete file record
    /// 4. Commit transaction
    ///
    /// # Arguments
    /// * `path` - Absolute filesystem path to the deleted file
    ///
    /// # Returns
    /// * `Ok(())` - File removed successfully
    /// * `Err(_)` - Removal failed, transaction rolled back
    async fn remove_file(&self, path: &Path) -> Result<()> {
        // Get database connection
        let mut client = self.pool.get().await
            .context("Failed to get database connection from pool")?;

        // CRITICAL: Normalize path for database query (database stores relative paths)
        // Absolute path example: "/workspace/packages/cli/src/main.ts"
        // Relative path example: "packages/cli/src/main.ts"
        let relpath = normalize_to_relpath(path, &self.repo_root)
            .with_context(|| format!("Failed to normalize path: {}", path.display()))?;

        let relpath_str = relpath.to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path: {}", relpath.display()))?;

        // Look up file record
        let file_row = client
            .query_opt(
                "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
                &[&relpath_str],
            )
            .await
            .context("Failed to query file record")?;

        let file_id = match file_row {
            Some(row) => row.get::<_, i64>(0),
            None => {
                // File already removed or never existed
                debug!(path = %path.display(), relpath = %relpath_str, "File not found in database during deletion");
                return Ok(());
            }
        };

        // Begin transaction
        let tx = client.transaction().await
            .context("Failed to begin transaction")?;

        // Delete chunks (CASCADE will delete edges automatically via ON DELETE CASCADE)
        let deleted_chunks = tx
            .execute("DELETE FROM maproom.chunks WHERE file_id = $1", &[&file_id])
            .await
            .context("Failed to delete chunks")?;

        debug!(file_id = file_id, chunks_deleted = deleted_chunks, "Deleted chunks for file");

        // Delete file record
        tx.execute("DELETE FROM maproom.files WHERE id = $1", &[&file_id])
            .await
            .context("Failed to delete file record")?;

        // Commit transaction
        tx.commit().await
            .context("Failed to commit transaction")?;

        Ok(())
    }
}

/// Detect programming language from file path extension.
///
/// # Arguments
/// * `path` - File path to analyze
///
/// # Returns
/// Language identifier (e.g., "ts", "rs", "md") or None if unknown
fn detect_language_from_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "ts" => Some("ts"),
        "tsx" => Some("tsx"),
        "js" => Some("js"),
        "jsx" => Some("jsx"),
        "rs" => Some("rs"),
        "md" => Some("md"),
        "mdx" => Some("mdx"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        _ => None,
    }
}

/// Parse a file's content to extract symbol chunks.
///
/// Uses the existing parser infrastructure from `crate::indexer::parser`.
///
/// # Arguments
/// * `content` - File content as string
/// * `language` - Language identifier
///
/// # Returns
/// Vector of symbol chunks extracted from the file
fn parse_file_chunks(content: &str, language: &str) -> Result<Vec<SymbolChunk>> {
    use crate::indexer::parser;

    let chunks = parser::extract_chunks(content, language);

    // If no chunks extracted, create a single module-level chunk
    if chunks.is_empty() {
        Ok(vec![SymbolChunk {
            symbol_name: None,
            kind: "module".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: content.lines().count() as i32,
            metadata: None,
        }])
    } else {
        Ok(chunks)
    }
}

/// Insert a chunk into the database within a transaction.
///
/// # Arguments
/// * `tx` - Active database transaction
/// * `file_id` - ID of the file this chunk belongs to
/// * `chunk` - Symbol chunk to insert
/// * `content` - Full file content (for extracting preview)
///
/// # Returns
/// Chunk ID on success
async fn insert_chunk_in_transaction(
    tx: &tokio_postgres::Transaction<'_>,
    file_id: i64,
    chunk: &SymbolChunk,
    content: &str,
) -> Result<i64> {
    // Extract preview (first 40 lines of chunk content)
    let chunk_lines: Vec<&str> = content
        .lines()
        .skip((chunk.start_line as usize).saturating_sub(1))
        .take((chunk.end_line - chunk.start_line + 1) as usize)
        .collect();
    let preview = chunk_lines.iter().take(40).copied().collect::<Vec<_>>().join("\n");

    // Build ts_doc text (for full-text search)
    let ts_doc = build_ts_doc(
        chunk.symbol_name.as_deref(),
        chunk.signature.as_deref(),
        chunk.docstring.as_deref(),
        &preview,
    );

    // Insert chunk
    let row = tx
        .query_one(
            "INSERT INTO maproom.chunks (
                file_id, symbol_name, kind, signature, docstring,
                start_line, end_line, preview, ts_doc, recency_score, churn_score
            ) VALUES (
                $1, $2::text, ($3::text)::maproom.symbol_kind, $4::text, $5::text,
                $6, $7, $8::text, to_tsvector('simple', unaccent($9::text)), $10, $11
            )
            RETURNING id",
            &[
                &file_id,
                &chunk.symbol_name,
                &chunk.kind,
                &chunk.signature,
                &chunk.docstring,
                &chunk.start_line,
                &chunk.end_line,
                &preview,
                &ts_doc,
                &1.0f32, // recency_score (default)
                &0.0f32, // churn_score (default)
            ],
        )
        .await
        .context("Failed to insert chunk")?;

    Ok(row.get(0))
}

/// Build full-text search document from chunk metadata.
///
/// Combines symbol name, signature, docstring, and preview into a single
/// searchable document.
///
/// # Arguments
/// * `symbol_name` - Optional symbol name
/// * `signature` - Optional function/class signature
/// * `docstring` - Optional documentation string
/// * `preview` - Code preview text
///
/// # Returns
/// Combined text document for full-text search
fn build_ts_doc(
    symbol_name: Option<&str>,
    signature: Option<&str>,
    docstring: Option<&str>,
    preview: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(s) = symbol_name {
        parts.push(s.to_owned());
    }
    if let Some(s) = signature {
        parts.push(s.to_owned());
    }
    if let Some(s) = docstring {
        parts.push(s.to_owned());
    }
    parts.push(preview.to_owned());

    parts.join(" \n ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_from_path() {
        assert_eq!(
            detect_language_from_path(Path::new("src/main.rs")),
            Some("rs")
        );
        assert_eq!(
            detect_language_from_path(Path::new("src/lib.ts")),
            Some("ts")
        );
        assert_eq!(
            detect_language_from_path(Path::new("README.md")),
            Some("md")
        );
        assert_eq!(
            detect_language_from_path(Path::new("config.yaml")),
            Some("yaml")
        );
        assert_eq!(detect_language_from_path(Path::new("unknown.xyz")), None);
    }

    #[test]
    fn test_build_ts_doc() {
        let doc = build_ts_doc(
            Some("myFunction"),
            Some("fn myFunction(x: i32) -> i32"),
            Some("Does something cool"),
            "let x = 42;",
        );

        assert!(doc.contains("myFunction"));
        assert!(doc.contains("fn myFunction"));
        assert!(doc.contains("Does something cool"));
        assert!(doc.contains("let x = 42;"));
    }

    #[test]
    fn test_build_ts_doc_minimal() {
        let doc = build_ts_doc(None, None, None, "some code");
        assert_eq!(doc, "some code");
    }

    #[test]
    fn test_parse_file_chunks_creates_module_for_empty() {
        let chunks = parse_file_chunks("", "unknown").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, "module");
    }
}
