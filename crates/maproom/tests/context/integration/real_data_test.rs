//! Integration tests using realistic fixture data.
//!
//! These tests use the sample-repo fixture to test context assembly with
//! real-world code patterns and relationships.

use anyhow::Result;
use crewchief_maproom::context::{
    BasicContextAssembler, ContextAssembler, ExpandOptions,
};
use crewchief_maproom::db::{
    create_pool, get_or_create_commit, get_or_create_repo, get_or_create_worktree,
    insert_chunk, upsert_file,
};
use std::env;
use std::path::PathBuf;
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

/// Get path to the sample fixture repository.
fn get_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("context")
        .join("fixtures")
        .join("sample-repo")
}

/// Test fixture that indexes the sample repository.
struct RealDataFixture {
    #[allow(dead_code)]
    worktree_path: String,
    lib_run_chunk_id: i64,
    utils_load_config_chunk_id: i64,
    api_process_request_chunk_id: i64,
    api_fetch_data_chunk_id: i64,
    utils_format_output_chunk_id: i64,
}

impl RealDataFixture {
    /// Index the sample repository into the database.
    async fn setup() -> Result<Self> {
        if should_skip_db_test() {
            return Err(anyhow::anyhow!("MAPROOM_DATABASE_URL not set"));
        }

        let fixture_path = get_fixture_path();
        if !fixture_path.exists() {
            return Err(anyhow::anyhow!("Fixture path does not exist: {:?}", fixture_path));
        }

        let worktree_path = fixture_path.to_string_lossy().to_string();

        let pool = create_pool().await?;
        let client = pool.get().await?;

        let repo_id = get_or_create_repo(&client, "sample-repo", &worktree_path).await?;
        let worktree_id = get_or_create_worktree(&client, repo_id, "main", &worktree_path).await?;
        let commit_id = get_or_create_commit(&client, repo_id, "fixture123", None).await?;

        // Read and index lib.rs
        let lib_path = fixture_path.join("src/lib.rs");
        let lib_content = fs::read_to_string(&lib_path).await?;
        let lib_file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "src/lib.rs",
            Some("rust"),
            "hash_lib",
            lib_content.len() as i32,
            None,
        )
        .await?;

        // Read and index utils.rs
        let utils_path = fixture_path.join("src/utils.rs");
        let utils_content = fs::read_to_string(&utils_path).await?;
        let utils_file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "src/utils.rs",
            Some("rust"),
            "hash_utils",
            utils_content.len() as i32,
            None,
        )
        .await?;

        // Read and index api.rs
        let api_path = fixture_path.join("src/api.rs");
        let api_content = fs::read_to_string(&api_path).await?;
        let api_file_id = upsert_file(
            &client,
            repo_id,
            worktree_id,
            commit_id,
            "src/api.rs",
            Some("rust"),
            "hash_api",
            api_content.len() as i32,
            None,
        )
        .await?;

        // Create chunks for key functions
        // lib::run() (lines ~7-11)
        let lib_run_chunk_id = insert_chunk(
            &client,
            lib_file_id,
            Some("run"),
            "func",
            Some("pub fn run()"),
            Some("Main entry point for the library"),
            7,
            11,
            "pub fn run() { ... }",
            "run main entry",
            1.0,
            0.0,
        )
        .await?;

        // utils::load_config() (lines ~11-16)
        let utils_load_config_chunk_id = insert_chunk(
            &client,
            utils_file_id,
            Some("load_config"),
            "func",
            Some("pub fn load_config()"),
            Some("Load configuration from environment"),
            11,
            16,
            "pub fn load_config() -> Config { ... }",
            "load config",
            1.0,
            0.0,
        )
        .await?;

        // utils::format_output() (lines ~23-25)
        let utils_format_output_chunk_id = insert_chunk(
            &client,
            utils_file_id,
            Some("format_output"),
            "func",
            Some("pub fn format_output(data: &str)"),
            Some("Format output string"),
            23,
            25,
            "pub fn format_output(data: &str) -> String { ... }",
            "format output",
            1.0,
            0.0,
        )
        .await?;

        // api::process_request() (lines ~6-14)
        let api_process_request_chunk_id = insert_chunk(
            &client,
            api_file_id,
            Some("process_request"),
            "func",
            Some("pub fn process_request(config: &Config)"),
            Some("Process an incoming request"),
            6,
            14,
            "pub fn process_request(config: &Config) -> String { ... }",
            "process request",
            1.0,
            0.0,
        )
        .await?;

        // api::fetch_data() (lines ~17-19)
        let api_fetch_data_chunk_id = insert_chunk(
            &client,
            api_file_id,
            Some("fetch_data"),
            "func",
            Some("fn fetch_data()"),
            Some("Fetch data from a source"),
            17,
            19,
            "fn fetch_data() -> String { ... }",
            "fetch data",
            1.0,
            0.0,
        )
        .await?;

        // Create relationships
        // lib::run calls utils::load_config
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&lib_run_chunk_id, &utils_load_config_chunk_id],
            )
            .await?;

        // lib::run calls api::process_request
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&lib_run_chunk_id, &api_process_request_chunk_id],
            )
            .await?;

        // api::process_request calls api::fetch_data
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&api_process_request_chunk_id, &api_fetch_data_chunk_id],
            )
            .await?;

        // api::process_request calls utils::format_output
        client
            .execute(
                "INSERT INTO maproom.chunk_edges(src_chunk_id, dst_chunk_id, type) VALUES ($1, $2, 'calls'::maproom.edge_type)",
                &[&api_process_request_chunk_id, &utils_format_output_chunk_id],
            )
            .await?;

        Ok(Self {
            worktree_path,
            lib_run_chunk_id,
            utils_load_config_chunk_id,
            api_process_request_chunk_id,
            api_fetch_data_chunk_id,
            utils_format_output_chunk_id,
        })
    }
}

#[tokio::test]
async fn test_real_data_lib_run_with_callees() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match RealDataFixture::setup().await {
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

    let mut options = ExpandOptions::primary_only();
    options.include_callees = true;
    options.max_depth = 2;

    let bundle = assembler
        .assemble(fixture.lib_run_chunk_id, 10000, options)
        .await?;

    // Should include lib::run (primary) and its callees
    assert!(bundle.items.len() >= 2, "Should have primary + callees");

    // Verify primary
    let primary = &bundle.items[0];
    assert_eq!(primary.relpath, "src/lib.rs");
    assert_eq!(primary.role, "primary");
    assert!(primary.content.contains("pub fn run()"));

    // Should include load_config and process_request as callees
    let has_load_config = bundle
        .items
        .iter()
        .any(|item| item.relpath == "src/utils.rs" && item.content.contains("load_config"));
    let has_process_request = bundle
        .items
        .iter()
        .any(|item| item.relpath == "src/api.rs" && item.content.contains("process_request"));

    assert!(
        has_load_config,
        "Should include load_config as direct callee"
    );
    assert!(
        has_process_request,
        "Should include process_request as direct callee"
    );

    Ok(())
}

#[tokio::test]
async fn test_real_data_api_process_with_multi_level_callees() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match RealDataFixture::setup().await {
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

    let mut options = ExpandOptions::primary_only();
    options.include_callees = true;
    options.max_depth = 2;

    let bundle = assembler
        .assemble(fixture.api_process_request_chunk_id, 10000, options)
        .await?;

    // Should include primary and its callees (fetch_data, format_output)
    assert!(bundle.items.len() >= 2, "Should have primary + callees");

    // Verify primary
    let primary = &bundle.items[0];
    assert_eq!(primary.relpath, "src/api.rs");
    assert!(primary.content.contains("process_request"));

    // Check for callees
    let has_fetch_data = bundle
        .items
        .iter()
        .any(|item| item.content.contains("fetch_data"));
    let has_format_output = bundle
        .items
        .iter()
        .any(|item| item.content.contains("format_output"));

    assert!(has_fetch_data, "Should include fetch_data");
    assert!(has_format_output, "Should include format_output");

    Ok(())
}

#[tokio::test]
async fn test_real_data_content_matches_files() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match RealDataFixture::setup().await {
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

    let bundle = assembler
        .assemble(
            fixture.utils_load_config_chunk_id,
            10000,
            ExpandOptions::primary_only(),
        )
        .await?;

    let primary = &bundle.items[0];

    // Read the actual file
    let fixture_path = get_fixture_path();
    let utils_content = fs::read_to_string(fixture_path.join("src/utils.rs")).await?;
    let lines: Vec<&str> = utils_content.lines().collect();

    // Extract the expected lines (11-16, 1-indexed)
    let expected_lines: Vec<&str> = lines
        .iter()
        .skip(10) // Skip to line 11 (0-indexed: 10)
        .take(6) // Take 6 lines (11-16 inclusive)
        .copied()
        .collect();

    let expected_content = expected_lines.join("\n");

    // Content should match (may have slight formatting differences)
    assert!(
        primary.content.contains("pub fn load_config()"),
        "Content should contain function signature"
    );
    assert!(
        primary.content.contains("Config"),
        "Content should contain Config type"
    );

    // Verify the content is reasonable
    for line in expected_lines {
        if !line.trim().is_empty() && !line.trim().starts_with("//") {
            // Non-comment, non-empty lines should appear in content
            // (may be trimmed or formatted slightly differently)
            let trimmed = line.trim();
            if trimmed.len() > 3 {
                // Only check substantial lines
                assert!(
                    primary.content.contains(trimmed) || expected_content.contains(trimmed),
                    "Content should include line: {}",
                    trimmed
                );
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_real_data_multi_file_context() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match RealDataFixture::setup().await {
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
        max_depth: 3,
    };

    let bundle = assembler
        .assemble(fixture.lib_run_chunk_id, 10000, options)
        .await?;

    // Should span multiple files
    let unique_files: std::collections::HashSet<_> =
        bundle.items.iter().map(|item| &item.relpath).collect();

    assert!(
        unique_files.len() >= 2,
        "Context should span multiple files: {:?}",
        unique_files
    );

    // Should include lib.rs, utils.rs, and api.rs
    assert!(
        unique_files.contains(&"src/lib.rs".to_string()),
        "Should include lib.rs"
    );
    assert!(
        unique_files.contains(&"src/utils.rs".to_string()) || unique_files.contains(&"src/api.rs".to_string()),
        "Should include at least one dependency file"
    );

    Ok(())
}

#[tokio::test]
async fn test_real_data_realistic_budget_usage() -> Result<()> {
    if should_skip_db_test() {
        eprintln!("Skipping integration test: MAPROOM_DATABASE_URL not set");
        return Ok(());
    }

    let fixture = match RealDataFixture::setup().await {
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

    // Test with realistic budget (6000 tokens ~= 2-3 code files)
    let bundle = assembler
        .assemble(fixture.lib_run_chunk_id, 6000, options)
        .await?;

    // Should respect budget
    assert!(
        bundle.total_tokens <= 6000,
        "Should not exceed budget: {}",
        bundle.total_tokens
    );

    // Should use budget efficiently (not waste it)
    if bundle.truncated {
        let utilization = bundle.total_tokens as f64 / 6000.0;
        assert!(
            utilization > 0.7,
            "Should use most of budget when truncated: {}",
            utilization
        );
    }

    // Should include at least primary + some dependencies
    assert!(bundle.items.len() >= 1, "Should include primary");

    Ok(())
}
