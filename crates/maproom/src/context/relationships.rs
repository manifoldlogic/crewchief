//! Relationship-specific query functions.
//!
//! This module provides high-level functions for common relationship queries:
//! - Finding test files for implementation chunks
//! - Finding callers (what calls this)
//! - Finding callees (what this calls)
//! - Finding imports (dependencies)
//!
//! These functions build on the core graph traversal in graph.rs but provide
//! semantic meaning and specialized handling for each relationship type.

use super::graph::{EdgeType, RelatedChunk};
use crate::db::sqlite::graph::ImportDirection;
use crate::db::SqliteStore;
use anyhow::{Context as AnyhowContext, Result};

/// Find test files that test the given chunk.
///
/// This function looks for test_of edges pointing to the target chunk.
///
/// # Arguments
/// * `store` - SqliteStore
/// * `chunk_id` - Implementation chunk to find tests for
///
/// # Returns
/// Vector of test chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let tests = find_test_files(&store, 1234).await?;
/// for test in tests {
///     println!("Test: {} in {}", test.symbol_name.unwrap_or_default(), test.relpath);
/// }
/// ```
pub async fn find_test_files(store: &SqliteStore, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // SQLite doesn't have test_links table, so we query chunk_edges directly
    // Look for edges where type='test_of' and dst_chunk_id = chunk_id
    store.run(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT
              c.id,
              f.relpath,
              c.symbol_name,
              c.kind,
              c.start_line,
              c.end_line,
              c.preview,
              0 as depth,
              1.0 as relevance
            FROM chunk_edges e
            JOIN chunks c ON c.id = e.src_chunk_id
            JOIN files f ON f.id = c.file_id
            WHERE e.dst_chunk_id = ?1 AND e.type = 'test_of'
            ORDER BY relevance DESC"
        )?;

        let rows = stmt.query_map(rusqlite::params![chunk_id], |row| {
            Ok(RelatedChunk {
                id: row.get(0)?,
                relpath: row.get(1)?,
                symbol_name: row.get(2)?,
                kind: row.get(3)?,
                start_line: row.get(4)?,
                end_line: row.get(5)?,
                preview: row.get(6)?,
                depth: row.get(7)?,
                relevance: row.get(8)?,
            })
        })?;

        let mut tests = Vec::new();
        for test_result in rows {
            tests.push(test_result?);
        }

        Ok(tests)
    }).await.context("Failed to find test files")
}

/// Find callers of the given chunk (what calls this function/method).
///
/// This follows 'calls' edges backward to find chunks that invoke the target chunk.
///
/// # Arguments
/// * `store` - SqliteStore
/// * `chunk_id` - Chunk to find callers for
/// * `max_depth` - Maximum traversal depth (typically 2-3)
///
/// # Returns
/// Vector of caller chunks ordered by relevance (closest callers first)
///
/// # Example
/// ```ignore
/// let callers = find_callers(&store, 1234, 2).await?;
/// for caller in callers {
///     println!("Called by: {} (depth {})", caller.symbol_name.unwrap_or_default(), caller.depth);
/// }
/// ```
pub async fn find_callers(
    store: &SqliteStore,
    chunk_id: i64,
    max_depth: i32,
) -> Result<Vec<RelatedChunk>> {
    // Use SqliteStore's find_callers method
    let graph_results = store.find_callers(chunk_id, Some(max_depth as usize)).await?;

    // Convert GraphResult to RelatedChunk
    let mut related_chunks = Vec::new();
    for result in graph_results {
        // Get chunk details for each caller
        if let Some(chunk) = store.get_chunk_by_id(result.chunk_id).await? {
            related_chunks.push(RelatedChunk {
                id: chunk.id,
                relpath: chunk.file_path,
                symbol_name: chunk.symbol_name,
                kind: chunk.kind,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                preview: chunk.preview,
                depth: result.depth as i32,
                relevance: 0.7_f64.powi(result.depth as i32), // Decay by 0.7 per hop
            });
        }
    }

    Ok(related_chunks)
}

/// Find callees of the given chunk (what this function/method calls).
///
/// This follows 'calls' edges forward to find chunks that are invoked
/// by the target chunk.
///
/// # Arguments
/// * `store` - SqliteStore
/// * `chunk_id` - Chunk to find callees for
/// * `max_depth` - Maximum traversal depth (typically 2-3)
///
/// # Returns
/// Vector of callee chunks ordered by relevance (direct callees first)
///
/// # Example
/// ```ignore
/// let callees = find_callees(&store, 1234, 2).await?;
/// for callee in callees {
///     println!("Calls: {} (depth {})", callee.symbol_name.unwrap_or_default(), callee.depth);
/// }
/// ```
pub async fn find_callees(
    store: &SqliteStore,
    chunk_id: i64,
    max_depth: i32,
) -> Result<Vec<RelatedChunk>> {
    // Use SqliteStore's find_callees method
    let graph_results = store.find_callees(chunk_id, Some(max_depth as usize)).await?;

    // Convert GraphResult to RelatedChunk
    let mut related_chunks = Vec::new();
    for result in graph_results {
        // Get chunk details for each callee
        if let Some(chunk) = store.get_chunk_by_id(result.chunk_id).await? {
            related_chunks.push(RelatedChunk {
                id: chunk.id,
                relpath: chunk.file_path,
                symbol_name: chunk.symbol_name,
                kind: chunk.kind,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                preview: chunk.preview,
                depth: result.depth as i32,
                relevance: 0.7_f64.powi(result.depth as i32), // Decay by 0.7 per hop
            });
        }
    }

    Ok(related_chunks)
}

/// Find imports (dependencies) of the given chunk.
///
/// This follows 'imports' edges forward to find modules/files that
/// the target chunk depends on.
///
/// # Arguments
/// * `store` - SqliteStore
/// * `chunk_id` - Chunk to find imports for
///
/// # Returns
/// Vector of imported chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let imports = find_imports(&store, 1234).await?;
/// for import in imports {
///     println!("Imports: {} from {}", import.symbol_name.unwrap_or_default(), import.relpath);
/// }
/// ```
pub async fn find_imports(store: &SqliteStore, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // Use SqliteStore's find_imports method (outgoing imports)
    let graph_results = store.find_imports(chunk_id, ImportDirection::Outgoing, Some(1)).await?;

    // Convert GraphResult to RelatedChunk
    let mut related_chunks = Vec::new();
    for result in graph_results {
        if let Some(chunk) = store.get_chunk_by_id(result.chunk_id).await? {
            related_chunks.push(RelatedChunk {
                id: chunk.id,
                relpath: chunk.file_path,
                symbol_name: chunk.symbol_name,
                kind: chunk.kind,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                preview: chunk.preview,
                depth: result.depth as i32,
                relevance: 1.0, // Direct imports have full relevance
            });
        }
    }

    Ok(related_chunks)
}

/// Find exports (what exports this chunk).
///
/// This follows 'exports' edges to find modules/files that
/// export the target chunk.
///
/// # Arguments
/// * `store` - SqliteStore
/// * `chunk_id` - Chunk to find exports for
///
/// # Returns
/// Vector of exporting chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let exports = find_exports(&store, 1234).await?;
/// for export in exports {
///     println!("Exported by: {}", export.relpath);
/// }
/// ```
pub async fn find_exports(store: &SqliteStore, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // SQLite doesn't have a specific exports edge type in graph module
    // Query chunk_edges directly for 'exports' edges
    store.run(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT
              c.id,
              f.relpath,
              c.symbol_name,
              c.kind,
              c.start_line,
              c.end_line,
              c.preview
            FROM chunk_edges e
            JOIN chunks c ON c.id = e.src_chunk_id
            JOIN files f ON f.id = c.file_id
            WHERE e.dst_chunk_id = ?1 AND e.type = 'exports'"
        )?;

        let rows = stmt.query_map(rusqlite::params![chunk_id], |row| {
            Ok(RelatedChunk {
                id: row.get(0)?,
                relpath: row.get(1)?,
                symbol_name: row.get(2)?,
                kind: row.get(3)?,
                start_line: row.get(4)?,
                end_line: row.get(5)?,
                preview: row.get(6)?,
                depth: 1,
                relevance: 1.0,
            })
        })?;

        let mut exports = Vec::new();
        for export_result in rows {
            exports.push(export_result?);
        }

        Ok(exports)
    }).await.context("Failed to find exports")
}

/// Find route definitions that use the given component chunk.
///
/// This is specific to web frameworks (React, Vue, etc.) where routes
/// reference component chunks.
///
/// # Arguments
/// * `store` - SqliteStore
/// * `chunk_id` - Component chunk to find routes for
///
/// # Returns
/// Vector of route chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let routes = find_routes(&store, 1234).await?;
/// for route in routes {
///     println!("Route: {} in {}", route.symbol_name.unwrap_or_default(), route.relpath);
/// }
/// ```
pub async fn find_routes(store: &SqliteStore, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // Query chunk_edges directly for 'route_of' edges
    store.run(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT
              c.id,
              f.relpath,
              c.symbol_name,
              c.kind,
              c.start_line,
              c.end_line,
              c.preview
            FROM chunk_edges e
            JOIN chunks c ON c.id = e.src_chunk_id
            JOIN files f ON f.id = c.file_id
            WHERE e.dst_chunk_id = ?1 AND e.type = 'route_of'"
        )?;

        let rows = stmt.query_map(rusqlite::params![chunk_id], |row| {
            Ok(RelatedChunk {
                id: row.get(0)?,
                relpath: row.get(1)?,
                symbol_name: row.get(2)?,
                kind: row.get(3)?,
                start_line: row.get(4)?,
                end_line: row.get(5)?,
                preview: row.get(6)?,
                depth: 1,
                relevance: 1.0,
            })
        })?;

        let mut routes = Vec::new();
        for route_result in rows {
            routes.push(route_result?);
        }

        Ok(routes)
    }).await.context("Failed to find routes")
}

/// Find all relationship types for a chunk (comprehensive).
///
/// This is a convenience function that queries all relationship types
/// and returns them organized by category.
///
/// # Arguments
/// * `store` - SqliteStore
/// * `chunk_id` - Chunk to analyze
/// * `max_depth` - Maximum depth for caller/callee traversal
///
/// # Returns
/// Tuple of (tests, callers, callees, imports, exports, routes)
pub async fn find_all_relationships(
    store: &SqliteStore,
    chunk_id: i64,
    max_depth: i32,
) -> Result<(
    Vec<RelatedChunk>, // tests
    Vec<RelatedChunk>, // callers
    Vec<RelatedChunk>, // callees
    Vec<RelatedChunk>, // imports
    Vec<RelatedChunk>, // exports
    Vec<RelatedChunk>, // routes
)> {
    // Execute all queries in parallel for performance
    let (tests, callers, callees, imports, exports, routes) = tokio::try_join!(
        find_test_files(store, chunk_id),
        find_callers(store, chunk_id, max_depth),
        find_callees(store, chunk_id, max_depth),
        find_imports(store, chunk_id),
        find_exports(store, chunk_id),
        find_routes(store, chunk_id),
    )?;

    Ok((tests, callers, callees, imports, exports, routes))
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    // Unit tests for relationship logic
    // Integration tests with real database are in tests/ directory

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_find_test_files() {
        // This is a placeholder for integration testing
        // Real tests go in tests/context/relationship_test.rs
    }

    #[tokio::test]
    #[ignore]
    async fn test_find_callers() {
        // Placeholder
    }

    #[tokio::test]
    #[ignore]
    async fn test_find_callees() {
        // Placeholder
    }
}
