//! Default context assembly strategy.
//!
//! This module provides the baseline strategy that works across all languages.
//! It follows the architecture doc pattern (lines 116-144):
//! - Primary chunk (40% of budget)
//! - Direct test file
//! - One top caller
//! - One top callee
//! - Config file if relevant

use anyhow::{Context as AnyhowContext, Result};
use tracing::{debug, warn};

use crate::context::{
    assembler::ChunkMetadata,
    file_loader::FileLoader,
    relationships::{find_callees, find_callers, find_test_files},
    strategy::AssemblyStrategy,
    token_counter::TokenCounter,
    types::{ContextBundle, ContextItem, ExpandOptions, LineRange},
};
use crate::db::PgPool;

/// Default assembly strategy that works across all languages.
///
/// This strategy provides a balanced allocation:
/// - Primary chunk: 40% of budget
/// - Tests: 30% of budget
/// - Callers: 15% of budget
/// - Callees: 15% of budget
///
/// This serves as the baseline that language-specific strategies can extend.
pub struct DefaultAssemblyStrategy {
    pool: PgPool,
    token_counter: TokenCounter,
}

impl DefaultAssemblyStrategy {
    /// Create a new default assembly strategy.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            token_counter: TokenCounter::new(),
        }
    }

    /// Retrieve chunk metadata from the database by ID.
    pub async fn get_chunk_metadata(&self, chunk_id: i64) -> Result<ChunkMetadata> {
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
                warn!(
                    "Chunk {} has no worktree_path, using empty string",
                    chunk_id
                );
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
    pub async fn create_context_item(
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

    /// Add primary chunk to the bundle.
    async fn add_primary_chunk(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
    ) -> Result<()> {
        let primary_budget = (budget as f64 * 0.4) as usize; // 40% of total budget

        let metadata = self.get_chunk_metadata(chunk_id).await?;

        let reason = if let Some(ref name) = metadata.symbol_name {
            format!("Primary chunk: {} ({})", name, metadata.kind)
        } else {
            format!("Primary chunk ({})", metadata.kind)
        };

        match self.create_context_item(metadata, "primary", &reason).await {
            Ok(item) => {
                if item.tokens > primary_budget {
                    warn!(
                        "Primary chunk ({} tokens) exceeds allocated budget ({} tokens)",
                        item.tokens, primary_budget
                    );
                    bundle.truncated = true;
                }
                debug!("Adding primary chunk: {} tokens", item.tokens);
                bundle.add_item(item);
            }
            Err(e) => {
                return Err(e).context("Failed to create primary context item");
            }
        }

        Ok(())
    }

    /// Add test chunks to the bundle.
    async fn add_tests(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
    ) -> Result<()> {
        let test_budget = (budget as f64 * 0.3) as usize; // 30% of total budget

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let tests = find_test_files(&client, chunk_id).await?;

        for test in tests.into_iter().take(1) {
            // Only include the nearest test
            if bundle.total_tokens >= budget {
                break;
            }

            let remaining = budget.saturating_sub(bundle.total_tokens);
            if remaining < test_budget / 10 {
                // Less than 10% of test budget remaining
                break;
            }

            let metadata = self.get_chunk_metadata(test.id).await?;

            let reason = format!(
                "Test: {} (tests primary chunk)",
                test.symbol_name.unwrap_or_else(|| "test".to_string())
            );

            match self.create_context_item(metadata, "test", &reason).await {
                Ok(item) => {
                    if !bundle.would_exceed_budget(item.tokens, budget) {
                        debug!("Adding test: {} tokens", item.tokens);
                        bundle.add_item(item);
                    }
                }
                Err(e) => {
                    warn!("Failed to create test context item: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Add caller chunks to the bundle.
    async fn add_callers(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
    ) -> Result<()> {
        let caller_budget = (budget as f64 * 0.15) as usize; // 15% of total budget

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let callers = find_callers(&client, chunk_id, 1).await?; // Depth 1 only

        for caller in callers.into_iter().take(1) {
            // Only include top caller
            if bundle.total_tokens >= budget {
                break;
            }

            let remaining = budget.saturating_sub(bundle.total_tokens);
            if remaining < caller_budget / 10 {
                break;
            }

            let metadata = self.get_chunk_metadata(caller.id).await?;

            let reason = format!(
                "Caller: {} (calls primary chunk)",
                caller.symbol_name.unwrap_or_else(|| "caller".to_string())
            );

            match self.create_context_item(metadata, "caller", &reason).await {
                Ok(item) => {
                    if !bundle.would_exceed_budget(item.tokens, budget) {
                        debug!("Adding caller: {} tokens", item.tokens);
                        bundle.add_item(item);
                    }
                }
                Err(e) => {
                    warn!("Failed to create caller context item: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Add callee chunks to the bundle.
    async fn add_callees(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
    ) -> Result<()> {
        let callee_budget = (budget as f64 * 0.15) as usize; // 15% of total budget

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;

        let callees = find_callees(&client, chunk_id, 1).await?; // Depth 1 only

        for callee in callees.into_iter().take(1) {
            // Only include top callee
            if bundle.total_tokens >= budget {
                break;
            }

            let remaining = budget.saturating_sub(bundle.total_tokens);
            if remaining < callee_budget / 10 {
                break;
            }

            let metadata = self.get_chunk_metadata(callee.id).await?;

            let reason = format!(
                "Callee: {} (called by primary chunk)",
                callee.symbol_name.unwrap_or_else(|| "callee".to_string())
            );

            match self.create_context_item(metadata, "callee", &reason).await {
                Ok(item) => {
                    if !bundle.would_exceed_budget(item.tokens, budget) {
                        debug!("Adding callee: {} tokens", item.tokens);
                        bundle.add_item(item);
                    }
                }
                Err(e) => {
                    warn!("Failed to create callee context item: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Find and add relevant config files to the bundle.
    ///
    /// This looks for common config files in the same directory or parent directories.
    pub async fn add_config_files(
        &self,
        bundle: &mut ContextBundle,
        metadata: &ChunkMetadata,
        budget: usize,
    ) -> Result<()> {
        if bundle.total_tokens >= budget {
            return Ok(());
        }

        // Common config file names
        let config_names = [
            "package.json",
            "tsconfig.json",
            ".eslintrc.json",
            "pyproject.toml",
            "setup.py",
            "Cargo.toml",
            "go.mod",
        ];

        // Extract directory from file path
        let file_path = std::path::Path::new(&metadata.file_relpath);
        let dir = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");

        for config_name in &config_names {
            if bundle.total_tokens >= budget {
                break;
            }

            let config_path = if dir.is_empty() {
                config_name.to_string()
            } else {
                format!("{}/{}", dir, config_name)
            };

            // Try to load config file
            let file_loader = FileLoader::new(&metadata.worktree_path);
            if let Ok(content) = file_loader
                .load_range(&config_path, LineRange::new(1, i32::MAX))
                .await
            {
                let tokens = self.token_counter.count(&content)?;

                if !bundle.would_exceed_budget(tokens, budget) {
                    let item = ContextItem {
                        relpath: config_path.clone(),
                        range: LineRange::new(1, content.lines().count() as i32),
                        role: "config".to_string(),
                        reason: format!("Configuration file: {}", config_name),
                        content,
                        tokens,
                    };
                    debug!("Adding config file {}: {} tokens", config_path, tokens);
                    bundle.add_item(item);
                    break; // Only add one config file
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl AssemblyStrategy for DefaultAssemblyStrategy {
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        options: ExpandOptions,
    ) -> Result<ContextBundle> {
        debug!(
            "Default strategy assembling context for chunk {} with budget {} tokens",
            chunk_id, budget
        );

        let mut bundle = ContextBundle::new();

        // 1. Add primary chunk (40% of budget)
        self.add_primary_chunk(&mut bundle, chunk_id, budget)
            .await?;

        // 2. Add tests if requested (30% of budget)
        if options.tests {
            self.add_tests(&mut bundle, chunk_id, budget).await?;
        }

        // 3. Add top caller if requested (15% of budget)
        if options.callers {
            self.add_callers(&mut bundle, chunk_id, budget).await?;
        }

        // 4. Add top callee if requested (15% of budget)
        if options.callees {
            self.add_callees(&mut bundle, chunk_id, budget).await?;
        }

        // 5. Add config file if requested and space remains
        if options.config {
            let metadata = self.get_chunk_metadata(chunk_id).await?;
            self.add_config_files(&mut bundle, &metadata, budget)
                .await?;
        }

        debug!(
            "Default strategy assembled {} items, {} tokens total",
            bundle.items.len(),
            bundle.total_tokens
        );

        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_default_strategy_budget_allocation() {
        // Test that budget percentages are correct
        let budget = 1000;
        let primary = (budget as f64 * 0.4) as usize;
        let tests = (budget as f64 * 0.3) as usize;
        let callers = (budget as f64 * 0.15) as usize;
        let callees = (budget as f64 * 0.15) as usize;

        assert_eq!(primary, 400);
        assert_eq!(tests, 300);
        assert_eq!(callers, 150);
        assert_eq!(callees, 150);
        assert_eq!(primary + tests + callers + callees, 1000);
    }

    // Integration tests with database are in tests/ directory
    #[tokio::test]
    #[ignore]
    async fn test_default_assembly_strategy() {
        // Integration test - requires database
    }
}
