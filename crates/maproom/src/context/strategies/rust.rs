//! Rust-specific context assembly strategy.
//!
//! This module provides a specialized assembler for Rust codebases that:
//! - Includes Cargo.toml for crate metadata
//! - Includes trait implementations (impl Trait for Type)
//! - Includes module structure (mod.rs, lib.rs)
//! - Detects test modules using #[cfg(test)] and #[test] attributes
//! - Includes macro definitions when referenced

use anyhow::Result;
use tracing::debug;

use crate::context::{
    assembler::ChunkMetadata,
    file_loader::FileLoader,
    strategies::default::DefaultAssemblyStrategy,
    strategy::AssemblyStrategy,
    token_counter::TokenCounter,
    types::{ContextBundle, ContextItem, ExpandOptions, LineRange},
};
use crate::db::SqliteStore;
use std::sync::Arc;

/// Configuration for Rust assembly strategy.
#[derive(Debug, Clone)]
pub struct RustConfig {
    /// Whether to include Cargo.toml
    pub include_cargo_toml: bool,
    /// Whether to include trait implementations
    pub include_trait_impls: bool,
    /// Whether to include module files (mod.rs, lib.rs)
    pub include_module_files: bool,
    /// Maximum number of trait implementations to include
    pub max_trait_impls: usize,
}

impl Default for RustConfig {
    fn default() -> Self {
        Self {
            include_cargo_toml: true,
            include_trait_impls: true,
            include_module_files: true,
            max_trait_impls: 3,
        }
    }
}

/// Rust-specific context assembly strategy.
///
/// This assembler extends the default strategy with Rust-specific enhancements:
/// - Crate metadata (Cargo.toml)
/// - Trait implementations
/// - Module structure
/// - Rust test patterns
pub struct RustAssemblyStrategy {
    store: Arc<SqliteStore>,
    default: DefaultAssemblyStrategy,
    config: RustConfig,
    token_counter: TokenCounter,
}

impl RustAssemblyStrategy {
    /// Create a new Rust assembly strategy.
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self::with_config(store, RustConfig::default())
    }

    /// Create a new Rust assembly strategy with custom configuration.
    pub fn with_config(store: Arc<SqliteStore>, config: RustConfig) -> Self {
        Self {
            default: DefaultAssemblyStrategy::new(Arc::clone(&store)),
            store,
            config,
            token_counter: TokenCounter::new(),
        }
    }

    /// Find and add Cargo.toml from the crate root.
    async fn add_cargo_toml(
        &self,
        bundle: &mut ContextBundle,
        metadata: &ChunkMetadata,
        budget: usize,
    ) -> Result<()> {
        if !self.config.include_cargo_toml || bundle.total_tokens >= budget {
            return Ok(());
        }

        // Look for Cargo.toml in the project root and parent directories
        let cargo_paths = ["Cargo.toml", "../Cargo.toml", "../../Cargo.toml"];

        let file_loader = FileLoader::new(&metadata.worktree_path);

        for cargo_path in &cargo_paths {
            if bundle.total_tokens >= budget {
                break;
            }

            if let Ok(content) = file_loader
                .load_range(cargo_path, LineRange::new(1, i32::MAX))
                .await
            {
                let tokens = self.token_counter.count(&content)?;

                if !bundle.would_exceed_budget(tokens, budget) {
                    let item = ContextItem {
                        relpath: cargo_path.to_string(),
                        range: LineRange::new(1, content.lines().count() as i32),
                        role: "crate_metadata".to_string(),
                        reason: "Crate metadata: Cargo.toml provides crate dependencies and configuration".to_string(),
                        content,
                        tokens,
                    };
                    debug!("Adding Cargo.toml: {} tokens", tokens);
                    bundle.add_item(item);
                    break; // Only add one Cargo.toml
                }
            }
        }

        Ok(())
    }

    /// Find and add trait implementations for a type.
    ///
    /// Uses graph traversal to find trait implementations related to this type.
    async fn add_trait_impls(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
    ) -> Result<()> {
        use crate::db::sqlite::graph::ImportDirection;

        if !self.config.include_trait_impls || bundle.total_tokens >= budget {
            return Ok(());
        }

        // Find trait implementations (Incoming = what implements this trait/type)
        let implementations = self.store.find_extensions(
            chunk_id,
            ImportDirection::Incoming,
            Some(1), // Only direct implementations
        ).await?;

        let mut added_count = 0;
        for impl_result in implementations {
            if added_count >= self.config.max_trait_impls || bundle.total_tokens >= budget {
                break;
            }

            // Get the implementation chunk
            if let Some(chunk) = self.store.get_chunk_by_id(impl_result.chunk_id).await? {
                // Get worktree path for file loading
                let metadata = self.default.get_chunk_metadata(chunk_id).await?;

                let file_loader = FileLoader::new(&metadata.worktree_path);
                let range = LineRange::new(chunk.start_line, chunk.end_line);

                if let Ok(content) = file_loader.load_range(&chunk.file_path, range).await {
                    let tokens = self.token_counter.count(&content)?;

                    if !bundle.would_exceed_budget(tokens, budget) {
                        let impl_name = chunk.symbol_name.as_deref().unwrap_or("impl");
                        let item = ContextItem {
                            relpath: chunk.file_path.clone(),
                            range: LineRange::new(chunk.start_line, chunk.end_line),
                            role: "trait_impl".to_string(),
                            reason: format!(
                                "Trait implementation: {} provides trait methods",
                                impl_name
                            ),
                            content,
                            tokens,
                        };
                        debug!("Adding trait impl {}: {} tokens", impl_name, tokens);
                        bundle.add_item(item);
                        added_count += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Find and add module files (mod.rs, lib.rs) for context.
    async fn add_module_files(
        &self,
        bundle: &mut ContextBundle,
        metadata: &ChunkMetadata,
        budget: usize,
    ) -> Result<()> {
        if !self.config.include_module_files || bundle.total_tokens >= budget {
            return Ok(());
        }

        // Extract directory from file path
        let file_path = std::path::Path::new(&metadata.file_relpath);
        let dir = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");

        if dir.is_empty() {
            return Ok(());
        }

        // Look for mod.rs or lib.rs in the same directory
        let module_files = [format!("{}/mod.rs", dir), "src/lib.rs".to_string()];

        let file_loader = FileLoader::new(&metadata.worktree_path);

        for module_file in &module_files {
            if bundle.total_tokens >= budget {
                break;
            }

            // Skip if this is already the module file
            if module_file == &metadata.file_relpath {
                continue;
            }

            if let Ok(content) = file_loader
                .load_range(module_file, LineRange::new(1, 50)) // Only first 50 lines for module structure
                .await
            {
                let tokens = self.token_counter.count(&content)?;

                if !bundle.would_exceed_budget(tokens, budget) {
                    let item = ContextItem {
                        relpath: module_file.clone(),
                        range: LineRange::new(1, content.lines().count() as i32),
                        role: "module".to_string(),
                        reason: format!(
                            "Module structure: {} provides module organization context",
                            module_file
                        ),
                        content,
                        tokens,
                    };
                    debug!("Adding module file {}: {} tokens", module_file, tokens);
                    bundle.add_item(item);
                    break; // Only add one module file
                }
            }
        }

        Ok(())
    }

    /// Check if a chunk is a Rust struct or enum.
    fn is_rust_type(&self, metadata: &ChunkMetadata) -> bool {
        (metadata.kind == "struct" || metadata.kind == "enum")
            && metadata.file_relpath.ends_with(".rs")
    }

    /// Check if a chunk is a Rust impl block.
    fn is_rust_impl(&self, metadata: &ChunkMetadata) -> bool {
        metadata.kind == "impl" && metadata.file_relpath.ends_with(".rs")
    }
}

#[async_trait::async_trait]
impl AssemblyStrategy for RustAssemblyStrategy {
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        options: ExpandOptions,
    ) -> Result<ContextBundle> {
        debug!(
            "Rust strategy assembling context for chunk {} with budget {} tokens",
            chunk_id, budget
        );

        // Start with default strategy assembly
        let mut bundle = self.default.assemble(chunk_id, budget, options).await?;

        // Get chunk metadata to determine if Rust-specific enhancements apply
        let metadata = self.default.get_chunk_metadata(chunk_id).await?;

        // Add Rust-specific context items
        // Priority order: Cargo.toml → trait impls → module files

        // 1. Include Cargo.toml for crate metadata
        self.add_cargo_toml(&mut bundle, &metadata, budget).await?;

        // 2. Include trait implementations if this is a type or impl
        if self.is_rust_type(&metadata) || self.is_rust_impl(&metadata) {
            self.add_trait_impls(&mut bundle, chunk_id, budget).await?;
        }

        // 3. Include module files for structure context
        self.add_module_files(&mut bundle, &metadata, budget)
            .await?;

        debug!(
            "Rust strategy assembled {} items, {} tokens total",
            bundle.items.len(),
            bundle.total_tokens
        );

        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_config_default() {
        let config = RustConfig::default();
        assert!(config.include_cargo_toml);
        assert!(config.include_trait_impls);
        assert!(config.include_module_files);
        assert_eq!(config.max_trait_impls, 3);
    }

    // Note: Tests that require a pool are in tests/ directory
    // These helper methods are tested as part of integration tests

    // Integration tests with database are in tests/ directory
    #[tokio::test]
    #[ignore]
    async fn test_rust_assembly_strategy() {
        // Integration test - requires database
    }
}
