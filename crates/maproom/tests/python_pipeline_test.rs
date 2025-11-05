//! Integration tests for Python file indexing pipeline.
//!
//! Verifies end-to-end integration of Python language support:
//! - .py file detection as Python language
//! - Python parser invocation for .py files
//! - File scanner inclusion of Python files during scan operations
//! - Indexed chunks creation with language="python" tag
//! - Full scan → parse → index flow with sample Python code

use anyhow::Result;
use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::indexer;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test helper to set up a temporary repository with database.
struct TestRepo {
    temp_dir: TempDir,
    pool: PgPool,
}

impl TestRepo {
    /// Create a new test repository with initialized database schema.
    async fn new() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let temp_dir = TempDir::new()?;
        let pool = create_pool().await?;

        // Set up database schema
        Self::setup_schema(&pool).await?;

        Ok(Self { temp_dir, pool })
    }

    /// Set up minimal database schema for testing.
    async fn setup_schema(pool: &PgPool) -> Result<()> {
        let client = pool.get().await?;

        // Create schema if it doesn't exist
        client
            .batch_execute(
                r#"
                CREATE SCHEMA IF NOT EXISTS maproom;

                CREATE TABLE IF NOT EXISTS maproom.repos (
                    id BIGSERIAL PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    root_path TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS maproom.worktrees (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    name TEXT NOT NULL,
                    abs_path TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(repo_id, name)
                );

                CREATE TABLE IF NOT EXISTS maproom.commits (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    sha TEXT NOT NULL,
                    committed_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(repo_id, sha)
                );

                CREATE TABLE IF NOT EXISTS maproom.files (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    worktree_id BIGINT NOT NULL REFERENCES maproom.worktrees(id),
                    commit_id BIGINT NOT NULL REFERENCES maproom.commits(id),
                    relpath TEXT NOT NULL,
                    language TEXT,
                    blake3_hash TEXT,
                    size_bytes INTEGER,
                    last_modified TIMESTAMPTZ,
                    UNIQUE(commit_id, relpath, blake3_hash)
                );

                CREATE INDEX IF NOT EXISTS idx_files_worktree ON maproom.files(worktree_id);
                CREATE INDEX IF NOT EXISTS idx_files_hash ON maproom.files(blake3_hash);

                CREATE TABLE IF NOT EXISTS maproom.chunks (
                    id BIGSERIAL PRIMARY KEY,
                    file_id BIGINT NOT NULL REFERENCES maproom.files(id) ON DELETE CASCADE,
                    symbol_name TEXT,
                    kind TEXT NOT NULL,
                    signature TEXT,
                    docstring TEXT,
                    start_line INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    preview TEXT NOT NULL,
                    ts_doc TEXT NOT NULL,
                    base_score DOUBLE PRECISION DEFAULT 1.0,
                    boost DOUBLE PRECISION DEFAULT 0.0,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS idx_chunks_file ON maproom.chunks(file_id);
                CREATE INDEX IF NOT EXISTS idx_chunks_symbol ON maproom.chunks(symbol_name);
                "#,
            )
            .await?;

        Ok(())
    }

    /// Get the temp directory path.
    fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get the database pool.
    fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Query chunks from database for a given file path.
    async fn get_chunks(&self, relpath: &str) -> Result<Vec<ChunkRow>> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                "SELECT c.symbol_name, c.kind, c.signature, c.docstring, c.start_line, c.end_line, f.language
                 FROM maproom.chunks c
                 JOIN maproom.files f ON c.file_id = f.id
                 WHERE f.relpath = $1
                 ORDER BY c.start_line",
                &[&relpath],
            )
            .await?;

        Ok(rows
            .iter()
            .map(|row| ChunkRow {
                symbol_name: row.get(0),
                kind: row.get(1),
                signature: row.get(2),
                docstring: row.get(3),
                start_line: row.get(4),
                end_line: row.get(5),
                language: row.get(6),
            })
            .collect())
    }

    /// Query file metadata from database.
    async fn get_file_metadata(&self, relpath: &str) -> Result<Option<FileRow>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT language, blake3_hash, size_bytes FROM maproom.files WHERE relpath = $1",
                &[&relpath],
            )
            .await?;

        Ok(row.map(|r| FileRow {
            language: r.get(0),
            blake3_hash: r.get(1),
            size_bytes: r.get(2),
        }))
    }
}

#[derive(Debug)]
struct ChunkRow {
    symbol_name: Option<String>,
    kind: String,
    signature: Option<String>,
    docstring: Option<String>,
    start_line: i32,
    end_line: i32,
    language: Option<String>,
}

#[derive(Debug)]
struct FileRow {
    language: Option<String>,
    blake3_hash: Option<String>,
    size_bytes: Option<i32>,
}

/// Test that .py files are correctly detected as Python language.
#[tokio::test]
async fn test_py_file_language_detection() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Create a simple Python file
    let python_file = test_repo.path().join("test.py");
    fs::write(
        &python_file,
        r#"
def hello():
    """Say hello."""
    print("Hello, world!")
"#,
    )?;

    // Scan the worktree
    let client = test_repo.pool().get().await?;
    indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        None,
        None,
        None,
    )
    .await?;

    // Verify file was detected as Python
    let file_metadata = test_repo
        .get_file_metadata("test.py")
        .await?
        .expect("File should be indexed");

    assert_eq!(
        file_metadata.language.as_deref(),
        Some("py"),
        "File should be detected as Python language"
    );

    Ok(())
}

/// Test that Python parser is invoked for .py files and extracts symbols correctly.
#[tokio::test]
async fn test_python_parser_invocation() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Create Python file with functions and classes
    let python_file = test_repo.path().join("example.py");
    fs::write(
        &python_file,
        r#"
class Calculator:
    """A simple calculator class."""

    def add(self, a, b):
        """Add two numbers."""
        return a + b

    def subtract(self, a, b):
        """Subtract two numbers."""
        return a - b

def multiply(x, y):
    """Multiply two numbers."""
    return x * y
"#,
    )?;

    // Scan the worktree
    let client = test_repo.pool().get().await?;
    indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        None,
        None,
        None,
    )
    .await?;

    // Verify chunks were extracted
    let chunks = test_repo.get_chunks("example.py").await?;

    assert!(
        chunks.len() >= 4,
        "Should extract at least 4 symbols (Calculator, add, subtract, multiply)"
    );

    // Verify class extraction
    let calculator_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Calculator".to_string()));
    assert!(calculator_class.is_some(), "Should extract Calculator class");
    let calculator_class = calculator_class.unwrap();
    assert_eq!(calculator_class.kind, "class");
    assert!(calculator_class.docstring.is_some());
    assert!(calculator_class
        .docstring
        .as_ref()
        .unwrap()
        .contains("simple calculator"));

    // Verify method extraction
    let add_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("add".to_string()));
    assert!(add_method.is_some(), "Should extract add method");
    let add_method = add_method.unwrap();
    assert_eq!(add_method.kind, "method");

    // Verify function extraction
    let multiply_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("multiply".to_string()));
    assert!(multiply_func.is_some(), "Should extract multiply function");
    let multiply_func = multiply_func.unwrap();
    assert_eq!(multiply_func.kind, "func");

    Ok(())
}

/// Test that file scanner includes Python files during scan operations.
#[tokio::test]
async fn test_scanner_includes_python_files() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Create multiple Python files
    fs::write(test_repo.path().join("module1.py"), "def func1(): pass")?;
    fs::write(test_repo.path().join("module2.py"), "class Class2: pass")?;
    fs::create_dir_all(test_repo.path().join("subdir"))?;
    fs::write(
        test_repo.path().join("subdir/module3.py"),
        "def func3(): pass",
    )?;

    // Create non-Python files (should be skipped or indexed separately)
    fs::write(test_repo.path().join("README.md"), "# Readme")?;
    fs::write(test_repo.path().join("data.txt"), "some data")?;

    // Scan the worktree
    let client = test_repo.pool().get().await?;
    indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        None,
        None,
        None,
    )
    .await?;

    // Verify all Python files were scanned
    let module1_meta = test_repo.get_file_metadata("module1.py").await?;
    assert!(module1_meta.is_some(), "module1.py should be indexed");
    assert_eq!(module1_meta.unwrap().language.as_deref(), Some("py"));

    let module2_meta = test_repo.get_file_metadata("module2.py").await?;
    assert!(module2_meta.is_some(), "module2.py should be indexed");
    assert_eq!(module2_meta.unwrap().language.as_deref(), Some("py"));

    let module3_meta = test_repo.get_file_metadata("subdir/module3.py").await?;
    assert!(module3_meta.is_some(), "subdir/module3.py should be indexed");
    assert_eq!(module3_meta.unwrap().language.as_deref(), Some("py"));

    // Verify markdown was also indexed (we support markdown)
    let readme_meta = test_repo.get_file_metadata("README.md").await?;
    assert!(readme_meta.is_some(), "README.md should be indexed");

    Ok(())
}

/// Test that indexed chunks have language="python" tag.
#[tokio::test]
async fn test_chunks_have_python_language_tag() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Create Python file
    let python_file = test_repo.path().join("tagged.py");
    fs::write(
        &python_file,
        r#"
def process_data(data):
    """Process the data."""
    return data.strip()

class DataProcessor:
    """Data processor class."""
    pass
"#,
    )?;

    // Scan the worktree
    let client = test_repo.pool().get().await?;
    indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        None,
        None,
        None,
    )
    .await?;

    // Verify all chunks have Python language tag
    let chunks = test_repo.get_chunks("tagged.py").await?;

    assert!(!chunks.is_empty(), "Should have extracted chunks");

    for chunk in &chunks {
        assert_eq!(
            chunk.language.as_deref(),
            Some("py"),
            "All chunks should have language='py' tag"
        );
    }

    Ok(())
}

/// Test full scan → parse → index flow with complex Python code.
#[tokio::test]
async fn test_full_pipeline_with_complex_python() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Use the sample_api.py fixture (comprehensive Python file)
    let fixture_source = fs::read_to_string("tests/fixtures/python/sample_api.py")?;
    let api_file = test_repo.path().join("api.py");
    fs::write(&api_file, fixture_source)?;

    // Scan the worktree
    let client = test_repo.pool().get().await?;
    indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        None,
        None,
        None,
    )
    .await?;

    // Verify file was indexed
    let file_metadata = test_repo
        .get_file_metadata("api.py")
        .await?
        .expect("api.py should be indexed");

    assert_eq!(file_metadata.language.as_deref(), Some("py"));
    assert!(file_metadata.blake3_hash.is_some());
    assert!(file_metadata.size_bytes.is_some());

    // Verify comprehensive chunk extraction
    let chunks = test_repo.get_chunks("api.py").await?;

    // Should extract: imports, constants, classes, methods, functions, decorators
    assert!(
        chunks.len() >= 10,
        "Should extract many symbols from sample_api.py, got {}",
        chunks.len()
    );

    // Verify specific symbols exist

    // 1. Check for constants
    let api_version_const = chunks
        .iter()
        .find(|c| c.symbol_name == Some("API_VERSION".to_string()));
    assert!(
        api_version_const.is_some(),
        "Should extract API_VERSION constant"
    );
    assert_eq!(api_version_const.unwrap().kind, "constant");

    // 2. Check for classes with decorators
    let request_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Request".to_string()));
    assert!(request_class.is_some(), "Should extract Request class");
    assert_eq!(request_class.unwrap().kind, "class");

    let response_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Response".to_string()));
    assert!(response_class.is_some(), "Should extract Response class");

    // 3. Check for exception class
    let api_exception = chunks
        .iter()
        .find(|c| c.symbol_name == Some("APIException".to_string()));
    assert!(
        api_exception.is_some(),
        "Should extract APIException class"
    );

    // 4. Check for abstract base class
    let base_client = chunks
        .iter()
        .find(|c| c.symbol_name == Some("BaseClient".to_string()));
    assert!(base_client.is_some(), "Should extract BaseClient class");

    // 5. Check for inherited class
    let http_client = chunks
        .iter()
        .find(|c| c.symbol_name == Some("HTTPClient".to_string()));
    assert!(http_client.is_some(), "Should extract HTTPClient class");

    // 6. Check for async method
    let send_request = chunks
        .iter()
        .find(|c| c.symbol_name == Some("send_request".to_string()));
    if let Some(send_req) = send_request {
        assert_eq!(send_req.kind, "async_method");
    }

    // 7. Check for static method
    let validate_url = chunks
        .iter()
        .find(|c| c.symbol_name == Some("validate_url".to_string()));
    assert!(validate_url.is_some(), "Should extract validate_url method");

    // 8. Check for async function
    let fetch_data = chunks
        .iter()
        .find(|c| c.symbol_name == Some("fetch_data".to_string()));
    assert!(fetch_data.is_some(), "Should extract fetch_data function");
    if let Some(fetch) = fetch_data {
        assert_eq!(fetch.kind, "async_func");
    }

    // 9. Check for regular function
    let parse_response = chunks
        .iter()
        .find(|c| c.symbol_name == Some("parse_response".to_string()));
    assert!(
        parse_response.is_some(),
        "Should extract parse_response function"
    );

    // 10. Check for decorator function
    let retry_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("retry".to_string()));
    assert!(retry_func.is_some(), "Should extract retry function");

    // Verify docstrings were extracted
    let documented_chunks: Vec<_> = chunks.iter().filter(|c| c.docstring.is_some()).collect();
    assert!(
        documented_chunks.len() >= 5,
        "Should extract docstrings from multiple symbols"
    );

    Ok(())
}

/// Test that language filter works with Python files.
#[tokio::test]
async fn test_language_filter_for_python() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Create mixed language files
    fs::write(test_repo.path().join("script.py"), "def main(): pass")?;
    fs::write(test_repo.path().join("lib.js"), "function main() {}")?;
    fs::write(test_repo.path().join("app.ts"), "function main(): void {}")?;

    // Scan with Python filter only
    let client = test_repo.pool().get().await?;
    indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        Some(vec!["py".to_string()]),
        None,
        None,
    )
    .await?;

    // Verify only Python file was indexed
    let py_meta = test_repo.get_file_metadata("script.py").await?;
    assert!(py_meta.is_some(), "Python file should be indexed");

    let js_meta = test_repo.get_file_metadata("lib.js").await?;
    assert!(
        js_meta.is_none(),
        "JavaScript file should not be indexed with py filter"
    );

    let ts_meta = test_repo.get_file_metadata("app.ts").await?;
    assert!(
        ts_meta.is_none(),
        "TypeScript file should not be indexed with py filter"
    );

    Ok(())
}

/// Test that Python files with imports are indexed correctly.
#[tokio::test]
async fn test_python_imports_extraction() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Create Python file with various import styles
    let python_file = test_repo.path().join("imports.py");
    fs::write(
        &python_file,
        r#"
import os
import sys
from typing import List, Dict
from pathlib import Path

def process_files(files: List[Path]) -> Dict[str, str]:
    """Process a list of files."""
    return {str(f): os.path.basename(str(f)) for f in files}
"#,
    )?;

    // Scan the worktree
    let client = test_repo.pool().get().await?;
    indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        None,
        None,
        None,
    )
    .await?;

    // Verify chunks were extracted including imports chunk
    let chunks = test_repo.get_chunks("imports.py").await?;

    // Should have imports chunk + process_files function
    assert!(chunks.len() >= 2, "Should extract imports and function");

    // Check for imports chunk
    let imports_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("__imports__".to_string()));
    assert!(imports_chunk.is_some(), "Should extract imports chunk");
    assert_eq!(imports_chunk.unwrap().kind, "imports");

    // Check for function
    let func_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("process_files".to_string()));
    assert!(func_chunk.is_some(), "Should extract process_files function");

    Ok(())
}

/// Test that malformed Python files are handled gracefully.
#[tokio::test]
async fn test_malformed_python_handling() -> Result<()> {
    let test_repo = TestRepo::new().await?;

    // Create malformed Python file
    let python_file = test_repo.path().join("malformed.py");
    fs::write(
        &python_file,
        r#"
def incomplete_function(
    # Missing closing parenthesis and body

class IncompleteClass
    # Missing colon and body
"#,
    )?;

    // Scan should not crash
    let client = test_repo.pool().get().await?;
    let result = indexer::scan_worktree(
        &client,
        "test-repo",
        "main",
        test_repo.path(),
        "abc123",
        4,
        None,
        None,
        None,
    )
    .await;

    // Should complete successfully even with malformed file
    assert!(result.is_ok(), "Scan should handle malformed Python gracefully");

    // File should still be indexed (even if chunks are minimal/empty)
    let file_metadata = test_repo.get_file_metadata("malformed.py").await?;
    assert!(
        file_metadata.is_some(),
        "Malformed file should still be indexed"
    );

    Ok(())
}
