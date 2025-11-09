//! Type definitions for search processing.
//!
//! This module defines the core types used throughout the search pipeline:
//! - ProcessedQuery: Output of query processing
//! - SearchMode: Query type detection (Code, Text, Auto)
//! - Query-related enums and structures

use crate::embedding::cache::Vector;
use serde::{Deserialize, Serialize};

/// Search mode indicating query type and optimal search strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SearchMode {
    /// Code-focused search - query contains code patterns (::, ->, etc.)
    Code,
    /// Text-focused search - natural language query with multiple words
    Text,
    /// Automatic mode selection - ambiguous queries
    #[default]
    Auto,
}

impl SearchMode {
    /// Returns true if this mode prioritizes code embeddings over text embeddings.
    pub fn prefers_code(&self) -> bool {
        matches!(self, SearchMode::Code)
    }

    /// Returns true if this mode prioritizes text embeddings over code embeddings.
    pub fn prefers_text(&self) -> bool {
        matches!(self, SearchMode::Text)
    }

    /// Returns true if mode is automatic (balanced approach).
    pub fn is_auto(&self) -> bool {
        matches!(self, SearchMode::Auto)
    }
}

/// Processed query output containing all representations needed for hybrid search.
///
/// This structure is the output of the QueryProcessor and contains:
/// - Original query text
/// - Tokenized terms for FTS
/// - Query embedding vector
/// - Expanded terms with synonyms
/// - Detected search mode
#[derive(Debug, Clone)]
pub struct ProcessedQuery {
    /// Original query string
    pub original: String,

    /// Tokenized terms compatible with FTS indexing
    pub tokens: Vec<String>,

    /// Query embedding vector (1536 dimensions)
    pub embedding: Vector,

    /// Expanded query terms with synonyms and related concepts
    pub expanded_terms: Vec<String>,

    /// Detected search mode
    pub mode: SearchMode,
}

impl ProcessedQuery {
    /// Create a new ProcessedQuery.
    pub fn new(
        original: String,
        tokens: Vec<String>,
        embedding: Vector,
        expanded_terms: Vec<String>,
        mode: SearchMode,
    ) -> Self {
        Self {
            original,
            tokens,
            embedding,
            expanded_terms,
            mode,
        }
    }

    /// Get all query terms (tokens + expanded terms, deduplicated).
    pub fn all_terms(&self) -> Vec<String> {
        let mut terms: Vec<String> = self.tokens.clone();
        terms.extend(self.expanded_terms.clone());
        terms.sort();
        terms.dedup();
        terms
    }

    /// Build FTS query string for PostgreSQL tsquery.
    ///
    /// Combines original tokens into an AND query with proper escaping.
    /// Example: ["auth", "login"] -> "auth & login"
    pub fn fts_query_string(&self) -> String {
        if self.tokens.is_empty() {
            return String::new();
        }

        // Escape special characters and join with &
        self.tokens
            .iter()
            .map(|t| Self::escape_fts_term(t))
            .collect::<Vec<_>>()
            .join(" & ")
    }

    /// Escape FTS special characters.
    fn escape_fts_term(term: &str) -> String {
        // PostgreSQL tsquery special characters: & | ! ( ) <-> ' "
        term.replace(['&', '|', '!', '(', ')', '<', '>', '\'', '"'], "")
            .trim()
            .to_string()
    }

    /// Build expanded FTS query string including synonyms.
    ///
    /// Uses OR operator to include expanded terms as alternatives.
    /// Example: ["auth", "login"] + ["authentication"] -> "(auth | authentication) & login"
    pub fn expanded_fts_query_string(&self) -> String {
        if self.tokens.is_empty() {
            return String::new();
        }

        let mut query_parts = Vec::new();

        for token in &self.tokens {
            // Find related expanded terms
            let related: Vec<_> = self
                .expanded_terms
                .iter()
                .filter(|t| t.contains(token) || token.contains(t.as_str()))
                .collect();

            if related.is_empty() {
                query_parts.push(Self::escape_fts_term(token));
            } else {
                let mut alternatives = vec![Self::escape_fts_term(token)];
                alternatives.extend(related.iter().map(|t| Self::escape_fts_term(t)));
                query_parts.push(format!("({})", alternatives.join(" | ")));
            }
        }

        query_parts.join(" & ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_mode_defaults() {
        assert_eq!(SearchMode::default(), SearchMode::Auto);
    }

    #[test]
    fn test_search_mode_predicates() {
        assert!(SearchMode::Code.prefers_code());
        assert!(!SearchMode::Code.prefers_text());
        assert!(!SearchMode::Code.is_auto());

        assert!(SearchMode::Text.prefers_text());
        assert!(!SearchMode::Text.prefers_code());
        assert!(!SearchMode::Text.is_auto());

        assert!(SearchMode::Auto.is_auto());
        assert!(!SearchMode::Auto.prefers_code());
        assert!(!SearchMode::Auto.prefers_text());
    }

    #[test]
    fn test_processed_query_all_terms() {
        let query = ProcessedQuery::new(
            "test query".to_string(),
            vec!["test".to_string(), "query".to_string()],
            vec![0.1; 1536],
            vec!["test".to_string(), "search".to_string()],
            SearchMode::Text,
        );

        let all_terms = query.all_terms();
        assert_eq!(all_terms.len(), 3); // test, query, search (deduplicated)
        assert!(all_terms.contains(&"test".to_string()));
        assert!(all_terms.contains(&"query".to_string()));
        assert!(all_terms.contains(&"search".to_string()));
    }

    #[test]
    fn test_fts_query_string() {
        let query = ProcessedQuery::new(
            "auth login".to_string(),
            vec!["auth".to_string(), "login".to_string()],
            vec![0.1; 1536],
            vec![],
            SearchMode::Code,
        );

        assert_eq!(query.fts_query_string(), "auth & login");
    }

    #[test]
    fn test_fts_query_string_escaping() {
        let query = ProcessedQuery::new(
            "test & query | not".to_string(),
            vec!["test".to_string(), "&".to_string(), "query".to_string()],
            vec![0.1; 1536],
            vec![],
            SearchMode::Text,
        );

        let fts = query.fts_query_string();
        assert!(!fts.contains('&') || fts.contains(" & ")); // Only & as operator
        assert!(!fts.contains('|')); // | should be removed
    }

    #[test]
    fn test_expanded_fts_query_string() {
        let query = ProcessedQuery::new(
            "auth".to_string(),
            vec!["auth".to_string()],
            vec![0.1; 1536],
            vec!["authentication".to_string(), "authorize".to_string()],
            SearchMode::Code,
        );

        let expanded = query.expanded_fts_query_string();
        assert!(expanded.contains("auth"));
        assert!(expanded.contains("authentication"));
        assert!(expanded.contains("|")); // OR operator for alternatives
    }

    #[test]
    fn test_empty_query() {
        let query = ProcessedQuery::new(
            "".to_string(),
            vec![],
            vec![0.1; 1536],
            vec![],
            SearchMode::Auto,
        );

        assert_eq!(query.fts_query_string(), "");
        assert_eq!(query.expanded_fts_query_string(), "");
        assert_eq!(query.all_terms().len(), 0);
    }
}
