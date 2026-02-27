//! Query expansion with synonym and concept mapping.
//!
//! This module expands query terms with related synonyms and concepts to
//! improve recall. It focuses on code-related terminology and common patterns.

use std::collections::{HashMap, HashSet};

/// Query expander that adds synonyms and related concepts to queries.
///
/// The expander maintains a knowledge base of code-related synonyms and
/// concepts to improve search recall by including alternative terminology.
pub struct QueryExpander {
    /// Synonym map: term -> related terms
    synonyms: HashMap<String, Vec<String>>,
}

impl QueryExpander {
    /// Create a new query expander with default code-related synonyms.
    pub fn new() -> Self {
        Self {
            synonyms: Self::default_synonyms(),
        }
    }

    /// Create an expander with custom synonyms.
    pub fn with_synonyms(synonyms: HashMap<String, Vec<String>>) -> Self {
        Self { synonyms }
    }

    /// Default code-related synonym mappings.
    fn default_synonyms() -> HashMap<String, Vec<String>> {
        let mut synonyms = HashMap::new();

        // Function-related terms
        synonyms.insert(
            "function".to_string(),
            vec!["fn".to_string(), "func".to_string(), "method".to_string()],
        );
        synonyms.insert(
            "fn".to_string(),
            vec![
                "function".to_string(),
                "func".to_string(),
                "method".to_string(),
            ],
        );
        synonyms.insert(
            "method".to_string(),
            vec!["function".to_string(), "fn".to_string(), "func".to_string()],
        );

        // Class/object-related terms
        synonyms.insert(
            "class".to_string(),
            vec![
                "type".to_string(),
                "struct".to_string(),
                "object".to_string(),
            ],
        );
        synonyms.insert(
            "struct".to_string(),
            vec!["class".to_string(), "type".to_string()],
        );
        synonyms.insert(
            "interface".to_string(),
            vec!["trait".to_string(), "protocol".to_string()],
        );
        synonyms.insert("trait".to_string(), vec!["interface".to_string()]);

        // Variable-related terms
        synonyms.insert(
            "variable".to_string(),
            vec!["var".to_string(), "let".to_string(), "const".to_string()],
        );
        synonyms.insert(
            "var".to_string(),
            vec!["variable".to_string(), "let".to_string()],
        );
        synonyms.insert("const".to_string(), vec!["constant".to_string()]);
        synonyms.insert("constant".to_string(), vec!["const".to_string()]);

        // Authentication/authorization terms
        synonyms.insert(
            "auth".to_string(),
            vec![
                "authentication".to_string(),
                "authorize".to_string(),
                "login".to_string(),
            ],
        );
        synonyms.insert(
            "authentication".to_string(),
            vec!["auth".to_string(), "login".to_string()],
        );
        synonyms.insert(
            "login".to_string(),
            vec!["signin".to_string(), "auth".to_string()],
        );
        synonyms.insert("logout".to_string(), vec!["signout".to_string()]);

        // Component-related terms (React/UI)
        synonyms.insert(
            "component".to_string(),
            vec!["comp".to_string(), "widget".to_string()],
        );
        synonyms.insert("hook".to_string(), vec!["usehook".to_string()]);

        // API/network terms
        synonyms.insert(
            "api".to_string(),
            vec!["endpoint".to_string(), "service".to_string()],
        );
        synonyms.insert("request".to_string(), vec!["req".to_string()]);
        synonyms.insert(
            "response".to_string(),
            vec!["resp".to_string(), "res".to_string()],
        );

        // Database terms
        synonyms.insert(
            "database".to_string(),
            vec!["db".to_string(), "store".to_string()],
        );
        synonyms.insert(
            "query".to_string(),
            vec!["search".to_string(), "find".to_string()],
        );

        // Error handling terms
        synonyms.insert(
            "error".to_string(),
            vec!["err".to_string(), "exception".to_string()],
        );
        synonyms.insert(
            "exception".to_string(),
            vec!["error".to_string(), "err".to_string()],
        );

        // Configuration terms
        synonyms.insert(
            "config".to_string(),
            vec!["configuration".to_string(), "settings".to_string()],
        );
        synonyms.insert(
            "configuration".to_string(),
            vec!["config".to_string(), "settings".to_string()],
        );

        // Test terms
        synonyms.insert(
            "test".to_string(),
            vec!["spec".to_string(), "unittest".to_string()],
        );
        synonyms.insert(
            "mock".to_string(),
            vec!["stub".to_string(), "fake".to_string()],
        );

        // Async/concurrency terms
        synonyms.insert(
            "async".to_string(),
            vec!["asynchronous".to_string(), "await".to_string()],
        );
        synonyms.insert("promise".to_string(), vec!["future".to_string()]);

        // Module/import terms
        synonyms.insert(
            "import".to_string(),
            vec!["require".to_string(), "use".to_string()],
        );
        synonyms.insert("export".to_string(), vec!["expose".to_string()]);

        synonyms
    }

    /// Expand a query by adding synonyms and related terms.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maproom::search::QueryExpander;
    ///
    /// let expander = QueryExpander::new();
    /// let expanded = expander.expand(&["auth".to_string(), "user".to_string()]);
    /// // Returns: ["authentication", "authorize", "login", ...] (related to "auth")
    /// ```
    pub fn expand(&self, tokens: &[String]) -> Vec<String> {
        let mut expanded = HashSet::new();

        for token in tokens {
            // Look up synonyms for this token
            if let Some(synonyms) = self.synonyms.get(token) {
                for synonym in synonyms {
                    expanded.insert(synonym.clone());
                }
            }

            // Also try prefix matching for partial terms
            // e.g., "authenticate" might match "auth" synonyms
            for (key, synonyms) in &self.synonyms {
                if token.len() > 4 && (token.starts_with(key) || key.starts_with(token)) {
                    for synonym in synonyms {
                        expanded.insert(synonym.clone());
                    }
                }
            }
        }

        // Convert to vector and sort for consistency
        let mut result: Vec<String> = expanded.into_iter().collect();
        result.sort();
        result
    }

    /// Expand query asynchronously (for consistency with async pipeline).
    pub async fn expand_async(&self, tokens: &[String]) -> Vec<String> {
        self.expand(tokens)
    }

    /// Add custom synonyms to the expander.
    pub fn add_synonym(&mut self, term: String, synonyms: Vec<String>) {
        self.synonyms.insert(term, synonyms);
    }

    /// Get all synonyms for a term.
    pub fn get_synonyms(&self, term: &str) -> Option<&Vec<String>> {
        self.synonyms.get(term)
    }
}

impl Default for QueryExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_synonyms() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["function".to_string()]);

        assert!(expanded.contains(&"fn".to_string()));
        assert!(expanded.contains(&"func".to_string()));
        assert!(expanded.contains(&"method".to_string()));
    }

    #[test]
    fn test_auth_synonyms() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["auth".to_string()]);

        assert!(expanded.contains(&"authentication".to_string()));
        assert!(expanded.contains(&"authorize".to_string()));
        assert!(expanded.contains(&"login".to_string()));
    }

    #[test]
    fn test_multiple_tokens() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["auth".to_string(), "error".to_string()]);

        // Should have synonyms from both terms
        assert!(expanded.contains(&"authentication".to_string()));
        assert!(expanded.contains(&"err".to_string()));
        assert!(expanded.contains(&"exception".to_string()));
    }

    #[test]
    fn test_no_synonyms() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["unknownterm".to_string()]);

        // Should return empty if no synonyms found
        assert!(expanded.is_empty() || !expanded.contains(&"unknownterm".to_string()));
    }

    #[test]
    fn test_prefix_matching() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["authentication".to_string()]);

        // Should match "auth" synonyms via prefix matching
        assert!(expanded.contains(&"auth".to_string()) || expanded.contains(&"login".to_string()));
    }

    #[test]
    fn test_deduplication() {
        let expander = QueryExpander::new();
        // "fn" and "function" should produce overlapping synonyms
        let expanded = expander.expand(&["fn".to_string(), "function".to_string()]);

        // Should be deduplicated
        let unique_count = expanded.iter().collect::<HashSet<_>>().len();
        assert_eq!(unique_count, expanded.len());
    }

    #[test]
    fn test_custom_synonyms() {
        let mut synonyms = HashMap::new();
        synonyms.insert(
            "custom".to_string(),
            vec!["synonym1".to_string(), "synonym2".to_string()],
        );

        let expander = QueryExpander::with_synonyms(synonyms);
        let expanded = expander.expand(&["custom".to_string()]);

        assert_eq!(expanded.len(), 2);
        assert!(expanded.contains(&"synonym1".to_string()));
        assert!(expanded.contains(&"synonym2".to_string()));
    }

    #[test]
    fn test_add_synonym() {
        let mut expander = QueryExpander::new();
        expander.add_synonym(
            "newterm".to_string(),
            vec!["related1".to_string(), "related2".to_string()],
        );

        let expanded = expander.expand(&["newterm".to_string()]);
        assert!(expanded.contains(&"related1".to_string()));
        assert!(expanded.contains(&"related2".to_string()));
    }

    #[test]
    fn test_get_synonyms() {
        let expander = QueryExpander::new();

        let synonyms = expander.get_synonyms("auth");
        assert!(synonyms.is_some());
        assert!(synonyms.unwrap().contains(&"authentication".to_string()));

        let synonyms = expander.get_synonyms("nonexistent");
        assert!(synonyms.is_none());
    }

    #[tokio::test]
    async fn test_async_expansion() {
        let expander = QueryExpander::new();
        let expanded = expander.expand_async(&["auth".to_string()]).await;

        assert!(expanded.contains(&"authentication".to_string()));
    }

    #[test]
    fn test_database_synonyms() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["database".to_string()]);

        assert!(expanded.contains(&"db".to_string()));
        assert!(expanded.contains(&"store".to_string()));
    }

    #[test]
    fn test_class_synonyms() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["class".to_string()]);

        assert!(expanded.contains(&"type".to_string()));
        assert!(expanded.contains(&"struct".to_string()));
        assert!(expanded.contains(&"object".to_string()));
    }

    #[test]
    fn test_empty_tokens() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&[]);
        assert!(expanded.is_empty());
    }

    #[test]
    fn test_sorted_output() {
        let expander = QueryExpander::new();
        let expanded = expander.expand(&["auth".to_string()]);

        // Verify output is sorted
        let mut sorted = expanded.clone();
        sorted.sort();
        assert_eq!(expanded, sorted);
    }
}
