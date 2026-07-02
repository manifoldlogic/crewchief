//! Negative tests for backend selection (F70 / R-SEL-3, R-SEL-4).
//!
//! Proves the fail-loud contract: a `postgres://` database URL must NEVER
//! silently fall back to SQLite, and must never create SQLite artifacts
//! derived from the DSN (the data-residency guarantee).
//!
//! Two build-specific arms (each compiled only for its build):
//!   * default build:  the pre-dispatch guard in main.rs rejects the URL with
//!     exit 2 (EXIT_CONFIG_ERROR) before any command dispatch.
//!   * --features postgres build: an unreachable Postgres errors out of
//!     `db::connect()` (exit 1 via anyhow) — no SQLite fallback either way.
//!
//! Neither arm requires a live Postgres server (127.0.0.1:1 is always refused).
//! Follows the binary-spawning pattern of `exit_codes.rs`.

use std::process::Command;

/// Get the path to the compiled `maproom` binary (same pattern as exit_codes.rs).
fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current exe path");
    path.pop(); // test binary name
    path.pop(); // deps/
    path.push("maproom");
    assert!(
        path.exists(),
        "Binary not found at: {}. Run `cargo build` first.",
        path.display()
    );
    path
}

/// Command with a scrubbed environment and a fresh temp working directory.
/// Returns (command, tempdir). The tempdir doubles as the artifact-leak probe:
/// after the run it must contain exactly what it contained before (nothing).
fn maproom_cmd_in_tempdir() -> (Command, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let mut cmd = Command::new(binary_path());
    cmd.current_dir(dir.path());
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
    (cmd, dir)
}

/// Everything under `dir`, sorted — the before/after leak snapshot.
fn snapshot(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    fn walk(d: &std::path::Path, acc: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(d) {
            for e in entries.flatten() {
                acc.push(e.path());
                if e.path().is_dir() {
                    walk(&e.path(), acc);
                }
            }
        }
    }
    let mut acc = Vec::new();
    walk(dir, &mut acc);
    acc.sort();
    acc
}

/// Default build: a postgres:// URL is a configuration error — the pre-dispatch
/// guard exits 2 BEFORE `serve --socket` dispatches, and nothing is created on
/// disk (in particular no directory tree derived from the DSN).
#[cfg(not(feature = "postgres"))]
#[test]
fn default_build_postgres_url_exits_2_with_no_artifacts() {
    let (mut cmd, dir) = maproom_cmd_in_tempdir();
    let before = snapshot(dir.path());

    let output = cmd
        .env("MAPROOM_DATABASE_URL", "postgres://u:p@127.0.0.1:1/db")
        .args(["serve", "--socket"])
        .output()
        .expect("Failed to execute binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected EXIT_CONFIG_ERROR (2) from the pre-dispatch guard; stderr: {stderr}"
    );
    assert!(
        stderr.contains("postgres scheme") || stderr.contains("Configuration error"),
        "guard message should name the postgres-scheme misconfiguration; stderr: {stderr}"
    );

    let after = snapshot(dir.path());
    assert_eq!(
        before, after,
        "no artifacts may be created for a rejected postgres:// URL (found: {after:?})"
    );
}

/// Feature build: an unreachable Postgres must ERROR (non-zero exit via the
/// shared `db::connect()` factory), not fall back to SQLite. Port 1 on
/// loopback is never listening, so this needs no live server. The temp cwd
/// must stay empty: the pre-F70 bug created a DSN-derived directory tree via
/// SqliteStore::connect's create_dir_all.
#[cfg(feature = "postgres")]
#[test]
fn feature_build_unreachable_postgres_errors_with_no_sqlite_fallback() {
    use std::time::{Duration, Instant};

    let (mut cmd, dir) = maproom_cmd_in_tempdir();
    let before = snapshot(dir.path());
    let sock = dir.path().join("pg_negative_test.sock");

    // Spawn + poll with a deadline instead of blocking on output(): the
    // pre-F70 bug made `serve --socket` SUCCEED against SQLite and listen
    // forever — a regression must fail this test, not hang the suite.
    let mut child = cmd
        .env(
            "MAPROOM_DATABASE_URL",
            "postgres://maproom:maproom@127.0.0.1:1/maproom_test",
        )
        .args(["serve", "--socket", "--socket-path"])
        .arg(&sock)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn binary");

    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        match child.try_wait().expect("try_wait failed") {
            Some(status) => break status,
            None if Instant::now() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "daemon still running after 20s with an unreachable postgres:// URL — \
                     it must fail fast, not fall back to SQLite and start serving"
                );
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    let mut stderr_buf = String::new();
    {
        use std::io::Read;
        if let Some(mut e) = child.stderr.take() {
            let _ = e.read_to_string(&mut stderr_buf);
        }
    }
    let stderr = stderr_buf;
    assert_ne!(
        status.code(),
        Some(0),
        "unreachable postgres:// must not report success; stderr: {stderr}"
    );
    // The failure must come from the Postgres connection path, not from a
    // SQLite fallback silently proceeding to a different error later.
    assert!(
        !stderr.contains("SQLite"),
        "failure must not route through SQLite; stderr: {stderr}"
    );

    let after = snapshot(dir.path());
    assert_eq!(
        before, after,
        "no SQLite/DSN-derived artifacts may be created for a postgres:// URL (found: {after:?})"
    );
}
