//! Quality validation tests for context assembly output.
//!
//! These tests verify that assembled context meets quality standards:
//! - Relevance: All included chunks are actually related
//! - Completeness: No missing critical dependencies
//! - Correctness: Relationships are accurately identified
//! - Efficiency: Budget is well-utilized
//! - Consistency: Results are deterministic and repeatable
//! - No duplicates: Each chunk appears only once
//! - Proper ordering: Most relevant items prioritized

use anyhow::Result;
use crewchief_maproom::context::{
    BasicContextAssembler, ContextAssembler, ExpandOptions,
};
use crewchief_maproom::db::{
    create_pool, get_or_create_commit, get_or_create_repo, get_or_create_worktree,
    insert_chunk, upsert_file,
};
use std::collections::{HashMap, HashSet};
use std::env;
use tempfile::TempDir;
use tokio::fs;

fn should_skip_db_test() -> bool {
    env::var("MAPROOM_DATABASE_URL").is_err()
}

fn is_skip_error(e: &anyhow::Error) -> bool {
    let err_str = e.to_string();
    err_str.contains("MAPROOM_DATABASE_URL not set")
        || err_str.contains("Connection refused")
        || err_str.contains("connection")
}

/// Test fixture with known, verifiable relationships.
struct QualityTestFixture {
    _temp_dir: TempDir,
    #[allow(dead_code)] // Justification: set during fixture setup; retained for consistency with database state
    worktree_path: String,
    primary_chunk_id: i64,
    direct_callee_id: i64,
    indirect_callee_id: i64,
    caller_id: i64,
    test_id: i64,
    unrelated_id: i64,
}

impl QualityTestFixture {
    /// Set up fixture with clearly defined relationships for validation.
    async fn setup() -> Result<Self> {
        if should_skip_db_test() {
            return Err(anyhow::anyhow!("MAPROOM_DATABASE_URL not set"));
        }

        let temp_dir = TempDir::new()?;
        let worktree_path = temp_dir.path().to_string_lossy().to_string();

        // Create files with known relationships
        let primary_file = temp_dir.path().join("primary.rs");
        let primary_content = r#"pub fn primary() {
    direct_callee();
}
"#;
        fs::write(&primary_file, primary_content).await?;

        let direct_file = temp_dir.path().join("direct.rs");
        let direct_content = r#"pub fn direct_callee() {
    indirect_callee();
}
"#;
        fs::write(&direct_file, direct_content).await?;

        let indirect_file = temp_dir.path().join("indirect.rs");
        let indirect_content = r#"pub fn indirect_callee() {
    println!("indirect");
}
"#;
        fs::write(&indirect_file, indirect_content).await?;

        let caller_file = temp_dir.path().join("caller.rs");
        let caller_content = r#"pub fn caller() {
    primary();
}
"#;
        fs::write(&caller_file, caller_content).await?;

        let test_file = temp_dir.path().join("test.rs");
        let test_content = r#"#[test]
fn test_primary() {
    primary();
}
"#;
        fs::write(&test_file, test_content).await?;

        let unrelated_file = temp_dir.path().join("unrelated.rs");
        let unrelated_content = r#"pub fn unrelated() {
    println!("unrelated");
}
"#;
        fs::write(&unrelated_file, unrelated_content).await?;

        let pool = create_pool().await?;
        let client = pool.get().await?;

        let repo_id = get_or_create_repo(&client, "test-quality", &worktree_path).await?;
        let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
        let commit_id = get_or_create_commit(&client, repo_id, "quality123", None).await?;

        // Insert files
        let files = vec![
            ("primary.rs", primary_content.len()),
            ("direct.rs", direct_content.len()),
            ("indirect.rs", indirect_content.len()),
            ("caller.rs", caller_content.len()),
            ("test.rs", test_content.len()),
            ("unrelated.rs", unrelated_content.len()),
        ];

        let mut file_ids = HashMap::new();
        for (name, size) in files {
            let file_id = upsert_file(
                &client,
                repo_id,
                worktree_id,
                commit_id,
                name,
                Some("rust"),
                &format!("hash_{}", name),
                size as i32,
                None,
            )
            .await?;
            file_ids.insert(name, file_id);
        }

        // Insert chunks
        let primary_chunk_id = insert_chunk(
            &client,
            file_ids["primary.rs"],
            Some("primary"),
            "func",
            Some("pub fn primary()"),
            None,
            1,
            3,
            "pub fn primary() { direct_callee(); }",
            "primary",
            1.0,
            0.0,
        )
        .await?;

        let direct_callee_id = insert_chunk(
            &client,
            file_ids["direct.rs"],
            Some("direct_callee"),
            "func",
            Some("pub fn direct_callee()"),
            None,
            1,
            3,
            "pub fn direct_callee() { indirect_callee(); }",
            "direct callee",
            1.0,
            0.0,
        )
        .await?;

        let indirect_callee_id = insert_chunk(
            &client,
            file_ids["indirect.rs"],
            Some("indirect_callee"),
            "func",
            Some("pub fn indirect_callee()"),
            None,
            1,
            3,
            "pub fn indirect_callee() { println!(\"indirect\"); }",
            "indirect callee",
            1.0,
            0.0,
        )
        .await?;

        let caller_id = insert_chunk(
            &client,
            file_ids["caller.rs"],
            Some("caller"),
            "func",
            Some("pub fn caller()"),
            None,
            1,
            3,
            "pub fn caller() { primary(); }",
            "caller",
            1.0,
            0.0,
        )
        .await?;

        let test_id = insert_chunk(
            &client,
            file_ids["test.rs"],
            Some("test_primary"),
            "test",
            Some("fn test_primary()"),
            None,
            1,
            4,
            "#[test] fn test_primary() { primary(); }",
            "test primary",
            1.0,
            0.0,
        )
        .await?;

        let unrelated_id = insert_chunk(
            &client,
            file_ids["unrelated.rs"],
            Some("unrelated"),
            "func",
            Some("pub fn unrelated()"),
            None,
            1,
            3,
            "pub fn unrelated() { println!(\"unrelated\"); }",
            "unrelated",
            1.0,
            0.0,
        )
        .await?;

        // Create relationships
        // primary calls direct_callee
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&primary_chunk_id, &direct_callee_id],
            )
            .await?;

        // direct_callee calls indirect_callee
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&direct_callee_id, &indirect_callee_id],
            )
            .await?;

        // caller calls primary
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&caller_id, &primary_chunk_id],
            )
            .await?;

        // test tests primary
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'test_of'::maproom.edge_type)",
                &[&test_id, &primary_chunk_id],
            )
            .await?;

        Ok(Self {
            _temp_dir: temp_dir,
            worktree_path,
            primary_chunk_id,
            direct_callee_id,
            indirect_callee_id,
            caller_id,
            test_id,
            unrelated_id,
        })
    }
}

#[tokio::test]
async fn test_quality_no_unrelated_chunks() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Verify no unrelated chunks are included
    let has_unrelated = bundle
        .items
        .iter()
        .any(|item| item.relpath == "unrelated.rs");

    assert!(
        !has_unrelated,
        "Unrelated chunk should not be included in context"
    );

    Ok(())
}

#[tokio::test]
async fn test_quality_all_direct_dependencies_included() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Verify all direct dependencies are included
    let has_primary = bundle
        .items
        .iter()
        .any(|item| item.relpath == "primary.rs");
    let has_direct_callee = bundle
        .items
        .iter()
        .any(|item| item.relpath == "direct.rs");
    let has_caller = bundle
        .items
        .iter()
        .any(|item| item.relpath == "caller.rs");
    let has_test = bundle.items.iter().any(|item| item.relpath == "test.rs");

    assert!(has_primary, "Primary chunk should be included");
    assert!(has_direct_callee, "Direct callee should be included");
    assert!(has_caller, "Caller should be included");
    assert!(has_test, "Test should be included");

    Ok(())
}

#[tokio::test]
async fn test_quality_relationship_types_correct() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Verify relationship types are correct
    for item in &bundle.items {
        match item.relpath.as_str() {
            "primary.rs" => {
                assert_eq!(item.role, "primary", "Primary should have 'primary' role");
            }
            "direct.rs" => {
                assert_eq!(
                    item.role, "callee",
                    "Direct callee should have 'callee' role"
                );
            }
            "caller.rs" => {
                assert_eq!(item.role, "caller", "Caller should have 'caller' role");
            }
            "test.rs" => {
                assert_eq!(item.role, "test", "Test should have 'test' role");
            }
            _ => {}
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_quality_budget_efficiency() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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

    let budget = 10000;
    let bundle = assembler
        .assemble(fixture.primary_chunk_id, budget, options)
        .await?;

    // Budget efficiency: utilized tokens should be reasonable (not wasting budget)
    let utilization = bundle.total_tokens as f64 / budget as f64;

    if !bundle.truncated {
        // If not truncated, we used what we needed (efficiency may vary)
        assert!(
            utilization <= 1.0,
            "Utilization should not exceed 100%: {}",
            utilization
        );
    } else {
        // If truncated, we should be near the budget
        assert!(
            utilization > 0.8,
            "Truncated bundle should use most of budget: {}",
            utilization
        );
        assert!(
            utilization <= 1.0,
            "Should not exceed budget: {}",
            utilization
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_quality_deterministic_results() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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

    let options = ExpandOptions {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        include_imports: true,
        include_definitions: true,
        max_depth: 2,
    };

    // Assemble same chunk multiple times
    let assembler1 = BasicContextAssembler::new_without_cache(pool.clone());
    let assembler2 = BasicContextAssembler::new_without_cache(pool);

    let bundle1 = assembler1
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    let bundle2 = assembler2
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Results should be identical
    assert_eq!(
        bundle1.items.len(),
        bundle2.items.len(),
        "Item count should be deterministic"
    );
    assert_eq!(
        bundle1.total_tokens, bundle2.total_tokens,
        "Token count should be deterministic"
    );
    assert_eq!(
        bundle1.truncated, bundle2.truncated,
        "Truncation should be deterministic"
    );

    // Items should be in same order
    for (item1, item2) in bundle1.items.iter().zip(bundle2.items.iter()) {
        assert_eq!(item1.relpath, item2.relpath, "Items should be in same order");
        assert_eq!(item1.role, item2.role, "Roles should match");
        assert_eq!(
            item1.range, item2.range,
            "Ranges should match for same item"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_quality_no_duplicate_chunks() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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
        max_depth: 3, // Deep traversal to stress duplicate detection
    };

    let bundle = assembler
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Check for duplicates
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

#[tokio::test]
async fn test_quality_importance_ordering() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Primary should be first
    assert_eq!(
        bundle.items[0].role,
        "primary",
        "Primary chunk should be first"
    );

    // Primary should have highest importance
    let primary_importance = bundle.items[0].importance;
    assert!(
        primary_importance >= 0.8,
        "Primary should have high importance: {}",
        primary_importance
    );

    // Verify all items have valid importance scores
    for item in &bundle.items {
        assert!(
            item.importance >= 0.0 && item.importance <= 1.0,
            "Importance should be in [0, 1]: {} for {}",
            item.importance,
            item.relpath
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_quality_content_extraction_accuracy() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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

    let options = ExpandOptions::primary_only();

    let bundle = assembler
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Verify content matches expected
    let primary = &bundle.items[0];
    assert!(
        primary.content.contains("pub fn primary()"),
        "Content should contain function signature"
    );
    assert!(
        primary.content.contains("direct_callee()"),
        "Content should contain function call"
    );

    // Content should not contain lines outside the range
    assert!(
        !primary.content.contains("unrelated"),
        "Content should not contain unrelated code"
    );

    Ok(())
}

#[tokio::test]
async fn test_quality_reason_explanations() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match QualityTestFixture::setup().await {
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
        .assemble(fixture.primary_chunk_id, 10000, options)
        .await?;

    // Verify all items have explanatory reasons
    for item in &bundle.items {
        assert!(
            !item.reason.is_empty(),
            "All items should have a reason: {}",
            item.relpath
        );

        // Reason should mention the symbol or relationship
        if item.role != "primary" {
            assert!(
                item.reason.len() > 5,
                "Reason should be descriptive: {} - {}",
                item.relpath,
                item.reason
            );
        }
    }

    Ok(())
}
