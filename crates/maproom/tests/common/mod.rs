//! Common test utilities for integration tests.
//!
//! This module provides shared test infrastructure including:
//! - Database setup with in-memory SQLite
//! - Test fixture helpers for chunks, embeddings, and edges
//! - Assertion utilities for search results

use anyhow::Result;
use std::path::PathBuf;

use crewchief_maproom::db::sqlite::SqliteStore;
use crewchief_maproom::db::{ChunkRecord, FileRecord};

// Re-export for test convenience
pub use crewchief_maproom::db::sqlite::SqliteStore as TestStore;

/// Create an in-memory SQLite store for testing.
///
/// The store is fully initialized with schema migrations.
/// Each call creates an isolated database instance.
pub async fn setup_test_db() -> Result<SqliteStore> {
    let store = SqliteStore::connect(":memory:").await?;
    store.migrate().await?;
    Ok(store)
}

/// Synchronous wrapper for setup_test_db (for non-async test contexts).
/// Note: Requires tokio runtime to be available.
pub fn test_store() -> SqliteStore {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        setup_test_db().await.expect("Failed to create test database")
    })
}

/// Test database wrapper with helper methods.
pub struct TestDb {
    pub store: SqliteStore,
    pub repo_id: i64,
    pub worktree_id: i64,
    pub commit_id: i64,
}

impl TestDb {
    /// Create a new test database with a pre-initialized repo, worktree, and commit.
    pub async fn new() -> Result<Self> {
        let store = setup_test_db().await?;

        let repo_id = store
            .get_or_create_repo("test-repo", "/tmp/test-repo")
            .await?;

        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/tmp/test-repo")
            .await?;

        let commit_id = store
            .get_or_create_commit(repo_id, "abc123def456", None)
            .await?;

        Ok(Self {
            store,
            repo_id,
            worktree_id,
            commit_id,
        })
    }

    /// Get a reference to the underlying store.
    pub fn store(&self) -> &SqliteStore {
        &self.store
    }

    /// Insert test data with sample chunks.
    pub async fn insert_test_data(&self) -> Result<()> {
        for (relpath, symbol, content, ts_doc) in sample_chunk_data() {
            let file = FileRecord {
                repo_id: self.repo_id,
                worktree_id: self.worktree_id,
                commit_id: self.commit_id,
                relpath: relpath.to_string(),
                language: Some(detect_language(relpath)),
                content_hash: format!("hash_{}", relpath.replace("/", "_")),
                size_bytes: content.len() as i64,
                last_modified: None,
            };
            let file_id = self.store.upsert_file(&file).await?;

            let chunk = ChunkRecord {
                file_id,
                blob_sha: format!("blob_{}", symbol),
                symbol_name: Some(symbol.to_string()),
                kind: "function".to_string(),
                signature: Some(format!("fn {}()", symbol)),
                docstring: None,
                start_line: 1,
                end_line: 10,
                preview: content.to_string(),
                ts_doc_text: ts_doc.to_string(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
                worktree_id: self.worktree_id,
            };
            self.store.insert_chunk(&chunk).await?;
        }
        Ok(())
    }
}

/// Sample chunk data for test fixtures: (relpath, symbol_name, content, ts_doc_text)
pub fn sample_chunk_data() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "src/auth.ts",
            "authenticate",
            "export function authenticate(user: User) { return user.isValid(); }",
            "authenticate user validation",
        ),
        (
            "src/user.ts",
            "User",
            "export class User { constructor(public name: string) {} }",
            "user class constructor",
        ),
        (
            "README.md",
            "readme",
            "# Test Project\n\nThis is a test project for authentication.",
            "test project authentication readme",
        ),
    ]
}

/// Create sample chunks for testing.
pub fn sample_chunks(file_id: i64, worktree_id: i64) -> Vec<ChunkRecord> {
    sample_chunk_data()
        .into_iter()
        .map(|(relpath, symbol, content, ts_doc)| ChunkRecord {
            file_id,
            blob_sha: format!("blob_{}", symbol),
            symbol_name: Some(symbol.to_string()),
            kind: "function".to_string(),
            signature: Some(format!("fn {}()", symbol)),
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: content.to_string(),
            ts_doc_text: ts_doc.to_string(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
            worktree_id,
        })
        .collect()
}

/// Create a sample file record for testing.
pub fn sample_file(repo_id: i64, worktree_id: i64, commit_id: i64, relpath: &str) -> FileRecord {
    FileRecord {
        repo_id,
        worktree_id,
        commit_id,
        relpath: relpath.to_string(),
        language: Some(detect_language(relpath)),
        content_hash: format!("hash_{}", relpath.replace("/", "_")),
        size_bytes: 1024,
        last_modified: None,
    }
}

/// Detect language from file extension.
fn detect_language(path: &str) -> String {
    match path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("md") => "markdown",
        _ => "unknown",
    }
    .to_string()
}

/// Test configuration helper.
pub struct TestConfig {
    config_dir: PathBuf,
}

impl TestConfig {
    /// Create a temporary test configuration directory.
    pub fn new() -> Result<Self> {
        let config_dir = std::env::temp_dir().join(format!(
            "maproom_test_config_{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&config_dir)?;
        Ok(Self { config_dir })
    }

    /// Write a test configuration file.
    pub fn write_config(&self, filename: &str, content: &str) -> Result<PathBuf> {
        let path = self.config_dir.join(filename);
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// Get the configuration directory path.
    pub fn dir(&self) -> &PathBuf {
        &self.config_dir
    }
}

impl Drop for TestConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.config_dir);
    }
}

/// Assertion utilities for search results.
pub mod assertions {
    use crewchief_maproom::search::ChunkSearchResult;

    /// Assert that search results contain expected content.
    pub fn assert_contains_result(results: &[ChunkSearchResult], expected_content: &str) {
        assert!(
            results.iter().any(|r| r.preview.contains(expected_content)),
            "Expected to find content '{}' in results, but it was not present.\nResults: {:?}",
            expected_content,
            results.iter().map(|r| &r.preview).collect::<Vec<_>>()
        );
    }

    /// Assert that search results are ordered by score (descending).
    pub fn assert_ordered_by_score(results: &[ChunkSearchResult]) {
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results are not ordered by score: {} < {}",
                results[i - 1].score,
                results[i].score
            );
        }
    }

    /// Assert that all results have a minimum score threshold.
    pub fn assert_min_score(results: &[ChunkSearchResult], min_score: f64) {
        for result in results {
            assert!(
                result.score as f64 >= min_score,
                "Result has score {} which is below minimum {}",
                result.score,
                min_score
            );
        }
    }

    /// Assert that no results are empty.
    pub fn assert_non_empty_results(results: &[ChunkSearchResult]) {
        assert!(!results.is_empty(), "Expected non-empty search results");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_test_db() {
        let store = setup_test_db().await.expect("Failed to create test database");
        // Verify we can create a repo (proves migrations ran)
        let repo_id = store
            .get_or_create_repo("test", "/test")
            .await
            .expect("Failed to create repo");
        assert!(repo_id > 0);
    }

    #[tokio::test]
    async fn test_db_wrapper() {
        let test_db = TestDb::new().await.expect("Failed to create TestDb");
        assert!(test_db.repo_id > 0);
        assert!(test_db.worktree_id > 0);
        assert!(test_db.commit_id > 0);
    }

    #[tokio::test]
    async fn test_insert_test_data() {
        let test_db = TestDb::new().await.expect("Failed to create TestDb");
        test_db.insert_test_data().await.expect("Failed to insert test data");

        // Verify data was inserted by searching
        let results = test_db.store
            .search_chunks_fts("test-repo", Some("main"), "authenticate", 10, false)
            .await
            .expect("Search failed");

        assert!(!results.is_empty(), "Should find authenticate chunk");
    }

    #[test]
    fn test_config_creation() {
        let test_config = TestConfig::new().expect("Failed to create test config");
        assert!(test_config.dir().exists());
    }

    #[test]
    fn test_sample_chunks_generation() {
        let chunks = sample_chunks(1, 1);
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().any(|c| c.symbol_name.as_deref() == Some("authenticate")));
    }
}
