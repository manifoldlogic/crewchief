//! Query processing pipeline for hybrid search.
//!
//! This module implements the QueryProcessor, which transforms raw search queries
//! into ProcessedQuery structures containing all representations needed for
//! hybrid search (tokens, embeddings, expanded terms, and mode detection).

use crate::embedding::error::EmbeddingError;
use crate::embedding::EmbeddingService;
use crate::search::expander::QueryExpander;
use crate::search::tokenizer::Tokenizer;
use crate::search::types::{ProcessedQuery, SearchMode};
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// Query processor that transforms raw queries into multi-faceted representations.
///
/// The QueryProcessor orchestrates:
/// 1. Tokenization (FTS-compatible)
/// 2. Embedding generation (for vector similarity)
/// 3. Query expansion (synonyms and related terms)
/// 4. Search mode detection (Code/Text/Auto)
///
/// Processing is parallelized using tokio::join! for optimal performance.
pub struct QueryProcessor {
    /// Tokenizer for FTS-compatible tokenization
    tokenizer: Arc<Tokenizer>,
    /// Embedding service for query vector generation
    embedder: Arc<EmbeddingService>,
    /// Query expander for synonym expansion
    expander: Arc<QueryExpander>,
}

impl QueryProcessor {
    /// Create a new QueryProcessor with the given embedding service.
    pub fn new(embedder: Arc<EmbeddingService>) -> Self {
        Self {
            tokenizer: Arc::new(Tokenizer::new()),
            embedder,
            expander: Arc::new(QueryExpander::new()),
        }
    }

    /// Create a QueryProcessor with custom components.
    pub fn with_components(
        tokenizer: Tokenizer,
        embedder: Arc<EmbeddingService>,
        expander: QueryExpander,
    ) -> Self {
        Self {
            tokenizer: Arc::new(tokenizer),
            embedder,
            expander: Arc::new(expander),
        }
    }

    /// Process a query into all representations needed for hybrid search.
    ///
    /// This method runs tokenization, embedding, and expansion in parallel
    /// for optimal performance, then detects the appropriate search mode.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maproom::search::QueryProcessor;
    /// use maproom::embedding::EmbeddingService;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let embedder = Arc::new(EmbeddingService::from_env()?);
    ///     let processor = QueryProcessor::new(embedder);
    ///
    ///     let processed = processor.process("authenticate user").await?;
    ///     println!("Tokens: {:?}", processed.tokens);
    ///     println!("Mode: {:?}", processed.mode);
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(query_len = query.len()))]
    pub async fn process(&self, query: &str) -> Result<ProcessedQuery, QueryProcessorError> {
        if query.is_empty() {
            return Err(QueryProcessorError::EmptyQuery);
        }

        info!("Processing query: '{}'", query);

        // Detect mode first (synchronous, cheap operation)
        let mode = self.detect_mode(query);
        debug!("Detected search mode: {:?}", mode);

        // Clone for parallel tasks
        let query_for_tokenize = query.to_string();
        let query_for_embed = query.to_string();
        let tokenizer = Arc::clone(&self.tokenizer);
        let embedder = Arc::clone(&self.embedder);
        let expander = Arc::clone(&self.expander);

        // Parallel processing: tokenization and embedding
        let (tokens, embedding_result) = tokio::join!(
            async move { tokenizer.tokenize_async(&query_for_tokenize).await },
            async move { embedder.embed_text(&query_for_embed).await }
        );

        let embedding = embedding_result.map_err(QueryProcessorError::Embedding)?;

        // Expand based on tokens (can reuse tokens, no need to re-tokenize)
        let expanded_terms = expander.expand_async(&tokens).await;

        debug!(
            "Processed query: {} tokens, {} expanded terms, mode: {:?}",
            tokens.len(),
            expanded_terms.len(),
            mode
        );

        Ok(ProcessedQuery::new(
            query.to_string(),
            tokens,
            embedding,
            expanded_terms,
            mode,
        ))
    }

    /// Detect the appropriate search mode based on query heuristics.
    ///
    /// Detection logic:
    /// - **Code mode**: Contains code operators (::, ->, etc.) or is a short identifier
    /// - **Text mode**: Natural language query with 4+ words
    /// - **Auto mode**: Ambiguous queries (2-3 words without clear code patterns)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maproom::search::{QueryProcessor, SearchMode};
    /// use maproom::embedding::EmbeddingService;
    /// use std::sync::Arc;
    ///
    /// let embedder = Arc::new(EmbeddingService::from_env().unwrap());
    /// let processor = QueryProcessor::new(embedder);
    ///
    /// // Code patterns
    /// assert_eq!(processor.detect_mode("User::authenticate"), SearchMode::Code);
    /// assert_eq!(processor.detect_mode("array->length"), SearchMode::Code);
    ///
    /// // Natural language
    /// assert_eq!(processor.detect_mode("how to authenticate a user"), SearchMode::Text);
    ///
    /// // Ambiguous
    /// assert_eq!(processor.detect_mode("user auth"), SearchMode::Auto);
    /// ```
    pub fn detect_mode(&self, query: &str) -> SearchMode {
        // Code pattern indicators
        let code_patterns = [
            "::", // Rust, C++ namespace
            "->", // Arrow/pointer
            "=>", // Fat arrow
            "<-", // R assignment
            "!=", // Not equal
            "==", // Equality
            "<=", // Less than or equal
            ">=", // Greater than or equal
        ];

        // Check for code operators
        for pattern in &code_patterns {
            if query.contains(pattern) {
                debug!("Code mode: contains pattern '{}'", pattern);
                return SearchMode::Code;
            }
        }

        // Check for function call patterns
        if query.contains('(') && query.contains(')') {
            debug!("Code mode: contains parentheses (function call pattern)");
            return SearchMode::Code;
        }

        // Count words
        let word_count = query.split_whitespace().count();

        // Check for camelCase or snake_case (code identifier patterns)
        let has_code_naming = query.chars().any(|c| c == '_')
            || (query.chars().any(|c| c.is_uppercase())
                && query.chars().any(|c| c.is_lowercase())
                && word_count <= 2);

        if word_count == 1 || (word_count == 2 && has_code_naming) {
            debug!("Code mode: short identifier (words: {})", word_count);
            return SearchMode::Code;
        }

        // Natural language queries (4+ words)
        if word_count > 3 {
            debug!("Text mode: natural language (words: {})", word_count);
            return SearchMode::Text;
        }

        // Ambiguous: 2-3 words without clear code patterns
        debug!("Auto mode: ambiguous query (words: {})", word_count);
        SearchMode::Auto
    }

    /// Validate a query before processing.
    ///
    /// Checks:
    /// - Not empty
    /// - Not too long (> 1000 chars)
    /// - Contains meaningful content (not just whitespace/punctuation)
    pub fn validate_query(&self, query: &str) -> Result<(), QueryProcessorError> {
        if query.is_empty() || query.trim().is_empty() {
            return Err(QueryProcessorError::EmptyQuery);
        }

        if query.len() > 1000 {
            return Err(QueryProcessorError::QueryTooLong(query.len()));
        }

        // Check for meaningful content (at least one alphanumeric character)
        if !query.chars().any(|c| c.is_alphanumeric()) {
            return Err(QueryProcessorError::NoMeaningfulContent);
        }

        Ok(())
    }

    /// Process a query with validation.
    pub async fn process_validated(
        &self,
        query: &str,
    ) -> Result<ProcessedQuery, QueryProcessorError> {
        self.validate_query(query)?;
        self.process(query).await
    }

    /// Get references to internal components for testing.
    #[cfg(test)]
    pub fn components(&self) -> (&Tokenizer, &EmbeddingService, &QueryExpander) {
        (&self.tokenizer, &self.embedder, &self.expander)
    }
}

/// Errors that can occur during query processing.
#[derive(Debug, thiserror::Error)]
pub enum QueryProcessorError {
    /// Empty or whitespace-only query
    #[error("Query is empty or contains only whitespace")]
    EmptyQuery,

    /// Query is too long
    #[error("Query is too long: {0} characters (max 1000)")]
    QueryTooLong(usize),

    /// Query contains no meaningful content
    #[error("Query contains no meaningful content (alphanumeric characters)")]
    NoMeaningfulContent,

    /// Embedding generation failed
    #[error("Embedding generation failed: {0}")]
    Embedding(#[from] EmbeddingError),

    /// Generic error
    #[error("Query processing error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::cache::EmbeddingCache;
    use crate::embedding::client::OpenAIClient;
    use crate::embedding::EmbeddingConfig;

    fn test_embedder() -> Arc<EmbeddingService> {
        // Use a test config with API key for OpenAI provider
        let mut config = EmbeddingConfig::default();
        config.api_key = Some("test-key".to_string());

        // Create provider and cache
        let provider = Box::new(OpenAIClient::new(config.clone()).unwrap());
        let cache = Arc::new(EmbeddingCache::new(config.cache.clone()).unwrap());

        Arc::new(EmbeddingService::new(provider, cache))
    }

    #[test]
    fn test_mode_detection_code() {
        let processor = QueryProcessor::new(test_embedder());

        // Code operators
        assert_eq!(
            processor.detect_mode("User::authenticate"),
            SearchMode::Code
        );
        assert_eq!(processor.detect_mode("array->length"), SearchMode::Code);
        assert_eq!(processor.detect_mode("a => b"), SearchMode::Code);
        assert_eq!(processor.detect_mode("x != y"), SearchMode::Code);

        // Function calls
        assert_eq!(processor.detect_mode("console.log()"), SearchMode::Code);

        // Short identifiers
        assert_eq!(processor.detect_mode("authenticate"), SearchMode::Code);
        assert_eq!(processor.detect_mode("user_name"), SearchMode::Code);
        assert_eq!(processor.detect_mode("UserAuth"), SearchMode::Code);
    }

    #[test]
    fn test_mode_detection_text() {
        let processor = QueryProcessor::new(test_embedder());

        // Natural language queries
        assert_eq!(
            processor.detect_mode("how to authenticate a user"),
            SearchMode::Text
        );
        assert_eq!(
            processor.detect_mode("find all authentication functions"),
            SearchMode::Text
        );
        assert_eq!(
            processor.detect_mode("what is the login process"),
            SearchMode::Text
        );
    }

    #[test]
    fn test_mode_detection_auto() {
        let processor = QueryProcessor::new(test_embedder());

        // Ambiguous queries (2-3 words)
        assert_eq!(
            processor.detect_mode("user authentication"),
            SearchMode::Auto
        );
        assert_eq!(
            processor.detect_mode("login error handler"),
            SearchMode::Auto
        );
    }

    #[test]
    fn test_query_validation() {
        let processor = QueryProcessor::new(test_embedder());

        // Valid queries
        assert!(processor.validate_query("hello world").is_ok());
        assert!(processor.validate_query("User::authenticate").is_ok());

        // Invalid queries
        assert!(matches!(
            processor.validate_query(""),
            Err(QueryProcessorError::EmptyQuery)
        ));
        assert!(matches!(
            processor.validate_query("   "),
            Err(QueryProcessorError::EmptyQuery)
        ));

        // Too long
        let long_query = "a".repeat(1001);
        assert!(matches!(
            processor.validate_query(&long_query),
            Err(QueryProcessorError::QueryTooLong(_))
        ));

        // No meaningful content
        assert!(matches!(
            processor.validate_query("!!!???"),
            Err(QueryProcessorError::NoMeaningfulContent)
        ));
    }

    #[test]
    fn test_processor_creation() {
        let embedder = test_embedder();
        let processor = QueryProcessor::new(embedder);

        // Verify components are initialized
        let (tokenizer, _, expander) = processor.components();
        assert!(tokenizer.tokenize("test").len() > 0);
        assert!(expander.get_synonyms("auth").is_some());
    }

    #[test]
    fn test_custom_components() {
        let embedder = test_embedder();
        let tokenizer = Tokenizer::new();
        let expander = QueryExpander::new();

        let processor = QueryProcessor::with_components(tokenizer, embedder, expander);

        // Verify custom components work
        let mode = processor.detect_mode("test query");
        assert!(matches!(mode, SearchMode::Auto | SearchMode::Text));
    }

    // Note: Full integration tests with async processing are in integration tests
    // to avoid needing real embedding API calls in unit tests
}
