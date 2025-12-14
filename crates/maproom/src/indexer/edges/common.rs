//! Common utilities for edge extraction.
//!
//! This module provides shared utilities used by all language-specific extractors:
//! - Finding the enclosing chunk for a given line number
//! - Building symbol tables for name resolution

use super::ChunkWithId;
use std::collections::HashMap;

/// Find the chunk that contains a given line number.
///
/// Performs a linear search through chunks to find one whose line range
/// contains the target line. This is acceptable for typical file sizes
/// (<100 chunks per file).
///
/// # Arguments
///
/// * `chunks` - Slice of chunks to search (must be from the same file)
/// * `line` - Target line number (1-indexed)
///
/// # Returns
///
/// * `Some(&ChunkWithId)` - The chunk containing the line
/// * `None` - No chunk contains the line
///
/// # Example
///
/// ```
/// use crewchief_maproom::indexer::edges::common::find_enclosing_chunk;
/// use crewchief_maproom::indexer::edges::ChunkWithId;
///
/// let chunks = vec![
///     ChunkWithId {
///         id: 1,
///         symbol_name: Some("foo".to_string()),
///         kind: "function".to_string(),
///         start_line: 1,
///         end_line: 5,
///         file_id: 100,
///     },
///     ChunkWithId {
///         id: 2,
///         symbol_name: Some("bar".to_string()),
///         kind: "function".to_string(),
///         start_line: 7,
///         end_line: 12,
///         file_id: 100,
///     },
/// ];
///
/// assert_eq!(find_enclosing_chunk(&chunks, 3).unwrap().id, 1);
/// assert_eq!(find_enclosing_chunk(&chunks, 10).unwrap().id, 2);
/// assert!(find_enclosing_chunk(&chunks, 6).is_none());
/// ```
pub fn find_enclosing_chunk(chunks: &[ChunkWithId], line: i32) -> Option<&ChunkWithId> {
    chunks
        .iter()
        .find(|chunk| chunk.start_line <= line && line <= chunk.end_line)
}

/// Build a symbol table mapping symbol names to chunk IDs.
///
/// Creates a hash map for fast lookup of chunks by symbol name. Chunks without
/// symbol names are excluded. If multiple chunks have the same symbol name
/// (e.g., overloaded methods), only the last one is kept.
///
/// # Arguments
///
/// * `chunks` - Slice of chunks to build the table from
///
/// # Returns
///
/// Hash map from symbol name to chunk ID
///
/// # Example
///
/// ```
/// use crewchief_maproom::indexer::edges::common::build_symbol_table;
/// use crewchief_maproom::indexer::edges::ChunkWithId;
///
/// let chunks = vec![
///     ChunkWithId {
///         id: 1,
///         symbol_name: Some("foo".to_string()),
///         kind: "function".to_string(),
///         start_line: 1,
///         end_line: 5,
///         file_id: 100,
///     },
///     ChunkWithId {
///         id: 2,
///         symbol_name: None,
///         kind: "statement".to_string(),
///         start_line: 7,
///         end_line: 8,
///         file_id: 100,
///     },
/// ];
///
/// let table = build_symbol_table(&chunks);
/// assert_eq!(table.len(), 1);
/// assert_eq!(table.get("foo"), Some(&1));
/// ```
pub fn build_symbol_table(chunks: &[ChunkWithId]) -> HashMap<String, i64> {
    chunks
        .iter()
        .filter_map(|chunk| {
            chunk
                .symbol_name
                .as_ref()
                .map(|name| (name.clone(), chunk.id))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_enclosing_chunk() {
        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("foo".to_string()),
                kind: "function".to_string(),
                start_line: 1,
                end_line: 5,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("bar".to_string()),
                kind: "function".to_string(),
                start_line: 7,
                end_line: 12,
                file_id: 100,
            },
        ];

        assert_eq!(find_enclosing_chunk(&chunks, 3).unwrap().id, 1);
        assert_eq!(find_enclosing_chunk(&chunks, 10).unwrap().id, 2);
        assert!(find_enclosing_chunk(&chunks, 6).is_none());
    }

    #[test]
    fn test_build_symbol_table() {
        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("foo".to_string()),
                kind: "function".to_string(),
                start_line: 1,
                end_line: 5,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: None,
                kind: "statement".to_string(),
                start_line: 7,
                end_line: 8,
                file_id: 100,
            },
        ];

        let table = build_symbol_table(&chunks);
        assert_eq!(table.len(), 1);
        assert_eq!(table.get("foo"), Some(&1));
    }
}
