//! React-specific context assembly strategy.
//!
//! This module provides a specialized assembler for React codebases that:
//! - Detects React components via naming conventions
//! - Includes route definitions for components
//! - Discovers and includes hooks (built-in and custom)
//! - Handles JSX parent/child relationships
//! - Applies React-specific budget allocation

use anyhow::{Context as AnyhowContext, Result};
use tracing::debug;

use crate::context::{
    assembler::{BasicContextAssembler, ChunkMetadata, ContextAssembler},
    detectors::{ComponentDetector, HookDetector, JsxRelationshipDetector},
    file_loader::FileLoader,
    token_counter::TokenCounter,
    types::{ContextBundle, ContextItem, ExpandOptions, LineRange},
};
use crate::db::SqliteStore;
use std::sync::Arc;

/// Configuration for React assembly strategy.
#[derive(Debug, Clone)]
pub struct ReactConfig {
    /// Whether to include route definitions
    pub include_routes: bool,
    /// Whether to include hooks
    pub include_hooks: bool,
    /// Whether to include JSX parent components
    pub include_jsx_parents: bool,
    /// Whether to include JSX child components
    pub include_jsx_children: bool,
    /// Maximum number of hooks to include
    pub max_hooks: usize,
    /// Maximum number of JSX parents to include
    pub max_jsx_parents: usize,
    /// Maximum number of JSX children to include
    pub max_jsx_children: usize,
}

impl Default for ReactConfig {
    fn default() -> Self {
        Self {
            include_routes: true,
            include_hooks: true,
            include_jsx_parents: true,
            include_jsx_children: true,
            max_hooks: 5,
            max_jsx_parents: 2,
            max_jsx_children: 5,
        }
    }
}

impl ReactConfig {
    /// Create a configuration from ExpandOptions.
    pub fn from_expand_options(options: &ExpandOptions) -> Self {
        Self {
            include_routes: options.routes,
            include_hooks: options.hooks,
            include_jsx_parents: options.jsx_parents,
            include_jsx_children: options.jsx_children,
            ..Default::default()
        }
    }
}

/// React-specific context assembly strategy.
///
/// This assembler extends the basic assembler with React-specific enhancements:
/// - Component detection
/// - Route discovery
/// - Hook inclusion
/// - JSX relationship handling
pub struct ReactAssemblyStrategy {
    store: Arc<SqliteStore>,
    base_assembler: BasicContextAssembler,
    config: ReactConfig,
    component_detector: ComponentDetector,
    hook_detector: HookDetector,
    jsx_detector: JsxRelationshipDetector,
    token_counter: TokenCounter,
}

impl ReactAssemblyStrategy {
    /// Create a new React assembly strategy.
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self::with_config(store, ReactConfig::default())
    }

    /// Create a new React assembly strategy with custom configuration.
    pub fn with_config(store: Arc<SqliteStore>, config: ReactConfig) -> Self {
        Self {
            base_assembler: BasicContextAssembler::new_without_cache(Arc::clone(&store)),
            store,
            config,
            component_detector: ComponentDetector::new(),
            hook_detector: HookDetector::new(),
            jsx_detector: JsxRelationshipDetector::new(),
            token_counter: TokenCounter::new(),
        }
    }

    /// Check if a chunk is a React component.
    async fn is_component(&self, metadata: &ChunkMetadata) -> Result<bool> {
        // Quick check: file extension
        if !metadata.file_relpath.ends_with(".tsx") && !metadata.file_relpath.ends_with(".jsx") {
            return Ok(false);
        }

        // Use component detector for file path heuristics
        if !self
            .component_detector
            .is_component_file_path(&metadata.file_relpath)
        {
            return Ok(false);
        }

        // Load file content to verify JSX presence
        let file_loader = FileLoader::new(&metadata.worktree_path);
        let range = LineRange::new(metadata.start_line, metadata.end_line);

        match file_loader.load_range(&metadata.file_relpath, range).await {
            Ok(content) => Ok(self.component_detector.has_jsx_return(&content)),
            Err(_) => {
                // If we can't load content, rely on file path heuristics
                Ok(true)
            }
        }
    }

    /// Create a ContextItem from metadata.
    async fn create_context_item(
        &self,
        metadata: ChunkMetadata,
        role: &str,
        reason: &str,
    ) -> Result<ContextItem> {
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

        let tokens = self
            .token_counter
            .count(&content)
            .context("Failed to count tokens")?;

        Ok(ContextItem {
            relpath: metadata.file_relpath,
            range,
            role: role.to_string(),
            reason: reason.to_string(),
            content,
            tokens,
        })
    }

    /// Retrieve chunk metadata from the database by ID.
    async fn get_chunk_metadata(&self, chunk_id: i64) -> Result<ChunkMetadata> {
        // Use the base assembler's implementation which works with SQLite
        self.base_assembler.get_chunk_metadata(chunk_id).await
    }

    /// Add route definitions to the context bundle.
    async fn add_routes(
        &self,
        _bundle: &mut ContextBundle,
        _chunk_id: i64,
        _budget: usize,
    ) -> Result<()> {
        // TODO: Implement route queries for SQLite backend using graph module
        // For now, this feature is disabled
        Ok(())
    }

    /// Add hooks to the context bundle.
    async fn add_hooks(
        &self,
        _bundle: &mut ContextBundle,
        _chunk_id: i64,
        _budget: usize,
    ) -> Result<()> {
        // TODO: Implement hook queries for SQLite backend using graph module
        // For now, this feature is disabled
        Ok(())
    }

    /// Add JSX parent components to the context bundle.
    async fn add_jsx_parents(
        &self,
        _bundle: &mut ContextBundle,
        _chunk_id: i64,
        _symbol_name: &str,
        _budget: usize,
    ) -> Result<()> {
        // TODO: Implement JSX parent queries for SQLite backend using graph module
        // For now, this feature is disabled
        Ok(())
    }

    /// Add JSX child components to the context bundle.
    async fn add_jsx_children(
        &self,
        _bundle: &mut ContextBundle,
        _chunk_id: i64,
        _budget: usize,
    ) -> Result<()> {
        // TODO: Implement JSX child queries for SQLite backend using graph module
        // For now, this feature is disabled
        Ok(())
    }
}

#[async_trait::async_trait]
impl ContextAssembler for ReactAssemblyStrategy {
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        options: ExpandOptions,
    ) -> Result<ContextBundle> {
        debug!(
            "React strategy assembling context for chunk {} with budget {} tokens",
            chunk_id, budget
        );

        // Update config from options
        let mut config = self.config.clone();
        config.include_routes = options.routes;
        config.include_hooks = options.hooks;
        config.include_jsx_parents = options.jsx_parents;
        config.include_jsx_children = options.jsx_children;

        // Get chunk metadata
        let metadata = self
            .get_chunk_metadata(chunk_id)
            .await
            .context("Failed to retrieve chunk metadata")?;

        // Check if this is a React component
        let is_component = self.is_component(&metadata).await?;

        debug!(
            "Chunk {} is {}a React component",
            chunk_id,
            if is_component { "" } else { "not " }
        );

        // Start with the basic assembly (primary chunk + tests if requested)
        let mut bundle = self
            .base_assembler
            .assemble(chunk_id, budget, options.clone())
            .await?;

        // If not a component, return the basic bundle
        if !is_component {
            return Ok(bundle);
        }

        // Add React-specific context items
        let symbol_name = metadata.symbol_name.as_deref().unwrap_or("Component");

        // Priority order: routes → hooks → jsx_parents → jsx_children
        if config.include_routes {
            self.add_routes(&mut bundle, chunk_id, budget).await?;
        }

        if config.include_hooks {
            self.add_hooks(&mut bundle, chunk_id, budget).await?;
        }

        if config.include_jsx_parents {
            self.add_jsx_parents(&mut bundle, chunk_id, symbol_name, budget)
                .await?;
        }

        if config.include_jsx_children {
            self.add_jsx_children(&mut bundle, chunk_id, budget).await?;
        }

        debug!(
            "React strategy assembled {} items, {} tokens total",
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
    fn test_react_config_default() {
        let config = ReactConfig::default();
        assert!(config.include_routes);
        assert!(config.include_hooks);
        assert!(config.include_jsx_parents);
        assert!(config.include_jsx_children);
        assert_eq!(config.max_hooks, 5);
        assert_eq!(config.max_jsx_parents, 2);
        assert_eq!(config.max_jsx_children, 5);
    }

    #[test]
    fn test_react_config_from_expand_options() {
        let options = ExpandOptions {
            routes: true,
            hooks: true,
            jsx_parents: false,
            jsx_children: true,
            ..Default::default()
        };

        let config = ReactConfig::from_expand_options(&options);
        assert!(config.include_routes);
        assert!(config.include_hooks);
        assert!(!config.include_jsx_parents);
        assert!(config.include_jsx_children);
    }

    #[test]
    fn test_expand_options_for_react_component() {
        let options = ExpandOptions::for_react_component();
        assert!(!options.callers);
        assert!(!options.callees);
        assert!(options.tests);
        assert!(options.routes);
        assert!(options.hooks);
        assert!(options.jsx_parents);
        assert!(options.jsx_children);
    }

    // Integration tests with database are in tests/ directory
    #[tokio::test]
    #[ignore]
    async fn test_react_assembly_strategy() {
        // Integration test - requires database
    }
}
