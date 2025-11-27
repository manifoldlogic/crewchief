use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::db::SqliteStore;
use tracing::{error, info, warn};

use crate::indexer::parser;

/// Statistics for a single file migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMigrationStats {
    pub file_id: i64,
    pub relpath: String,
    pub old_chunks: usize,
    pub new_chunks: usize,
    pub delta: i64,
}

/// Overall migration statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MigrationStats {
    pub files_processed: usize,
    pub total_old_chunks: usize,
    pub total_new_chunks: usize,
    pub files_with_errors: usize,
    pub backup_table: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl MigrationStats {
    pub fn record_file(&mut self, file_stats: FileMigrationStats) {
        self.files_processed += 1;
        self.total_old_chunks += file_stats.old_chunks;
        self.total_new_chunks += file_stats.new_chunks;
    }

    pub fn record_error(&mut self) {
        self.files_with_errors += 1;
    }

    pub fn delta(&self) -> i64 {
        self.total_new_chunks as i64 - self.total_old_chunks as i64
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        if let (Some(start), Some(end)) = (self.started_at, self.completed_at) {
            Some(end - start)
        } else {
            None
        }
    }
}

/// Result type for migration operations
#[derive(Debug)]
pub struct MigrationResult {
    pub stats: MigrationStats,
    pub backup_table: String,
}

/// Markdown file record from database
#[derive(Debug)]
struct MarkdownFile {
    id: i64,
    relpath: String,
    content: String,
}

/// Handles migration of markdown chunks from regex parser to tree-sitter parser
pub struct MarkdownMigrator {
    store: SqliteStore,
}

impl MarkdownMigrator {
    /// Create a new migrator with a database store
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    /// Run the complete migration for a repository
    pub async fn migrate(
        &self,
        repo_name: &str,
        worktree_name: Option<&str>,
    ) -> Result<MigrationResult> {
        let mut stats = MigrationStats {
            started_at: Some(chrono::Utc::now()),
            ..Default::default()
        };

        info!("Starting markdown migration for repo: {}", repo_name);

        // Create backup table
        let backup_table = self.create_backup().await?;
        stats.backup_table = Some(backup_table.clone());
        info!("Created backup table: {}", backup_table);

        // Get markdown files
        let files = self.get_markdown_files(repo_name, worktree_name).await?;
        info!("Found {} markdown files to migrate", files.len());

        // Migrate each file
        for file in files {
            match self.migrate_file(&file, &mut stats).await {
                Ok(file_stats) => {
                    info!(
                        "Migrated {}: {} → {} chunks ({:+})",
                        file_stats.relpath,
                        file_stats.old_chunks,
                        file_stats.new_chunks,
                        file_stats.delta
                    );
                }
                Err(e) => {
                    error!("Failed to migrate file {}: {}", file.relpath, e);
                    stats.record_error();
                }
            }
        }

        stats.completed_at = Some(chrono::Utc::now());

        info!(
            "Migration complete: {} files processed, {} → {} chunks ({:+}), {} errors",
            stats.files_processed,
            stats.total_old_chunks,
            stats.total_new_chunks,
            stats.delta(),
            stats.files_with_errors
        );

        if let Some(duration) = stats.duration() {
            info!(
                "Migration took {:.2}s",
                duration.num_milliseconds() as f64 / 1000.0
            );
        }

        Ok(MigrationResult {
            stats,
            backup_table,
        })
    }

    /// Create a backup table for existing chunks
    async fn create_backup(&self) -> Result<String> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_table = format!("chunks_backup_{}", timestamp);
        let backup_table_clone = backup_table.clone();

        self.store.run(move |conn| {
            // Create backup table with same schema as chunks
            conn.execute(
                &format!(
                    "CREATE TABLE {} AS
                     SELECT * FROM chunks
                     WHERE file_id IN (
                         SELECT id FROM files WHERE language IN ('md', 'mdx')
                     )",
                    backup_table_clone
                ),
                [],
            )
            .context("Failed to create backup table")?;

            // Add index on file_id for faster rollback
            conn.execute(
                &format!("CREATE INDEX idx_{}_file_id ON {} (file_id)", backup_table_clone, backup_table_clone),
                [],
            )
            .context("Failed to create backup index")?;

            Ok(())
        }).await?;

        Ok(backup_table)
    }

    /// Get all markdown files from the database
    async fn get_markdown_files(
        &self,
        repo_name: &str,
        worktree_name: Option<&str>,
    ) -> Result<Vec<MarkdownFile>> {
        let repo_name = repo_name.to_string();
        let worktree_name = worktree_name.map(|s| s.to_string());

        self.store.run(move |conn| {
            use rusqlite::params;

            let (query, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(ref worktree) = worktree_name {
                (
                    "SELECT f.id, f.relpath,
                            COALESCE(
                                (SELECT content FROM file_contents WHERE file_id = f.id LIMIT 1),
                                ''
                            ) as content
                     FROM files f
                     JOIN repos r ON f.repo_id = r.id
                     JOIN worktrees w ON f.worktree_id = w.id
                     WHERE r.name = ?1 AND w.name = ?2 AND f.language IN ('md', 'mdx')
                     ORDER BY f.relpath".to_string(),
                    vec![Box::new(repo_name.clone()), Box::new(worktree.clone())]
                )
            } else {
                (
                    "SELECT f.id, f.relpath,
                            COALESCE(
                                (SELECT content FROM file_contents WHERE file_id = f.id LIMIT 1),
                                ''
                            ) as content
                     FROM files f
                     JOIN repos r ON f.repo_id = r.id
                     WHERE r.name = ?1 AND f.language IN ('md', 'mdx')
                     ORDER BY f.relpath".to_string(),
                    vec![Box::new(repo_name.clone())]
                )
            };

            let mut stmt = conn.prepare(&query)?;
            let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

            let rows = stmt.query_map(params_slice.as_slice(), |row| {
                Ok(MarkdownFile {
                    id: row.get(0)?,
                    relpath: row.get(1)?,
                    content: row.get(2)?,
                })
            })?;

            let mut files = Vec::new();
            for row_result in rows {
                let file = row_result?;

                // If content is empty, skip the file
                if file.content.is_empty() {
                    warn!("File {} has no content in database, skipping", file.relpath);
                    continue;
                }

                files.push(file);
            }

            Ok(files)
        }).await
    }

    /// Migrate a single file
    async fn migrate_file(
        &self,
        file: &MarkdownFile,
        stats: &mut MigrationStats,
    ) -> Result<FileMigrationStats> {
        let file_id = file.id;
        let file_relpath = file.relpath.clone();
        let file_content = file.content.clone();
        let store = self.store.clone();

        let file_stats = store.run(move |conn| {
            use rusqlite::params;

            // Start transaction
            let tx = conn.transaction()?;

            // Count old chunks
            let old_chunks: i64 = tx.query_row(
                "SELECT COUNT(*) FROM chunks WHERE file_id = ?1",
                params![file_id],
                |row| row.get(0),
            )?;

            // Parse with new parser
            let new_chunks = parser::extract_chunks(&file_content, "md");

            // Delete old chunks
            tx.execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])
                .map_err(|e| anyhow::anyhow!("Failed to delete old chunks: {}", e))?;

            // Insert new chunks
            let mut inserted_count = 0;
            for chunk in &new_chunks {
                let preview = Self::extract_preview(&file_content, chunk.start_line, chunk.end_line);
                let ts_doc = Self::build_ts_doc(&file_relpath, chunk, &preview);

                // Convert metadata to JSON string for SQLite
                let metadata_json = chunk.metadata.as_ref().map(|v| v.to_string());

                // Insert chunk directly in transaction
                // TODO: SQLite doesn't have FTS tsvector - consider using FTS5 table or storing as text
                tx.execute(
                    "INSERT INTO chunks (
                       file_id, blob_sha, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc_text, recency_score, churn_score, metadata
                     ) VALUES (
                       ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
                     )
                     ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
                       symbol_name = excluded.symbol_name,
                       kind = excluded.kind,
                       signature = excluded.signature,
                       docstring = excluded.docstring,
                       preview = excluded.preview,
                       ts_doc_text = excluded.ts_doc_text,
                       metadata = excluded.metadata",
                    params![
                        file_id,
                        "", // blob_sha - TODO: compute actual blob SHA
                        chunk.symbol_name,
                        chunk.kind,
                        chunk.signature,
                        chunk.docstring,
                        chunk.start_line,
                        chunk.end_line,
                        preview,
                        ts_doc,
                        1.0f32,
                        0.0f32,
                        metadata_json,
                    ],
                )?;

                inserted_count += 1;
            }

            // Commit transaction
            tx.commit()?;

            Ok(FileMigrationStats {
                file_id,
                relpath: file_relpath,
                old_chunks: old_chunks as usize,
                new_chunks: inserted_count,
                delta: inserted_count as i64 - old_chunks,
            })
        }).await?;

        stats.record_file(file_stats.clone());

        Ok(file_stats)
    }

    /// Rollback migration from a backup table
    pub async fn rollback(&self, backup_table: &str) -> Result<()> {
        info!("Starting rollback from backup table: {}", backup_table);

        let backup_table = backup_table.to_string();
        let store = self.store.clone();

        store.run(move |conn| {
            use rusqlite::params;

            // Verify backup table exists
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name = ?1",
                params![&backup_table],
                |row| row.get(0),
            )?;

            if !exists {
                anyhow::bail!("Backup table {} does not exist", backup_table);
            }

            // Start transaction
            let tx = conn.transaction()?;

            // Get file IDs from backup
            let file_ids: Vec<i64> = {
                let mut stmt = tx.prepare(&format!("SELECT DISTINCT file_id FROM {}", backup_table))?;
                let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
                rows.collect::<Result<Vec<_>, _>>()?
            };

            info!("Restoring {} files from backup", file_ids.len());

            // Delete current chunks for these files
            let placeholders = file_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let delete_query = format!("DELETE FROM chunks WHERE file_id IN ({})", placeholders);
            let file_id_params: Vec<&dyn rusqlite::ToSql> = file_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
            tx.execute(&delete_query, file_id_params.as_slice())?;

            // Restore from backup
            tx.execute(
                &format!("INSERT INTO chunks SELECT * FROM {}", backup_table),
                [],
            )?;

            // Commit transaction
            tx.commit()?;

            info!("Rollback complete");

            Ok(())
        }).await
    }

    /// List available backup tables
    pub async fn list_backups(&self) -> Result<Vec<String>> {
        self.store.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT name FROM sqlite_master
                 WHERE type='table'
                 AND name LIKE 'chunks_backup_%'
                 ORDER BY name DESC"
            )?;

            let backups = stmt.query_map([], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(backups)
        }).await
    }

    /// Delete a backup table
    pub async fn delete_backup(&self, backup_table: &str) -> Result<()> {
        info!("Deleting backup table: {}", backup_table);

        let backup_table = backup_table.to_string();
        self.store.run(move |conn| {
            conn.execute(
                &format!("DROP TABLE IF EXISTS {}", backup_table),
                [],
            )?;
            Ok(())
        }).await?;

        info!("Backup table deleted");

        Ok(())
    }

    /// Extract preview text from content
    fn extract_preview(content: &str, start_line: i32, end_line: i32) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let start_idx = (start_line as usize).saturating_sub(1).min(lines.len());
        let end_idx = (end_line as usize).min(lines.len());

        let preview_lines = &lines[start_idx..end_idx];
        let preview = preview_lines.join("\n");

        // Limit to 40 lines
        preview.lines().take(40).collect::<Vec<_>>().join("\n")
    }

    /// Build ts_doc for full-text search
    fn build_ts_doc(relpath: &str, chunk: &crate::indexer::SymbolChunk, preview: &str) -> String {
        let mut parts: Vec<String> = Vec::new();
        parts.push(relpath.to_owned());
        if let Some(s) = &chunk.symbol_name {
            parts.push(s.clone());
        }
        if let Some(s) = &chunk.signature {
            parts.push(s.clone());
        }
        if let Some(s) = &chunk.docstring {
            parts.push(s.clone());
        }
        parts.push(preview.to_owned());
        parts.join(" \n ")
    }
}

/// Verify migration integrity
pub async fn verify_migration(store: &SqliteStore, repo_name: &str) -> Result<HashMap<String, usize>> {
    let repo_name = repo_name.to_string();

    store.run(move |conn| {
        use rusqlite::params;

        let mut results = HashMap::new();

        // Count markdown files
        let file_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files f
             JOIN repos r ON f.repo_id = r.id
             WHERE r.name = ?1 AND f.language IN ('md', 'mdx')",
            params![&repo_name],
            |row| row.get(0),
        )?;
        results.insert("markdown_files".to_string(), file_count as usize);

        // Count chunks
        let chunk_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks c
             JOIN files f ON c.file_id = f.id
             JOIN repos r ON f.repo_id = r.id
             WHERE r.name = ?1 AND f.language IN ('md', 'mdx')",
            params![&repo_name],
            |row| row.get(0),
        )?;
        results.insert("total_chunks".to_string(), chunk_count as usize);

        // Count chunks with parent_path (from metadata)
        // In SQLite, we need to parse JSON differently
        let parent_path_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks c
             JOIN files f ON c.file_id = f.id
             JOIN repos r ON f.repo_id = r.id
             WHERE r.name = ?1 AND f.language IN ('md', 'mdx')
               AND c.metadata IS NOT NULL
               AND json_extract(c.metadata, '$.parent_path') IS NOT NULL",
            params![&repo_name],
            |row| row.get(0),
        )?;
        results.insert(
            "chunks_with_parent_path".to_string(),
            parent_path_count as usize,
        );

        // Count code blocks with language metadata
        let code_block_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks c
             JOIN files f ON c.file_id = f.id
             JOIN repos r ON f.repo_id = r.id
             WHERE r.name = ?1 AND f.language IN ('md', 'mdx')
               AND c.kind = 'code_block'
               AND c.metadata IS NOT NULL
               AND json_extract(c.metadata, '$.language') IS NOT NULL",
            params![&repo_name],
            |row| row.get(0),
        )?;
        results.insert(
            "code_blocks_with_language".to_string(),
            code_block_count as usize,
        );

        Ok(results)
    }).await
}
