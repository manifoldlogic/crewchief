//! Context assembler implementation for building intelligent code context bundles.

use anyhow::{Context as AnyhowContext, Result};
use tracing::{debug, warn};

use super::file_loader::FileLoader;
use super::token_counter::TokenCounter;
use super::types::{ContextBundle, ContextItem, ExpandOptions, LineRange};
use crate::db::PgPool;

/// Chunk metadata retrieved from the database.
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub id: i64,
    pub file_relpath: String,
    pub worktree_path: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub signature: Option<String>,
    pub docstring: Option<String>,
}

/// Trait for assembling context bundles from chunk IDs.
///
/// Implementations are responsible for:
/// - Retrieving chunk metadata from the database
/// - Loading file content from the filesystem
/// - Counting tokens accurately
/// - Assembling a ContextBundle within the specified budget
#[async_trait::async_trait]
pub trait ContextAssembler: Send + Sync {
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
    /// A ContextBundle containing the primary chunk and any related context
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

/// Basic context assembler that retrieves and formats a single chunk.
///
/// This is the foundational implementation that:
/// - Queries the database for chunk metadata
/// - Loads file content from the worktree
/// - Extracts the specified line range
/// - Counts tokens accurately
/// - Returns a simple ContextBundle with just the primary chunk
///
/// Future implementations will add:
/// - Relationship traversal (callers, callees, tests)
/// - Budget allocation across multiple items
/// - Truncation strategies for large chunks
/// - Priority-based context selection
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::context::{BasicContextAssembler, ContextAssembler, ExpandOptions};
/// use crewchief_maproom::db::create_pool;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pool = create_pool().await?;
///     let assembler = BasicContextAssembler::new(pool);
///
///     let bundle = assembler.assemble(
///         12345,
///         6000,
///         ExpandOptions::primary_only()
///     ).await?;
///
///     println!("Assembled {} items, {} tokens", bundle.items.len(), bundle.total_tokens);
///     Ok(())
/// }
/// ```
pub struct BasicContextAssembler {
    pool: PgPool,
    token_counter: TokenCounter,
}

impl BasicContextAssembler {
    /// Create a new basic context assembler.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            token_counter: TokenCounter::new(),
        }
    }

    /// Retrieve chunk metadata from the database by ID.
    async fn get_chunk_metadata(&self, chunk_id: i64) -> Result<ChunkMetadata> {
        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let row = client
            .query_opt(
                "SELECT
                    c.id,
                    f.relpath,
                    w.abs_path as worktree_path,
                    c.symbol_name,
                    c.kind::text,
                    c.start_line,
                    c.end_line,
                    c.signature,
                    c.docstring
                FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                LEFT JOIN maproom.worktrees w ON w.id = f.worktree_id
                WHERE c.id = $1",
                &[&chunk_id],
            )
            .await
            .context("Failed to query chunk metadata")?;

        let row = row.ok_or_else(|| anyhow::anyhow!("Chunk not found: {}", chunk_id))?;

        Ok(ChunkMetadata {
            id: row.get(0),
            file_relpath: row.get(1),
            worktree_path: row.get::<_, Option<String>>(2).unwrap_or_else(|| {
                warn!("Chunk {} has no worktree_path, using empty string", chunk_id);
                String::new()
            }),
            symbol_name: row.get(3),
            kind: row.get(4),
            start_line: row.get(5),
            end_line: row.get(6),
            signature: row.get(7),
            docstring: row.get(8),
        })
    }

    /// Create a ContextItem from chunk metadata.
    async fn create_context_item(
        &self,
        metadata: ChunkMetadata,
        role: &str,
        reason: &str,
    ) -> Result<ContextItem> {
        // Load file content
        let file_loader = FileLoader::new(&metadata.worktree_path);
        let range = LineRange::new(metadata.start_line, metadata.end_line);

        let content = file_loader
            .load_range(&metadata.file_relpath, range)
            .await
            .with_context(|| {
                format!(
                    "Failed to load file content: {} (lines {}-{})",
                    metadata.file_relpath, metadata.start_line, metadata.end_line
                )
            })?;

        // Count tokens
        let tokens = self
            .token_counter
            .count(&content)
            .context("Failed to count tokens")?;

        debug!(
            "Created context item: {} lines {}-{}, {} tokens",
            metadata.file_relpath, metadata.start_line, metadata.end_line, tokens
        );

        Ok(ContextItem {
            relpath: metadata.file_relpath,
            range,
            role: role.to_string(),
            reason: reason.to_string(),
            content,
            tokens,
        })
    }
}

#[async_trait::async_trait]
impl ContextAssembler for BasicContextAssembler {
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        _options: ExpandOptions,
    ) -> Result<ContextBundle> {
        debug!(
            "Assembling context for chunk {} with budget {} tokens",
            chunk_id, budget
        );

        // Retrieve chunk metadata
        let metadata = self
            .get_chunk_metadata(chunk_id)
            .await
            .context("Failed to retrieve chunk metadata")?;

        // Create context item for the primary chunk
        let reason = if let Some(ref name) = metadata.symbol_name {
            format!("Primary chunk: {} ({})", name, metadata.kind)
        } else {
            format!("Primary chunk ({})", metadata.kind)
        };

        let item = self
            .create_context_item(metadata, "primary", &reason)
            .await
            .context("Failed to create context item")?;

        // Check if it fits within budget
        let mut bundle = ContextBundle::new();
        if item.tokens > budget {
            warn!(
                "Primary chunk ({} tokens) exceeds budget ({} tokens), truncating",
                item.tokens, budget
            );
            bundle.truncated = true;
            // TODO: Implement intelligent truncation in future ticket
            // For now, include it anyway and mark as truncated
        }

        bundle.add_item(item);

        debug!(
            "Assembled context bundle: {} items, {} tokens, truncated: {}",
            bundle.items.len(),
            bundle.total_tokens,
            bundle.truncated
        );

        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_metadata_creation() {
        let metadata = ChunkMetadata {
            id: 1,
            file_relpath: "src/main.rs".to_string(),
            worktree_path: "/workspace".to_string(),
            symbol_name: Some("main".to_string()),
            kind: "func".to_string(),
            start_line: 1,
            end_line: 10,
            signature: Some("fn main()".to_string()),
            docstring: None,
        };

        assert_eq!(metadata.id, 1);
        assert_eq!(metadata.file_relpath, "src/main.rs");
        assert_eq!(metadata.symbol_name, Some("main".to_string()));
    }

    // Note: Database integration tests are in tests/context/assembler_test.rs
}
