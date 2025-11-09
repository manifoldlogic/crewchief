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

use super::graph::{find_related_chunks_directional, EdgeType, RelatedChunk};
use anyhow::{Context as AnyhowContext, Result};
use tokio_postgres::Client;

/// Find test files that test the given chunk.
///
/// This function looks for test_of edges pointing to the target chunk.
/// It uses both the chunk_edges table and the test_links table (which may
/// have precomputed test relationships for performance).
///
/// # Arguments
/// * `client` - PostgreSQL client
/// * `chunk_id` - Implementation chunk to find tests for
///
/// # Returns
/// Vector of test chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let tests = find_test_files(&client, 1234).await?;
/// for test in tests {
///     println!("Test: {} in {}", test.symbol_name.unwrap_or_default(), test.relpath);
/// }
/// ```
pub async fn find_test_files(client: &Client, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // Use test_links table for direct test relationships
    let query = r#"
        SELECT DISTINCT
          c.id,
          f.relpath,
          c.symbol_name,
          c.kind::text,
          c.start_line,
          c.end_line,
          c.preview,
          0 as depth,
          1.0 as relevance
        FROM maproom.test_links tl
        JOIN maproom.chunks c ON c.id = tl.test_chunk_id
        JOIN maproom.files f ON f.id = c.file_id
        WHERE tl.target_chunk_id = $1
        ORDER BY relevance DESC;
    "#;

    let rows = client
        .query(query, &[&chunk_id])
        .await
        .context("Failed to query test_links table")?;

    let mut tests: Vec<RelatedChunk> = rows
        .into_iter()
        .map(|row| RelatedChunk {
            id: row.get(0),
            relpath: row.get(1),
            symbol_name: row.get(2),
            kind: row.get(3),
            start_line: row.get(4),
            end_line: row.get(5),
            preview: row.get(6),
            depth: row.get(7),
            relevance: row.get(8),
        })
        .collect();

    // Also check chunk_edges for test_of relationships
    // (in case edge extraction found tests that aren't in test_links yet)
    let edge_tests = find_related_chunks_directional(
        client,
        chunk_id,
        1, // Only direct tests (depth 1)
        Some(vec![EdgeType::TestOf]),
        false, // Backward: find chunks where dst_chunk_id = chunk_id
    )
    .await?;

    // Merge results, avoiding duplicates
    for edge_test in edge_tests {
        if !tests.iter().any(|t| t.id == edge_test.id) {
            tests.push(edge_test);
        }
    }

    // Re-sort by relevance
    tests.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

    Ok(tests)
}

/// Find callers of the given chunk (what calls this function/method).
///
/// This follows 'called_by' edges backward or 'calls' edges backward
/// to find chunks that invoke the target chunk.
///
/// # Arguments
/// * `client` - PostgreSQL client
/// * `chunk_id` - Chunk to find callers for
/// * `max_depth` - Maximum traversal depth (typically 2-3)
///
/// # Returns
/// Vector of caller chunks ordered by relevance (closest callers first)
///
/// # Example
/// ```ignore
/// let callers = find_callers(&client, 1234, 2).await?;
/// for caller in callers {
///     println!("Called by: {} (depth {})", caller.symbol_name.unwrap_or_default(), caller.depth);
/// }
/// ```
pub async fn find_callers(
    client: &Client,
    chunk_id: i64,
    max_depth: i32,
) -> Result<Vec<RelatedChunk>> {
    // Find chunks where this chunk is the destination of a 'calls' edge
    // This means: other chunks call this chunk
    find_related_chunks_directional(
        client,
        chunk_id,
        max_depth,
        Some(vec![EdgeType::Calls]),
        false, // Backward: find src where dst = this chunk
    )
    .await
}

/// Find callees of the given chunk (what this function/method calls).
///
/// This follows 'calls' edges forward to find chunks that are invoked
/// by the target chunk.
///
/// # Arguments
/// * `client` - PostgreSQL client
/// * `chunk_id` - Chunk to find callees for
/// * `max_depth` - Maximum traversal depth (typically 2-3)
///
/// # Returns
/// Vector of callee chunks ordered by relevance (direct callees first)
///
/// # Example
/// ```ignore
/// let callees = find_callees(&client, 1234, 2).await?;
/// for callee in callees {
///     println!("Calls: {} (depth {})", callee.symbol_name.unwrap_or_default(), callee.depth);
/// }
/// ```
pub async fn find_callees(
    client: &Client,
    chunk_id: i64,
    max_depth: i32,
) -> Result<Vec<RelatedChunk>> {
    // Find chunks where this chunk is the source of a 'calls' edge
    // This means: this chunk calls other chunks
    find_related_chunks_directional(
        client,
        chunk_id,
        max_depth,
        Some(vec![EdgeType::Calls]),
        true, // Forward: find dst where src = this chunk
    )
    .await
}

/// Find imports (dependencies) of the given chunk.
///
/// This follows 'imports' edges forward to find modules/files that
/// the target chunk depends on.
///
/// # Arguments
/// * `client` - PostgreSQL client
/// * `chunk_id` - Chunk to find imports for
///
/// # Returns
/// Vector of imported chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let imports = find_imports(&client, 1234).await?;
/// for import in imports {
///     println!("Imports: {} from {}", import.symbol_name.unwrap_or_default(), import.relpath);
/// }
/// ```
pub async fn find_imports(client: &Client, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // Find direct imports only (depth 1)
    find_related_chunks_directional(
        client,
        chunk_id,
        1, // Only direct imports
        Some(vec![EdgeType::Imports]),
        true, // Forward: find what this imports
    )
    .await
}

/// Find exports (what exports this chunk).
///
/// This follows 'exports' edges backward to find modules/files that
/// export the target chunk.
///
/// # Arguments
/// * `client` - PostgreSQL client
/// * `chunk_id` - Chunk to find exports for
///
/// # Returns
/// Vector of exporting chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let exports = find_exports(&client, 1234).await?;
/// for export in exports {
///     println!("Exported by: {}", export.relpath);
/// }
/// ```
pub async fn find_exports(client: &Client, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // Find direct exports only (depth 1)
    find_related_chunks_directional(
        client,
        chunk_id,
        1, // Only direct exports
        Some(vec![EdgeType::Exports]),
        false, // Backward: find what exports this
    )
    .await
}

/// Find route definitions that use the given component chunk.
///
/// This is specific to web frameworks (React, Vue, etc.) where routes
/// reference component chunks.
///
/// # Arguments
/// * `client` - PostgreSQL client
/// * `chunk_id` - Component chunk to find routes for
///
/// # Returns
/// Vector of route chunks ordered by relevance
///
/// # Example
/// ```ignore
/// let routes = find_routes(&client, 1234).await?;
/// for route in routes {
///     println!("Route: {} in {}", route.symbol_name.unwrap_or_default(), route.relpath);
/// }
/// ```
pub async fn find_routes(client: &Client, chunk_id: i64) -> Result<Vec<RelatedChunk>> {
    // Find routes that reference this component (backward edge)
    find_related_chunks_directional(
        client,
        chunk_id,
        1, // Only direct routes
        Some(vec![EdgeType::RouteOf]),
        false, // Backward: find routes where dst = this component
    )
    .await
}

/// Find all relationship types for a chunk (comprehensive).
///
/// This is a convenience function that queries all relationship types
/// and returns them organized by category.
///
/// # Arguments
/// * `client` - PostgreSQL client
/// * `chunk_id` - Chunk to analyze
/// * `max_depth` - Maximum depth for caller/callee traversal
///
/// # Returns
/// Tuple of (tests, callers, callees, imports, exports, routes)
pub async fn find_all_relationships(
    client: &Client,
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
        find_test_files(client, chunk_id),
        find_callers(client, chunk_id, max_depth),
        find_callees(client, chunk_id, max_depth),
        find_imports(client, chunk_id),
        find_exports(client, chunk_id),
        find_routes(client, chunk_id),
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
