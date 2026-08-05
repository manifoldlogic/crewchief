//! Integration tests for the exit code contract.
//!
//! AFM-06.2002: These tests spawn the `maproom` binary and assert
//! exit codes match the documented contract:
//!
//!   0 - Success (with or without results)
//!   1 - Runtime error (transient, may retry)
//!   2 - Configuration error (persistent, do not retry)
//!
//! Each test is isolated: it spawns a fresh binary process with controlled
//! environment variables. No shared state between tests.

use std::process::Command;

/// Get the path to the compiled `maproom` binary.
///
/// Uses `std::env::current_exe()` to navigate from the test binary location
/// to the main binary in the same target directory. Works for both
/// `cargo test` and `cargo test --release`.
fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current exe path");
    // Remove test binary name (e.g., "exit_codes-<hash>")
    path.pop();
    // Remove "deps/" directory
    path.pop();
    path.push("maproom");
    assert!(
        path.exists(),
        "Binary not found at: {}. Run `cargo build` first.",
        path.display()
    );
    path
}

/// Helper to create a Command for the maproom binary with a clean environment.
///
/// Removes potentially interfering environment variables while preserving
/// system essentials (PATH, HOME, TMPDIR, etc.).
fn maproom_cmd() -> Command {
    let mut cmd = Command::new(binary_path());
    // Remove env vars that could interfere with tests
    cmd.env_remove("MAPROOM_DATABASE_URL");
    cmd.env_remove("MAPROOM_DB_ROOT");
    cmd.env_remove("MAPROOM_EMBEDDING_PROVIDER");
    cmd.env_remove("MAPROOM_EMBEDDING_MODEL");
    cmd.env_remove("MAPROOM_EMBEDDING_DIMENSION");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("GOOGLE_PROJECT_ID");
    cmd.env_remove("GOOGLE_APPLICATION_CREDENTIALS");
    cmd.env_remove("OLLAMA_URL");
    cmd.env_remove("RUST_LOG");
    cmd
}

/// `--help` should exit 0 and include the EXIT CODES documentation section.
///
/// This is a sanity check that the binary runs and the help text includes
/// the exit code contract documentation from `docs/cli-help-after.md`.
#[test]
fn test_help_exits_0() {
    let output = maproom_cmd()
        .arg("--help")
        .output()
        .expect("Failed to execute binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 for --help.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    assert!(
        stdout.contains("EXIT CODES"),
        "Expected EXIT CODES section in help output.\nstdout: {}",
        stdout
    );
}

/// An invalid top-level flag should exit 2 (clap argument parsing error).
///
/// Clap returns exit code 2 for argument parsing failures. This verifies
/// the binary propagates clap's exit code correctly.
#[test]
fn test_invalid_flag_exits_2() {
    let output = maproom_cmd()
        .arg("--invalid-flag")
        .output()
        .expect("Failed to execute binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 for invalid flag (clap error).\nstderr: {}",
        stderr
    );
}

/// `vector-search` without MAPROOM_EMBEDDING_PROVIDER should exit 2
/// (configuration error).
///
/// The vector-search command requires an embedding provider to generate
/// query embeddings. When the provider is missing, it should exit with
/// code 2 and a "Configuration error" message.
#[test]
fn test_vector_search_missing_provider_exits_2() {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");

    let output = maproom_cmd()
        .args(["vector-search", "--repo", "test", "--query", "test"])
        .env("MAPROOM_DATABASE_URL", db_path.to_str().unwrap())
        .output()
        .expect("Failed to execute binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 for missing embedding provider.\nstderr: {}",
        stderr
    );

    assert!(
        stderr.contains("Configuration error"),
        "Expected 'Configuration error' in stderr.\nstderr: {}",
        stderr
    );
}

/// `generate-embeddings` without MAPROOM_EMBEDDING_PROVIDER should exit 2
/// (configuration error).
///
/// Similar to vector-search, generate-embeddings requires a configured
/// embedding provider. Missing provider is a persistent configuration
/// error (exit 2).
#[test]
fn test_generate_embeddings_missing_provider_exits_2() {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");

    let output = maproom_cmd()
        .args(["generate-embeddings"])
        .env("MAPROOM_DATABASE_URL", db_path.to_str().unwrap())
        .output()
        .expect("Failed to execute binary");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 for missing embedding provider.\nstderr: {}",
        stderr
    );

    assert!(
        stderr.contains("Configuration error"),
        "Expected 'Configuration error' in stderr.\nstderr: {}",
        stderr
    );
}

/// `db cleanup-stale` on an empty database should exit 0.
///
/// This was the original AFM-06 bug: cleanup-stale with no stale worktrees
/// should be a success (exit 0), not an error. A fresh database has no
/// worktrees at all, so there are no stale worktrees to clean up.
#[test]
fn test_cleanup_stale_empty_exits_0() {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");

    let output = maproom_cmd()
        .args(["db", "cleanup-stale"])
        .env("MAPROOM_DATABASE_URL", db_path.to_str().unwrap())
        .output()
        .expect("Failed to execute binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 for empty cleanup-stale.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    assert!(
        stdout.contains("No stale worktrees found"),
        "Expected 'No stale worktrees found' message.\nstdout: {}",
        stdout
    );
}

// ============================================================================
// Wave C (fix spec §5.1/§5.2) + R02/R03 process-level exit contracts
// ============================================================================

/// R13 / R-EXIT-1/2: identical config error → exit 2 in BOTH formats.
#[test]
fn config_error_exit_is_format_independent() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = format!("sqlite://{}/x.db", dir.path().display());
    let json = maproom_cmd()
        .env("MAPROOM_EMBEDDING_PROVIDER", "invalid")
        .env("MAPROOM_DATABASE_URL", &db)
        .args(["vector-search", "--repo", "x", "--query", "y"])
        .output()
        .unwrap();
    let agent = maproom_cmd()
        .env("MAPROOM_EMBEDDING_PROVIDER", "invalid")
        .env("MAPROOM_DATABASE_URL", &db)
        .args([
            "vector-search",
            "--repo",
            "x",
            "--query",
            "y",
            "--format",
            "agent",
        ])
        .output()
        .unwrap();
    assert_eq!(
        json.status.code(),
        Some(2),
        "json format must classify config errors (was 1)"
    );
    assert_eq!(agent.status.code(), Some(2));
}

/// R13: missing credentials in default json format exits 2 (was 1).
#[test]
fn vector_search_missing_credentials_exits_2_json() {
    let dir = tempfile::TempDir::new().unwrap();
    let out = maproom_cmd()
        .env("MAPROOM_EMBEDDING_PROVIDER", "openai") // no OPENAI_API_KEY (scrubbed)
        .env(
            "MAPROOM_DATABASE_URL",
            format!("sqlite://{}/x.db", dir.path().display()),
        )
        .args(["vector-search", "--repo", "x", "--query", "y"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// R13 + R12: serve with an empty DSN is a config error (exit 2).
#[test]
fn serve_db_config_error_exits_2() {
    let out = maproom_cmd()
        .args(["--database-url", "", "serve"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// R02 (deferred exact-code assertion): bad backup name exits 2 via R-EXIT-5.
#[test]
fn delete_backup_bad_name_exits_2() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = format!("sqlite://{}/g.db", dir.path().display());
    let mig = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &db)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(mig.status.success());
    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &db)
        .args(["migrate", "delete-backup", "--backup", "chunks"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "Configuration error: bails must exit 2 once top-level classification runs; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// R03: structurally damaged DB → db migrate exits 1 and names the table.
#[tokio::test]
async fn db_migrate_damaged_db_exits_1_names_table() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = format!("{}/dmg.db", dir.path().display());
    let db = format!("sqlite://{db_path}");
    let mig = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &db)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(mig.status.success());
    // Drop the chunks table through the crate's own store handle.
    let store = maproom::db::SqliteStore::connect(&db_path).await.unwrap();
    store
        .run(|conn| {
            conn.execute("DROP TABLE chunks", [])?;
            Ok(())
        })
        .await
        .unwrap();
    drop(store);
    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &db)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "structural damage is a runtime error"
    );
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        all.contains("chunks"),
        "must name the missing table; got: {all}"
    );
}

/// R12: empty --database-url flag exits 2.
#[test]
fn empty_database_url_flag_exits_2() {
    let out = maproom_cmd()
        .args(["--database-url", "", "status"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// R12: empty MAPROOM_DATABASE_URL env exits 2.
#[test]
fn empty_database_url_env_exits_2() {
    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", "")
        .args(["status"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// F13: `context --format agent` emits ONE structured error line on stdout
/// (classified type + suggestion), instead of leaking a raw anyhow chain.
#[test]
fn context_agent_error_is_structured() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/f13.db", db.path().display());
    // migrate so the failure is chunk-not-found, not schema
    let mig = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(mig.status.success());

    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["context", "--chunk-id", "424242", "--format", "agent"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "not_found class exits 1");
    assert!(
        stdout.contains("ERROR | type=not_found"),
        "structured stdout line required (F13); got: {stdout}"
    );
    assert!(
        stdout.contains("suggestion="),
        "actionable suggestion required; got: {stdout}"
    );
}

/// F15: a typo'd repo classifies as repository_not_found — not `unknown`.
#[test]
fn search_unknown_repo_is_repository_not_found() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/f15.db", db.path().display());
    let mig = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(mig.status.success());

    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args([
            "search",
            "--repo",
            "definitely-a-typo",
            "--query",
            "x",
            "--format",
            "agent",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stdout.contains("type=repository_not_found"),
        "typed classification required (F15); got: {stdout}"
    );
    assert!(
        stdout.contains("maproom status"),
        "suggestion names the fix: {stdout}"
    );
}

/// F01: `search --mode hybrid` degrades to FTS (exit 0, honest mode
/// metadata, stderr notice) when the provider is deterministically broken.
#[test]
fn search_hybrid_degrades_without_provider() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(repo.path())
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap();
        assert!(out.status.success());
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(
        repo.path().join("a.ts"),
        "export function hybridProbe() { return 1; }\n",
    )
    .unwrap();
    git(&["add", "a.ts"]);
    git(&["commit", "-qm", "i"]);
    let url = format!("sqlite://{}/f01.db", db.path().display());

    let scan = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["scan", "--repo", "fx", "--path"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(scan.status.success());

    // Broken-google env: provider construction fails deterministically
    // (auto-detect is NOT consulted when a provider is explicitly set).
    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .env("MAPROOM_EMBEDDING_PROVIDER", "google")
        .env("GOOGLE_APPLICATION_CREDENTIALS", "/nonexistent")
        .args([
            "search",
            "--repo",
            "fx",
            "--query",
            "hybridProbe",
            "--mode",
            "hybrid",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "hybrid must degrade, not die; stderr: {stderr}"
    );
    assert!(
        stdout.contains("hybridProbe"),
        "FTS results served: {stdout}"
    );
    assert!(
        stdout.contains("\"mode\": \"fts\"") || stdout.contains("\"mode\":\"fts\""),
        "metadata reports the EFFECTIVE mode: {stdout}"
    );
    assert!(
        stderr.contains("degraded to FTS"),
        "notice required: {stderr}"
    );
}

/// F01: `--mode vector` with a broken provider is a hard config error
/// (exit 2) — the user asked for semantics only vectors deliver.
#[test]
fn search_vector_broken_provider_exits_2() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/f01v.db", db.path().display());
    let mig = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(mig.status.success());

    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .env("MAPROOM_EMBEDDING_PROVIDER", "google")
        .env("GOOGLE_APPLICATION_CREDENTIALS", "/nonexistent")
        .args([
            "search", "--repo", "fx", "--query", "x", "--mode", "vector", "--format", "agent",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("type=embedding_provider"), "{stdout}");
}

// ── R5: D-8a / exactly-one-of CLI validation ──────────────────────────────

/// D-8a: `search` with no --repo and no --all-repos exits 2 (config error).
#[test]
fn search_no_repo_scope_exits_2() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/scope.db", db.path().display());
    // No repo or all-repos → should exit 2 without even touching the DB.
    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["search", "--query", "anything", "--format", "agent"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "no repo scope must be exit 2 (config error); stderr: {stderr}"
    );
    assert!(
        stderr.contains("at least one --repo") || stderr.contains("all-repos"),
        "error message must name the fix; stderr: {stderr}"
    );
}

/// D-8g: `search --all-repos --mode vector` exits 2 because multi-repo
/// vector/hybrid is not supported (only FTS is implemented for multi-repo).
#[test]
fn search_all_repos_vector_mode_exits_2() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/d8g.db", db.path().display());
    let mig = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(mig.status.success());

    // --all-repos + --mode vector must exit 2 (D-8g structured error)
    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args([
            "search",
            "--all-repos",
            "--query",
            "anything",
            "--mode",
            "vector",
            "--format",
            "agent",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "multi-repo vector mode must be exit 2 (D-8g); stderr: {stderr}"
    );
    assert!(
        stderr.contains("fts") || stderr.contains("D-8g") || stderr.contains("multi-repo"),
        "error must explain FTS-only constraint; stderr: {stderr}"
    );
}

/// D-8a: `search --all-repos --mode hybrid` exits 2 for the same reason (D-8g).
#[test]
fn search_all_repos_hybrid_mode_exits_2() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/d8g_hyb.db", db.path().display());
    let mig = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(mig.status.success());

    let out = maproom_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args([
            "search",
            "--all-repos",
            "--query",
            "anything",
            "--mode",
            "hybrid",
            "--format",
            "agent",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "multi-repo hybrid mode must be exit 2 (D-8g); stderr: {stderr}"
    );
}

// ── AWS Bedrock provider configuration ────────────────────────────────────

/// Build a command with every AWS credential source neutralized.
///
/// Without this, a developer machine with `~/.aws/credentials` or an EC2
/// builder with an instance role would resolve real credentials and these
/// tests would assert the wrong thing — or worse, make a billable API call.
fn bedrock_cmd() -> Command {
    let mut cmd = maproom_cmd();
    for variable in [
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "AWS_PROFILE",
        "MAPROOM_AWS_PROFILE",
        "AWS_ROLE_ARN",
        "AWS_WEB_IDENTITY_TOKEN_FILE",
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        "MAPROOM_BEDROCK_ENDPOINT_URL",
        "AWS_ENDPOINT_URL",
        "AWS_ENDPOINT_URL_BEDROCK_RUNTIME",
    ] {
        cmd.env_remove(variable);
    }
    // Point the shared-config lookups at nothing, and keep IMDS from being
    // probed at all (it would otherwise cost a 2s timeout per test).
    cmd.env("AWS_CONFIG_FILE", "/nonexistent/aws/config");
    cmd.env("AWS_SHARED_CREDENTIALS_FILE", "/nonexistent/aws/credentials");
    cmd.env("AWS_EC2_METADATA_DISABLED", "true");
    cmd.env("MAPROOM_BEDROCK_REGION", "us-east-1");
    cmd
}

/// A Bedrock model id we cannot infer a dimension for is a config error (2).
///
/// Guessing would silently build an index at the wrong width, which only
/// surfaces much later as vector search returning nothing.
#[test]
fn bedrock_unknown_model_without_dimension_exits_2() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/bedrock-model.db", db.path().display());
    let migrate = bedrock_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(migrate.status.success());

    let out = bedrock_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .env("MAPROOM_EMBEDDING_PROVIDER", "bedrock")
        .env("MAPROOM_EMBEDDING_MODEL", "some.unknown-embed-model")
        .args(["vector-search", "--repo", "fx", "--query", "x"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2), "config errors must exit 2");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("MAPROOM_EMBEDDING_DIMENSION"),
        "the error must say how to fix it: {combined}"
    );
}

/// Bedrock with no credentials anywhere is a config error (2), and the message
/// enumerates every source that was tried.
#[test]
fn bedrock_without_credentials_exits_2_and_lists_what_it_tried() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/bedrock-creds.db", db.path().display());
    let migrate = bedrock_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(migrate.status.success());

    let out = bedrock_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .env("MAPROOM_EMBEDDING_PROVIDER", "bedrock")
        .args(["vector-search", "--repo", "fx", "--query", "x"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2), "config errors must exit 2");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("No AWS credentials found"),
        "expected the credential-chain diagnostic: {combined}"
    );
    for expected in ["environment", "web identity", "default profile", "IMDSv2"] {
        assert!(
            combined.contains(expected),
            "the diagnostic must list the '{expected}' source it tried: {combined}"
        );
    }
    assert!(
        combined.contains("aws sts get-caller-identity"),
        "the diagnostic should name the command that verifies a fix: {combined}"
    );
}

/// An explicitly named profile that does not exist is a hard error, never a
/// silent fallback to some other credential source.
#[test]
fn bedrock_named_profile_that_is_missing_exits_2() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/bedrock-profile.db", db.path().display());
    let migrate = bedrock_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert!(migrate.status.success());

    let out = bedrock_cmd()
        .env("MAPROOM_DATABASE_URL", &url)
        .env("MAPROOM_EMBEDDING_PROVIDER", "bedrock")
        .env("MAPROOM_AWS_PROFILE", "no-such-profile")
        .args(["vector-search", "--repo", "fx", "--query", "x"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("no-such-profile"),
        "the error must name the profile that was requested: {combined}"
    );
}

/// `--provider aws` and `--provider aws-bedrock` are accepted aliases.
#[test]
fn bedrock_provider_aliases_are_accepted_by_the_cli() {
    for alias in ["bedrock", "aws", "aws-bedrock"] {
        let out = maproom_cmd()
            .args(["scan", "--path", "/nonexistent", "--provider", alias])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("Invalid provider"),
            "'{alias}' must be accepted as a Bedrock alias: {stderr}"
        );
    }
}

/// An unsupported provider name is still rejected, and the message lists the
/// real set including bedrock.
#[test]
fn unknown_provider_name_lists_bedrock_as_an_option() {
    let out = maproom_cmd()
        .args(["scan", "--path", "/tmp", "--provider", "voyage"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Invalid provider"), "{stderr}");
    assert!(
        stderr.contains("bedrock"),
        "the supported-provider list must mention bedrock: {stderr}"
    );
}
