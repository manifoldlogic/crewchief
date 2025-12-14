//! TypeScript/JavaScript edge extraction.
//!
//! This module extracts call edges from TypeScript and JavaScript source code.
//! Full implementation is in EDGEEXT-1002.

use anyhow::Result;

use super::{ChunkWithId, Edge};

/// Extract call edges from TypeScript/JavaScript source.
///
/// This is a stub implementation. Full tree-sitter based extraction will be
/// implemented in EDGEEXT-1002.
///
/// # Arguments
///
/// * `_source` - TypeScript/JavaScript source code
/// * `_chunks` - Chunks with database IDs from the same file
///
/// # Returns
///
/// * `Ok(Vec<Edge>)` - Extracted call edges (currently empty)
/// * `Err(_)` - Critical failure (not used in stub)
///
/// # Future Implementation (EDGEEXT-1002)
///
/// The full implementation will:
/// 1. Parse source with tree-sitter
/// 2. Find call_expression nodes
/// 3. Extract caller chunk via `find_enclosing_chunk()`
/// 4. Resolve callee via `build_symbol_table()`
/// 5. Create Edge with type=Calls
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::indexer::edges::typescript::extract_calls;
/// use crewchief_maproom::indexer::edges::ChunkWithId;
///
/// let source = "function foo() { bar(); }";
/// let chunks = vec![
///     ChunkWithId {
///         id: 1,
///         symbol_name: Some("foo".to_string()),
///         kind: "function".to_string(),
///         start_line: 1,
///         end_line: 1,
///         file_id: 100,
///     }
/// ];
///
/// let edges = extract_calls(source, &chunks)?;
/// assert_eq!(edges.len(), 0); // Stub returns empty
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract_calls(_source: &str, _chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    // Stub: will be implemented in EDGEEXT-1002
    Ok(Vec::new())
}
