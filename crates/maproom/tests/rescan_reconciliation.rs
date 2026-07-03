//! R09 regression tests: re-index reconciles superseded chunk generations —
//! the E2E symptom was chunks 10→20 across two `--force` rescans, with
//! deleted functions still searchable.
//!
//! Binary-driven (the exact user-visible path). Fix spec §4.1.
//! Run serialized: `cargo test --test rescan_reconciliation -- --test-threads=1`

use std::path::{Path, PathBuf};
use std::process::Command;

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
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

fn maproom(db_url: &str, args: &[&str], path_arg: Option<&Path>) -> std::process::Output {
    let mut cmd = Command::new(binary_path());
    cmd.args(args).env("MAPROOM_DATABASE_URL", db_url);
    if let Some(p) = path_arg {
        cmd.arg("--path").arg(p);
    }
    cmd.output().unwrap()
}

async fn chunk_count(db_dir: &Path) -> i64 {
    use maproom::db::traits::StoreCore;
    let store = maproom::db::SqliteStore::connect(&format!("{}/w.db", db_dir.display()))
        .await
        .unwrap();
    store.get_global_chunk_count().await.unwrap()
}

fn setup(repo: &Path) {
    git(repo, &["init", "-q", "-b", "main"]);
    std::fs::write(
        repo.join("a.ts"),
        "export function alphaOne() { return 1; }\n",
    )
    .unwrap();
    git(repo, &["add", "a.ts"]);
    git(repo, &["commit", "-qm", "init"]);
}

#[tokio::test]
async fn force_rescan_does_not_accumulate_chunks() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    setup(repo.path());
    let url = format!("sqlite://{}/w.db", db.path().display());

    assert!(maproom(&url, &["scan", "--repo", "fx"], Some(repo.path())).status.success());
    let n1 = chunk_count(db.path()).await;

    // Replace the file content, commit, force-rescan TWICE (E2E: 10 -> 20).
    std::fs::write(
        repo.path().join("a.ts"),
        "export function gammaThree() { return 3; }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "a.ts"]);
    git(repo.path(), &["commit", "-qm", "edit"]);
    assert!(maproom(&url, &["scan", "--repo", "fx", "--force"], Some(repo.path())).status.success());
    assert!(maproom(&url, &["scan", "--repo", "fx", "--force"], Some(repo.path())).status.success());
    let n2 = chunk_count(db.path()).await;

    assert!(
        n2 <= n1 + 1,
        "chunk generations must not accumulate across rescans (before={n1}, after={n2})"
    );
}

#[tokio::test]
async fn deleted_file_unsearchable_after_rescan() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    setup(repo.path());
    // Second file that will be deleted.
    std::fs::write(
        repo.path().join("b.ts"),
        "export function doomedFn() { return 0; }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "b.ts"]);
    git(repo.path(), &["commit", "-qm", "add b"]);
    let url = format!("sqlite://{}/w.db", db.path().display());

    assert!(maproom(&url, &["scan", "--repo", "fx"], Some(repo.path())).status.success());
    let found = maproom(&url, &["search", "--repo", "fx", "--query", "doomedFn", "--format", "agent"], None);
    assert!(String::from_utf8_lossy(&found.stdout).contains("doomedFn"), "baseline: doomedFn indexed");

    // Delete the file, commit, rescan (R-GC-5 walk reconciliation).
    std::fs::remove_file(repo.path().join("b.ts")).unwrap();
    git(repo.path(), &["add", "-A"]);
    git(repo.path(), &["commit", "-qm", "rm b"]);
    assert!(maproom(&url, &["scan", "--repo", "fx", "--force"], Some(repo.path())).status.success());

    let after = maproom(&url, &["search", "--repo", "fx", "--query", "doomedFn", "--format", "json"], None);
    let stdout = String::from_utf8_lossy(&after.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json output");
    let n = json["hits"].as_array().map(|a| a.len()).unwrap_or(usize::MAX);
    assert_eq!(
        n, 0,
        "deleted file's chunks must be unsearchable after rescan; got:\n{stdout}"
    );
}

#[tokio::test]
async fn status_language_counts_stable_across_rescans() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    setup(repo.path());
    let url = format!("sqlite://{}/w.db", db.path().display());

    assert!(maproom(&url, &["scan", "--repo", "fx"], Some(repo.path())).status.success());
    let n1 = chunk_count(db.path()).await;
    // Unchanged content: two more force rescans must not inflate anything.
    assert!(maproom(&url, &["scan", "--repo", "fx", "--force"], Some(repo.path())).status.success());
    assert!(maproom(&url, &["scan", "--repo", "fx", "--force"], Some(repo.path())).status.success());
    let n2 = chunk_count(db.path()).await;
    assert_eq!(n1, n2, "identical content must be chunk-count stable across force rescans");
}

// ============================================================================
// Review H1: reconciliation must be FULL-SCOPE-ONLY (filtered/subdir scans
// must never wipe out-of-scope index entries)
// ============================================================================

#[tokio::test]
async fn language_filtered_rescan_does_not_wipe_other_languages() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    setup(repo.path());
    std::fs::write(repo.path().join("m.py"), "def python_probe_fn():\n    return 1\n").unwrap();
    git(repo.path(), &["add", "m.py"]);
    git(repo.path(), &["commit", "-qm", "py"]);
    let url = format!("sqlite://{}/w.db", db.path().display());

    assert!(maproom(&url, &["scan", "--repo", "fx"], Some(repo.path())).status.success());
    let baseline = maproom(&url, &["search", "--repo", "fx", "--query", "python_probe_fn", "--format", "agent"], None);
    assert!(String::from_utf8_lossy(&baseline.stdout).contains("python_probe_fn"), "baseline: py indexed");

    // Language-scoped rescan: ts only. The H1 bug wiped everything not walked.
    assert!(maproom(&url, &["scan", "--repo", "fx", "--force", "--languages", "ts"], Some(repo.path())).status.success());
    let after = maproom(&url, &["search", "--repo", "fx", "--query", "python_probe_fn", "--format", "agent"], None);
    assert!(
        String::from_utf8_lossy(&after.stdout).contains("python_probe_fn"),
        "filtered scan must NOT unmap out-of-scope languages (H1)"
    );
}

#[tokio::test]
async fn subdirectory_rescan_does_not_wipe_worktree_index() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    setup(repo.path());
    std::fs::create_dir(repo.path().join("sub")).unwrap();
    std::fs::write(repo.path().join("sub/s.ts"), "export function subFn() { return 9; }\n").unwrap();
    git(repo.path(), &["add", "sub/s.ts"]);
    git(repo.path(), &["commit", "-qm", "sub"]);
    let url = format!("sqlite://{}/w.db", db.path().display());

    assert!(maproom(&url, &["scan", "--repo", "fx"], Some(repo.path())).status.success());
    // Rescan from the SUBDIRECTORY (registered root is the repo root).
    assert!(maproom(&url, &["scan", "--repo", "fx", "--force"], Some(&repo.path().join("sub"))).status.success());

    let after = maproom(&url, &["search", "--repo", "fx", "--query", "alphaOne", "--format", "agent"], None);
    assert!(
        String::from_utf8_lossy(&after.stdout).contains("alphaOne"),
        "subdir scan must NOT wipe the rest of the worktree's index (H1)"
    );
}
