//! Integration tests for edge cases and error handling in context assembly.
//!
//! Tests include:
//! - Missing chunk IDs
//! - File read errors
//! - Database connection failures
//! - Empty files and chunks
//! - Malformed data
//! - Circular dependencies
//! - Very large files
//! - Zero budget scenarios
//! - Invalid line ranges

use anyhow::Result;
use crewchief_maproom::context::{
    BasicContextAssembler, ContextAssembler, ExpandOptions,
};
use crewchief_maproom::db::{
    create_pool, get_or_create_commit, get_or_create_repo, get_or_create_worktree,
    insert_chunk, upsert_file,
};
use std::env;
use tempfile::TempDir;
use tokio::fs;

fn should_skip_db_test() -> bool {
    env::var("DATABASE_URL").is_err()
}

fn is_skip_error(e: &anyhow::Error) -> bool {
    let err_str = e.to_string();
    err_str.contains("DATABASE_URL not set")
        || err_str.contains("Connection refused")
        || err_str.contains("connection")
}

#[tokio::test]
async fn test_missing_chunk_id() -> Result<()> {
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

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Try to assemble non-existent chunk
    let result = assembler
        .assemble(999999999, 6000, ExpandOptions::primary_only())
        .await;

    assert!(result.is_err(), "Should fail for non-existent chunk");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("not found") || err_msg.contains("does not exist"),
        "Error should mention missing chunk: {}",
        err_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_empty_file_handling() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    // Create empty file
    let empty_file = temp_dir.path().join("empty.rs");
    fs::write(&empty_file, "").await?;

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-empty", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "empty123", None).await?;

    let file_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "empty.rs",
        Some("rust"),
        "hash_empty",
        0,
        None,
    )
    .await?;

    // Try to insert chunk with valid line numbers (even though file is empty)
    let chunk_id = insert_chunk(
        &client,
        file_id,
        Some("empty_func"),
        "func",
        Some("fn empty_func()"),
        None,
        1,
        1,
        "empty",
        "empty function",
        1.0,
        0.0,
    )
    .await?;

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble the empty chunk
    let result = assembler
        .assemble(chunk_id, 6000, ExpandOptions::primary_only())
        .await;

    // This may succeed with empty content or fail depending on implementation
    // Either behavior is acceptable, but should not panic
    match result {
        Ok(bundle) => {
            assert_eq!(bundle.items.len(), 1);
            // Content may be empty or have minimal content
            assert!(bundle.items[0].tokens >= 0);
        }
        Err(e) => {
            // Also acceptable if it fails gracefully
            eprintln!("Empty file handling failed gracefully: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_missing_file_on_disk() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    // Create file in database but not on disk
    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-missing-file", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "missing123", None).await?;

    let file_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "nonexistent.rs",
        Some("rust"),
        "hash_nonexistent",
        100,
        None,
    )
    .await?;

    let chunk_id = insert_chunk(
        &client,
        file_id,
        Some("missing_func"),
        "func",
        Some("fn missing_func()"),
        None,
        1,
        5,
        "fn missing_func() { }",
        "missing function",
        1.0,
        0.0,
    )
    .await?;

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Try to assemble - should fail with file not found
    let result = assembler
        .assemble(chunk_id, 6000, ExpandOptions::primary_only())
        .await;

    assert!(result.is_err(), "Should fail when file is missing on disk");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("No such file") || err_msg.contains("not found") || err_msg.contains("failed"),
        "Error should indicate file issue: {}",
        err_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_line_range() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    // Create file with 5 lines
    let test_file = temp_dir.path().join("short.rs");
    let content = "line1\nline2\nline3\nline4\nline5\n";
    fs::write(&test_file, content).await?;

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-invalid-range", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "range123", None).await?;

    let file_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "short.rs",
        Some("rust"),
        "hash_short",
        content.len() as i32,
        None,
    )
    .await?;

    // Create chunk with line range beyond file length
    let chunk_id = insert_chunk(
        &client,
        file_id,
        Some("out_of_range"),
        "func",
        Some("fn out_of_range()"),
        None,
        10, // Beyond file length
        20,
        "out of range",
        "test",
        1.0,
        0.0,
    )
    .await?;

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble - should handle gracefully
    let result = assembler
        .assemble(chunk_id, 6000, ExpandOptions::primary_only())
        .await;

    // Should either fail gracefully or return partial content
    match result {
        Ok(bundle) => {
            // If it succeeds, content should be valid (may be empty or partial)
            assert_eq!(bundle.items.len(), 1);
            eprintln!("Invalid range handled gracefully with partial content");
        }
        Err(e) => {
            // Also acceptable if it fails gracefully
            eprintln!("Invalid range failed gracefully: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_circular_dependencies() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    // Create files with circular calls: a calls b, b calls a
    let file_a = temp_dir.path().join("a.rs");
    let content_a = "pub fn func_a() { func_b(); }";
    fs::write(&file_a, content_a).await?;

    let file_b = temp_dir.path().join("b.rs");
    let content_b = "pub fn func_b() { func_a(); }";
    fs::write(&file_b, content_b).await?;

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-circular", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "circular123", None).await?;

    let file_a_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "a.rs",
        Some("rust"),
        "hash_a",
        content_a.len() as i32,
        None,
    )
    .await?;

    let file_b_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "b.rs",
        Some("rust"),
        "hash_b",
        content_b.len() as i32,
        None,
    )
    .await?;

    let chunk_a = insert_chunk(
        &client,
        file_a_id,
        Some("func_a"),
        "func",
        Some("pub fn func_a()"),
        None,
        1,
        1,
        "pub fn func_a() { func_b(); }",
        "func a",
        1.0,
        0.0,
    )
    .await?;

    let chunk_b = insert_chunk(
        &client,
        file_b_id,
        Some("func_b"),
        "func",
        Some("pub fn func_b()"),
        None,
        1,
        1,
        "pub fn func_b() { func_a(); }",
        "func b",
        1.0,
        0.0,
    )
    .await?;

    // Create circular edges
    client
        .execute(
            "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
            &[&chunk_a, &chunk_b],
        )
        .await?;

    client
        .execute(
            "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
            &[&chunk_b, &chunk_a],
        )
        .await?;

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble with deep traversal - should handle circular deps
    let options = ExpandOptions {
        include_callers: true,
        include_callees: true,
        include_tests: false,
        include_imports: true,
        include_definitions: true,
        max_depth: 5, // Deep enough to expose circular issues
    };

    let result = assembler.assemble(chunk_a, 6000, options).await;

    // Should not hang or panic
    assert!(
        result.is_ok(),
        "Should handle circular dependencies gracefully"
    );

    if let Ok(bundle) = result {
        // Should include both chunks (but not infinitely)
        assert!(bundle.items.len() >= 1);
        assert!(bundle.items.len() <= 10, "Should not expand infinitely");

        // Check for duplicates - there should be none
        let mut seen = std::collections::HashSet::new();
        for item in &bundle.items {
            let key = format!("{}:{}:{}", item.relpath, item.range.start, item.range.end);
            assert!(seen.insert(key.clone()), "Duplicate chunk: {}", key);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_zero_budget() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn test() {}").await?;

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-zero-budget", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "zero123", None).await?;

    let file_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "test.rs",
        Some("rust"),
        "hash_test",
        13,
        None,
    )
    .await?;

    let chunk_id = insert_chunk(
        &client,
        file_id,
        Some("test"),
        "func",
        Some("fn test()"),
        None,
        1,
        1,
        "fn test() {}",
        "test",
        1.0,
        0.0,
    )
    .await?;

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Try with zero budget
    let result = assembler
        .assemble(chunk_id, 0, ExpandOptions::primary_only())
        .await;

    // Should handle gracefully - may fail or return minimal bundle
    match result {
        Ok(bundle) => {
            assert!(bundle.truncated, "Zero budget should be truncated");
            assert_eq!(bundle.total_tokens, 0, "Zero budget should have 0 tokens");
            eprintln!("Zero budget handled by returning empty bundle");
        }
        Err(e) => {
            eprintln!("Zero budget failed gracefully: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_very_large_file() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    // Create large file (1000 lines)
    let test_file = temp_dir.path().join("large.rs");
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("// Line {}\n", i));
    }
    fs::write(&test_file, &content).await?;

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-large", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "large123", None).await?;

    let file_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "large.rs",
        Some("rust"),
        "hash_large",
        content.len() as i32,
        None,
    )
    .await?;

    // Create chunk spanning many lines
    let chunk_id = insert_chunk(
        &client,
        file_id,
        Some("large_func"),
        "func",
        Some("fn large_func()"),
        None,
        100,
        500, // 400 lines
        "large function",
        "large",
        1.0,
        0.0,
    )
    .await?;

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Assemble - should handle large chunk
    let result = assembler
        .assemble(chunk_id, 6000, ExpandOptions::primary_only())
        .await;

    assert!(result.is_ok(), "Should handle large files");

    if let Ok(bundle) = result {
        assert_eq!(bundle.items.len(), 1);
        let item = &bundle.items[0];

        // Verify line count
        assert_eq!(item.range.line_count(), 401); // 100 to 500 inclusive

        // Should have reasonable token count (large but not infinite)
        assert!(item.tokens > 100);
        assert!(item.tokens < 10000);
    }

    Ok(())
}

#[tokio::test]
async fn test_malformed_chunk_data() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn test() {}").await?;

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-malformed", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "malformed123", None).await?;

    let file_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "test.rs",
        Some("rust"),
        "hash_test",
        13,
        None,
    )
    .await?;

    // Create chunk with inverted line range (end < start)
    let chunk_id = insert_chunk(
        &client,
        file_id,
        Some("malformed"),
        "func",
        Some("fn malformed()"),
        None,
        10, // start
        5,  // end < start (malformed)
        "malformed",
        "test",
        1.0,
        0.0,
    )
    .await?;

    let assembler = BasicContextAssembler::new_without_cache(pool);

    // Try to assemble - should handle gracefully
    let result = assembler
        .assemble(chunk_id, 6000, ExpandOptions::primary_only())
        .await;

    // Should either fail gracefully or handle the malformed data
    match result {
        Ok(bundle) => {
            eprintln!("Malformed data handled: {:?}", bundle);
            // If it succeeds, verify it didn't produce invalid results
            assert_eq!(bundle.items.len(), 1);
        }
        Err(e) => {
            eprintln!("Malformed data failed gracefully: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_assembly_same_chunk() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: DATABASE_URL not set");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let worktree_path = temp_dir.path().to_string_lossy().to_string();

    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn test() { println!(\"test\"); }").await?;

    let pool = match create_pool().await {
        Ok(p) => p,
        Err(e) if is_skip_error(&e) => {
            eprintln!("Skipping integration test: Cannot connect to database");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let client = pool.get().await?;

    let repo_id = get_or_create_repo(&client, "test-concurrent", &worktree_path).await?;
    let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
    let commit_id = get_or_create_commit(&client, repo_id, "concurrent123", None).await?;

    let file_id = upsert_file(
        &client,
        repo_id,
        worktree_id,
        commit_id,
        "test.rs",
        Some("rust"),
        "hash_test",
        32,
        None,
    )
    .await?;

    let chunk_id = insert_chunk(
        &client,
        file_id,
        Some("test"),
        "func",
        Some("fn test()"),
        None,
        1,
        1,
        "fn test() { println!(\"test\"); }",
        "test",
        1.0,
        0.0,
    )
    .await?;

    // Create multiple assemblers (simulating concurrent requests)
    let assembler1 = BasicContextAssembler::new_without_cache(pool.clone());
    let assembler2 = BasicContextAssembler::new_without_cache(pool.clone());
    let assembler3 = BasicContextAssembler::new_without_cache(pool);

    // Assemble concurrently
    let (result1, result2, result3) = tokio::join!(
        assembler1.assemble(chunk_id, 6000, ExpandOptions::primary_only()),
        assembler2.assemble(chunk_id, 6000, ExpandOptions::primary_only()),
        assembler3.assemble(chunk_id, 6000, ExpandOptions::primary_only()),
    );

    // All should succeed
    assert!(result1.is_ok(), "Concurrent assembly 1 should succeed");
    assert!(result2.is_ok(), "Concurrent assembly 2 should succeed");
    assert!(result3.is_ok(), "Concurrent assembly 3 should succeed");

    // All should produce identical results
    let bundle1 = result1.unwrap();
    let bundle2 = result2.unwrap();
    let bundle3 = result3.unwrap();

    assert_eq!(bundle1.items.len(), bundle2.items.len());
    assert_eq!(bundle1.items.len(), bundle3.items.len());
    assert_eq!(bundle1.total_tokens, bundle2.total_tokens);
    assert_eq!(bundle1.total_tokens, bundle3.total_tokens);

    Ok(())
}
