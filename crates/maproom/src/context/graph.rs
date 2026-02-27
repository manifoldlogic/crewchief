//! Graph traversal queries for code relationships.
//!
//! This module implements recursive graph traversal using SQLite CTEs
//! to discover code relationships through chunk_edges. It supports:
//! - Bidirectional edge traversal
//! - Depth limiting to prevent unbounded queries
//! - Relevance decay (0.7 per hop)
//! - Multiple relationship types filtering

use crate::db::sqlite::graph::{GraphResult, ImportDirection};
use crate::db::traits::StoreChunks;
use crate::db::traits::StoreGraph;
use crate::db::SqliteStore;
use anyhow::Result;

/// Relevance decay factor per hop in graph traversal
const RELEVANCE_DECAY: f64 = 0.7;

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

    /// Create from database edge type string
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "imports" => Some(EdgeType::Imports),
            "exports" => Some(EdgeType::Exports),
            "calls" => Some(EdgeType::Calls),
            "called_by" => Some(EdgeType::CalledBy),
            "test_of" => Some(EdgeType::TestOf),
            "route_of" => Some(EdgeType::RouteOf),
            _ => None,
        }
    }
}

/// Convert a GraphResult to a RelatedChunk by fetching chunk details
async fn graph_result_to_related_chunk(
    store: &SqliteStore,
    result: GraphResult,
) -> Result<Option<RelatedChunk>> {
    // Fetch chunk details from the store
    let chunk = store.get_chunk_by_id(result.chunk_id).await?;

    match chunk {
        Some(c) => Ok(Some(RelatedChunk {
            id: c.id,
            relpath: c.file_path,
            symbol_name: c.symbol_name,
            kind: c.kind,
            start_line: c.start_line,
            end_line: c.end_line,
            preview: c.preview,
            depth: result.depth as i32,
            relevance: RELEVANCE_DECAY.powi(result.depth as i32),
        })),
        None => Ok(None),
    }
}

/// Convert multiple GraphResults to RelatedChunks in batch
async fn graph_results_to_related_chunks(
    store: &SqliteStore,
    results: Vec<GraphResult>,
    edge_types: Option<&[EdgeType]>,
) -> Result<Vec<RelatedChunk>> {
    let mut related_chunks = Vec::new();

    for result in results {
        // Filter by edge type if specified
        if let Some(types) = edge_types {
            let result_edge_type = EdgeType::from_db_str(&result.edge_type);
            if let Some(edge_type) = result_edge_type {
                if !types.contains(&edge_type) {
                    continue;
                }
            } else {
                // Unknown edge type, skip if filtering
                continue;
            }
        }

        if let Some(chunk) = graph_result_to_related_chunk(store, result).await? {
            related_chunks.push(chunk);
        }
    }

    // Sort by relevance (highest first)
    related_chunks.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(related_chunks)
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
    let depth = Some(max_depth as usize);
    let mut all_results = Vec::new();

    // Determine which relationship types to query based on edge_types filter
    let query_callers = edge_types
        .as_ref()
        .is_none_or(|types| types.iter().any(|t| matches!(t, EdgeType::CalledBy)));
    let query_callees = edge_types
        .as_ref()
        .is_none_or(|types| types.iter().any(|t| matches!(t, EdgeType::Calls)));
    let query_imports = edge_types.as_ref().is_none_or(|types| {
        types
            .iter()
            .any(|t| matches!(t, EdgeType::Imports | EdgeType::Exports))
    });

    // Query callers (who calls this chunk)
    if query_callers {
        let callers = store.find_callers(chunk_id, depth).await?;
        all_results.extend(callers);
    }

    // Query callees (who this chunk calls)
    if query_callees {
        let callees = store.find_callees(chunk_id, depth).await?;
        all_results.extend(callees);
    }

    // Query imports (incoming and outgoing)
    if query_imports {
        let incoming_imports = store
            .find_imports(chunk_id, ImportDirection::Incoming, depth)
            .await?;
        let outgoing_imports = store
            .find_imports(chunk_id, ImportDirection::Outgoing, depth)
            .await?;
        all_results.extend(incoming_imports);
        all_results.extend(outgoing_imports);
    }

    // Convert to RelatedChunks with filtering
    let edge_types_ref = edge_types.as_deref();
    graph_results_to_related_chunks(store, all_results, edge_types_ref).await
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
    let depth = Some(max_depth as usize);
    let mut all_results = Vec::new();

    if forward {
        // Forward direction: find what this chunk references (callees, outgoing imports)
        let query_callees = edge_types
            .as_ref()
            .is_none_or(|types| types.iter().any(|t| matches!(t, EdgeType::Calls)));
        let query_imports = edge_types
            .as_ref()
            .is_none_or(|types| types.iter().any(|t| matches!(t, EdgeType::Imports)));

        if query_callees {
            let callees = store.find_callees(chunk_id, depth).await?;
            all_results.extend(callees);
        }
        if query_imports {
            let imports = store
                .find_imports(chunk_id, ImportDirection::Outgoing, depth)
                .await?;
            all_results.extend(imports);
        }
    } else {
        // Backward direction: find what references this chunk (callers, incoming imports)
        let query_callers = edge_types
            .as_ref()
            .is_none_or(|types| types.iter().any(|t| matches!(t, EdgeType::CalledBy)));
        let query_imports = edge_types
            .as_ref()
            .is_none_or(|types| types.iter().any(|t| matches!(t, EdgeType::Exports)));

        if query_callers {
            let callers = store.find_callers(chunk_id, depth).await?;
            all_results.extend(callers);
        }
        if query_imports {
            let imports = store
                .find_imports(chunk_id, ImportDirection::Incoming, depth)
                .await?;
            all_results.extend(imports);
        }
    }

    // Convert to RelatedChunks with filtering
    let edge_types_ref = edge_types.as_deref();
    graph_results_to_related_chunks(store, all_results, edge_types_ref).await
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
    let depth = Some(max_depth as usize);

    // Run all three queries in parallel using tokio::join!
    let (callers_result, callees_result, tests_result) = tokio::join!(
        async {
            match store.find_callers(chunk_id, depth).await {
                Ok(results) => graph_results_to_related_chunks(store, results, None)
                    .await
                    .unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        },
        async {
            match store.find_callees(chunk_id, depth).await {
                Ok(results) => graph_results_to_related_chunks(store, results, None)
                    .await
                    .unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        },
        async {
            // Find test relationships - tests that target this chunk
            // Using imports with test_of edge type filter
            match store
                .find_imports(chunk_id, ImportDirection::Incoming, depth)
                .await
            {
                Ok(results) => {
                    // Filter for test_of edge type
                    let test_results: Vec<_> = results
                        .into_iter()
                        .filter(|r| r.edge_type == "test_of")
                        .collect();
                    graph_results_to_related_chunks(store, test_results, None)
                        .await
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        }
    );

    (callers_result, callees_result, tests_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::StoreMigration;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    #[test]
    fn test_edge_type_from_db_str() {
        assert_eq!(EdgeType::from_db_str("imports"), Some(EdgeType::Imports));
        assert_eq!(EdgeType::from_db_str("exports"), Some(EdgeType::Exports));
        assert_eq!(EdgeType::from_db_str("calls"), Some(EdgeType::Calls));
        assert_eq!(EdgeType::from_db_str("called_by"), Some(EdgeType::CalledBy));
        assert_eq!(EdgeType::from_db_str("test_of"), Some(EdgeType::TestOf));
        assert_eq!(EdgeType::from_db_str("route_of"), Some(EdgeType::RouteOf));
        assert_eq!(EdgeType::from_db_str("unknown"), None);
    }

    #[test]
    fn test_relevance_decay() {
        // Test that relevance decays correctly with depth
        let depth_0_relevance = RELEVANCE_DECAY.powi(0);
        let depth_1_relevance = RELEVANCE_DECAY.powi(1);
        let depth_2_relevance = RELEVANCE_DECAY.powi(2);
        let depth_3_relevance = RELEVANCE_DECAY.powi(3);

        assert!((depth_0_relevance - 1.0).abs() < 0.001);
        assert!((depth_1_relevance - 0.7).abs() < 0.001);
        assert!((depth_2_relevance - 0.49).abs() < 0.001);
        assert!((depth_3_relevance - 0.343).abs() < 0.001);
    }

    #[test]
    fn test_related_chunk_structure() {
        let chunk = RelatedChunk {
            id: 123,
            relpath: "src/main.rs".to_string(),
            symbol_name: Some("main".to_string()),
            kind: "function".to_string(),
            start_line: 1,
            end_line: 10,
            preview: "fn main() {}".to_string(),
            depth: 1,
            relevance: 0.7,
        };

        assert_eq!(chunk.id, 123);
        assert_eq!(chunk.relpath, "src/main.rs");
        assert_eq!(chunk.symbol_name, Some("main".to_string()));
        assert_eq!(chunk.kind, "function");
        assert_eq!(chunk.depth, 1);
        assert!((chunk.relevance - 0.7).abs() < 0.001);
    }

    // Integration tests with database
    static TEST_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    async fn setup_test_store() -> SqliteStore {
        let counter = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("file:memdb_graph_test_{}?mode=memory&cache=shared", counter);
        let store = SqliteStore::connect(&db_name)
            .await
            .expect("Failed to create test store");
        store.migrate().await.expect("Failed to run migrations");
        store
    }

    #[tokio::test]
    async fn test_find_related_chunks_empty_result() {
        let store = setup_test_store().await;

        // Query with non-existent chunk - should return empty
        let results = find_related_chunks(&store, 99999, 3, None).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_find_related_chunks_directional_empty() {
        let store = setup_test_store().await;

        // Forward direction with non-existent chunk
        let forward = find_related_chunks_directional(&store, 99999, 3, None, true)
            .await
            .unwrap();
        assert!(forward.is_empty());

        // Backward direction with non-existent chunk
        let backward = find_related_chunks_directional(&store, 99999, 3, None, false)
            .await
            .unwrap();
        assert!(backward.is_empty());
    }

    #[tokio::test]
    async fn test_load_relationships_parallel_empty() {
        let store = setup_test_store().await;

        // Parallel load with non-existent chunk
        let (callers, callees, tests) = load_relationships_parallel(&store, 99999, 3).await;
        assert!(callers.is_empty());
        assert!(callees.is_empty());
        assert!(tests.is_empty());
    }

    #[tokio::test]
    async fn test_find_related_chunks_with_edge_filter() {
        let store = setup_test_store().await;

        // Filter for only Calls edge type
        let results = find_related_chunks(&store, 99999, 3, Some(vec![EdgeType::Calls]))
            .await
            .unwrap();
        assert!(results.is_empty());
    }
}
