//! F69 acceptance: the daemon holds a REAL search-response cache.
//!
//! - a repeated identical search is served from cache (hits counter rises)
//! - `cache.warm` runs queries through the SAME cached path, so a warmed
//!   query's next search is a cache hit
//! - `cache.stats` reports live counters, not fiction
//!
//! Run serialized: shares the stdio-daemon spawn pattern with
//! daemon_stdio_degraded.

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

fn run_serve_with_lines(db_url: &str, lines: &[String]) -> (String, String, i32) {
    let mut child = Command::new(binary_path())
        .env_remove("MAPROOM_EMBEDDING_PROVIDER")
        .env_remove("OLLAMA_URL")
        .env_remove("OLLAMA_HOST")
        .env_remove("MAPROOM_OLLAMA_URL")
        .env("MAPROOM_DATABASE_URL", db_url)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn serve");
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

/// Parse newline-delimited JSON-RPC responses into (id -> result).
fn responses_by_id(stdout: &str) -> std::collections::HashMap<i64, serde_json::Value> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            let id = v.get("id")?.as_i64()?;
            Some((id, v))
        })
        .collect()
}

#[test]
fn daemon_search_cache_hits_and_warm_rpc() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    git(repo.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(
        repo.path().join("a.ts"),
        "export function cacheProbeOne() { return 1; }\nexport function cacheProbeTwo() { return 2; }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "a.ts"]);
    git(repo.path(), &["commit", "-qm", "i"]);
    let url = format!("sqlite://{}/f69.db", db.path().display());

    let scan = Command::new(binary_path())
        .args(["scan", "--repo", "fx", "--path"])
        .arg(repo.path())
        .env("MAPROOM_DATABASE_URL", &url)
        .output()
        .unwrap();
    assert!(scan.status.success());

    let search =
        |id: i64, q: &str| format!(r#"{{"jsonrpc":"2.0","method":"search","params":{{"repo":"fx","query":"{q}","mode":"fts"}},"id":{id}}}"#);
    let lines = vec![
        search(1, "cacheProbeOne"),                // miss -> executes + populates
        search(2, "cacheProbeOne"),                // identical -> CACHE HIT
        r#"{"jsonrpc":"2.0","method":"cache.stats","id":3}"#.to_string(),
        // warm a NEW query through the RPC...
        r#"{"jsonrpc":"2.0","method":"cache.warm","params":{"queries":["cacheProbeTwo"],"repo":"fx","mode":"fts"},"id":4}"#.to_string(),
        search(5, "cacheProbeTwo"),                // ...so this is a hit too
        r#"{"jsonrpc":"2.0","method":"cache.stats","id":6}"#.to_string(),
    ];
    let (stdout, stderr, code) = run_serve_with_lines(&url, &lines);
    assert_eq!(code, 0, "stderr: {stderr}");
    let by_id = responses_by_id(&stdout);

    // Both identical searches return the same hits payload.
    let r1 = &by_id[&1]["result"];
    let r2 = &by_id[&2]["result"];
    assert_eq!(r1, r2, "cached response must be identical to the original");
    assert!(
        r1["hits"].as_array().is_some_and(|h| !h.is_empty()),
        "the cached search must carry real hits: {r1}"
    );

    // stats after the pair: exactly 1 hit (search 2), >=1 miss (search 1).
    let s3 = &by_id[&3]["result"];
    assert_eq!(s3["hits"].as_u64(), Some(1), "second identical search is a cache HIT: {s3}");
    assert!(s3["misses"].as_u64().unwrap_or(0) >= 1, "{s3}");

    // warm reports real work, no fiction.
    let w = &by_id[&4]["result"];
    assert_eq!(w["warmed"].as_u64(), Some(1), "{w}");
    assert_eq!(w["failed"].as_array().map(Vec::len), Some(0), "{w}");

    // the warmed query's subsequent search was a hit: hits grew to 2.
    let s6 = &by_id[&6]["result"];
    assert_eq!(
        s6["hits"].as_u64(),
        Some(2),
        "search after cache.warm must be served from cache: {s6}"
    );
    assert!(s6["hit_rate"].as_f64().unwrap_or(0.0) > 0.0, "{s6}");
}

/// F69: `serve --warm-queries` populates the cache before/while serving.
#[test]
fn serve_warm_queries_flag_populates_cache() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    git(repo.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(repo.path().join("a.ts"), "export function warmStartProbe() { return 1; }\n")
        .unwrap();
    git(repo.path(), &["add", "a.ts"]);
    git(repo.path(), &["commit", "-qm", "i"]);
    let url = format!("sqlite://{}/f69w.db", db.path().display());

    let scan = Command::new(binary_path())
        .args(["scan", "--repo", "fx", "--path"])
        .arg(repo.path())
        .env("MAPROOM_DATABASE_URL", &url)
        .output()
        .unwrap();
    assert!(scan.status.success());

    let qfile = db.path().join("warm.txt");
    std::fs::write(&qfile, "warmStartProbe\n").unwrap();

    // Startup warming is async: poll cache.stats interleaved (write one
    // request, read its one reply) until size >= 1 or a REAL deadline —
    // never a fixed sleep window (the exact flake class F81's [39] fixed).
    // Deterministically provider-LESS (broken google, no auto-detect): the
    // warm query degrades hybrid->FTS and must STILL be cached — the cache
    // must not be inert in provider-less deployments.
    let mut child = Command::new(binary_path())
        .env("MAPROOM_EMBEDDING_PROVIDER", "google")
        .env("GOOGLE_APPLICATION_CREDENTIALS", "/nonexistent")
        .env_remove("OLLAMA_URL")
        .env_remove("OLLAMA_HOST")
        .env_remove("MAPROOM_OLLAMA_URL")
        .env("MAPROOM_DATABASE_URL", &url)
        .args(["serve", "--warm-queries"])
        .arg(&qfile)
        .args(["--warm-repo", "fx"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn serve");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = std::io::BufReader::new(stdout);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut warmed = false;
    let mut last = String::new();
    let mut id: u64 = 0;
    while std::time::Instant::now() < deadline {
        use std::io::BufRead as _;
        use std::io::Write as _;
        id += 1;
        writeln!(stdin, r#"{{"jsonrpc":"2.0","method":"cache.stats","id":{id}}}"#).unwrap();
        stdin.flush().unwrap();
        last.clear();
        if reader.read_line(&mut last).unwrap() == 0 {
            panic!("daemon exited early");
        }
        let v: serde_json::Value = serde_json::from_str(&last).expect("stats reply");
        if v["result"]["size"].as_u64().unwrap_or(0) >= 1 {
            warmed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    drop(stdin); // EOF -> daemon exits
    let status = child.wait().expect("wait serve");
    assert!(status.success());
    assert!(
        warmed,
        "startup warming must populate the cache within the 60s deadline; last stats: {last}"
    );
}
