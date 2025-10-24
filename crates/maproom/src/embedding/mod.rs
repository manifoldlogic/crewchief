//! Embedding service module for generating and caching text embeddings.
//!
//! This module provides a complete embedding service infrastructure with:
//! - OpenAI API integration (text-embedding-3-small)
//! - LRU caching with TTL support
//! - Retry logic with exponential backoff
//! - Batch processing for efficient bulk operations
//! - Cost tracking and metrics
//! - Extensible provider architecture
//!
//! # Examples
//!
//! ```no_run
//! use crewchief_maproom::embedding::{EmbeddingService, EmbeddingConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create service from environment variables
//!     let service = EmbeddingService::from_env()?;
//!
//!     // Embed a single text
//!     let embedding = service.embed_text("Hello, world!").await?;
//!     println!("Embedding dimension: {}", embedding.len());
//!
//!     // Embed a batch of texts
//!     let texts = vec![
//!         "First text".to_string(),
//!         "Second text".to_string(),
//!     ];
//!     let embeddings = service.embed_batch(texts).await?;
//!     println!("Generated {} embeddings", embeddings.len());
//!
//!     // Get metrics
//!     let cache_metrics = service.cache_metrics().await;
//!     println!("Cache hit rate: {:.1}%", cache_metrics.hit_rate() * 100.0);
//!
//!     let cost_metrics = service.cost_metrics();
//!     println!("Estimated cost: ${:.4}", cost_metrics.estimated_cost_usd());
//!
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod client;
pub mod config;
pub mod error;
pub mod service;

// Re-export main types for convenience
pub use cache::{CacheMetrics, EmbeddingCache, Vector};
pub use client::{CostMetrics, OpenAIClient};
pub use config::{CacheConfig, EmbeddingConfig, Provider, RetryConfig};
pub use error::{ApiError, CacheError, ConfigError, EmbeddingError};
pub use service::{BatchResult, EmbeddingService};
