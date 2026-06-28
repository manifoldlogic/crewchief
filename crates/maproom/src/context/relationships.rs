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

use super::graph::RelatedChunk;
use crate::db::{ImportDirection, Store};
use anyhow::Result;

/// Build `RelatedChunk`s for chunks connected to `chunk_id` by an incoming edge of
/// `edge_type` (e.g. `test_of`/`exports`/`route_of`). Mirrors the old per-type raw
/// joins via the `get_direct_edges` + `get_chunk_by_id` trait methods, so it works
/// on any backend.
async fn related_via_incoming_edge(
    store: &(dyn Store + Send + Sync),
    chunk_id: i64,
    edge_type: &str,
    depth: i32,
) -> Result<Vec<RelatedChunk>> {
    let edges = store
        .get_direct_edges(chunk_id, ImportDirection::Incoming)
        .await?;
    let mut out = Vec::new();
    for e in edges.into_iter().filter(|e| e.edge_type == edge_type) {
        if let Some(chunk) = store.get_chunk_by_id(e.chunk_id).await? {
            out.push(RelatedChunk {
                id: chunk.id,
                relpath: chunk.file_path,
                symbol_name: chunk.symbol_name,
                kind: chunk.kind,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                preview: chunk.preview,
                depth,
                relevance: 1.0,
            });
        }
    }
    Ok(out)
}

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
pub async fn find_test_files(
    store: &(dyn Store + Send + Sync),
    chunk_id: i64,
) -> Result<Vec<RelatedChunk>> {
    // Chunks linked to the target by a 'test_of' edge (depth 0 per the legacy query).
    related_via_incoming_edge(store, chunk_id, "test_of", 0).await
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
    store: &(dyn Store + Send + Sync),
    chunk_id: i64,
    max_depth: i32,
) -> Result<Vec<RelatedChunk>> {
    // Use SqliteStore's find_callers method
    let graph_results = store
        .find_callers(chunk_id, Some(max_depth as usize))
        .await?;

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
    store: &(dyn Store + Send + Sync),
    chunk_id: i64,
    max_depth: i32,
) -> Result<Vec<RelatedChunk>> {
    // Use SqliteStore's find_callees method
    let graph_results = store
        .find_callees(chunk_id, Some(max_depth as usize))
        .await?;

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
pub async fn find_imports(
    store: &(dyn Store + Send + Sync),
    chunk_id: i64,
) -> Result<Vec<RelatedChunk>> {
    // Use SqliteStore's find_imports method (outgoing imports)
    let graph_results = store
        .find_imports(chunk_id, ImportDirection::Outgoing, Some(1))
        .await?;

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
pub async fn find_exports(
    store: &(dyn Store + Send + Sync),
    chunk_id: i64,
) -> Result<Vec<RelatedChunk>> {
    related_via_incoming_edge(store, chunk_id, "exports", 1).await
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
pub async fn find_routes(
    store: &(dyn Store + Send + Sync),
    chunk_id: i64,
) -> Result<Vec<RelatedChunk>> {
    related_via_incoming_edge(store, chunk_id, "route_of", 1).await
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
    store: &(dyn Store + Send + Sync),
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
