//! Integration tests for the exit code contract.
//!
//! AFM-06.2002: These tests spawn the `crewchief-maproom` binary and assert
//! exit codes match the documented contract:
//!
//!   0 - Success (with or without results)
//!   1 - Runtime error (transient, may retry)
//!   2 - Configuration error (persistent, do not retry)
//!
//! Each test is isolated: it spawns a fresh binary process with controlled
//! environment variables. No shared state between tests.

use std::process::Command;

/// Get the path to the compiled `crewchief-maproom` binary.
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
    path.push("crewchief-maproom");
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
