use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_postgres::Client;
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
    client: Client,
}

impl MarkdownMigrator {
    /// Create a new migrator with a database client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Run the complete migration for a repository
    pub async fn migrate(
        &mut self,
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

        // Create backup table with same schema as chunks
        self.client
            .execute(
                &format!(
                    "CREATE TABLE maproom.{} AS
                     SELECT * FROM maproom.chunks
                     WHERE file_id IN (
                         SELECT id FROM maproom.files WHERE language IN ('md', 'mdx')
                     )",
                    backup_table
                ),
                &[],
            )
            .await
            .context("Failed to create backup table")?;

        // Add index on file_id for faster rollback
        self.client
            .execute(
                &format!("CREATE INDEX ON maproom.{} (file_id)", backup_table),
                &[],
            )
            .await
            .context("Failed to create backup index")?;

        Ok(backup_table)
    }

    /// Get all markdown files from the database
    async fn get_markdown_files(
        &self,
        repo_name: &str,
        worktree_name: Option<&str>,
    ) -> Result<Vec<MarkdownFile>> {
        let query = if let Some(_worktree) = worktree_name {
            "SELECT f.id, f.relpath,
                    COALESCE(
                        (SELECT content FROM maproom.file_contents WHERE file_id = f.id LIMIT 1),
                        ''
                    ) as content
             FROM maproom.files f
             JOIN maproom.repos r ON f.repo_id = r.id
             JOIN maproom.worktrees w ON f.worktree_id = w.id
             WHERE r.name = $1 AND w.name = $2 AND f.language IN ('md', 'mdx')
             ORDER BY f.relpath"
        } else {
            "SELECT f.id, f.relpath,
                    COALESCE(
                        (SELECT content FROM maproom.file_contents WHERE file_id = f.id LIMIT 1),
                        ''
                    ) as content
             FROM maproom.files f
             JOIN maproom.repos r ON f.repo_id = r.id
             WHERE r.name = $1 AND f.language IN ('md', 'mdx')
             ORDER BY f.relpath"
        };

        let rows = if let Some(worktree) = worktree_name {
            self.client.query(query, &[&repo_name, &worktree]).await?
        } else {
            self.client.query(query, &[&repo_name]).await?
        };

        let mut files = Vec::new();
        for row in rows {
            let id: i64 = row.get(0);
            let relpath: String = row.get(1);
            let content: String = row.get(2);

            // If content is empty, try to read from filesystem
            // This is a fallback for when file_contents table doesn't have the data
            if content.is_empty() {
                warn!("File {} has no content in database, skipping", relpath);
                continue;
            }

            files.push(MarkdownFile {
                id,
                relpath,
                content,
            });
        }

        Ok(files)
    }

    /// Migrate a single file
    async fn migrate_file(
        &mut self,
        file: &MarkdownFile,
        stats: &mut MigrationStats,
    ) -> Result<FileMigrationStats> {
        // Start transaction
        let tx = self.client.build_transaction().start().await?;

        // Count old chunks
        let old_count_row = tx
            .query_one(
                "SELECT COUNT(*) FROM maproom.chunks WHERE file_id = $1",
                &[&file.id],
            )
            .await?;
        let old_chunks: i64 = old_count_row.get(0);

        // Parse with new parser
        let new_chunks = parser::extract_chunks(&file.content, "md");

        // Delete old chunks
        tx.execute("DELETE FROM maproom.chunks WHERE file_id = $1", &[&file.id])
            .await
            .context("Failed to delete old chunks")?;

        // Insert new chunks
        let mut inserted_count = 0;
        for chunk in &new_chunks {
            let preview = Self::extract_preview(&file.content, chunk.start_line, chunk.end_line);
            let ts_doc = Self::build_ts_doc(&file.relpath, chunk, &preview);

            // Insert chunk directly in transaction
            tx.query_one(
                "INSERT INTO maproom.chunks (
                   file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, metadata
                 ) VALUES (
                   $1, $2::text, ($3::text)::maproom.symbol_kind, $4::text, $5::text, $6, $7, $8::text, to_tsvector('simple', unaccent($9::text)), $10, $11, $12::jsonb
                 )
                 ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
                   symbol_name = EXCLUDED.symbol_name,
                   kind = EXCLUDED.kind,
                   signature = EXCLUDED.signature,
                   docstring = EXCLUDED.docstring,
                   preview = EXCLUDED.preview,
                   ts_doc = EXCLUDED.ts_doc,
                   metadata = EXCLUDED.metadata
                 RETURNING id",
                &[
                    &file.id,
                    &chunk.symbol_name,
                    &chunk.kind,
                    &chunk.signature,
                    &chunk.docstring,
                    &chunk.start_line,
                    &chunk.end_line,
                    &preview,
                    &ts_doc,
                    &1.0f32,
                    &0.0f32,
                    &chunk.metadata,
                ],
            )
            .await?;

            inserted_count += 1;
        }

        // Commit transaction
        tx.commit().await?;

        let file_stats = FileMigrationStats {
            file_id: file.id,
            relpath: file.relpath.clone(),
            old_chunks: old_chunks as usize,
            new_chunks: inserted_count,
            delta: inserted_count as i64 - old_chunks,
        };

        stats.record_file(file_stats.clone());

        Ok(file_stats)
    }

    /// Rollback migration from a backup table
    pub async fn rollback(&mut self, backup_table: &str) -> Result<()> {
        info!("Starting rollback from backup table: {}", backup_table);

        // Verify backup table exists
        let exists_row = self
            .client
            .query_opt(
                "SELECT EXISTS (
                    SELECT FROM information_schema.tables
                    WHERE table_schema = 'maproom'
                    AND table_name = $1
                )",
                &[&backup_table],
            )
            .await?;

        let exists: bool = exists_row.and_then(|row| row.get(0)).unwrap_or(false);

        if !exists {
            anyhow::bail!("Backup table {} does not exist", backup_table);
        }

        // Start transaction
        let tx = self.client.build_transaction().start().await?;

        // Get file IDs from backup
        let file_ids: Vec<i64> = tx
            .query(
                &format!("SELECT DISTINCT file_id FROM maproom.{}", backup_table),
                &[],
            )
            .await?
            .iter()
            .map(|row| row.get(0))
            .collect();

        info!("Restoring {} files from backup", file_ids.len());

        // Delete current chunks for these files
        tx.execute(
            "DELETE FROM maproom.chunks WHERE file_id = ANY($1)",
            &[&file_ids],
        )
        .await?;

        // Restore from backup
        tx.execute(
            &format!(
                "INSERT INTO maproom.chunks
                 SELECT * FROM maproom.{}",
                backup_table
            ),
            &[],
        )
        .await?;

        // Commit transaction
        tx.commit().await?;

        info!("Rollback complete");

        Ok(())
    }

    /// List available backup tables
    pub async fn list_backups(&self) -> Result<Vec<String>> {
        let rows = self
            .client
            .query(
                "SELECT table_name FROM information_schema.tables
                 WHERE table_schema = 'maproom'
                 AND table_name LIKE 'chunks_backup_%'
                 ORDER BY table_name DESC",
                &[],
            )
            .await?;

        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    /// Delete a backup table
    pub async fn delete_backup(&mut self, backup_table: &str) -> Result<()> {
        info!("Deleting backup table: {}", backup_table);

        self.client
            .execute(
                &format!("DROP TABLE IF EXISTS maproom.{}", backup_table),
                &[],
            )
            .await?;

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
pub async fn verify_migration(client: &Client, repo_name: &str) -> Result<HashMap<String, usize>> {
    let mut results = HashMap::new();

    // Count markdown files
    let file_count_row = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.files f
             JOIN maproom.repos r ON f.repo_id = r.id
             WHERE r.name = $1 AND f.language IN ('md', 'mdx')",
            &[&repo_name],
        )
        .await?;
    let file_count: i64 = file_count_row.get(0);
    results.insert("markdown_files".to_string(), file_count as usize);

    // Count chunks
    let chunk_count_row = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks c
             JOIN maproom.files f ON c.file_id = f.id
             JOIN maproom.repos r ON f.repo_id = r.id
             WHERE r.name = $1 AND f.language IN ('md', 'mdx')",
            &[&repo_name],
        )
        .await?;
    let chunk_count: i64 = chunk_count_row.get(0);
    results.insert("total_chunks".to_string(), chunk_count as usize);

    // Count chunks with parent_path (from metadata)
    let parent_path_count_row = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks c
             JOIN maproom.files f ON c.file_id = f.id
             JOIN maproom.repos r ON f.repo_id = r.id
             WHERE r.name = $1 AND f.language IN ('md', 'mdx')
               AND c.metadata IS NOT NULL
               AND c.metadata->>'parent_path' IS NOT NULL",
            &[&repo_name],
        )
        .await?;
    let parent_path_count: i64 = parent_path_count_row.get(0);
    results.insert(
        "chunks_with_parent_path".to_string(),
        parent_path_count as usize,
    );

    // Count code blocks with language metadata
    let code_block_count_row = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks c
             JOIN maproom.files f ON c.file_id = f.id
             JOIN maproom.repos r ON f.repo_id = r.id
             WHERE r.name = $1 AND f.language IN ('md', 'mdx')
               AND c.kind = 'code_block'
               AND c.metadata IS NOT NULL
               AND c.metadata->>'language' IS NOT NULL",
            &[&repo_name],
        )
        .await?;
    let code_block_count: i64 = code_block_count_row.get(0);
    results.insert(
        "code_blocks_with_language".to_string(),
        code_block_count as usize,
    );

    Ok(results)
}
