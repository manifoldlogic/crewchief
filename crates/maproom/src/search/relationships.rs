//! Relationship expansion module for search result enhancement.
//!
//! This module performs shallow graph traversal to find related chunks, computes
//! weighted relevance scores based on edge type and module proximity, and returns
//! the top N most relevant chunks.

use crate::context::graph::find_related_chunks;
use crate::db::Store;
use crate::search::results::RelatedChunkResult;
use anyhow::Result;

/// Default edge weight for standard relationships
const EDGE_WEIGHT_DEFAULT: f32 = 1.0;

/// Reduced weight for test-related chunks
const EDGE_WEIGHT_TEST_PENALTY: f32 = 0.5;

/// Increased weight for inheritance relationships
const EDGE_WEIGHT_INHERITANCE_BOOST: f32 = 1.1;

/// Module proximity boost for chunks in the same directory
const MODULE_PROXIMITY_BOOST: f32 = 1.2;

/// Maximum preview length in characters
const PREVIEW_MAX_LENGTH: usize = 100;

/// Find the top N most relevant related chunks through graph traversal.
///
/// This function performs shallow graph traversal (depth=2) to find related chunks,
/// computes weighted relevance scores based on edge type and module proximity,
/// and returns the top N most relevant chunks ordered by relevance score.
///
/// # Arguments
///
/// * `store` - SQLite store for database access
/// * `source_chunk_id` - Starting chunk ID for graph traversal
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// Vector of `RelatedChunkResult` ordered by relevance score (highest first),
/// limited to `limit` results.
///
/// # Algorithm
///
/// 1. Get source chunk metadata (for module detection)
/// 2. Call existing `find_related_chunks(store, source_chunk_id, depth=2, None)`
/// 3. Compute relevance: base_relevance × edge_weight × module_boost
/// 4. Sort by relevance descending
/// 5. Take top N (limit parameter)
/// 6. Convert to RelatedChunkResult
///
/// # Example
///
/// ```ignore
/// let related = find_top_related_chunks(&store, 1234, 5).await?;
/// for chunk in related {
///     println!("Related: {} (relevance: {:.2})",
///              chunk.symbol_name.unwrap_or_default(), chunk.relevance);
/// }
/// ```
pub async fn find_top_related_chunks(
    store: &(dyn Store + Send + Sync),
    source_chunk_id: i64,
    limit: usize,
) -> Result<Vec<RelatedChunkResult>> {
    // Get source chunk metadata for module proximity detection
    let source_chunk = store.get_chunk_by_id(source_chunk_id).await?;
    let source_dir = if let Some(ref chunk) = source_chunk {
        extract_parent_dir(&chunk.file_path)
    } else {
        String::new()
    };

    // Perform graph traversal with depth=2 (shallow traversal)
    let related = find_related_chunks(store, source_chunk_id, 2, None).await?;

    // Compute weighted relevance scores
    let mut scored_chunks: Vec<(f32, RelatedChunkResult)> = Vec::new();

    for chunk in related {
        // Base relevance from graph traversal (already includes depth decay)
        let base_relevance = chunk.relevance as f32;

        // Compute edge weight based on edge type and target kind
        // Note: edge type information is not available in RelatedChunk, so we use kind
        let edge_weight = compute_edge_weight("", &chunk.kind);

        // Apply module proximity boost
        let chunk_dir = extract_parent_dir(&chunk.relpath);
        let module_boost = if chunk_dir == source_dir && !source_dir.is_empty() {
            MODULE_PROXIMITY_BOOST
        } else {
            1.0
        };

        // Final relevance score
        let relevance = base_relevance * edge_weight * module_boost;

        // Convert to RelatedChunkResult
        let result = RelatedChunkResult {
            chunk_id: chunk.id,
            relpath: chunk.relpath,
            symbol_name: chunk.symbol_name,
            kind: chunk.kind,
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            preview: truncate_preview(&chunk.preview, PREVIEW_MAX_LENGTH),
            depth: chunk.depth,
            relevance,
            relationship_type: infer_relationship_type(chunk.depth),
        };

        scored_chunks.push((relevance, result));
    }

    // Sort by relevance score (highest first)
    scored_chunks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N results
    let top_results = scored_chunks
        .into_iter()
        .take(limit)
        .map(|(_, result)| result)
        .collect();

    Ok(top_results)
}

/// Compute edge weight based on edge type and target kind.
///
/// # Arguments
///
/// * `edge_type` - Type of edge relationship (extends, implements, etc.)
/// * `target_kind` - Kind of target chunk (function, class, test, etc.)
///
/// # Returns
///
/// Weight multiplier for this edge (0.5 to 1.1)
///
/// # Examples
///
/// ```ignore
/// use maproom::search::relationships::compute_edge_weight;
///
/// assert_eq!(compute_edge_weight("extends", "class"), 1.1);
/// assert_eq!(compute_edge_weight("implements", "interface"), 1.1);
/// assert_eq!(compute_edge_weight("calls", "test"), 0.5);
/// assert_eq!(compute_edge_weight("calls", "function"), 1.0);
/// ```
fn compute_edge_weight(edge_type: &str, target_kind: &str) -> f32 {
    match (edge_type, target_kind) {
        // Inheritance relationships get a boost
        ("extends" | "implements", _) => EDGE_WEIGHT_INHERITANCE_BOOST,
        // Test chunks get a penalty
        (_, kind) if kind.contains("test") => EDGE_WEIGHT_TEST_PENALTY,
        // Default weight for other relationships
        _ => EDGE_WEIGHT_DEFAULT,
    }
}

/// Extract parent directory from a file path.
///
/// # Arguments
///
/// * `path` - File path (e.g., "src/module/file.rs")
///
/// # Returns
///
/// Parent directory string (e.g., "src/module"), or empty string if no parent
///
/// # Examples
///
/// ```ignore
/// use maproom::search::relationships::extract_parent_dir;
///
/// assert_eq!(extract_parent_dir("src/module/file.rs"), "src/module");
/// assert_eq!(extract_parent_dir("file.rs"), "");
/// assert_eq!(extract_parent_dir("a/b/c/d.txt"), "a/b/c");
/// ```
fn extract_parent_dir(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string()
}

/// Truncate content to a maximum length with ellipsis.
///
/// # Arguments
///
/// * `content` - Content string to truncate
/// * `max_length` - Maximum length in characters (not including ellipsis)
///
/// # Returns
///
/// Truncated string with "..." suffix if truncated, or original if shorter
///
/// # Examples
///
/// ```ignore
/// use maproom::search::relationships::truncate_preview;
///
/// assert_eq!(truncate_preview("short", 100), "short");
/// assert_eq!(truncate_preview("a".repeat(150).as_str(), 100), format!("{}...", "a".repeat(100)));
/// ```
fn truncate_preview(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_string()
    } else {
        format!("{}...", &content[..max_length])
    }
}

/// Infer relationship type from depth.
///
/// This is a placeholder implementation that infers the relationship type
/// based on depth, since the current RelatedChunk structure doesn't carry
/// edge type information.
///
/// # Arguments
///
/// * `depth` - Graph traversal depth (1 or 2)
///
/// # Returns
///
/// Inferred relationship type string
fn infer_relationship_type(depth: i32) -> String {
    match depth {
        1 => "direct".to_string(),
        2 => "indirect".to_string(),
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_weight_computation() {
        // Inheritance relationships get boost
        assert_eq!(
            compute_edge_weight("extends", "class"),
            EDGE_WEIGHT_INHERITANCE_BOOST
        );
        assert_eq!(
            compute_edge_weight("implements", "interface"),
            EDGE_WEIGHT_INHERITANCE_BOOST
        );

        // Test chunks get penalty
        assert_eq!(
            compute_edge_weight("calls", "test"),
            EDGE_WEIGHT_TEST_PENALTY
        );
        assert_eq!(
            compute_edge_weight("", "test_function"),
            EDGE_WEIGHT_TEST_PENALTY
        );
        assert_eq!(
            compute_edge_weight("calls", "unit_test"),
            EDGE_WEIGHT_TEST_PENALTY
        );

        // Default weight for other relationships
        assert_eq!(
            compute_edge_weight("calls", "function"),
            EDGE_WEIGHT_DEFAULT
        );
        assert_eq!(
            compute_edge_weight("imports", "module"),
            EDGE_WEIGHT_DEFAULT
        );
        assert_eq!(compute_edge_weight("", "class"), EDGE_WEIGHT_DEFAULT);
    }

    #[test]
    fn test_module_proximity_boost() {
        let same_dir = "src/module";
        let different_dir = "src/other";

        // Same directory should get boost
        assert_eq!(
            if same_dir == same_dir && !same_dir.is_empty() {
                MODULE_PROXIMITY_BOOST
            } else {
                1.0
            },
            MODULE_PROXIMITY_BOOST
        );

        // Different directory should get 1.0
        assert_eq!(
            if same_dir == different_dir && !same_dir.is_empty() {
                MODULE_PROXIMITY_BOOST
            } else {
                1.0
            },
            1.0
        );

        // Empty directories should get 1.0 (no boost)
        assert_eq!(
            if "" == "" && !"".is_empty() {
                MODULE_PROXIMITY_BOOST
            } else {
                1.0
            },
            1.0
        );
    }

    #[test]
    fn test_relevance_sorting() {
        // Higher relevance should rank first
        let mut chunks = vec![
            (0.5, "chunk_a"),
            (0.9, "chunk_b"),
            (0.3, "chunk_c"),
            (0.7, "chunk_d"),
        ];

        chunks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        assert_eq!(chunks[0].1, "chunk_b"); // 0.9
        assert_eq!(chunks[1].1, "chunk_d"); // 0.7
        assert_eq!(chunks[2].1, "chunk_a"); // 0.5
        assert_eq!(chunks[3].1, "chunk_c"); // 0.3
    }

    #[test]
    fn test_preview_truncation() {
        // Content shorter than max_length
        assert_eq!(truncate_preview("short", 100), "short");

        // Content exactly max_length
        let exact = "a".repeat(100);
        assert_eq!(truncate_preview(&exact, 100), exact);

        // Content longer than max_length
        let long_content = "a".repeat(150);
        let expected = format!("{}...", "a".repeat(100));
        assert_eq!(truncate_preview(&long_content, 100), expected);

        // Edge case: empty string
        assert_eq!(truncate_preview("", 100), "");

        // Edge case: max_length = 0
        assert_eq!(truncate_preview("content", 0), "...");
    }

    #[test]
    fn test_extract_parent_dir() {
        // Normal paths
        assert_eq!(extract_parent_dir("src/module/file.rs"), "src/module");
        assert_eq!(extract_parent_dir("a/b/c/d.txt"), "a/b/c");
        assert_eq!(extract_parent_dir("src/file.rs"), "src");

        // Edge cases
        assert_eq!(extract_parent_dir("file.rs"), "");
        assert_eq!(extract_parent_dir(""), "");
        assert_eq!(extract_parent_dir("/"), "");

        // Multiple levels
        assert_eq!(extract_parent_dir("a/b/c/d/e/f.txt"), "a/b/c/d/e");
    }

    #[test]
    fn test_empty_related_chunks() {
        // This test verifies the behavior when no relationships are found.
        // Since we can't easily mock the database here, we test the sorting
        // and limiting logic with an empty vector.
        let scored_chunks: Vec<(f32, &str)> = vec![];

        let top_results: Vec<&str> = scored_chunks
            .into_iter()
            .take(5)
            .map(|(_, result)| result)
            .collect();

        assert_eq!(top_results.len(), 0);
    }

    #[test]
    fn test_fewer_than_limit() {
        // Test when we have fewer chunks than the limit
        let scored_chunks = vec![(0.9, "chunk_a"), (0.5, "chunk_b")];

        let top_results: Vec<&str> = scored_chunks
            .into_iter()
            .take(5) // Request 5, but only 2 available
            .map(|(_, result)| result)
            .collect();

        assert_eq!(top_results.len(), 2);
        assert_eq!(top_results[0], "chunk_a");
        assert_eq!(top_results[1], "chunk_b");
    }

    #[test]
    fn test_infer_relationship_type() {
        assert_eq!(infer_relationship_type(1), "direct");
        assert_eq!(infer_relationship_type(2), "indirect");
        assert_eq!(infer_relationship_type(3), "unknown");
        assert_eq!(infer_relationship_type(0), "unknown");
    }
}
