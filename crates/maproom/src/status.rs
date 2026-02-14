//! Status module for querying indexed repositories and worktrees.
//!
//! This module uses SqliteStore for database access.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::db::traits::StoreCore;
use crate::db::SqliteStore;

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub repos: Vec<RepoStatus>,
    pub index_size_bytes: Option<u64>,
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
    pub embedding_count: i64,
    pub embedding_percentage: f64,
    pub languages: HashMap<String, i64>,
}

/// Query database for worktree status information.
///
/// Uses SqliteStore for database access.
pub async fn get_status(
    store: Arc<SqliteStore>,
    repo_filter: Option<String>,
    worktree_filter: Option<String>,
    _verbose: bool,
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

            // Get embedding count for this worktree
            let embedding_count = store.get_worktree_embedding_count(worktree.id).await?;

            // Calculate embedding percentage (handle division by zero)
            let embedding_percentage = if chunk_count == 0 {
                0.0
            } else {
                (embedding_count as f64 / chunk_count as f64) * 100.0
            };

            // Get language breakdown for this worktree
            let language_breakdown = store.get_worktree_language_breakdown(worktree.id).await?;
            let languages: HashMap<String, i64> = language_breakdown.into_iter().collect();

            // Get last scan timestamp for this worktree
            let last_updated = store.get_worktree_last_scan(worktree.id).await?;

            worktree_statuses.push(WorktreeStatus {
                name: worktree.name,
                chunk_count,
                last_updated,
                embedding_count,
                embedding_percentage,
                languages,
            });
        }

        repo_statuses.push(RepoStatus {
            name: repo.name,
            worktrees: worktree_statuses,
        });
    }

    // Sort repos by name for consistent output
    repo_statuses.sort_by(|a, b| a.name.cmp(&b.name));

    // Get database file size
    let index_size_bytes = get_database_size().await;

    Ok(StatusResponse {
        repos: repo_statuses,
        index_size_bytes,
    })
}

/// Get the size of the database file in bytes.
///
/// Returns `None` for in-memory databases or if the file cannot be read.
async fn get_database_size() -> Option<u64> {
    use crate::db::get_database_url;

    let db_url = match get_database_url() {
        Ok(url) => url,
        Err(_) => return None,
    };

    if db_url.starts_with("sqlite://") {
        let path = db_url.strip_prefix("sqlite://").unwrap();
        match std::fs::metadata(path) {
            Ok(metadata) => Some(metadata.len()),
            Err(_) => None,
        }
    } else {
        None
    }
}

/// Format status as human-readable text
pub fn format_text(status: &StatusResponse, verbose: bool) -> String {
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

                // Embeddings line with count and percentage
                output.push_str(&format!(
                    "    Embeddings: {} ({:.1}%)\n",
                    format_number(worktree.embedding_count),
                    worktree.embedding_percentage
                ));

                // Languages line with sorted list
                output.push_str("    Languages: ");
                if worktree.languages.is_empty() {
                    output.push_str("(none)\n");
                } else {
                    // Sort languages by count descending, then alphabetically
                    let mut lang_pairs: Vec<_> = worktree.languages.iter().collect();
                    lang_pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

                    // Determine how many to show
                    let show_count = if verbose {
                        lang_pairs.len()
                    } else {
                        lang_pairs.len().min(5)
                    };
                    let truncated = !verbose && lang_pairs.len() > 5;

                    // Format the languages
                    let lang_strs: Vec<String> = lang_pairs
                        .iter()
                        .take(show_count)
                        .map(|(lang, count)| format!("{} ({})", lang, count))
                        .collect();
                    output.push_str(&lang_strs.join(", "));

                    // Add truncation indicator if needed
                    if truncated {
                        let remaining = lang_pairs.len() - show_count;
                        output.push_str(&format!(" ...and {} more (use --verbose)", remaining));
                    }
                    output.push('\n');
                }

                // Last scan line
                output.push_str("    Last scan: ");
                if let Some(ref last_updated) = worktree.last_updated {
                    output.push_str(last_updated);
                } else {
                    output.push_str("never");
                }
                output.push('\n');
            }
        }

        output.push('\n');
    }

    // Add total index size at the end (once, not per-worktree)
    if let Some(bytes) = status.index_size_bytes {
        output.push_str(&format!("Total index size: {}\n", format_size_mb(bytes)));
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

/// Format bytes as megabytes with 2 decimal places
fn format_size_mb(bytes: u64) -> String {
    format!("{:.2} MB", bytes as f64 / 1_048_576.0)
}

/// Format status as JSON
pub fn format_json(status: &StatusResponse) -> Result<String> {
    serde_json::to_string_pretty(status)
        .map_err(|e| anyhow::anyhow!("Failed to serialize status to JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sqlite::SqliteStore;
    use crate::db::traits::StoreChunks;
    use crate::db::traits::StoreMigration;
    use crate::db::{ChunkRecord, FileRecord};
    use rusqlite::params;

    #[tokio::test]
    async fn test_worktree_status_with_populated_data() {
        let store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());
        store.migrate().await.unwrap();

        // Create test data using the available methods
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create files with different languages
        let file1 = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file1_id = store.upsert_file(&file1).await.unwrap();

        let file2 = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.py".to_string(),
            language: Some("python".to_string()),
            content_hash: "hash2".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file2_id = store.upsert_file(&file2).await.unwrap();

        // Create chunks
        let chunk1 = ChunkRecord {
            file_id: file1_id,
            worktree_id,
            blob_sha: "blob1".to_string(),
            symbol_name: Some("fn1".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn fn1() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk1).await.unwrap();

        let chunk2 = ChunkRecord {
            file_id: file2_id,
            worktree_id,
            blob_sha: "blob2".to_string(),
            symbol_name: Some("fn2".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "def fn2(): pass".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk2).await.unwrap();

        // Create embeddings for some chunks
        store
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
                     VALUES (?1, ?2, ?3, ?4)",
                    params!["blob1", vec![0u8; 4096], 1024, "test-model"],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        // Create index_state entry
        store
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO index_state (worktree_id, tree_sha, chunks_processed, embeddings_generated, last_indexed)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![worktree_id, "tree123", 0, 0, "2024-01-01T12:00:00Z"],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        // Get status
        let status = get_status(store.clone(), None, None, false).await.unwrap();

        // Verify response structure
        assert_eq!(status.repos.len(), 1);
        assert_eq!(status.repos[0].name, "test-repo");
        assert_eq!(status.repos[0].worktrees.len(), 1);

        // Verify worktree status
        let worktree_status = &status.repos[0].worktrees[0];
        assert_eq!(worktree_status.name, "main");
        assert_eq!(worktree_status.chunk_count, 2);
        assert_eq!(worktree_status.embedding_count, 1);
        assert_eq!(worktree_status.embedding_percentage, 50.0);
        assert_eq!(
            worktree_status.last_updated,
            Some("2024-01-01T12:00:00Z".to_string())
        );

        // Verify language breakdown
        assert_eq!(worktree_status.languages.len(), 2);
        assert_eq!(worktree_status.languages.get("rust"), Some(&1));
        assert_eq!(worktree_status.languages.get("python"), Some(&1));
    }

    #[tokio::test]
    async fn test_worktree_status_with_zero_chunks() {
        let store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());
        store.migrate().await.unwrap();

        // Create test data with no chunks
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let _worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();

        // Get status
        let status = get_status(store.clone(), None, None, false).await.unwrap();

        // Verify worktree status
        let worktree_status = &status.repos[0].worktrees[0];
        assert_eq!(worktree_status.chunk_count, 0);
        assert_eq!(worktree_status.embedding_count, 0);
        assert_eq!(worktree_status.embedding_percentage, 0.0); // Division by zero handled
        assert_eq!(worktree_status.last_updated, None); // No index_state entry
        assert_eq!(worktree_status.languages.len(), 0);
    }

    #[tokio::test]
    async fn test_index_size_bytes_for_in_memory_database() {
        // Set environment variable to use :memory: database
        std::env::set_var("MAPROOM_DATABASE_URL", ":memory:");

        let store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());
        store.migrate().await.unwrap();

        // Create minimal test data
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();

        // Get status
        let status = get_status(store.clone(), None, None, false).await.unwrap();

        // For in-memory database, index_size_bytes should be None
        assert_eq!(status.index_size_bytes, None);

        // Cleanup
        std::env::remove_var("MAPROOM_DATABASE_URL");
    }

    #[tokio::test]
    async fn test_embedding_percentage_calculation() {
        let store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());
        store.migrate().await.unwrap();

        // Create test data
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create 4 chunks
        for i in 0..4 {
            let chunk = ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: format!("blob{}", i),
                symbol_name: Some(format!("fn{}", i)),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
                start_line: (i * 10 + 1) as i32,
                end_line: (i * 10 + 10) as i32,
                preview: format!("fn fn{}() {{}}", i),
                ts_doc_text: String::new(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
            };
            store.insert_chunk(&chunk).await.unwrap();
        }

        // Create embeddings for 3 of 4 chunks
        for i in 0..3 {
            let blob_sha = format!("blob{}", i);
            store
                .run(move |conn| {
                    conn.execute(
                        "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![blob_sha, vec![0u8; 4096], 1024, "test-model"],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }

        // Get status
        let status = get_status(store.clone(), None, None, false).await.unwrap();

        // Verify embedding percentage (3/4 = 75%)
        let worktree_status = &status.repos[0].worktrees[0];
        assert_eq!(worktree_status.chunk_count, 4);
        assert_eq!(worktree_status.embedding_count, 3);
        assert_eq!(worktree_status.embedding_percentage, 75.0);
    }

    #[tokio::test]
    async fn test_language_breakdown_conversion() {
        let store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());
        store.migrate().await.unwrap();

        // Create test data
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        // Create files with various languages
        let files = vec![
            ("test1.rs", "rust"),
            ("test2.rs", "rust"),
            ("test.py", "python"),
            ("test.go", "go"),
        ];

        for (path, lang) in files {
            let file = FileRecord {
                repo_id,
                worktree_id,
                commit_id,
                relpath: path.to_string(),
                language: Some(lang.to_string()),
                content_hash: format!("hash_{}", path),
                size_bytes: 100,
                last_modified: None,
            };
            store.upsert_file(&file).await.unwrap();
        }

        // Get status
        let status = get_status(store.clone(), None, None, false).await.unwrap();

        // Verify language breakdown
        let worktree_status = &status.repos[0].worktrees[0];
        assert_eq!(worktree_status.languages.len(), 3);
        assert_eq!(worktree_status.languages.get("rust"), Some(&2));
        assert_eq!(worktree_status.languages.get("python"), Some(&1));
        assert_eq!(worktree_status.languages.get("go"), Some(&1));
    }

    #[test]
    fn test_format_text_full_output() {
        let mut languages = HashMap::new();
        languages.insert("md".to_string(), 122);
        languages.insert("json".to_string(), 36);
        languages.insert("py".to_string(), 12);

        let status = StatusResponse {
            repos: vec![RepoStatus {
                name: "manifoldlogic/claude-code-plugins".to_string(),
                worktrees: vec![WorktreeStatus {
                    name: "main".to_string(),
                    chunk_count: 6276,
                    embedding_count: 6226,
                    embedding_percentage: 99.2,
                    languages,
                    last_updated: Some("2026-02-04 00:59:41".to_string()),
                }],
            }],
            index_size_bytes: Some(1_405_456), // ~1.34 MB
        };

        let output = format_text(&status, false);
        assert!(output.contains("Repository: manifoldlogic/claude-code-plugins"));
        assert!(output.contains("Worktree: main"));
        assert!(output.contains("Chunks: 6,276"));
        assert!(output.contains("Embeddings: 6,226 (99.2%)"));
        assert!(output.contains("Languages: md (122), json (36), py (12)"));
        assert!(output.contains("Last scan: 2026-02-04 00:59:41"));
        assert!(output.contains("Total index size: 1.34 MB"));
    }

    #[test]
    fn test_format_text_zero_chunks() {
        let status = StatusResponse {
            repos: vec![RepoStatus {
                name: "test-repo".to_string(),
                worktrees: vec![WorktreeStatus {
                    name: "main".to_string(),
                    chunk_count: 0,
                    embedding_count: 0,
                    embedding_percentage: 0.0,
                    languages: HashMap::new(),
                    last_updated: None,
                }],
            }],
            index_size_bytes: None,
        };

        let output = format_text(&status, false);
        assert!(output.contains("Chunks: 0"));
        assert!(output.contains("Embeddings: 0 (0.0%)"));
        assert!(output.contains("Languages: (none)"));
        assert!(output.contains("Last scan: never"));
        assert!(!output.contains("Total index size"));
    }

    #[test]
    fn test_format_text_verbose_shows_all_languages() {
        let mut languages = HashMap::new();
        for i in 0..10 {
            languages.insert(format!("lang{}", i), i as i64);
        }

        let status = StatusResponse {
            repos: vec![RepoStatus {
                name: "test-repo".to_string(),
                worktrees: vec![WorktreeStatus {
                    name: "main".to_string(),
                    chunk_count: 100,
                    embedding_count: 100,
                    embedding_percentage: 100.0,
                    languages,
                    last_updated: Some("2026-02-04 00:00:00".to_string()),
                }],
            }],
            index_size_bytes: None,
        };

        let output = format_text(&status, true);
        // Should show all 10 languages in verbose mode
        assert!(output.contains("lang0"));
        assert!(output.contains("lang9"));
        assert!(!output.contains("...and"));
    }

    #[test]
    fn test_format_text_truncates_languages_non_verbose() {
        let mut languages = HashMap::new();
        for i in 0..10 {
            languages.insert(format!("lang{}", i), (10 - i) as i64); // Descending counts
        }

        let status = StatusResponse {
            repos: vec![RepoStatus {
                name: "test-repo".to_string(),
                worktrees: vec![WorktreeStatus {
                    name: "main".to_string(),
                    chunk_count: 100,
                    embedding_count: 100,
                    embedding_percentage: 100.0,
                    languages,
                    last_updated: Some("2026-02-04 00:00:00".to_string()),
                }],
            }],
            index_size_bytes: None,
        };

        let output = format_text(&status, false);
        // Should show only top 5 and truncation indicator
        assert!(output.contains("...and 5 more (use --verbose)"));
    }

    #[test]
    fn test_format_text_exactly_five_languages_no_truncation() {
        let mut languages = HashMap::new();
        languages.insert("lang1".to_string(), 5);
        languages.insert("lang2".to_string(), 4);
        languages.insert("lang3".to_string(), 3);
        languages.insert("lang4".to_string(), 2);
        languages.insert("lang5".to_string(), 1);

        let status = StatusResponse {
            repos: vec![RepoStatus {
                name: "test-repo".to_string(),
                worktrees: vec![WorktreeStatus {
                    name: "main".to_string(),
                    chunk_count: 100,
                    embedding_count: 100,
                    embedding_percentage: 100.0,
                    languages,
                    last_updated: Some("2026-02-04 00:00:00".to_string()),
                }],
            }],
            index_size_bytes: None,
        };

        let output = format_text(&status, false);
        // Exactly 5 languages should show no truncation indicator
        assert!(!output.contains("...and"));
        assert!(output.contains("lang1 (5)"));
        assert!(output.contains("lang5 (1)"));
    }

    #[test]
    fn test_format_text_six_languages_shows_truncation() {
        let mut languages = HashMap::new();
        for i in 0..6 {
            languages.insert(format!("lang{}", i), (6 - i) as i64);
        }

        let status = StatusResponse {
            repos: vec![RepoStatus {
                name: "test-repo".to_string(),
                worktrees: vec![WorktreeStatus {
                    name: "main".to_string(),
                    chunk_count: 100,
                    embedding_count: 100,
                    embedding_percentage: 100.0,
                    languages,
                    last_updated: Some("2026-02-04 00:00:00".to_string()),
                }],
            }],
            index_size_bytes: None,
        };

        let output = format_text(&status, false);
        // 6 languages should show truncation with "...and 1 more"
        assert!(output.contains("...and 1 more (use --verbose)"));
    }

    #[test]
    fn test_format_text_multi_repo_shows_index_size_once() {
        let status = StatusResponse {
            repos: vec![
                RepoStatus {
                    name: "repo1".to_string(),
                    worktrees: vec![WorktreeStatus {
                        name: "main".to_string(),
                        chunk_count: 100,
                        embedding_count: 100,
                        embedding_percentage: 100.0,
                        languages: HashMap::new(),
                        last_updated: Some("2026-02-04 00:00:00".to_string()),
                    }],
                },
                RepoStatus {
                    name: "repo2".to_string(),
                    worktrees: vec![WorktreeStatus {
                        name: "main".to_string(),
                        chunk_count: 200,
                        embedding_count: 200,
                        embedding_percentage: 100.0,
                        languages: HashMap::new(),
                        last_updated: Some("2026-02-04 00:00:00".to_string()),
                    }],
                },
            ],
            index_size_bytes: Some(2_097_152), // 2.00 MB
        };

        let output = format_text(&status, false);
        // Should show index size exactly once at the end
        let size_occurrences = output.matches("Total index size:").count();
        assert_eq!(size_occurrences, 1);
        assert!(output.contains("Total index size: 2.00 MB"));
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size_mb(0), "0.00 MB");
        assert_eq!(format_size_mb(1_048_576), "1.00 MB");
        assert_eq!(format_size_mb(1_405_456), "1.34 MB");
        assert_eq!(format_size_mb(2_097_152), "2.00 MB");
        assert_eq!(format_size_mb(10_485_760), "10.00 MB");
        assert_eq!(format_size_mb(1_073_741_824), "1024.00 MB");
    }

    #[test]
    fn test_language_sorting_by_count_then_alphabetically() {
        let mut languages = HashMap::new();
        languages.insert("python".to_string(), 10);
        languages.insert("rust".to_string(), 10); // Same count as python
        languages.insert("javascript".to_string(), 5);
        languages.insert("go".to_string(), 5); // Same count as javascript

        let status = StatusResponse {
            repos: vec![RepoStatus {
                name: "test-repo".to_string(),
                worktrees: vec![WorktreeStatus {
                    name: "main".to_string(),
                    chunk_count: 100,
                    embedding_count: 100,
                    embedding_percentage: 100.0,
                    languages,
                    last_updated: Some("2026-02-04 00:00:00".to_string()),
                }],
            }],
            index_size_bytes: None,
        };

        let output = format_text(&status, false);
        // Should be sorted: python (10), rust (10), go (5), javascript (5)
        // (descending by count, then alphabetically for ties)
        let lang_line = output.lines().find(|l| l.contains("Languages:")).unwrap();
        let python_pos = lang_line.find("python").unwrap();
        let rust_pos = lang_line.find("rust").unwrap();
        let go_pos = lang_line.find("go").unwrap();
        let javascript_pos = lang_line.find("javascript").unwrap();

        // Higher counts come first
        assert!(python_pos < go_pos);
        assert!(rust_pos < javascript_pos);

        // Within same count, alphabetical order
        assert!(python_pos < rust_pos); // 'p' < 'r'
        assert!(go_pos < javascript_pos); // 'g' < 'j'
    }
}
