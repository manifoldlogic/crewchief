//! R17 regression tests: the stdio daemon's stdout is PURE newline-delimited
//! JSON-RPC — tracing goes to stderr, ANSI only on real terminals.
//!
//! Fix spec §5.3. The E2E sweep captured ANSI-colored ERROR log lines
//! interleaved with protocol responses on stdout (root cause: tracing
//! fmt().init() defaults to stdout + unconditional ANSI).

use std::io::Write;
use std::process::{Command, Stdio};

fn binary_path() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.push("maproom");
    assert!(p.exists(), "maproom binary not built");
    p
}

fn serve_with_log(db_url: &str, lines: &[&str]) -> (Vec<u8>, Vec<u8>) {
    let mut child = Command::new(binary_path())
        .arg("serve")
        .env("MAPROOM_DATABASE_URL", db_url)
        .env("RUST_LOG", "info")
        .env_remove("MAPROOM_EMBEDDING_PROVIDER")
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
    (out.stdout, out.stderr)
}

#[test]
fn stdout_is_pure_jsonrpc_under_rust_log_info() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/x.db", db.path().display());
    let (stdout, stderr) = serve_with_log(
        &url,
        &[
            r#"{"jsonrpc":"2.0","method":"ping","id":1}"#,
            r#"{"jsonrpc":"2.0","method":"status","params":{},"id":2}"#,
        ],
    );
    let stdout_s = String::from_utf8_lossy(&stdout);
    assert!(
        stdout_s.contains(r#""result":"pong""#),
        "responses must be on stdout; got: {stdout_s}"
    );
    for line in stdout_s.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            serde_json::from_str::<serde_json::Value>(line).is_ok(),
            "every stdout line must parse as JSON (protocol purity); offending line: {line:?}"
        );
    }
    assert!(
        !stderr.is_empty(),
        "info-level logs must land on stderr (daemon startup logs at info)"
    );
}

#[test]
fn stdout_has_no_ansi_when_piped() {
    let db = tempfile::TempDir::new().unwrap();
    let url = format!("sqlite://{}/x.db", db.path().display());
    let (stdout, stderr) = serve_with_log(&url, &[r#"{"jsonrpc":"2.0","method":"ping","id":1}"#]);
    assert!(!stdout.contains(&0x1b), "no ANSI escapes on piped stdout");
    assert!(!stderr.contains(&0x1b), "no ANSI escapes on piped stderr (not a terminal)");
}
