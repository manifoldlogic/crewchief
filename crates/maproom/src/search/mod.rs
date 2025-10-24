//! Search module for hybrid retrieval system.
//!
//! This module implements the complete query processing and search execution pipeline
//! for hybrid search, combining full-text search (FTS), vector similarity, graph signals,
//! and temporal signals.
//!
//! # Architecture
//!
//! The search pipeline consists of two main stages:
//!
//! 1. **Query Processing** (`query_processor`):
//!    - Tokenization for FTS compatibility
//!    - Embedding generation for vector search
//!    - Query expansion with synonyms
//!    - Search mode detection (Code/Text/Auto)
//!
//! 2. **Search Execution** (`executors`):
//!    - FTS query execution with ts_rank_cd
//!    - Vector similarity search using pgvector
//!    - Graph-based importance from chunk_edges
//!    - Temporal signal scoring (recency/churn)
//!    - Parallel execution using tokio::join!
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
//! ## Parallel Search Execution
//!
//! ```no_run
//! use crewchief_maproom::search::{QueryProcessor, SearchExecutors};
//! use crewchief_maproom::embedding::EmbeddingService;
//! use std::sync::Arc;
//! use tokio_postgres::NoTls;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to database
//!     let (client, connection) = tokio_postgres::connect(
//!         "host=localhost user=postgres dbname=maproom",
//!         NoTls,
//!     ).await?;
//!     tokio::spawn(async move { connection.await });
//!
//!     // Initialize components
//!     let embedder = Arc::new(EmbeddingService::from_env()?);
//!     let processor = QueryProcessor::new(embedder);
//!     let executors = SearchExecutors::new(client);
//!
//!     // Process query
//!     let processed = processor.process("authenticate user").await?;
//!
//!     // Execute all searches in parallel
//!     let results = executors.execute_all(&processed, 1, None, 10).await?;
//!
//!     println!("Search completed: {}", results.summary());
//!     println!("FTS results: {}", results.fts.len());
//!     println!("Vector results: {}", results.vector.len());
//!     println!("Graph results: {}", results.graph.len());
//!     println!("Signal results: {}", results.signals.len());
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

// Query processing modules
pub mod expander;
pub mod query_processor;
pub mod tokenizer;
pub mod types;

// Search execution modules
pub mod executor_types;
pub mod executors;
pub mod fts;
pub mod graph;
pub mod signals;
pub mod vector;

// Search pipeline modules (Phase 2)
pub mod fusion;
pub mod pipeline;
pub mod results;

// Re-export main types for convenience
pub use expander::QueryExpander;
pub use query_processor::{QueryProcessor, QueryProcessorError};
pub use tokenizer::Tokenizer;
pub use types::{ProcessedQuery, SearchMode};

// Re-export executor types
pub use executor_types::{RankedResult, RankedResults, SearchSource};
pub use executors::{ExecutorError, SearchExecutors, SearchResults};
pub use fts::{FTSError, FTSExecutor};
pub use graph::{GraphError, GraphExecutor};
pub use signals::{SignalError, SignalExecutor, SignalWeights};
pub use vector::{VectorError, VectorExecutor};

// Re-export pipeline types (Phase 2 + Phase 3)
pub use fusion::{BasicWeightedFusion, FusedResult, FusionWeights, RRFFusion, ScoreFusion};
pub use pipeline::{PipelineError, SearchPipeline};
pub use results::{
    ChunkSearchResult, FinalSearchResults, QueryProcessingDetails, SearchMetadata, SearchOptions,
    SearchTiming,
};
