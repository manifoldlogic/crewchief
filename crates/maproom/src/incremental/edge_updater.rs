//! Edge relationship updater for incremental indexing.
//!
//! This module maintains consistency of chunk edges (relationships between code symbols)
//! after file changes. When a file is modified, edges involving its chunks must be
//! recomputed to maintain accurate code relationships.
//!
//! NOTE: This module is a placeholder for future edge computation implementation.
//! Most code is dead until the feature is completed.
//!
//! # Edge Types
//!
//! Supported edge types (from database schema):
//! - `imports` - Symbol imports another symbol
//! - `exports` - Symbol exports another symbol
//! - `calls` - Function calls another function
//! - `called_by` - Function is called by another function
//! - `test_of` - Test targets a specific function/class
//! - `route_of` - Route handler for a specific path
//!
//! # Architecture
//!
//! Edge updates follow this flow:
//! 1. Find all chunks in the modified file
//! 2. Delete all edges involving those chunks
//! 3. Recompute edges based on new chunk content
//! 4. Insert new edges into database
//!
//! # Performance
//!
//! - Edge deletion: O(n) where n = number of chunks in file
//! - Edge computation: Depends on chunk complexity (typically <100ms)
//! - Edge insertion: Batch operation, <50ms for typical files

use anyhow::{Context, Result};
use tracing::debug;

use crate::db::Store;
use std::sync::Arc;

/// Edge updater for maintaining chunk relationships.
///
/// Handles edge updates after file modifications to keep the code graph
/// consistent and accurate.
///
/// # Example
///
/// ```ignore
/// use maproom::db::create_pool;
/// use maproom::incremental::EdgeUpdater;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pool = create_pool().await?;
///     let updater = EdgeUpdater::new(pool);
///
///     // Update edges for a specific file
///     updater.update_edges(123).await?;
///     Ok(())
/// }
/// ```
pub struct EdgeUpdater {
    store: Arc<dyn Store + Send + Sync>,
}

impl EdgeUpdater {
    /// Create a new edge updater.
    ///
    /// # Arguments
    /// * `store` - backend store handle
    ///
    /// # Returns
    /// A new edge updater ready to maintain chunk relationships
    pub fn new(store: Arc<dyn Store + Send + Sync>) -> Self {
        Self { store }
    }

    /// Update edges for all chunks in a file.
    ///
    /// This method:
    /// 1. Finds all chunk IDs for the given file
    /// 2. Deletes all edges involving those chunks
    /// 3. Recomputes edges based on new chunk content
    /// 4. Inserts new edges into the database
    ///
    /// # Arguments
    /// * `file_id` - Database ID of the file whose edges need updating
    ///
    /// # Returns
    /// * `Ok(())` - Edges updated successfully
    /// * `Err(_)` - Update failed (database error or computation error)
    ///
    /// # Performance
    ///
    /// Typical execution times:
    /// - Small files (<10 chunks): 10-50ms
    /// - Medium files (10-50 chunks): 50-200ms
    /// - Large files (50+ chunks): 200-500ms
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use maproom::db::create_pool;
    /// # use maproom::incremental::EdgeUpdater;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let pool = create_pool().await?;
    /// let updater = EdgeUpdater::new(pool);
    ///
    /// // Update edges after file modification
    /// updater.update_edges(123).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_edges(&self, file_id: i64) -> Result<()> {
        use crate::indexer::edges::{self, ChunkWithId};

        debug!(file_id = file_id, "Updating edges for file");

        // 1. Delete old edges for chunks in this file
        self.delete_edges_for_file(file_id).await?;

        // 2. Recompute edges
        // Get file metadata (relpath, language) and worktree root path
        let (relpath, language, root_path) = match self.store.get_file_edge_context(file_id).await?
        {
            Some(ctx) => ctx,
            None => {
                debug!(file_id = file_id, "File not found; skipping edge update");
                return Ok(());
            }
        };

        // Spec A1: single shared language gate (kept as a gate so we never
        // read unsupported files from disk on incremental events).
        let language = match language {
            Some(lang) if crate::indexer::edges::supports_call_extraction(lang.as_str()) => lang,
            _ => {
                // No edge extraction for this language
                debug!(
                    file_id = file_id,
                    "No edge extraction for language {:?}", language
                );
                return Ok(());
            }
        };

        // Read file content (join root path with relpath)
        let full_path = std::path::Path::new(&root_path).join(&relpath);
        let content = std::fs::read_to_string(&full_path).with_context(|| {
            format!(
                "Failed to read file: {} (root: {}, relpath: {})",
                full_path.display(),
                root_path,
                relpath
            )
        })?;

        // Load chunks for this file (map ChunkSummary -> ChunkWithId).
        let chunks_with_ids: Vec<ChunkWithId> = self
            .store
            .get_file_chunks(file_id)
            .await?
            .into_iter()
            .map(|c| ChunkWithId {
                id: c.id,
                symbol_name: c.symbol_name,
                kind: c.kind,
                start_line: c.start_line,
                end_line: c.end_line,
                file_id,
            })
            .collect();

        // Extract edges. This incremental path resolves only SAME-FILE calls; the
        // production watch path is `indexer::upsert_files`, which runs the store-backed
        // cross-file post-pass (spec B2). Unresolved cross-file refs are dropped here
        // (this updater is exercised only by tests).
        let (edges_to_insert, unresolved) =
            edges::extract_edges(&content, &language, &chunks_with_ids)?;
        if !unresolved.is_empty() {
            debug!(
                file_id = file_id,
                unresolved = unresolved.len(),
                "EdgeUpdater drops cross-file refs; use upsert_files for cross-file resolution"
            );
        }

        // Insert edges
        for edge in edges_to_insert {
            self.store
                .insert_chunk_edge(
                    edge.src_chunk_id,
                    edge.dst_chunk_id,
                    edge.edge_type.as_str(),
                )
                .await?;
        }

        debug!(file_id = file_id, "Edges updated for file");

        Ok(())
    }

    /// Delete all edges for chunks in a file.
    ///
    /// This is useful when a file is being removed or completely reindexed.
    ///
    /// # Arguments
    /// * `file_id` - Database ID of the file
    ///
    /// # Returns
    /// Number of edges deleted
    pub async fn delete_edges_for_file(&self, file_id: i64) -> Result<u64> {
        let count = self.store.delete_edges_for_file(file_id).await?;

        debug!(
            file_id = file_id,
            edges_deleted = count,
            "Deleted edges for file"
        );
        Ok(count)
    }
}

/// Represents a chunk edge relationship.
///
/// Public for use by edge extractor module (`crate::indexer::edges`).
#[derive(Debug, Clone)]
pub struct Edge {
    pub src_chunk_id: i64,
    pub dst_chunk_id: i64,
    pub edge_type: EdgeType,
}

/// Edge type enumeration.
///
/// Matches the database enum `maproom.edge_type`.
/// Public for use by edge extractor module (`crate::indexer::edges`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeType {
    Imports,
    Exports,
    Calls,
    CalledBy,
    TestOf,
    RouteOf,
}

impl EdgeType {
    /// Convert edge type to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Imports => "imports",
            EdgeType::Exports => "exports",
            EdgeType::Calls => "calls",
            EdgeType::CalledBy => "called_by",
            EdgeType::TestOf => "test_of",
            EdgeType::RouteOf => "route_of",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_type_as_str() {
        assert_eq!(EdgeType::Imports.as_str(), "imports");
        assert_eq!(EdgeType::Exports.as_str(), "exports");
        assert_eq!(EdgeType::Calls.as_str(), "calls");
        assert_eq!(EdgeType::CalledBy.as_str(), "called_by");
        assert_eq!(EdgeType::TestOf.as_str(), "test_of");
        assert_eq!(EdgeType::RouteOf.as_str(), "route_of");
    }
}
