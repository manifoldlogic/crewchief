//! Search module for hybrid retrieval system.
//!
//! This module implements the query processing pipeline for hybrid search,
//! combining full-text search (FTS), vector similarity, and graph signals.
//!
//! # Architecture
//!
//! The search pipeline consists of:
//!
//! 1. **Query Processing** (`query_processor`):
//!    - Tokenization for FTS compatibility
//!    - Embedding generation for vector search
//!    - Query expansion with synonyms
//!    - Search mode detection (Code/Text/Auto)
//!
//! 2. **Search Execution** (future modules):
//!    - FTS query execution
//!    - Vector similarity search
//!    - Graph-based ranking
//!    - Score fusion
//!
//! # Examples
//!
//! ## Basic Query Processing
//!
//! ```no_run
//! use crewchief_maproom::search::{QueryProcessor, SearchMode};
//! use crewchief_maproom::embedding::EmbeddingService;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize embedding service
//!     let embedder = Arc::new(EmbeddingService::from_env()?);
//!
//!     // Create query processor
//!     let processor = QueryProcessor::new(embedder);
//!
//!     // Process a query
//!     let query = "authenticate user with OAuth";
//!     let processed = processor.process(query).await?;
//!
//!     println!("Original: {}", processed.original);
//!     println!("Tokens: {:?}", processed.tokens);
//!     println!("Expanded terms: {:?}", processed.expanded_terms);
//!     println!("Mode: {:?}", processed.mode);
//!     println!("FTS query: {}", processed.fts_query_string());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Custom Components
//!
//! ```no_run
//! use crewchief_maproom::search::{QueryProcessor, Tokenizer, QueryExpander};
//! use crewchief_maproom::embedding::EmbeddingService;
//! use std::sync::Arc;
//! use std::collections::{HashMap, HashSet};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Custom tokenizer with specific stop words
//!     let mut stop_words = HashSet::new();
//!     stop_words.insert("custom".to_string());
//!     let tokenizer = Tokenizer::with_stop_words(stop_words);
//!
//!     // Custom expander with domain-specific synonyms
//!     let mut synonyms = HashMap::new();
//!     synonyms.insert("oauth".to_string(), vec!["openid".to_string(), "saml".to_string()]);
//!     let expander = QueryExpander::with_synonyms(synonyms);
//!
//!     // Embedding service
//!     let embedder = Arc::new(EmbeddingService::from_env()?);
//!
//!     // Create processor with custom components
//!     let processor = QueryProcessor::with_components(tokenizer, embedder, expander);
//!
//!     let processed = processor.process("oauth authentication").await?;
//!     println!("Processed query: {:?}", processed.tokens);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Search Mode Detection
//!
//! ```no_run
//! use crewchief_maproom::search::{QueryProcessor, SearchMode};
//! use crewchief_maproom::embedding::EmbeddingService;
//! use std::sync::Arc;
//!
//! let embedder = Arc::new(EmbeddingService::from_env().unwrap());
//! let processor = QueryProcessor::new(embedder);
//!
//! // Code queries
//! assert_eq!(processor.detect_mode("User::authenticate"), SearchMode::Code);
//! assert_eq!(processor.detect_mode("array->map"), SearchMode::Code);
//!
//! // Natural language queries
//! assert_eq!(processor.detect_mode("how to handle authentication errors"), SearchMode::Text);
//!
//! // Ambiguous queries
//! assert_eq!(processor.detect_mode("user auth"), SearchMode::Auto);
//! ```

pub mod expander;
pub mod query_processor;
pub mod tokenizer;
pub mod types;

// Re-export main types for convenience
pub use expander::QueryExpander;
pub use query_processor::{QueryProcessor, QueryProcessorError};
pub use tokenizer::Tokenizer;
pub use types::{ProcessedQuery, SearchMode};
