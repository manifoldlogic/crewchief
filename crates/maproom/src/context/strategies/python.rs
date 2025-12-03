//! Python-specific context assembly strategy.
//!
//! This module provides a specialized assembler for Python codebases that:
//! - Includes __init__.py for package context
//! - Includes requirements.txt or pyproject.toml for dependency context
//! - Prioritizes class hierarchies (parent classes, child classes)
//! - Detects test files using test_*.py or *_test.py patterns
//! - Includes docstrings and type hints

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

/// Configuration for Python assembly strategy.
#[derive(Debug, Clone)]
pub struct PythonConfig {
    /// Whether to include __init__.py
    pub include_init: bool,
    /// Whether to include requirements.txt or pyproject.toml
    pub include_dependencies: bool,
    /// Whether to include parent classes
    pub include_parent_classes: bool,
    /// Maximum number of parent classes to include
    pub max_parent_classes: usize,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            include_init: true,
            include_dependencies: true,
            include_parent_classes: true,
            max_parent_classes: 2,
        }
    }
}

/// Python-specific context assembly strategy.
///
/// This assembler extends the default strategy with Python-specific enhancements:
/// - Package structure (__init__.py)
/// - Dependencies (requirements.txt, pyproject.toml)
/// - Class hierarchies
/// - Python test patterns
pub struct PythonAssemblyStrategy {
    store: Arc<SqliteStore>,
    default: DefaultAssemblyStrategy,
    config: PythonConfig,
    token_counter: TokenCounter,
}

impl PythonAssemblyStrategy {
    /// Create a new Python assembly strategy.
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self::with_config(store, PythonConfig::default())
    }

    /// Create a new Python assembly strategy with custom configuration.
    pub fn with_config(store: Arc<SqliteStore>, config: PythonConfig) -> Self {
        Self {
            default: DefaultAssemblyStrategy::new(Arc::clone(&store)),
            store,
            config,
            token_counter: TokenCounter::new(),
        }
    }

    /// Find and add __init__.py from the package directory.
    async fn add_init_py(
        &self,
        bundle: &mut ContextBundle,
        metadata: &ChunkMetadata,
        budget: usize,
    ) -> Result<()> {
        if !self.config.include_init || bundle.total_tokens >= budget {
            return Ok(());
        }

        // Extract directory from file path
        let file_path = std::path::Path::new(&metadata.file_relpath);
        let dir = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");

        if dir.is_empty() {
            return Ok(());
        }

        let init_path = format!("{}/__init__.py", dir);

        // Try to load __init__.py
        let file_loader = FileLoader::new(&metadata.worktree_path);
        if let Ok(content) = file_loader
            .load_range(&init_path, LineRange::new(1, i32::MAX))
            .await
        {
            let tokens = self.token_counter.count(&content)?;

            if !bundle.would_exceed_budget(tokens, budget) {
                let item = ContextItem {
                    relpath: init_path.clone(),
                    range: LineRange::new(1, content.lines().count() as i32),
                    role: "package_init".to_string(),
                    reason: "Package initialization: __init__.py provides package context"
                        .to_string(),
                    content,
                    tokens,
                };
                debug!("Adding __init__.py: {} tokens", tokens);
                bundle.add_item(item);
            }
        }

        Ok(())
    }

    /// Find and add Python dependency files (requirements.txt, pyproject.toml).
    async fn add_dependency_files(
        &self,
        bundle: &mut ContextBundle,
        metadata: &ChunkMetadata,
        budget: usize,
    ) -> Result<()> {
        if !self.config.include_dependencies || bundle.total_tokens >= budget {
            return Ok(());
        }

        // Look for dependency files in project root
        let dependency_files = ["requirements.txt", "pyproject.toml", "setup.py"];

        let file_loader = FileLoader::new(&metadata.worktree_path);

        for dep_file in &dependency_files {
            if bundle.total_tokens >= budget {
                break;
            }

            if let Ok(content) = file_loader
                .load_range(dep_file, LineRange::new(1, i32::MAX))
                .await
            {
                let tokens = self.token_counter.count(&content)?;

                if !bundle.would_exceed_budget(tokens, budget) {
                    let item = ContextItem {
                        relpath: dep_file.to_string(),
                        range: LineRange::new(1, content.lines().count() as i32),
                        role: "dependencies".to_string(),
                        reason: format!(
                            "Python dependencies: {} provides project dependency context",
                            dep_file
                        ),
                        content,
                        tokens,
                    };
                    debug!("Adding {}: {} tokens", dep_file, tokens);
                    bundle.add_item(item);
                    break; // Only add one dependency file
                }
            }
        }

        Ok(())
    }

    /// Find parent classes for a Python class chunk.
    ///
    /// Uses graph traversal to find classes that this class extends.
    async fn add_parent_classes(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
    ) -> Result<()> {
        use crate::db::sqlite::graph::ImportDirection;

        if !self.config.include_parent_classes || bundle.total_tokens >= budget {
            return Ok(());
        }

        // Find classes that this class extends (Outgoing direction = superclasses)
        let parent_classes = self
            .store
            .find_extensions(
                chunk_id,
                ImportDirection::Outgoing,
                Some(1), // Only direct parents
            )
            .await?;

        let mut added_count = 0;
        for parent in parent_classes {
            if added_count >= self.config.max_parent_classes || bundle.total_tokens >= budget {
                break;
            }

            // Get the parent class metadata
            if let Some(chunk) = self.store.get_chunk_by_id(parent.chunk_id).await? {
                // Get worktree path for file loading
                let metadata = self.default.get_chunk_metadata(chunk_id).await?;

                let file_loader = FileLoader::new(&metadata.worktree_path);
                let range = LineRange::new(chunk.start_line, chunk.end_line);

                if let Ok(content) = file_loader.load_range(&chunk.file_path, range).await {
                    let tokens = self.token_counter.count(&content)?;

                    if !bundle.would_exceed_budget(tokens, budget) {
                        let parent_name = chunk.symbol_name.as_deref().unwrap_or("ParentClass");
                        let item = ContextItem {
                            relpath: chunk.file_path.clone(),
                            range: LineRange::new(chunk.start_line, chunk.end_line),
                            role: "parent_class".to_string(),
                            reason: format!(
                                "Parent class: {} provides inherited functionality",
                                parent_name
                            ),
                            content,
                            tokens,
                        };
                        debug!("Adding parent class {}: {} tokens", parent_name, tokens);
                        bundle.add_item(item);
                        added_count += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a chunk is a Python class.
    async fn is_python_class(&self, metadata: &ChunkMetadata) -> bool {
        metadata.kind == "class" && metadata.file_relpath.ends_with(".py")
    }
}

#[async_trait::async_trait]
impl AssemblyStrategy for PythonAssemblyStrategy {
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        options: ExpandOptions,
    ) -> Result<ContextBundle> {
        debug!(
            "Python strategy assembling context for chunk {} with budget {} tokens",
            chunk_id, budget
        );

        // Start with default strategy assembly
        let mut bundle = self.default.assemble(chunk_id, budget, options).await?;

        // Get chunk metadata to determine if Python-specific enhancements apply
        let metadata = self.default.get_chunk_metadata(chunk_id).await?;

        // Add Python-specific context items
        // Priority order: __init__.py → parent classes → dependencies

        // 1. Include __init__.py for package context
        self.add_init_py(&mut bundle, &metadata, budget).await?;

        // 2. Include parent classes if this is a class
        if self.is_python_class(&metadata).await {
            self.add_parent_classes(&mut bundle, chunk_id, budget)
                .await?;
        }

        // 3. Include dependency files
        self.add_dependency_files(&mut bundle, &metadata, budget)
            .await?;

        debug!(
            "Python strategy assembled {} items, {} tokens total",
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
    fn test_python_config_default() {
        let config = PythonConfig::default();
        assert!(config.include_init);
        assert!(config.include_dependencies);
        assert!(config.include_parent_classes);
        assert_eq!(config.max_parent_classes, 2);
    }

    // Integration tests with database are in tests/ directory
    #[tokio::test]
    #[ignore]
    async fn test_python_assembly_strategy() {
        // Integration test - requires database
    }
}
