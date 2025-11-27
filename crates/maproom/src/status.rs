//! Status module for querying indexed repositories and worktrees.
//!
//! This module uses SqliteStore for database access.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::SqliteStore;

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub repos: Vec<RepoStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoStatus {
    pub name: String,
    pub worktrees: Vec<WorktreeStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorktreeStatus {
    pub name: String,
    pub chunk_count: i64,
    pub last_updated: Option<String>,
}

/// Query database for worktree status information.
///
/// Uses SqliteStore for database access.
pub async fn get_status(
    store: Arc<SqliteStore>,
    repo_filter: Option<String>,
    worktree_filter: Option<String>,
) -> Result<StatusResponse> {
    // Get all repositories
    let repos = store.list_repos().await?;

    let mut repo_statuses = Vec::new();

    for repo in repos {
        // Apply repo filter if specified
        if let Some(ref filter) = repo_filter {
            if repo.name != *filter {
                continue;
            }
        }

        // Get worktrees for this repo
        let worktrees = store.list_worktrees(repo.id).await?;

        let mut worktree_statuses = Vec::new();

        for worktree in worktrees {
            // Apply worktree filter if specified
            if let Some(ref filter) = worktree_filter {
                if worktree.name != *filter {
                    continue;
                }
            }

            // Get chunk count for this worktree
            let chunk_count = store.get_worktree_chunk_count(worktree.id).await?;

            worktree_statuses.push(WorktreeStatus {
                name: worktree.name,
                chunk_count,
                // Note: last_updated is not currently tracked in worktree metadata
                // This would require adding indexed_at to WorktreeInfo
                last_updated: None,
            });
        }

        repo_statuses.push(RepoStatus {
            name: repo.name,
            worktrees: worktree_statuses,
        });
    }

    // Sort repos by name for consistent output
    repo_statuses.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(StatusResponse { repos: repo_statuses })
}

/// Format status as human-readable text
pub fn format_text(status: &StatusResponse) -> String {
    if status.repos.is_empty() {
        return "No repositories indexed yet.\n\nRun 'crewchief-maproom scan' to index a repository.".to_string();
    }

    let mut output = String::new();

    for repo in &status.repos {
        output.push_str(&format!("Repository: {}\n", repo.name));

        if repo.worktrees.is_empty() {
            output.push_str("  No worktrees indexed\n");
        } else {
            for worktree in &repo.worktrees {
                output.push_str(&format!("  Worktree: {}\n", worktree.name));
                output.push_str(&format!(
                    "    Chunks: {}\n",
                    format_number(worktree.chunk_count)
                ));

                if let Some(ref last_updated) = worktree.last_updated {
                    output.push_str(&format!("    Last Updated: {}\n", last_updated));
                }
            }
        }

        output.push('\n');
    }

    output
}

/// Format number with thousands separator
fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
        count += 1;
    }

    result
}

/// Format status as JSON
pub fn format_json(status: &StatusResponse) -> Result<String> {
    serde_json::to_string_pretty(status).map_err(|e| anyhow::anyhow!("Failed to serialize status to JSON: {}", e))
}
