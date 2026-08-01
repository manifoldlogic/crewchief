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
    (repo, db, url)
}

/// R18 / R-WTF-1: unknown worktree name -> JSON-RPC -32602 naming it.
#[test]
fn search_unknown_worktree_returns_invalid_params() {
    let (_repo, _db, url) = scanned_fixture();
    let (stdout, _stderr) = serve(
        &url,
        &[
            r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne","worktree":"no-such-wt"},"id":7}"#,
        ],
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
        &[
            r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne","worktree":"main","mode":"fts"},"id":8}"#,
        ],
    );
    assert!(
        ok_out.contains("alphaOne") && !ok_out.contains("-32602"),
        "known worktree must search; got: {ok_out}"
    );
}

// ============================================================================
// R19 (fix spec §5.4): JSON-RPC 2.0 strictness
// ============================================================================

/// R-RPC-1: a request with NO id is a notification — no reply line at all.
#[test]
fn notification_receives_no_reply() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/x.db", db.path().display());
    let (stdout, _stderr) = serve(&url, &[r#"{"jsonrpc":"2.0","method":"ping"}"#]);
    assert_eq!(
        stdout.lines().filter(|l| !l.trim().is_empty()).count(),
        0,
        "notifications must not be answered; got: {stdout}"
    );
}

/// R-RPC-1: explicit "id": null is a REQUEST (answered with "id":null).
#[test]
fn explicit_null_id_still_answered() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/x.db", db.path().display());
    let (stdout, _stderr) = serve(&url, &[r#"{"jsonrpc":"2.0","method":"ping","id":null}"#]);
    assert!(
        stdout.contains(r#""result":"pong""#),
        "null-id request must be answered; got: {stdout}"
    );
}

/// R-RPC-2: version != "2.0" → -32600 Invalid Request.
#[test]
fn jsonrpc_version_1_0_rejected_32600() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/x.db", db.path().display());
    let (stdout, _stderr) = serve(&url, &[r#"{"jsonrpc":"1.0","method":"ping","id":23}"#]);
    assert!(stdout.contains("-32600"), "got: {stdout}");
}

/// R-RPC-2 (OD-10): MISSING version field → -32600, not -32700.
#[test]
fn missing_jsonrpc_field_rejected_32600() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/x.db", db.path().display());
    let (stdout, _stderr) = serve(&url, &[r#"{"method":"ping","id":24}"#]);
    assert!(
        stdout.contains("-32600"),
        "missing version is an invalid REQUEST, not a parse error; got: {stdout}"
    );
    assert!(!stdout.contains("-32700"), "got: {stdout}");
}

/// R-RPC-3 (OD-11): batch arrays rejected with a single -32600.
#[test]
fn batch_array_rejected_32600() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/x.db", db.path().display());
    let (stdout, _stderr) = serve(&url, &[r#"[{"jsonrpc":"2.0","method":"ping","id":1}]"#]);
    assert!(
        stdout.contains("Batch requests are not supported") && stdout.contains("-32600"),
        "got: {stdout}"
    );
}

// ============================================================================
// R5 / D-8e: daemon round-trip backward-compat + new scope shapes
// ============================================================================

/// R5 / D-8e: existing single-repo {repo: "name"} payload is byte-compatible
/// with the 0.2.0 response shape — the serde-additive change must not break it.
#[test]
fn old_shape_single_repo_compat() {
    let (_repo, _db, url) = scanned_fixture();
    // Old-shape: {"repo": "fx", "query": "..."} — no `repos` or `all_repos` fields.
    let (stdout, _stderr) = serve(
        &url,
        &[
            r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","query":"alphaOne","mode":"fts"},"id":42}"#,
        ],
    );
    assert!(
        !stdout.contains("-32602") && !stdout.contains("-32000"),
        "old-shape must search without error; got: {stdout}"
    );
    assert!(
        stdout.contains("alphaOne"),
        "old-shape must find indexed content; got: {stdout}"
    );
    // Response must have "result" key (not "error") — backward-compatible shape.
    assert!(
        stdout.contains("\"result\""),
        "response must carry 'result' key; got: {stdout}"
    );
}

/// R5 / D-8e: new {repos: [...]} list shape dispatches correctly.
#[test]
fn new_repos_list_shape_searches() {
    let (_repo, _db, url) = scanned_fixture();
    let (stdout, _stderr) = serve(
        &url,
        &[
            r#"{"jsonrpc":"2.0","method":"search","params":{"repos":["fx"],"query":"alphaOne","mode":"fts"},"id":43}"#,
        ],
    );
    assert!(
        !stdout.contains("-32602") && !stdout.contains("-32000"),
        "repos-list shape must search without error; got: {stdout}"
    );
    assert!(
        stdout.contains("alphaOne"),
        "repos-list shape must find indexed content; got: {stdout}"
    );
}

/// R5 / D-8e: new {all_repos: true} shape dispatches correctly.
#[test]
fn new_all_repos_shape_searches() {
    let (_repo, _db, url) = scanned_fixture();
    let (stdout, _stderr) = serve(
        &url,
        &[
            r#"{"jsonrpc":"2.0","method":"search","params":{"all_repos":true,"query":"alphaOne","mode":"fts"},"id":44}"#,
        ],
    );
    assert!(
        !stdout.contains("-32602") && !stdout.contains("-32000"),
        "all_repos shape must search without error; got: {stdout}"
    );
    assert!(
        stdout.contains("alphaOne"),
        "all_repos shape must find indexed content; got: {stdout}"
    );
}

/// R3 / D-8a: supplying both repo and repos returns JSON-RPC -32602 (not -32000).
#[test]
fn scope_conflict_returns_invalid_params_32602() {
    let (_repo, _db, url) = scanned_fixture();
    // Both repo + repos supplied → exactly-one-of violation → -32602.
    let (stdout, _stderr) = serve(
        &url,
        &[
            r#"{"jsonrpc":"2.0","method":"search","params":{"repo":"fx","repos":["fy"],"query":"x","mode":"fts"},"id":45}"#,
        ],
    );
    assert!(
        stdout.contains("-32602"),
        "scope conflict must return -32602 Invalid params (not -32000); got: {stdout}"
    );
    assert!(
        !stdout.contains("-32000"),
        "must not be -32000 Internal error; got: {stdout}"
    );
}

/// R3 / D-8a: no scope supplied returns JSON-RPC -32602 (not -32000).
#[test]
fn no_scope_returns_invalid_params_32602() {
    let (_repo, _db, url) = scanned_fixture();
    // Neither repo, repos, nor all_repos supplied → -32602.
    let (stdout, _stderr) = serve(
        &url,
        &[r#"{"jsonrpc":"2.0","method":"search","params":{"query":"x","mode":"fts"},"id":46}"#],
    );
    assert!(
        stdout.contains("-32602"),
        "no scope must return -32602 Invalid params (not -32000); got: {stdout}"
    );
}

/// D-8g: vector mode with multi-repo scope returns -32602.
#[test]
fn multi_repo_vector_mode_returns_32602() {
    let (_repo, _db, url) = scanned_fixture();
    let (stdout, _stderr) = serve(
        &url,
        &[
            r#"{"jsonrpc":"2.0","method":"search","params":{"all_repos":true,"query":"x","mode":"vector"},"id":47}"#,
        ],
    );
    assert!(
        stdout.contains("-32602"),
        "multi-repo vector mode must return -32602; got: {stdout}"
    );
}
