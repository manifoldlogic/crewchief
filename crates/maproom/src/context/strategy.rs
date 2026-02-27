//! Assembly strategy trait for language-specific context bundling.
//!
//! This module defines the core abstraction for context assembly strategies.
//! Different programming languages have different patterns for code organization,
//! testing, and dependencies, so we use the strategy pattern to customize
//! context selection based on the language being worked with.

use anyhow::Result;
use async_trait::async_trait;

use super::types::{ContextBundle, ExpandOptions};

/// Strategy for assembling context bundles with language-specific intelligence.
///
/// Each strategy implementation decides:
/// - How to allocate the token budget across context pieces
/// - What related code to include (tests, dependencies, configs)
/// - How to detect and include language-specific patterns
/// - How to prioritize context pieces when budget is limited
///
/// # Examples
///
/// ```no_run
/// use maproom::context::strategy::AssemblyStrategy;
/// use maproom::context::types::{ContextBundle, ExpandOptions};
/// use maproom::db::create_pool;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pool = create_pool().await?;
///
///     // Use the default strategy
///     let strategy = maproom::context::strategies::DefaultAssemblyStrategy::new(pool);
///
///     let bundle = strategy.assemble(
///         12345,           // chunk_id
///         6000,            // budget_tokens
///         ExpandOptions::with_common()
///     ).await?;
///
///     println!("Assembled {} items, {} tokens", bundle.items.len(), bundle.total_tokens);
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait AssemblyStrategy: Send + Sync {
    /// Assemble a context bundle for the specified chunk.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - The ID of the primary chunk to assemble context for
    /// * `budget` - Maximum number of tokens allowed in the bundle
    /// * `options` - Options for expanding context beyond the primary chunk
    ///
    /// # Returns
    ///
    /// A ContextBundle containing the primary chunk and related context
    /// that fits within the token budget.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Chunk ID is not found in the database
    /// - Database query fails
    /// - File cannot be read
    /// - Token counting fails
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        options: ExpandOptions,
    ) -> Result<ContextBundle>;
}
