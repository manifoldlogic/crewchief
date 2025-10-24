//! Integration tests for context assembler.
//!
//! These tests verify the assembler works end-to-end with a real database
//! and file system.

use anyhow::Result;
use crewchief_maproom::context::{
    BasicContextAssembler, ContextAssembler, ExpandOptions,
};
use crewchief_maproom::db::{create_pool, get_or_create_commit, get_or_create_repo, get_or_create_worktree, insert_chunk, upsert_file};
use std::env;
use tempfile::TempDir;
use tokio::fs;

/// Helper to check if we should skip database integration tests.
fn should_skip_db_test() -> bool {
    env::var("DATABASE_URL").is_err()
}

/// Helper to check if an error indicates we should skip the test.
fn is_skip_error(e: &anyhow::Error) -> bool {
    let err_str = e.to_string();
    err_str.contains("DATABASE_URL not set")
        || err_str.contains("Connection refused")
        || err_str.contains("connection")
}

/// Test fixture for assembler integration tests.
struct AssemblerTestFixture {
    _temp_dir: TempDir,
    #[allow(dead_code)]
    worktree_path: String,
    chunk_id: i64,
}

impl AssemblerTestFixture {
    /// Set up a test fixture with a sample file and chunk in the database.
    async fn setup() -> Result<Self> {
        // Skip if DATABASE_URL not set
        if should_skip_db_test() {
            return Err(anyhow::anyhow!("DATABASE_URL not set, skipping test setup"));
        }

        // Create temporary directory for test files
        let temp_dir = TempDir::new()?;
        let worktree_path = temp_dir.path().to_string_lossy().to_string();

        // Create a sample Rust file
        let test_file_path = temp_dir.path().join("test.rs");
        let test_content = r#"// Test file for context assembly

/// A simple test function
/// This function demonstrates the assembler
fn test_function() {
    println!("Hello from test function");
    let x = 42;
    let y = x * 2;
    println!("Result: {}", y);
}

fn another_function() {
    println!("Another function");
}
"#;
        fs::write(&test_file_path, test_content).await?;

        // Set up database
        let pool = create_pool().await?;
        let client = pool.get().await?;

        // Create repo, worktree, commit, file, and chunk
        let repo_id = get_or_create_repo(&client, "test-repo", &worktree_path).await?;
        let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
        let commit_id = get_or_create_commit(&client, repo_id, "abc123", None).await?;

        let file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "test.rs",
            Some("rust"),
            "hash123",
            test_content.len() as i32,
            None,
        )
        .await?;

        // Insert a chunk for test_function (lines 3-9)
        let chunk_id = insert_chunk(
            &client,
            file_id,
            Some("test_function"),
            "func",
            Some("fn test_function()"),
            Some("A simple test function\nThis function demonstrates the assembler"),
            3,
            9,
            "fn test_function() { ... }",
            "test function demonstrates assembler",
            1.0,
            0.0,
        )
        .await?;

        Ok(Self {
            _temp_dir: temp_dir,
            worktree_path,
            chunk_id,
        })
    }
}

#[tokio::test]
async fn test_assemble_primary_chunk() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match AssemblerTestFixture::setup().await {
        Ok(f) => f,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: {}", e);
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let assembler = BasicContextAssembler::new(pool);

    // Assemble context for the test function
    let bundle = assembler
        .assemble(
            fixture.chunk_id,
            6000,
            ExpandOptions::primary_only()
        )
        .await?;

    // Verify bundle structure
    assert_eq!(bundle.items.len(), 1, "Should have exactly 1 item (primary chunk)");
    assert!(!bundle.truncated, "Should not be truncated with 6000 token budget");
    assert!(bundle.total_tokens > 0, "Should have counted tokens");

    // Verify primary chunk
    let primary = &bundle.items[0];
    assert_eq!(primary.relpath, "test.rs");
    assert_eq!(primary.range.start, 3);
    assert_eq!(primary.range.end, 9);
    assert_eq!(primary.role, "primary");
    assert!(primary.reason.contains("test_function"));
    assert!(primary.content.contains("fn test_function()"));
    assert!(primary.content.contains("println!"));
    assert!(primary.tokens > 0);

    Ok(())
}

#[tokio::test]
async fn test_assemble_exceeds_budget() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match AssemblerTestFixture::setup().await {
        Ok(f) => f,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: {}", e);
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let assembler = BasicContextAssembler::new(pool);

    // Assemble with very small budget
    let bundle = assembler
        .assemble(
            fixture.chunk_id,
            10, // Very small budget
            ExpandOptions::primary_only()
        )
        .await?;

    // Should be marked as truncated
    assert!(bundle.truncated, "Should be marked as truncated");
    assert_eq!(bundle.items.len(), 1, "Should still include the primary chunk");

    Ok(())
}

#[tokio::test]
async fn test_assemble_missing_chunk() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let assembler = BasicContextAssembler::new(pool);

    // Try to assemble non-existent chunk
    let result = assembler
        .assemble(
            999999999, // Non-existent chunk ID
            6000,
            ExpandOptions::primary_only()
        )
        .await;

    assert!(result.is_err(), "Should fail for non-existent chunk");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"), "Error should mention 'not found'");

    Ok(())
}

#[tokio::test]
async fn test_token_counting_accuracy() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match AssemblerTestFixture::setup().await {
        Ok(f) => f,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: {}", e);
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let assembler = BasicContextAssembler::new(pool);

    let bundle = assembler
        .assemble(
            fixture.chunk_id,
            10000,
            ExpandOptions::primary_only()
        )
        .await?;

    // Verify token count is reasonable
    let primary = &bundle.items[0];
    let line_count = primary.range.line_count();

    // Rough heuristic: Rust code averages ~4-8 tokens per line
    let min_expected_tokens = line_count * 3;
    let max_expected_tokens = line_count * 15;

    assert!(
        primary.tokens >= min_expected_tokens,
        "Token count {} seems too low for {} lines (expected >= {})",
        primary.tokens, line_count, min_expected_tokens
    );

    assert!(
        primary.tokens <= max_expected_tokens,
        "Token count {} seems too high for {} lines (expected <= {})",
        primary.tokens, line_count, max_expected_tokens
    );

    // Total should match sum of items
    assert_eq!(
        bundle.total_tokens,
        bundle.items.iter().map(|item| item.tokens).sum::<usize>(),
        "Total tokens should equal sum of item tokens"
    );

    Ok(())
}

#[tokio::test]
async fn test_file_content_extraction() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match AssemblerTestFixture::setup().await {
        Ok(f) => f,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: {}", e);
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let assembler = BasicContextAssembler::new(pool);

    let bundle = assembler
        .assemble(
            fixture.chunk_id,
            10000,
            ExpandOptions::primary_only()
        )
        .await?;

    let primary = &bundle.items[0];

    // Verify content matches expected lines
    assert!(primary.content.contains("A simple test function"));
    assert!(primary.content.contains("fn test_function()"));
    assert!(primary.content.contains("println!(\"Hello from test function\")"));
    assert!(primary.content.contains("let x = 42"));
    assert!(primary.content.contains("let y = x * 2"));

    // Verify it doesn't contain lines outside the range
    assert!(!primary.content.contains("Test file for context assembly"), "Should not include comment from line 1");
    assert!(!primary.content.contains("fn another_function()"), "Should not include another_function from line 11");

    Ok(())
}
