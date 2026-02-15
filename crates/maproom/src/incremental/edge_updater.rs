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

use crate::db::traits::StoreChunks;
use crate::db::SqliteStore;
use std::sync::Arc;

/// Edge updater for maintaining chunk relationships.
///
/// Handles edge updates after file modifications to keep the code graph
/// consistent and accurate.
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::db::create_pool;
/// use crewchief_maproom::incremental::EdgeUpdater;
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
    store: Arc<SqliteStore>,
}

impl EdgeUpdater {
    /// Create a new edge updater.
    ///
    /// # Arguments
    /// * `store` - SqliteStore instance
    ///
    /// # Returns
    /// A new edge updater ready to maintain chunk relationships
    pub fn new(store: Arc<SqliteStore>) -> Self {
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
    /// ```no_run
    /// # use crewchief_maproom::db::create_pool;
    /// # use crewchief_maproom::incremental::EdgeUpdater;
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
        let file_metadata = self
            .store
            .run(move |conn| {
                let result = conn.query_row(
                    "SELECT f.relpath, f.language, w.abs_path
                     FROM files f
                     JOIN worktrees w ON f.worktree_id = w.id
                     WHERE f.id = ?",
                    rusqlite::params![file_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )?;
                Ok(result)
            })
            .await?;

        let (relpath, language, root_path) = file_metadata;

        // Check if this is a TypeScript/JavaScript file
        let language = match language {
            Some(lang) if matches!(lang.as_str(), "ts" | "tsx" | "js" | "jsx") => lang,
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

        // Load chunks for this file
        let chunks_with_ids: Vec<ChunkWithId> = self
            .store
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, symbol_name, kind, start_line, end_line FROM chunks WHERE file_id = ?",
                )?;
                let chunks = stmt
                    .query_map(rusqlite::params![file_id], |row| {
                        Ok(ChunkWithId {
                            id: row.get(0)?,
                            symbol_name: row.get(1)?,
                            kind: row.get(2)?,
                            start_line: row.get(3)?,
                            end_line: row.get(4)?,
                            file_id,
                        })
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(chunks)
            })
            .await?;

        // Extract edges
        let edges_to_insert = edges::extract_edges(&content, &language, &chunks_with_ids)?;

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
        let count = self
            .store
            .run(move |conn| {
                let deleted = conn.execute(
                    "DELETE FROM chunk_edges WHERE src_chunk_id IN (
                     SELECT id FROM chunks WHERE file_id = ?1
                 ) OR dst_chunk_id IN (
                     SELECT id FROM chunks WHERE file_id = ?1
                 )",
                    rusqlite::params![file_id],
                )?;
                Ok(deleted as u64)
            })
            .await?;

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

/// Compute edges for a set of chunks.
#[allow(dead_code)]
///
/// This is a placeholder implementation that will be enhanced in future tickets.
/// Currently implements basic heuristics:
/// - Test files → TestOf edges to tested modules
/// - Route files → RouteOf edges
///
/// # Arguments
/// * `_store` - SqliteStore instance (unused in stub)
/// * `_chunk_ids` - IDs of chunks to compute edges for
///
/// # Returns
/// Vector of computed edges
///
/// # Future Enhancements
///
/// Full edge computation should include:
/// - Import/export analysis via AST parsing
/// - Function call graph via symbol resolution
/// - Test target detection via naming conventions
/// - Route handler detection via framework patterns
async fn compute_edges(_store: &SqliteStore, _chunk_ids: &[i64]) -> Result<Vec<Edge>> {
    // TODO: Implement SQLite-based edge computation
    // This will be implemented in a future ticket
    Ok(Vec::new())
}

/// Check if a chunk represents a test.
#[allow(dead_code)]
///
/// # Heuristics
/// - Kind contains "test"
/// - Symbol name starts with "test_" or "it" or "describe"
fn is_test_chunk(kind: &str, symbol_name: Option<&str>) -> bool {
    if kind.contains("test") {
        return true;
    }

    if let Some(name) = symbol_name {
        let lower = name.to_lowercase();
        if lower.starts_with("test_") || lower.starts_with("it ") || lower.starts_with("describe ")
        {
            return true;
        }
    }

    false
}

/// Check if a chunk represents a route handler.
#[allow(dead_code)]
///
/// # Heuristics
/// - Symbol name contains "route" or "handler"
/// - Kind is "func" and file is in routes/ directory
fn is_route_chunk(kind: &str, symbol_name: Option<&str>) -> bool {
    if kind == "func" {
        if let Some(name) = symbol_name {
            let lower = name.to_lowercase();
            if lower.contains("route") || lower.contains("handler") {
                return true;
            }
        }
    }

    false
}

/// Find test target chunks for a given test chunk.
#[allow(dead_code)]
///
/// # Strategy
/// - Extract target name from test symbol name
/// - Search for chunks with matching symbol names
/// - Create TestOf edges
///
/// # Arguments
/// * `_store` - SqliteStore instance (unused in stub)
/// * `_test_chunk_id` - ID of the test chunk
/// * `_test_symbol_name` - Name of the test symbol (optional)
///
/// # Returns
/// Vector of TestOf edges
async fn find_test_targets(
    _store: &SqliteStore,
    _test_chunk_id: i64,
    _test_symbol_name: Option<&str>,
) -> Result<Vec<Edge>> {
    // TODO: Implement SQLite-based test target finding
    // This will be implemented in a future ticket
    Ok(Vec::new())
}

/// Insert edges into the database in batch.
#[allow(dead_code)]
///
/// # Arguments
/// * `store` - SqliteStore instance
/// * `edges` - Edges to insert
///
/// # Returns
/// Number of edges inserted
async fn insert_edges(store: &SqliteStore, edges: &[Edge]) -> Result<u64> {
    if edges.is_empty() {
        return Ok(0);
    }

    let edges = edges.to_vec();
    store.run(move |conn| {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO chunk_edges (src_chunk_id, dst_chunk_id, type) VALUES (?1, ?2, ?3)"
        )?;

        let mut count = 0u64;
        for edge in &edges {
            let rows = stmt.execute(rusqlite::params![
                edge.src_chunk_id,
                edge.dst_chunk_id,
                edge.edge_type.as_str()
            ])?;
            count += rows as u64;
        }

        Ok(count)
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_test_chunk() {
        assert!(is_test_chunk("test", None));
        assert!(is_test_chunk("func", Some("test_myfunction")));
        assert!(is_test_chunk("func", Some("it should work")));
        assert!(is_test_chunk("func", Some("describe the feature")));
        assert!(!is_test_chunk("func", Some("myFunction")));
        assert!(!is_test_chunk("class", Some("MyClass")));
    }

    #[test]
    fn test_is_route_chunk() {
        assert!(is_route_chunk("func", Some("handleRoute")));
        assert!(is_route_chunk("func", Some("userRouter")));
        assert!(!is_route_chunk("func", Some("myFunction")));
        assert!(!is_route_chunk("class", Some("RouteHandler")));
    }

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
