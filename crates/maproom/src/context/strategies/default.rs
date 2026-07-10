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
use crate::db::Store;
use std::sync::Arc;

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
    store: Arc<dyn Store + Send + Sync>,
    token_counter: TokenCounter,
}

impl DefaultAssemblyStrategy {
    /// Create a new default assembly strategy.
    pub fn new(store: Arc<dyn Store + Send + Sync>) -> Self {
        Self {
            store,
            token_counter: TokenCounter::new(),
        }
    }

    /// Retrieve chunk metadata from the database by ID.
    pub async fn get_chunk_metadata(&self, chunk_id: i64) -> Result<ChunkMetadata> {
        // Backend-agnostic: chunk fields via get_chunk_by_id, worktree abs_path via
        // get_file_edge_context (both Store trait methods).
        let chunk = self
            .store
            .get_chunk_by_id(chunk_id)
            .await
            .context("Failed to query chunk metadata")?
            .ok_or_else(|| anyhow::anyhow!("Chunk {chunk_id} not found"))?;

        // Missing file/worktree context must NOT silently fall back to an empty
        // worktree root — FileLoader::new("") would then resolve files relative to
        // the process directory and read the wrong file.
        let worktree_path = self
            .store
            .get_file_edge_context(chunk.file_id)
            .await?
            .map(|(_, _, abs_path)| abs_path)
            .ok_or_else(|| anyhow::anyhow!("Chunk {chunk_id} has no file/worktree context"))?;

        Ok(ChunkMetadata {
            id: chunk.id,
            file_relpath: chunk.file_path,
            worktree_path,
            symbol_name: chunk.symbol_name,
            kind: chunk.kind,
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            signature: chunk.signature,
            docstring: chunk.docstring,
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

    /// F81: budget-derived item cap for a relationship segment — replaces
    /// the hard-coded `.take(1)` ("one caller + one callee max"). ~400
    /// tokens is the working per-item estimate (mirrors the parallel
    /// assembler's allocation/400); the floor keeps small budgets useful,
    /// and the real token-budget guards still bound the total.
    fn segment_item_cap(segment_budget: usize, floor: usize) -> usize {
        (segment_budget / 400).max(floor)
    }

    /// Add test chunks to the bundle.
    async fn add_tests(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
        seen: &mut std::collections::HashSet<i64>,
    ) -> Result<()> {
        let test_budget = (budget as f64 * 0.2) as usize; // 20% of total budget

        let tests = find_test_files(self.store.as_ref(), chunk_id).await?;

        let cap = Self::segment_item_cap(test_budget, 3);
        for test in tests.into_iter().take(cap) {
            if bundle.total_tokens >= budget {
                break;
            }

            let remaining = budget.saturating_sub(bundle.total_tokens);
            if remaining < test_budget / 10 {
                // Less than 10% of test budget remaining
                break;
            }

            if !seen.insert(test.id) {
                continue; // already in the bundle via another segment
            }

            // Warn-and-continue: one unreadable related chunk must not kill
            // the whole bundle (densification amplifies the blast radius).
            let metadata = match self.get_chunk_metadata(test.id).await {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to load test chunk {}: {}", test.id, e);
                    continue;
                }
            };

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
    ///
    /// F81: honors `max_depth` (was hard-coded 1) and a budget-derived item
    /// cap (was `.take(1)`); results are relevance-ordered so direct callers
    /// come before transitive ones.
    async fn add_callers(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
        max_depth: i32,
        seen: &mut std::collections::HashSet<i64>,
    ) -> Result<()> {
        let caller_budget = (budget as f64 * 0.15) as usize; // 15% of total budget

        let mut callers = find_callers(self.store.as_ref(), chunk_id, max_depth).await?;
        callers.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let cap = Self::segment_item_cap(caller_budget, 5);
        for caller in callers.into_iter().take(cap) {
            if bundle.total_tokens >= budget {
                break;
            }

            let remaining = budget.saturating_sub(bundle.total_tokens);
            if remaining < caller_budget / 10 {
                break;
            }

            if !seen.insert(caller.id) {
                continue;
            }

            let metadata = match self.get_chunk_metadata(caller.id).await {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to load caller chunk {}: {}", caller.id, e);
                    continue;
                }
            };

            let reason = format!(
                "Caller: {} (calls primary chunk, depth {})",
                caller.symbol_name.unwrap_or_else(|| "caller".to_string()),
                caller.depth
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

    /// Add callee chunks to the bundle (F81: depth + cap, see add_callers).
    async fn add_callees(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
        max_depth: i32,
        seen: &mut std::collections::HashSet<i64>,
    ) -> Result<()> {
        let callee_budget = (budget as f64 * 0.15) as usize; // 15% of total budget

        let mut callees = find_callees(self.store.as_ref(), chunk_id, max_depth).await?;
        callees.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let cap = Self::segment_item_cap(callee_budget, 5);
        for callee in callees.into_iter().take(cap) {
            if bundle.total_tokens >= budget {
                break;
            }

            let remaining = budget.saturating_sub(bundle.total_tokens);
            if remaining < callee_budget / 10 {
                break;
            }

            if !seen.insert(callee.id) {
                continue;
            }

            let metadata = match self.get_chunk_metadata(callee.id).await {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to load callee chunk {}: {}", callee.id, e);
                    continue;
                }
            };

            let reason = format!(
                "Callee: {} (called by primary chunk, depth {})",
                callee.symbol_name.unwrap_or_else(|| "callee".to_string()),
                callee.depth
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

    /// Add import/export relationships (F82: the symmetric engine surfaces
    /// these on the search path; context never did). Both directions, each
    /// labeled — "Imports" (outgoing) and "Imported by" (incoming).
    async fn add_imports(
        &self,
        bundle: &mut ContextBundle,
        chunk_id: i64,
        budget: usize,
        max_depth: i32,
        seen: &mut std::collections::HashSet<i64>,
    ) -> Result<()> {
        use crate::db::ImportDirection;
        let import_budget = (budget as f64 * 0.10) as usize; // 10% of total budget
        let depth = Some(max_depth.max(1) as usize);

        let mut labeled: Vec<(crate::db::GraphResult, &'static str)> = Vec::new();
        for r in self
            .store
            .find_imports(chunk_id, ImportDirection::Outgoing, depth)
            .await?
        {
            labeled.push((r, "Imports"));
        }
        for r in self
            .store
            .find_imports(chunk_id, ImportDirection::Incoming, depth)
            .await?
        {
            labeled.push((r, "Imported by"));
        }
        labeled.sort_by_key(|(r, _)| r.depth);

        let cap = Self::segment_item_cap(import_budget, 4);
        for (rel, label) in labeled.into_iter().take(cap) {
            if bundle.total_tokens >= budget {
                break;
            }
            let remaining = budget.saturating_sub(bundle.total_tokens);
            if remaining < import_budget / 10 {
                break;
            }
            if !seen.insert(rel.chunk_id) {
                continue;
            }
            let metadata = match self.get_chunk_metadata(rel.chunk_id).await {
                Ok(m) => m,
                Err(e) => {
                    warn!(
                        "Failed to load import-related chunk {}: {}",
                        rel.chunk_id, e
                    );
                    continue;
                }
            };
            let reason = format!(
                "{label}: {} (depth {})",
                metadata
                    .symbol_name
                    .clone()
                    .unwrap_or_else(|| "module".to_string()),
                rel.depth
            );
            match self.create_context_item(metadata, "import", &reason).await {
                Ok(item) => {
                    if !bundle.would_exceed_budget(item.tokens, budget) {
                        debug!("Adding import relation: {} tokens", item.tokens);
                        bundle.add_item(item);
                    }
                }
                Err(e) => {
                    warn!("Failed to create import context item: {}", e);
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

        // F81: max_depth is honored end-to-end (it was silently discarded —
        // helpers hard-coded depth 1).
        let depth = options.max_depth.max(1);

        // Densified segments need cross-segment dedup: a chunk that is both
        // a caller and a callee (recursion, mutual calls) must appear once,
        // and the primary chunk must never re-appear as its own relative.
        let mut seen: std::collections::HashSet<i64> = std::collections::HashSet::new();
        seen.insert(chunk_id);

        // 1. Add primary chunk (40% of budget)
        self.add_primary_chunk(&mut bundle, chunk_id, budget)
            .await?;

        // 2. Add tests if requested (20% of budget)
        if options.tests {
            self.add_tests(&mut bundle, chunk_id, budget, &mut seen)
                .await?;
        }

        // 3. Add callers if requested (15% of budget)
        if options.callers {
            self.add_callers(&mut bundle, chunk_id, budget, depth, &mut seen)
                .await?;
        }

        // 4. Add callees if requested (15% of budget)
        if options.callees {
            self.add_callees(&mut bundle, chunk_id, budget, depth, &mut seen)
                .await?;
        }

        // 5. Add import/export relations if requested (10% of budget, F82)
        if options.imports {
            self.add_imports(&mut bundle, chunk_id, budget, depth, &mut seen)
                .await?;
        }

        // 6. Honesty over silence (F81): docs has NO engine yet — say so
        //    instead of silently ignoring the flag.
        if options.docs {
            warn!(
                "--docs requested but documentation expansion is not implemented yet; \
                 the flag is accepted for forward compatibility and currently adds nothing"
            );
        }

        // 7. Add config file if requested and space remains
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
        // F81/F82 split: primary 40, tests 20, callers 15, callees 15,
        // imports 10 — sums to 100%.
        let budget = 1000;
        let primary = (budget as f64 * 0.4) as usize;
        let tests = (budget as f64 * 0.2) as usize;
        let callers = (budget as f64 * 0.15) as usize;
        let callees = (budget as f64 * 0.15) as usize;
        let imports = (budget as f64 * 0.10) as usize;

        assert_eq!(primary + tests + callers + callees + imports, 1000);
    }

    #[test]
    fn test_segment_item_cap() {
        use super::DefaultAssemblyStrategy;
        // floor wins for small budgets; scales with tokens for big ones
        assert_eq!(DefaultAssemblyStrategy::segment_item_cap(150, 5), 5);
        assert_eq!(DefaultAssemblyStrategy::segment_item_cap(4000, 5), 10);
    }

    // Integration tests with database are in tests/ directory
    #[tokio::test]
    #[ignore]
    async fn test_default_assembly_strategy() {
        // Integration test - requires database
    }
}
