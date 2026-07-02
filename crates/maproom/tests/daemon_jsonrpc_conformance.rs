//! JSON-RPC 2.0 conformance tests for the stdio daemon (R18 + R19).
//!
//! Fix spec §4.4 (R-WTF-1) and §5.4 (R-RPC-1..4). The R19 protocol tests are
//! added in Wave C; the R18 unknown-worktree test lands with Wave B.
//! Run serialized: `cargo test --test daemon_jsonrpc_conformance -- --test-threads=1`

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn binary_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.push("maproom");
    assert!(p.exists(), "maproom binary not built");
    p
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

/// Pipe newline-delimited JSON-RPC lines into `serve`; return (stdout, stderr).
fn serve(db_url: &str, lines: &[&str]) -> (String, String) {
    let mut child = Command::new(binary_path())
        .arg("serve")
        .env("MAPROOM_DATABASE_URL", db_url)
        .env_remove("MAPROOM_EMBEDDING_PROVIDER")
        .env_remove("OPENAI_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for l in lines {
            writeln!(stdin, "{l}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn scanned_fixture() -> (tempfile::TempDir, tempfile::TempDir, String) {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    git(repo.path(), &["init", "-q"]);
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
    (repo, db, url)
}

/// R18 / R-WTF-1: unknown worktree name -> JSON-RPC -32602 naming it.
#[test]
fn search_unknown_worktree_returns_invalid_params() {
    let (_repo, _db, url) = scanned_fixture();
    let (stdout, _stderr) = serve(
        &url,
        &[r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne","worktree":"no-such-wt"},"id":7}"#],
    );
    assert!(
        stdout.contains("-32602"),
        "unknown worktree must be Invalid params (-32602); got: {stdout}"
    );
    assert!(
        stdout.contains("no-such-wt"),
        "the error should name the unknown worktree; got: {stdout}"
    );

    // Control: the KNOWN worktree still searches fine.
    let (ok_out, _) = serve(
        &url,
        &[r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne","worktree":"main","mode":"fts"},"id":8}"#],
    );
    assert!(
        ok_out.contains("alphaOne") && !ok_out.contains("-32602"),
        "known worktree must search; got: {ok_out}"
    );
}
