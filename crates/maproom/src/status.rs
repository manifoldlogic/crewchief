use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

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

/// Query database for worktree status information
pub async fn get_status(
    repo_filter: Option<String>,
    worktree_filter: Option<String>,
) -> Result<StatusResponse> {
    let database_url = env::var("MAPROOM_DATABASE_URL")
        .context("MAPROOM_DATABASE_URL must be set")?;

    let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls)
        .await
        .context("Failed to connect to database")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("database connection error: {}", e);
        }
    });

    // Build query based on filters
    let query = if let Some(ref _repo) = repo_filter {
        if let Some(ref _worktree) = worktree_filter {
            // Filter by both repo and worktree
            "SELECT r.name as repo_name, w.name as worktree_name, w.indexed_at,
                    COUNT(DISTINCT c.id) FILTER (WHERE c.worktree_ids @> jsonb_build_array(w.id)) as chunk_count
             FROM repos r
             JOIN worktrees w ON w.repo_id = r.id
             LEFT JOIN chunks c ON c.worktree_ids @> jsonb_build_array(w.id)
             WHERE r.name = $1 AND w.name = $2
             GROUP BY r.name, w.name, w.indexed_at
             ORDER BY r.name, w.name"
        } else {
            // Filter by repo only
            "SELECT r.name as repo_name, w.name as worktree_name, w.indexed_at,
                    COUNT(DISTINCT c.id) FILTER (WHERE c.worktree_ids @> jsonb_build_array(w.id)) as chunk_count
             FROM repos r
             JOIN worktrees w ON w.repo_id = r.id
             LEFT JOIN chunks c ON c.worktree_ids @> jsonb_build_array(w.id)
             WHERE r.name = $1
             GROUP BY r.name, w.name, w.indexed_at
             ORDER BY r.name, w.name"
        }
    } else {
        // No filters - get all
        "SELECT r.name as repo_name, w.name as worktree_name, w.indexed_at,
                COUNT(DISTINCT c.id) FILTER (WHERE c.worktree_ids @> jsonb_build_array(w.id)) as chunk_count
         FROM repos r
         JOIN worktrees w ON w.repo_id = r.id
         LEFT JOIN chunks c ON c.worktree_ids @> jsonb_build_array(w.id)
         GROUP BY r.name, w.name, w.indexed_at
         ORDER BY r.name, w.name"
    };

    // Execute query with appropriate parameters
    let rows = if let Some(ref repo) = repo_filter {
        if let Some(ref worktree) = worktree_filter {
            client.query(query, &[repo, worktree]).await?
        } else {
            client.query(query, &[repo]).await?
        }
    } else {
        client.query(query, &[]).await?
    };

    // Group results by repo
    let mut repos_map: std::collections::HashMap<String, Vec<WorktreeStatus>> =
        std::collections::HashMap::new();

    for row in rows {
        let repo_name: String = row.get("repo_name");
        let worktree_name: String = row.get("worktree_name");
        let chunk_count: i64 = row.get("chunk_count");
        let indexed_at: Option<chrono::DateTime<chrono::Utc>> = row.get("indexed_at");

        let worktree_status = WorktreeStatus {
            name: worktree_name,
            chunk_count,
            last_updated: indexed_at.map(|dt| dt.to_rfc3339()),
        };

        repos_map
            .entry(repo_name)
            .or_insert_with(Vec::new)
            .push(worktree_status);
    }

    // Convert to response format
    let mut repos: Vec<RepoStatus> = repos_map
        .into_iter()
        .map(|(name, worktrees)| RepoStatus { name, worktrees })
        .collect();

    // Sort repos by name for consistent output
    repos.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(StatusResponse { repos })
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
                output.push_str(&format!("    Chunks: {}\n", format_number(worktree.chunk_count)));

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
    serde_json::to_string_pretty(status).context("Failed to serialize status to JSON")
}
