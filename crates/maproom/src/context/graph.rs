//! Graph traversal queries for code relationships.
//!
//! This module implements recursive graph traversal using SQLite CTEs
//! to discover code relationships through chunk_edges. It supports:
//! - Bidirectional edge traversal
//! - Depth limiting to prevent unbounded queries
//! - Relevance decay (0.7 per hop)
//! - Multiple relationship types filtering

use anyhow::{Context as AnyhowContext, Result};
use crate::db::SqliteStore;

/// Represents a related chunk discovered through graph traversal
#[derive(Debug, Clone)]
pub struct RelatedChunk {
    /// Chunk ID
    pub id: i64,
    /// File relative path
    pub relpath: String,
    /// Symbol name (function, class, etc.)
    pub symbol_name: Option<String>,
    /// Symbol kind
    pub kind: String,
    /// Start line in file
    pub start_line: i32,
    /// End line in file
    pub end_line: i32,
    /// Content preview
    pub preview: String,
    /// Depth from source chunk (0 = source itself)
    pub depth: i32,
    /// Relevance score (decays by 0.7 per hop)
    pub relevance: f64,
}

/// Edge types for filtering graph traversal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeType {
    /// Module imports
    Imports,
    /// Module exports
    Exports,
    /// Function/method calls
    Calls,
    /// Called by relationship
    CalledBy,
    /// Test-to-implementation link
    TestOf,
    /// Route-to-component link
    RouteOf,
}

impl EdgeType {
    /// Convert to database enum value
    pub fn as_db_str(&self) -> &'static str {
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

/// Find related chunks through graph traversal with configurable depth and edge types.
///
/// This is the core graph traversal query using recursive CTEs. It follows edges
/// bidirectionally (both src_chunk_id and dst_chunk_id) and applies relevance decay.
///
/// # Arguments
/// * `store` - SQLite store
/// * `chunk_id` - Starting chunk ID
/// * `max_depth` - Maximum traversal depth (prevents unbounded queries)
/// * `edge_types` - Optional filter for edge types (if None, all types are included)
///
/// # Returns
/// Vector of related chunks ordered by relevance score (highest first)
///
/// # Example
/// ```ignore
/// let related = find_related_chunks(&store, 1234, 3, Some(vec![EdgeType::Calls])).await?;
/// for chunk in related {
///     println!("Found: {} at depth {} (relevance: {:.2})",
///              chunk.symbol_name.unwrap_or_default(), chunk.depth, chunk.relevance);
/// }
/// ```
pub async fn find_related_chunks(
    store: &SqliteStore,
    chunk_id: i64,
    max_depth: i32,
    edge_types: Option<Vec<EdgeType>>,
) -> Result<Vec<RelatedChunk>> {
    // TODO: Implement using SqliteStore graph methods in IDXABS-4001
    Ok(vec![])
}

/// Find related chunks in a specific direction (forward or backward).
///
/// This is a directional variant that only follows edges in one direction:
/// - Forward: src_chunk_id = current_id (finds what this chunk references)
/// - Backward: dst_chunk_id = current_id (finds what references this chunk)
///
/// # Arguments
/// * `store` - SQLite store
/// * `chunk_id` - Starting chunk ID
/// * `max_depth` - Maximum traversal depth
/// * `edge_types` - Optional filter for edge types
/// * `forward` - If true, follow src->dst; if false, follow dst->src
///
/// # Returns
/// Vector of related chunks ordered by relevance score
pub async fn find_related_chunks_directional(
    store: &SqliteStore,
    chunk_id: i64,
    max_depth: i32,
    edge_types: Option<Vec<EdgeType>>,
    forward: bool,
) -> Result<Vec<RelatedChunk>> {
    // TODO: Implement using SqliteStore graph methods in IDXABS-4001
    Ok(vec![])
}

/// Load callers, callees, and tests in parallel for a given chunk.
///
/// This function uses tokio::join! to load all three relationship types
/// concurrently, significantly reducing latency compared to sequential loading.
///
/// # Arguments
/// * `store` - SQLite store
/// * `chunk_id` - Starting chunk ID
/// * `max_depth` - Maximum traversal depth for each relationship type
///
/// # Returns
/// Tuple of (callers, callees, tests) where each is a vector of RelatedChunk.
/// If any query fails, that vector will be empty (graceful degradation).
///
/// # Example
/// ```ignore
/// let (callers, callees, tests) = load_relationships_parallel(&store, 1234, 2).await;
/// println!("Found {} callers, {} callees, {} tests",
///          callers.len(), callees.len(), tests.len());
/// ```
pub async fn load_relationships_parallel(
    store: &SqliteStore,
    chunk_id: i64,
    max_depth: i32,
) -> (Vec<RelatedChunk>, Vec<RelatedChunk>, Vec<RelatedChunk>) {
    // TODO: Implement using SqliteStore graph methods in IDXABS-4001
    (Vec::new(), Vec::new(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_type_conversion() {
        assert_eq!(EdgeType::Imports.as_db_str(), "imports");
        assert_eq!(EdgeType::Exports.as_db_str(), "exports");
        assert_eq!(EdgeType::Calls.as_db_str(), "calls");
        assert_eq!(EdgeType::CalledBy.as_db_str(), "called_by");
        assert_eq!(EdgeType::TestOf.as_db_str(), "test_of");
        assert_eq!(EdgeType::RouteOf.as_db_str(), "route_of");
    }

    #[test]
    fn test_edge_type_equality() {
        assert_eq!(EdgeType::Calls, EdgeType::Calls);
        assert_ne!(EdgeType::Calls, EdgeType::Imports);
    }
}
