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
/// use maproom::indexer::edges::common::find_enclosing_chunk;
/// use maproom::indexer::edges::ChunkWithId;
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
    // INNERMOST wins (spec A3): chunkers emit overlapping container chunks
    // (impl/mod/class) around method chunks; first-match attribution pinned
    // every method-body call to the CONTAINER. Smallest containing span is
    // the enclosing function/method.
    chunks
        .iter()
        .filter(|chunk| chunk.start_line <= line && line <= chunk.end_line)
        .min_by_key(|chunk| chunk.end_line - chunk.start_line)
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
/// use maproom::indexer::edges::common::build_symbol_table;
/// use maproom::indexer::edges::ChunkWithId;
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

    /// Spec A3: with an overlapping container (impl/class) chunk, the
    /// INNERMOST chunk (the method) is the enclosing one.
    #[test]
    fn test_innermost_wins_over_container() {
        let chunks = vec![
            ChunkWithId {
                id: 10,
                symbol_name: Some("MyImpl".to_string()),
                kind: "impl".to_string(),
                start_line: 1,
                end_line: 20,
                file_id: 100,
            },
            ChunkWithId {
                id: 11,
                symbol_name: Some("method_a".to_string()),
                kind: "method".to_string(),
                start_line: 3,
                end_line: 8,
                file_id: 100,
            },
        ];
        assert_eq!(find_enclosing_chunk(&chunks, 5).unwrap().id, 11);
        // Outside the method but inside the container: container it is.
        assert_eq!(find_enclosing_chunk(&chunks, 15).unwrap().id, 10);
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
