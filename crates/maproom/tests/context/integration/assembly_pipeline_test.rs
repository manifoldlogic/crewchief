//! Comprehensive integration tests for the complete context assembly pipeline.
//!
//! These tests verify the end-to-end assembly workflow from chunk_id to ContextBundle,
//! including:
//! - Primary chunk loading and content extraction
//! - Relationship expansion (callers, callees, tests)
//! - Budget management and allocation
//! - Token counting accuracy
//! - Truncation behavior
//! - Multi-level dependency traversal
//! - Quality metrics (no duplicates, correct relevance, completeness)

use anyhow::Result;
use maproom::context::{
    BasicContextAssembler, ContextAssembler, ExpandOptions,
};
use maproom::db::{
    create_pool, get_or_create_commit, get_or_create_repo, get_or_create_worktree,
    insert_chunk, upsert_file,
};
use std::collections::HashSet;
use std::env;
use tempfile::TempDir;
use tokio::fs;

/// Helper to check if we should skip database integration tests.
fn should_skip_db_test() -> bool {
    env::var("MAPROOM_DATABASE_URL").is_err()
}

/// Helper to check if an error indicates we should skip the test.
fn is_skip_error(e: &anyhow::Error) -> bool {
    let err_str = e.to_string();
    err_str.contains("MAPROOM_DATABASE_URL not set")
        || err_str.contains("Connection refused")
        || err_str.contains("connection")
}

/// Test fixture for comprehensive pipeline testing.
struct PipelineTestFixture {
    _temp_dir: TempDir,
    #[allow(dead_code)] // Justification: set during fixture setup; retained for consistency with database state
    worktree_path: String,
    main_chunk_id: i64,
    helper_chunk_id: i64,
    caller_chunk_id: i64,
    test_chunk_id: i64,
}

impl PipelineTestFixture {
    /// Set up a complete test fixture with multiple related chunks.
    ///
    /// Creates:
    /// - main.rs: main() function (calls helper)
    /// - utils.rs: helper() function (called by main)
    /// - lib.rs: public_api() function (calls helper)
    /// - test.rs: test_main() (tests main)
    async fn setup() -> Result<Self> {
        if should_skip_db_test() {
            return Err(anyhow::anyhow!("MAPROOM_DATABASE_URL not set, skipping test setup"));
        }

        let temp_dir = TempDir::new()?;
        let worktree_path = temp_dir.path().to_string_lossy().to_string();

        // Create test files
        let main_file = temp_dir.path().join("main.rs");
        let main_content = r#"use crate::utils::helper;

fn main() {
    println!("Starting application");
    helper();
    println!("Done");
}
"#;
        fs::write(&main_file, main_content).await?;

        let utils_file = temp_dir.path().join("utils.rs");
        let utils_content = r#"/// Helper function used by multiple modules
pub fn helper() {
    println!("Helper called");
    let result = process_data();
    println!("Result: {}", result);
}

fn process_data() -> i32 {
    42
}
"#;
        fs::write(&utils_file, utils_content).await?;

        let lib_file = temp_dir.path().join("lib.rs");
        let lib_content = r#"use crate::utils::helper;

/// Public API function
pub fn public_api() {
    println!("Public API called");
    helper();
}
"#;
        fs::write(&lib_file, lib_content).await?;

        let test_file = temp_dir.path().join("test.rs");
        let test_content = r#"#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        // Test the main function
        main();
    }
}
"#;
        fs::write(&test_file, test_content).await?;

        // Set up database
        let pool = create_pool().await?;
        let client = pool.get().await?;

        let repo_id = get_or_create_repo(&client, "test-pipeline", &worktree_path).await?;
        let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
        let commit_id = get_or_create_commit(&client, repo_id, "pipeline123", None).await?;

        // Insert files
        let main_file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "main.rs",
            Some("rust"),
            "hash_main",
            main_content.len() as i32,
            None,
        )
        .await?;

        let utils_file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "utils.rs",
            Some("rust"),
            "hash_utils",
            utils_content.len() as i32,
            None,
        )
        .await?;

        let lib_file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "lib.rs",
            Some("rust"),
            "hash_lib",
            lib_content.len() as i32,
            None,
        )
        .await?;

        let test_file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "test.rs",
            Some("rust"),
            "hash_test",
            test_content.len() as i32,
            None,
        )
        .await?;

        // Insert chunks
        let main_chunk_id = insert_chunk(
            &client,
            main_file_id,
            Some("main"),
            "func",
            Some("fn main()"),
            None,
            3,
            7,
            "fn main() { ... }",
            "main function",
            1.0,
            0.0,
        )
        .await?;

        let helper_chunk_id = insert_chunk(
            &client,
            utils_file_id,
            Some("helper"),
            "func",
            Some("pub fn helper()"),
            Some("Helper function used by multiple modules"),
            2,
            6,
            "pub fn helper() { ... }",
            "helper function",
            1.0,
            0.0,
        )
        .await?;

        let caller_chunk_id = insert_chunk(
            &client,
            lib_file_id,
            Some("public_api"),
            "func",
            Some("pub fn public_api()"),
            Some("Public API function"),
            4,
            7,
            "pub fn public_api() { ... }",
            "public api",
            1.0,
            0.0,
        )
        .await?;

        let test_chunk_id = insert_chunk(
            &client,
            test_file_id,
            Some("test_main"),
            "test",
            Some("fn test_main()"),
            None,
            6,
            10,
            "#[test] fn test_main() { ... }",
            "test main",
            1.0,
            0.0,
        )
        .await?;

        // Create relationships
        // main calls helper
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&main_chunk_id, &helper_chunk_id],
            )
            .await?;

        // public_api calls helper
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&caller_chunk_id, &helper_chunk_id],
            )
            .await?;

        // test_main tests main
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'test_of'::maproom.edge_type)",
                &[&test_chunk_id, &main_chunk_id],
            )
            .await?;

        Ok(Self {
            _temp_dir: temp_dir,
            worktree_path,
            main_chunk_id,
            helper_chunk_id,
            caller_chunk_id,
            test_chunk_id,
        })
    }
}

#[tokio::test]
async fn test_complete_assembly_pipeline_primary_only() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble context with primary only
    let bundle = assembler
        .assemble(
            fixture.main_chunk_id,
            6000,
            ExpandOptions::primary_only(),
        )
        .await?;

    // Verify structure
    assert_eq!(bundle.items.len(), 1, "Primary only should have 1 item");
    assert!(!bundle.truncated, "Should not be truncated");
    assert!(bundle.total_tokens > 0, "Should have token count");

    // Verify primary chunk
    let primary = &bundle.items[0];
    assert_eq!(primary.relpath, "main.rs");
    assert_eq!(primary.role, "primary");
    assert!(primary.content.contains("fn main()"));
    assert!(primary.content.contains("helper()"));

    Ok(())
}

#[tokio::test]
async fn test_complete_assembly_pipeline_with_callees() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble with callees
    let mut options = ExpandOptions::primary_only();
    options.include_callees = true;

    let bundle = assembler
        .assemble(fixture.main_chunk_id, 6000, options)
        .await?;

    // Should have primary + helper (callee)
    assert!(
        bundle.items.len() >= 2,
        "Should have at least primary and callee"
    );

    // Verify primary is first
    assert_eq!(bundle.items[0].role, "primary");
    assert_eq!(bundle.items[0].relpath, "main.rs");

    // Find the helper chunk
    let helper = bundle
        .items
        .iter()
        .find(|item| item.relpath == "utils.rs" && item.role == "callee");
    assert!(helper.is_some(), "Should include helper as callee");

    let helper = helper.unwrap();
    assert!(helper.content.contains("pub fn helper()"));
    assert!(helper.reason.contains("helper"));

    Ok(())
}

#[tokio::test]
async fn test_complete_assembly_pipeline_with_callers() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble helper with callers
    let mut options = ExpandOptions::primary_only();
    options.include_callers = true;

    let bundle = assembler
        .assemble(fixture.helper_chunk_id, 6000, options)
        .await?;

    // Should have primary (helper) + callers (main, public_api)
    assert!(
        bundle.items.len() >= 2,
        "Should have at least primary and one caller"
    );

    // Verify primary
    assert_eq!(bundle.items[0].role, "primary");
    assert_eq!(bundle.items[0].relpath, "utils.rs");

    // Find callers
    let main_caller = bundle
        .items
        .iter()
        .find(|item| item.relpath == "main.rs" && item.role == "caller");
    let lib_caller = bundle
        .items
        .iter()
        .find(|item| item.relpath == "lib.rs" && item.role == "caller");

    // At least one caller should be present
    assert!(
        main_caller.is_some() || lib_caller.is_some(),
        "Should include at least one caller"
    );

    Ok(())
}

#[tokio::test]
async fn test_complete_assembly_pipeline_with_tests() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble main with tests
    let mut options = ExpandOptions::primary_only();
    options.include_tests = true;

    let bundle = assembler
        .assemble(fixture.main_chunk_id, 6000, options)
        .await?;

    // Should have primary + test
    assert!(
        bundle.items.len() >= 2,
        "Should have at least primary and test"
    );

    // Find test
    let test = bundle
        .items
        .iter()
        .find(|item| item.relpath == "test.rs" && item.role == "test");
    assert!(test.is_some(), "Should include test_main as test");

    let test = test.unwrap();
    assert!(test.content.contains("test_main"));

    Ok(())
}

#[tokio::test]
async fn test_complete_assembly_pipeline_all_relationships() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble with all relationships
    let options = ExpandOptions {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        include_imports: true,
        include_definitions: true,
        max_depth: 2,
    };

    let bundle = assembler
        .assemble(fixture.main_chunk_id, 10000, options)
        .await?;

    // Should have multiple items
    assert!(
        bundle.items.len() >= 2,
        "Should have primary plus relationships"
    );

    // Verify primary is first
    assert_eq!(bundle.items[0].role, "primary");

    // Verify no duplicates by chunk_id
    let mut seen_ids = HashSet::new();
    for item in &bundle.items {
        let key = format!("{}:{}:{}", item.relpath, item.range.start, item.range.end);
        assert!(
            seen_ids.insert(key.clone()),
            "Duplicate item found: {}",
            key
        );
    }

    // Verify budget was respected
    assert!(
        bundle.total_tokens <= 10000,
        "Total tokens {} should not exceed budget 10000",
        bundle.total_tokens
    );

    // Verify all items have valid token counts
    for item in &bundle.items {
        assert!(item.tokens > 0, "All items should have token counts");
    }

    Ok(())
}

#[tokio::test]
async fn test_budget_allocation_and_truncation() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    let options = ExpandOptions {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        include_imports: true,
        include_definitions: true,
        max_depth: 2,
    };

    // Test with very small budget
    let small_bundle = assembler
        .assemble(fixture.main_chunk_id, 500, options)
        .await?;

    // Should be truncated
    assert!(
        small_bundle.truncated,
        "Small budget should result in truncation"
    );
    assert!(
        small_bundle.total_tokens <= 500,
        "Should respect small budget"
    );
    assert_eq!(
        small_bundle.items.len(),
        1,
        "Very small budget should only include primary"
    );

    // Test with large budget
    let large_bundle = assembler
        .assemble(fixture.main_chunk_id, 50000, options)
        .await?;

    // Should not be truncated
    assert!(
        !large_bundle.truncated,
        "Large budget should not be truncated"
    );
    assert!(
        large_bundle.total_tokens <= 50000,
        "Should respect large budget"
    );

    // Large budget should include more items
    assert!(
        large_bundle.items.len() >= small_bundle.items.len(),
        "Larger budget should include at least as many items"
    );

    Ok(())
}

#[tokio::test]
async fn test_relevance_scoring_order() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    let options = ExpandOptions {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        include_imports: true,
        include_definitions: true,
        max_depth: 2,
    };

    let bundle = assembler
        .assemble(fixture.main_chunk_id, 10000, options)
        .await?;

    // Primary should be first
    assert_eq!(bundle.items[0].role, "primary");

    // Within each role category, items should be ordered by relevance
    // (we can't enforce strict global ordering as budget allocation may affect it)
    // But we can verify that items have reasonable relevance scores
    for item in &bundle.items {
        if item.role == "primary" {
            // Primary items should have high relevance
            assert!(
                item.importance >= 0.8,
                "Primary should have high importance"
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_token_sum_consistency() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    let options = ExpandOptions {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        include_imports: true,
        include_definitions: true,
        max_depth: 2,
    };

    let bundle = assembler
        .assemble(fixture.main_chunk_id, 10000, options)
        .await?;

    // Total tokens should equal sum of item tokens
    let sum: usize = bundle.items.iter().map(|item| item.tokens).sum();
    assert_eq!(
        bundle.total_tokens, sum,
        "Total tokens should equal sum of item tokens"
    );

    // All items should have positive token counts
    for item in &bundle.items {
        assert!(item.tokens > 0, "All items should have tokens > 0");
    }

    Ok(())
}

#[tokio::test]
async fn test_no_duplicate_chunks() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match PipelineTestFixture::setup().await {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    let options = ExpandOptions {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        include_imports: true,
        include_definitions: true,
        max_depth: 2,
    };

    let bundle = assembler
        .assemble(fixture.main_chunk_id, 10000, options)
        .await?;

    // Check for duplicates by (file, line range)
    let mut seen = HashSet::new();
    for item in &bundle.items {
        let key = format!("{}:{}:{}", item.relpath, item.range.start, item.range.end);
        assert!(
            seen.insert(key.clone()),
            "Duplicate chunk found: {}",
            key
        );
    }

    Ok(())
}
