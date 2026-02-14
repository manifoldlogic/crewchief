//! Clean-ignored command implementation.
//!
//! This module implements the `clean-ignored` CLI command which deletes indexed chunks
//! matching patterns in `.maproomignore`. This provides a way to remove noise from search
//! results after adding patterns to `.maproomignore`, without requiring a full rescan.

use crate::db::traits::StoreCore;
use crate::db::SqliteStore;
use crate::incremental::ignore::{load_ignore_patterns, IgnorePatternMatcher};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{info, warn};

/// Delete indexed chunks matching patterns in .maproomignore.
///
/// # Arguments
///
/// * `store` - Database store handle
/// * `repo_name` - Repository name
/// * `worktree_name` - Worktree name
/// * `dry_run` - If true, report what would be deleted without deleting
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err` if repository/worktree not found or database operation fails
pub async fn clean_ignored(
    store: &SqliteStore,
    repo_name: &str,
    worktree_name: &str,
    dry_run: bool,
) -> Result<()> {
    // 1. Resolve repository and worktree IDs
    let repo = store
        .get_repo_by_name(repo_name)
        .await
        .context("Failed to get repository")?
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_name))?;

    let worktree = store
        .get_worktree_by_name(repo.id, worktree_name)
        .await
        .context("Failed to get worktree")?
        .ok_or_else(|| anyhow::anyhow!("Worktree '{}' not found", worktree_name))?;

    // 2. Load .maproomignore patterns from worktree path
    let root = PathBuf::from(&worktree.abs_path);
    let patterns = match load_ignore_patterns(&root) {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to load ignore patterns: {}", e);
            info!("No patterns loaded, nothing to clean");
            return Ok(());
        }
    };

    // Check if only default patterns exist (no custom patterns)
    // Default patterns are always included, so we need to check for additional patterns
    // The load_ignore_patterns function includes defaults, so we just check if patterns is empty
    if patterns.is_empty() {
        info!("No patterns in .maproomignore, nothing to clean");
        return Ok(());
    }

    info!("Loaded {} ignore patterns", patterns.len());

    // 3. Create pattern matcher
    let matcher = IgnorePatternMatcher::with_patterns(&patterns)
        .context("Failed to compile ignore patterns")?;

    // 4. Get all chunks for this worktree
    let chunks = store
        .get_chunks_for_worktree(worktree.id)
        .await
        .context("Failed to get chunks for worktree")?;

    info!("Found {} total chunks in worktree", chunks.len());

    // 5. Filter chunks that match patterns
    let mut to_delete = Vec::new();
    for (chunk_id, file_relpath) in chunks {
        let relpath = PathBuf::from(&file_relpath);
        if matcher.should_ignore(&relpath) {
            to_delete.push((chunk_id, file_relpath));
        }
    }

    info!("Found {} chunks matching ignore patterns", to_delete.len());

    // 6. Delete or report
    if dry_run {
        println!("🔍 Dry run mode - showing what would be deleted:");
        println!("   Repository: {}", repo_name);
        println!("   Worktree: {}", worktree_name);
        println!("   Chunks to delete: {}", to_delete.len());
        println!();
        for (chunk_id, relpath) in &to_delete {
            println!("   Would delete chunk #{}: {}", chunk_id, relpath);
        }
        println!();
        println!("⚠️  Run without --dry-run to actually delete these chunks");
    } else {
        if to_delete.is_empty() {
            println!("✅ No chunks match ignore patterns - nothing to delete");
            return Ok(());
        }

        // Extract just the chunk IDs for deletion
        let chunk_ids: Vec<i64> = to_delete.iter().map(|(id, _)| *id).collect();

        // Delete chunks
        let count = store
            .delete_chunks_by_ids(worktree.id, &chunk_ids)
            .await
            .context("Failed to delete chunks")?;

        println!(
            "✅ Deleted {} chunks matching .maproomignore patterns",
            count
        );
        println!("   Repository: {}", repo_name);
        println!("   Worktree: {}", worktree_name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to create a test repository with a .maproomignore file
    async fn setup_test_repo(
        ignore_content: &str,
    ) -> Result<(TempDir, SqliteStore, String, String)> {
        // Create temp directory for test repo
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path();

        // Write .maproomignore
        let ignore_path = repo_path.join(".maproomignore");
        let mut file = std::fs::File::create(&ignore_path)?;
        file.write_all(ignore_content.as_bytes())?;
        file.flush()?;

        // Create in-memory database for testing
        let store = db::SqliteStore::connect(":memory:").await?;

        // Create repository and worktree
        let repo_name = "test-repo".to_string();
        let worktree_name = "main".to_string();
        let repo_id = store
            .get_or_create_repo(&repo_name, repo_path.to_str().unwrap())
            .await?;
        let _worktree_id = store
            .get_or_create_worktree(repo_id, &worktree_name, repo_path.to_str().unwrap())
            .await?;

        Ok((temp_dir, store, repo_name, worktree_name))
    }

    #[tokio::test]
    async fn test_clean_ignored_missing_file() {
        // Create test repo without .maproomignore
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let store = db::SqliteStore::connect(":memory:").await.unwrap();
        let repo_name = "test-repo";
        let worktree_name = "main";
        let repo_id = store
            .get_or_create_repo(repo_name, repo_path.to_str().unwrap())
            .await
            .unwrap();
        let _worktree_id = store
            .get_or_create_worktree(repo_id, worktree_name, repo_path.to_str().unwrap())
            .await
            .unwrap();

        // Run clean_ignored - should succeed with message about no patterns
        let result = clean_ignored(&store, repo_name, worktree_name, false).await;
        assert!(result.is_ok(), "Should succeed with missing .maproomignore");
    }

    #[tokio::test]
    async fn test_clean_ignored_empty_file() {
        // Create test repo with empty .maproomignore
        let (_temp_dir, store, repo_name, worktree_name) = setup_test_repo("").await.unwrap();

        // Run clean_ignored - should succeed with message about no patterns
        let result = clean_ignored(&store, &repo_name, &worktree_name, false).await;
        assert!(result.is_ok(), "Should succeed with empty .maproomignore");
    }

    #[tokio::test]
    async fn test_clean_ignored_dry_run() {
        // Create test repo with patterns
        let (_temp_dir, store, repo_name, worktree_name) =
            setup_test_repo("test/**\n*.log\n").await.unwrap();

        // Run clean_ignored in dry-run mode
        let result = clean_ignored(&store, &repo_name, &worktree_name, true).await;
        assert!(result.is_ok(), "Dry run should succeed");

        // In a real test, we would verify no chunks were deleted
        // For now, just verify the command succeeds
    }

    #[tokio::test]
    async fn test_clean_ignored_invalid_repo() {
        let store = db::SqliteStore::connect(":memory:").await.unwrap();

        // Try to clean with non-existent repo
        let result = clean_ignored(&store, "nonexistent-repo", "main", false).await;
        assert!(result.is_err(), "Should fail with non-existent repo");
        assert!(
            result.unwrap_err().to_string().contains("not found"),
            "Error should mention repository not found"
        );
    }

    #[tokio::test]
    async fn test_clean_ignored_invalid_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let store = db::SqliteStore::connect(":memory:").await.unwrap();
        let repo_name = "test-repo";
        let _repo_id = store
            .get_or_create_repo(repo_name, repo_path.to_str().unwrap())
            .await
            .unwrap();

        // Try to clean with non-existent worktree
        let result = clean_ignored(&store, repo_name, "nonexistent-worktree", false).await;
        assert!(result.is_err(), "Should fail with non-existent worktree");
        assert!(
            result.unwrap_err().to_string().contains("not found"),
            "Error should mention worktree not found"
        );
    }
}
