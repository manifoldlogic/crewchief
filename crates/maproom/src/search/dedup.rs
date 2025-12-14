//! Search result deduplication.
//!
//! This module provides deduplication of search results across worktrees.
//! When the same code exists in multiple worktrees (e.g., main and feature
//! branches), search results may contain duplicates. This module groups
//! results by their logical identity and returns only the highest-scoring
//! representative from each group.
//!
//! # Identity Key
//!
//! Results are considered duplicates if they have the same:
//! - `relpath` (relative file path)
//! - `symbol_name` (function/class name, or empty)
//! - `start_line` (line number)
//!
//! Note: Line number sensitivity means code that has shifted by even one
//! line (e.g., due to added imports) will not be considered a duplicate.
//!
//! # Usage
//!
//! Deduplication is enabled by default. To disable:
//! ```ignore
//! let options = SearchOptions::new(repo_id, None, 10).without_dedup();
//! ```

use std::collections::HashMap;

use crate::search::results::ChunkSearchResult;

/// Unique identity for a code chunk across worktrees.
/// Chunks with the same ChunkIdentity are considered duplicates.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ChunkIdentity {
    /// Relative path to the file
    pub relpath: String,
    /// Symbol name (empty string if None)
    pub symbol_name: String,
    /// Starting line number
    pub start_line: i32,
}

impl ChunkIdentity {
    /// Create a ChunkIdentity from a search result.
    pub fn from_result(result: &ChunkSearchResult) -> Self {
        Self {
            relpath: result.relpath.clone(),
            symbol_name: result.symbol_name.clone().unwrap_or_default(),
            start_line: result.start_line,
        }
    }
}

/// Configuration for deduplication behavior.
#[derive(Debug, Clone)]
pub struct DeduplicationConfig {
    /// Enable deduplication (default: true)
    pub enabled: bool,
    /// Selection strategy for choosing representative
    pub strategy: SelectionStrategy,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: SelectionStrategy::HighestScore,
        }
    }
}

/// Strategy for selecting the representative chunk from duplicates.
///
/// Note: Only HighestScore is implemented in MVP. PreferMain requires
/// worktree_name to be added to ChunkSearchResult (future enhancement).
#[derive(Debug, Clone, Copy, Default)]
pub enum SelectionStrategy {
    /// Select the chunk with the highest score (MVP implementation)
    #[default]
    HighestScore,
    // Future: PreferMain - requires worktree_name in ChunkSearchResult
}

/// Deduplicate search results, keeping only the best representative per identity.
///
/// # Arguments
/// * `results` - Vector of search results (may contain duplicates)
/// * `config` - Deduplication configuration
///
/// # Returns
/// Vector of unique results, maintaining score order
pub fn deduplicate(
    results: Vec<ChunkSearchResult>,
    config: &DeduplicationConfig,
) -> Vec<ChunkSearchResult> {
    if !config.enabled || results.is_empty() {
        return results;
    }

    let mut groups: HashMap<ChunkIdentity, Vec<ChunkSearchResult>> = HashMap::new();

    // Group results by identity
    for result in results {
        let identity = ChunkIdentity::from_result(&result);
        groups.entry(identity).or_default().push(result);
    }

    // Select best representative from each group
    let mut deduplicated: Vec<ChunkSearchResult> = groups
        .into_values()
        .map(|mut group| select_representative(&mut group, config.strategy))
        .collect();

    // Re-sort by score (grouping may have disrupted order)
    deduplicated.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    deduplicated
}

/// Select the best representative from a group of duplicate chunks.
fn select_representative(
    group: &mut Vec<ChunkSearchResult>,
    strategy: SelectionStrategy,
) -> ChunkSearchResult {
    match strategy {
        SelectionStrategy::HighestScore => {
            group.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            group.remove(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::executor_types::SearchSource;
    use std::collections::HashMap as StdHashMap;

    /// Helper function to create a test ChunkSearchResult
    fn make_chunk_result(
        chunk_id: i64,
        relpath: &str,
        symbol_name: Option<&str>,
        start_line: i32,
        score: f32,
    ) -> ChunkSearchResult {
        ChunkSearchResult {
            chunk_id,
            file_id: 1,
            relpath: relpath.to_string(),
            symbol_name: symbol_name.map(|s| s.to_string()),
            kind: "function".to_string(),
            start_line,
            end_line: start_line + 10,
            preview: "...".to_string(),
            score,
            source_scores: StdHashMap::from([(SearchSource::FTS, score)]),
            confidence: None,
            related: None,
        }
    }

    /// Helper function to create a set of duplicates
    fn make_duplicates(
        count: usize,
        relpath: &str,
        symbol: &str,
        line: i32,
    ) -> Vec<ChunkSearchResult> {
        (0..count)
            .map(|i| {
                make_chunk_result(
                    i as i64,
                    relpath,
                    Some(symbol),
                    line,
                    0.9 - (i as f32 * 0.05), // Decreasing scores
                )
            })
            .collect()
    }

    #[test]
    fn test_identity_key_generation() {
        let chunk1 = make_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.9);
        let chunk2 = make_chunk_result(2, "src/auth.rs", Some("validate"), 10, 0.8);

        let id1 = ChunkIdentity::from_result(&chunk1);
        let id2 = ChunkIdentity::from_result(&chunk2);

        assert_eq!(id1, id2, "Same file/symbol/line should have same identity");

        // Different relpath
        let chunk3 = make_chunk_result(3, "src/other.rs", Some("validate"), 10, 0.7);
        let id3 = ChunkIdentity::from_result(&chunk3);
        assert_ne!(id1, id3, "Different relpath should have different identity");

        // Different symbol_name
        let chunk4 = make_chunk_result(4, "src/auth.rs", Some("other"), 10, 0.6);
        let id4 = ChunkIdentity::from_result(&chunk4);
        assert_ne!(id1, id4, "Different symbol should have different identity");

        // Different start_line
        let chunk5 = make_chunk_result(5, "src/auth.rs", Some("validate"), 20, 0.5);
        let id5 = ChunkIdentity::from_result(&chunk5);
        assert_ne!(id1, id5, "Different line should have different identity");
    }

    #[test]
    fn test_deduplicate_empty_results() {
        let results: Vec<ChunkSearchResult> = vec![];
        let config = DeduplicationConfig::default();

        let deduplicated = deduplicate(results, &config);
        assert!(
            deduplicated.is_empty(),
            "Empty input should return empty output"
        );
    }

    #[test]
    fn test_deduplicate_no_duplicates() {
        let results = vec![
            make_chunk_result(1, "src/a.rs", Some("func_a"), 10, 0.9),
            make_chunk_result(2, "src/b.rs", Some("func_b"), 20, 0.8),
            make_chunk_result(3, "src/c.rs", Some("func_c"), 30, 0.7),
        ];
        let config = DeduplicationConfig::default();

        let deduplicated = deduplicate(results.clone(), &config);
        assert_eq!(
            deduplicated.len(),
            3,
            "No duplicates should return same count"
        );
    }

    #[test]
    fn test_deduplicate_all_duplicates() {
        let results = make_duplicates(5, "src/auth.rs", "validate", 10);
        let config = DeduplicationConfig::default();

        let deduplicated = deduplicate(results, &config);
        assert_eq!(
            deduplicated.len(),
            1,
            "All duplicates should return one result"
        );
    }

    #[test]
    fn test_deduplicate_mixed() {
        let mut results = make_duplicates(3, "src/auth.rs", "validate", 10);
        results.push(make_chunk_result(
            10,
            "src/other.rs",
            Some("helper"),
            5,
            0.6,
        ));
        results.push(make_chunk_result(
            11,
            "src/other.rs",
            Some("helper"),
            5,
            0.5,
        ));

        let config = DeduplicationConfig::default();

        let deduplicated = deduplicate(results, &config);
        assert_eq!(
            deduplicated.len(),
            2,
            "3 duplicates + 2 duplicates = 2 unique"
        );
    }

    #[test]
    fn test_deduplicate_preserves_order() {
        let results = vec![
            make_chunk_result(1, "src/a.rs", Some("func_a"), 10, 0.5),
            make_chunk_result(2, "src/b.rs", Some("func_b"), 20, 0.9),
            make_chunk_result(3, "src/c.rs", Some("func_c"), 30, 0.7),
        ];
        let config = DeduplicationConfig::default();

        let deduplicated = deduplicate(results, &config);
        assert_eq!(deduplicated.len(), 3);
        // Should be sorted by score descending
        assert!(
            deduplicated[0].score >= deduplicated[1].score,
            "Results should be sorted by score"
        );
        assert!(
            deduplicated[1].score >= deduplicated[2].score,
            "Results should be sorted by score"
        );
    }

    #[test]
    fn test_highest_score_selection() {
        let results = vec![
            make_chunk_result(1, "src/auth.rs", Some("validate"), 10, 0.7),
            make_chunk_result(2, "src/auth.rs", Some("validate"), 10, 0.95), // highest
            make_chunk_result(3, "src/auth.rs", Some("validate"), 10, 0.8),
        ];
        let config = DeduplicationConfig::default();

        let deduplicated = deduplicate(results, &config);
        assert_eq!(deduplicated.len(), 1);
        assert_eq!(
            deduplicated[0].chunk_id, 2,
            "Should select highest scoring chunk"
        );
        assert!((deduplicated[0].score - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_disabled_config() {
        let results = make_duplicates(5, "src/auth.rs", "validate", 10);
        let config = DeduplicationConfig {
            enabled: false,
            strategy: SelectionStrategy::HighestScore,
        };

        let deduplicated = deduplicate(results.clone(), &config);
        assert_eq!(
            deduplicated.len(),
            5,
            "Disabled config should return all results"
        );
    }

    #[test]
    fn test_null_symbol_name_handling() {
        // Two chunks with None symbol_name, same file/line should be duplicates
        let results = vec![
            make_chunk_result(1, "src/auth.rs", None, 10, 0.9),
            make_chunk_result(2, "src/auth.rs", None, 10, 0.8),
        ];
        let config = DeduplicationConfig::default();

        let deduplicated = deduplicate(results, &config);
        assert_eq!(
            deduplicated.len(),
            1,
            "Chunks with None symbol_name should be grouped together"
        );

        // Verify the identity treats None as empty string
        let chunk_with_none = make_chunk_result(1, "src/auth.rs", None, 10, 0.9);
        let chunk_with_empty = make_chunk_result(2, "src/auth.rs", Some(""), 10, 0.8);
        let id1 = ChunkIdentity::from_result(&chunk_with_none);
        let id2 = ChunkIdentity::from_result(&chunk_with_empty);
        assert_eq!(id1, id2, "None and empty string should be equivalent");
    }
}
