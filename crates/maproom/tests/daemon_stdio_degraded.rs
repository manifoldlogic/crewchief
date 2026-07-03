//! R16 regression tests: `serve` starts and serves non-embedding requests in
//! provider-less / broken-provider environments (the E2E container default:
//! MAPROOM_EMBEDDING_PROVIDER=google with no usable credentials made the
//! daemon DOA in all modes on both backends).
//!
//! Fix spec: _SPECS/crewchief/research/maproom-cli-e2e-fix-spec.md §3.5.
//! Run serialized: `cargo test --test daemon_stdio_degraded -- --test-threads=1`

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn binary_path() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    p.pop();
    p.push("maproom");
    assert!(p.exists(), "maproom binary not built");
    p
}

/// Broken-google env: the exact shape the E2E container had.
fn serve_cmd(db_url: &str) -> Command {
    let mut cmd = Command::new(binary_path());
    cmd.env_remove("OPENAI_API_KEY")
        .env_remove("OLLAMA_URL")
        .env("MAPROOM_DATABASE_URL", db_url)
        .env("MAPROOM_EMBEDDING_PROVIDER", "google")
        .env("GOOGLE_APPLICATION_CREDENTIALS", "/nonexistent")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

fn run_serve_with_lines(db_url: &str, lines: &[&str]) -> (String, String, i32) {
    let mut child = serve_cmd(db_url).arg("serve").spawn().expect("spawn serve");
    {
        let stdin = child.stdin.as_mut().unwrap();
        for l in lines {
            writeln!(stdin, "{l}").unwrap();
        }
    } // drop -> EOF -> daemon exits
    let out = child.wait_with_output().expect("wait serve");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn serve_answers_ping_with_broken_google_env() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/r16.db", db.path().display());
    let (stdout, stderr, code) =
        run_serve_with_lines(&url, &[r#"{"jsonrpc":"2.0","method":"ping","id":1}"#]);
    assert_eq!(code, 0, "serve must not die on embedding config; stderr: {stderr}");
    assert!(
        stdout.contains(r#""result":"pong""#),
        "ping must be answered; stdout: {stdout}"
    );
}

#[test]
fn serve_fts_search_works_without_provider() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    git(repo.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(
        repo.path().join("a.ts"),
        "export function alphaOne() { return 1; }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "a.ts"]);
    git(repo.path(), &["commit", "-qm", "i"]);
    let url = format!("sqlite://{}/w.db", db.path().display());

    let scan = Command::new(binary_path())
        .args(["scan", "--repo", "fx", "--path"])
        .arg(repo.path())
        .env("MAPROOM_DATABASE_URL", &url)
        .output()
        .unwrap();
    assert!(scan.status.success());

    let req = r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne","mode":"fts"},"id":3}"#;
    let (stdout, stderr, code) = run_serve_with_lines(&url, &[req]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("alphaOne"),
        "FTS search must work without a provider; stdout: {stdout}"
    );
}

/// THE R-LAZY-4 GUARD: hybrid (the daemon DEFAULT mode) falls back to FTS
/// when the provider is unavailable — the exact default-flow the E2E
/// environment exercised.
#[test]
fn hybrid_search_falls_back_to_fts_without_provider() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    git(repo.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(
        repo.path().join("a.ts"),
        "export function alphaOne() { return 1; }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "a.ts"]);
    git(repo.path(), &["commit", "-qm", "i"]);
    let url = format!("sqlite://{}/w.db", db.path().display());

    let scan = Command::new(binary_path())
        .args(["scan", "--repo", "fx", "--path"])
        .arg(repo.path())
        .env("MAPROOM_DATABASE_URL", &url)
        .output()
        .unwrap();
    assert!(scan.status.success());

    // No explicit mode -> daemon default (hybrid).
    let req = r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne"},"id":4}"#;
    let (stdout, stderr, code) = run_serve_with_lines(&url, &[req]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("alphaOne") && !stdout.contains(r#""error""#),
        "hybrid must degrade to a successful FTS result set, not an error; stdout: {stdout}"
    );
}

/// R-LAZY-5/6 socket half: `serve --socket` starts without a provider and
/// idle-exits 0, with no "Database error" mislabel.
#[test]
fn serve_socket_starts_and_idle_exits_without_provider() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/r16s.db", db.path().display());
    let sock = db.path().join("r16.sock");

    // Review [34]/[37]: per-test --pid-path so this daemon can't lose the
    // global /tmp/maproom-{uid}.pid flock race against the lifecycle suite
    // or a genuinely running user daemon.
    let pid = db.path().join("r16.pid");
    let out = serve_cmd(&url)
        .args(["serve", "--socket", "--socket-path"])
        .arg(&sock)
        .arg("--pid-path")
        .arg(&pid)
        .args(["--idle-timeout", "3"])
        .stdin(Stdio::null())
        .output()
        .expect("run serve --socket");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "socket serve must start and idle-exit without a provider; stderr: {stderr}"
    );
    assert!(
        !stderr.contains("Database error"),
        "the R16 'Database error' mislabel must be gone; stderr: {stderr}"
    );
}
