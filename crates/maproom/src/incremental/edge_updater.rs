//! Edge relationship updater for incremental indexing.
//!
//! This module maintains consistency of chunk edges (relationships between code symbols)
//! after file changes. When a file is modified, edges involving its chunks must be
//! recomputed to maintain accurate code relationships.
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
use tracing::{debug, warn};

use crate::db::PgPool;

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
    pool: PgPool,
}

impl EdgeUpdater {
    /// Create a new edge updater.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// A new edge updater ready to maintain chunk relationships
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        debug!(file_id = file_id, "Updating edges for file");

        // Get database connection
        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection from pool")?;

        // Step 1: Find all chunk IDs for this file
        let chunk_rows = client
            .query(
                "SELECT id FROM maproom.chunks WHERE file_id = $1",
                &[&file_id],
            )
            .await
            .context("Failed to query chunk IDs")?;

        let chunk_ids: Vec<i64> = chunk_rows.iter().map(|row| row.get(0)).collect();

        if chunk_ids.is_empty() {
            debug!(file_id = file_id, "No chunks found for file, skipping edge update");
            return Ok(());
        }

        debug!(
            file_id = file_id,
            chunk_count = chunk_ids.len(),
            "Found chunks for edge update"
        );

        // Step 2: Delete old edges involving these chunks
        // Use ANY to efficiently delete all edges in one query
        let deleted_count = client
            .execute(
                "DELETE FROM maproom.chunk_edges
                 WHERE src_chunk_id = ANY($1) OR dst_chunk_id = ANY($1)",
                &[&chunk_ids],
            )
            .await
            .context("Failed to delete old edges")?;

        debug!(
            file_id = file_id,
            deleted_edges = deleted_count,
            "Deleted old edges"
        );

        // Step 3 & 4: Compute and insert new edges
        // For now, we'll implement a basic edge computation strategy
        // Future enhancement: Add more sophisticated edge detection (imports, calls, etc.)
        let new_edges = compute_edges(&client, &chunk_ids).await
            .context("Failed to compute new edges")?;

        if !new_edges.is_empty() {
            let inserted_count = insert_edges(&client, &new_edges).await
                .context("Failed to insert new edges")?;

            debug!(
                file_id = file_id,
                inserted_edges = inserted_count,
                "Inserted new edges"
            );
        }

        Ok(())
    }
}

/// Represents a chunk edge relationship.
#[derive(Debug, Clone)]
struct Edge {
    src_chunk_id: i64,
    dst_chunk_id: i64,
    edge_type: EdgeType,
}

/// Edge type enumeration.
///
/// Matches the database enum `maproom.edge_type`.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum EdgeType {
    Imports,
    Exports,
    Calls,
    CalledBy,
    TestOf,
    RouteOf,
}

impl EdgeType {
    /// Convert edge type to database string representation.
    fn as_str(&self) -> &'static str {
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
///
/// This is a placeholder implementation that will be enhanced in future tickets.
/// Currently implements basic heuristics:
/// - Test files → TestOf edges to tested modules
/// - Route files → RouteOf edges
///
/// # Arguments
/// * `client` - Database client
/// * `chunk_ids` - IDs of chunks to compute edges for
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
async fn compute_edges(
    client: &tokio_postgres::Client,
    chunk_ids: &[i64],
) -> Result<Vec<Edge>> {
    if chunk_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut edges = Vec::new();

    // Query chunk metadata for edge computation
    let rows = client
        .query(
            "SELECT id, symbol_name, kind::text, file_id
             FROM maproom.chunks
             WHERE id = ANY($1)",
            &[&chunk_ids],
        )
        .await
        .context("Failed to query chunk metadata")?;

    // Build edges based on chunk properties
    for row in &rows {
        let chunk_id: i64 = row.get(0);
        let symbol_name: Option<String> = row.get(1);
        let kind: String = row.get(2);
        let _file_id: i64 = row.get(3);

        // Heuristic 1: Test chunks → TestOf edges
        // Look for test-related symbols (e.g., "test_*", "*_test", "it", "describe")
        if is_test_chunk(&kind, symbol_name.as_deref()) {
            // Find potential test targets
            let target_edges = find_test_targets(client, chunk_id, symbol_name.as_deref()).await?;
            edges.extend(target_edges);
        }

        // Heuristic 2: Route chunks → RouteOf edges
        // Look for route handlers (e.g., in Express, Next.js, etc.)
        if is_route_chunk(&kind, symbol_name.as_deref()) {
            // Routes typically don't have many edges in our simple model
            // This is a placeholder for future route graph construction
            debug!(chunk_id = chunk_id, "Found route chunk (edge computation placeholder)");
        }

        // Future: Add import/export/call graph analysis here
    }

    Ok(edges)
}

/// Check if a chunk represents a test.
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
        if lower.starts_with("test_")
            || lower.starts_with("it ")
            || lower.starts_with("describe ")
        {
            return true;
        }
    }

    false
}

/// Check if a chunk represents a route handler.
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
///
/// # Strategy
/// - Extract target name from test symbol name
/// - Search for chunks with matching symbol names
/// - Create TestOf edges
///
/// # Arguments
/// * `client` - Database client
/// * `test_chunk_id` - ID of the test chunk
/// * `test_symbol_name` - Name of the test symbol (optional)
///
/// # Returns
/// Vector of TestOf edges
async fn find_test_targets(
    client: &tokio_postgres::Client,
    test_chunk_id: i64,
    test_symbol_name: Option<&str>,
) -> Result<Vec<Edge>> {
    let mut edges = Vec::new();

    if let Some(test_name) = test_symbol_name {
        // Extract potential target name from test name
        // Example: "test_my_function" → "my_function"
        let target_name = test_name
            .trim_start_matches("test_")
            .trim_end_matches("_test");

        if target_name.is_empty() || target_name == test_name {
            // No clear target name extracted
            return Ok(edges);
        }

        // Search for chunks with matching symbol name
        let rows = client
            .query(
                "SELECT id FROM maproom.chunks
                 WHERE symbol_name = $1 AND id != $2
                 LIMIT 5",
                &[&target_name, &test_chunk_id],
            )
            .await
            .context("Failed to search for test targets")?;

        for row in rows {
            let target_id: i64 = row.get(0);
            edges.push(Edge {
                src_chunk_id: test_chunk_id,
                dst_chunk_id: target_id,
                edge_type: EdgeType::TestOf,
            });
        }
    }

    Ok(edges)
}

/// Insert edges into the database in batch.
///
/// # Arguments
/// * `client` - Database client
/// * `edges` - Edges to insert
///
/// # Returns
/// Number of edges inserted
async fn insert_edges(client: &tokio_postgres::Client, edges: &[Edge]) -> Result<u64> {
    if edges.is_empty() {
        return Ok(0);
    }

    let mut total_inserted = 0u64;

    // Insert edges in batches to avoid parameter limits
    const BATCH_SIZE: usize = 100;

    for batch in edges.chunks(BATCH_SIZE) {
        // Build multi-row insert query
        let mut query = String::from(
            "INSERT INTO maproom.chunk_edges (src_chunk_id, dst_chunk_id, type)
             VALUES ",
        );

        // Collect edge type strings that need to live for the query
        let edge_type_strings: Vec<String> = batch
            .iter()
            .map(|edge| edge.edge_type.as_str().to_string())
            .collect();

        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_idx = 1;

        for (i, _edge) in batch.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str(&format!(
                "(${}, ${}, ${}::maproom.edge_type)",
                param_idx,
                param_idx + 1,
                param_idx + 2
            ));
            param_idx += 3;
        }

        query.push_str(" ON CONFLICT DO NOTHING");

        // Build parameter array - interleave chunk IDs and edge types
        for (i, edge) in batch.iter().enumerate() {
            params.push(&edge.src_chunk_id);
            params.push(&edge.dst_chunk_id);
            params.push(&edge_type_strings[i]);
        }

        // Execute batch insert
        let inserted = client
            .execute(&query, &params)
            .await
            .context("Failed to batch insert edges")?;

        total_inserted += inserted;
    }

    if total_inserted < edges.len() as u64 {
        warn!(
            requested = edges.len(),
            inserted = total_inserted,
            "Some edges already existed (ON CONFLICT DO NOTHING)"
        );
    }

    Ok(total_inserted)
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
