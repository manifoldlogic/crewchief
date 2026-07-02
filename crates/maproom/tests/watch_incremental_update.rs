//! R06-R08 regression tests: the watch/incremental pipeline performs REAL
//! indexing work, and `index_state.tree_sha` never advances without it.
//!
//! Fix spec: _SPECS/crewchief/research/maproom-cli-e2e-fix-spec.md §3.4.
//! - R06: watch indexes UNCOMMITTED edits (live events bypass the HEAD^{tree} gate)
//! - R07: no phantom "Incremental update complete" with zero rows written
//! - R08: tree_sha is never stamped without verified work (init included)
//!
//! Run serialized: `cargo test --test watch_incremental_update -- --test-threads=1`

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use maproom::db::traits::{StoreChunks, StoreCore, StoreIndexState, StoreMigration};
use maproom::db::{SqliteStore, UpdateStats};
use maproom::git::{get_git_tree_sha, get_head_commit};
use maproom::incremental::{handle_file_event, incremental_update, EventType, IndexingEvent};
use maproom::indexer::upsert_files;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("git spawn");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Create a temp git repo with one committed TypeScript file.
fn make_repo(dir: &Path) {
    git(dir, &["init", "-q"]);
    std::fs::write(
        dir.join("a.ts"),
        "export function alphaOne() { return 1; }\n",
    )
    .unwrap();
    git(dir, &["add", "a.ts"]);
    git(dir, &["commit", "-qm", "init"]);
}

async fn store_at(db_dir: &Path) -> SqliteStore {
    let url = format!("{}/maproom.db", db_dir.display());
    let store = SqliteStore::connect(&url).await.expect("store connect");
    store.migrate().await.expect("migrate");
    store
}

/// Seed the index for `repo_dir` via the same path the binary uses, and
/// return (store, worktree_id).
async fn seed(store: &SqliteStore, repo_dir: &Path) -> i64 {
    let commit = get_head_commit(repo_dir).unwrap();
    upsert_files(
        store,
        "fx",
        "main",
        repo_dir,
        &commit,
        &[PathBuf::from("a.ts")],
    )
    .await
    .expect("seed upsert");
    let repo_id = store
        .get_or_create_repo("fx", repo_dir.to_string_lossy().as_ref())
        .await
        .unwrap();
    store
        .get_or_create_worktree(repo_id, "main", repo_dir.to_string_lossy().as_ref())
        .await
        .unwrap()
}

async fn worktree_relpaths(store: &SqliteStore, worktree_id: i64) -> Vec<String> {
    // (chunk_id, relpath) pairs -> distinct relpaths
    let mut rels: Vec<String> = store
        .get_chunks_for_worktree(worktree_id)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, rel)| rel)
        .collect();
    rels.sort();
    rels.dedup();
    rels
}

fn modified_event(path: PathBuf) -> IndexingEvent {
    IndexingEvent {
        worktree_id: "main".to_string(),
        path,
        event_type: EventType::Modified,
        timestamp: SystemTime::now(),
        old_path: None,
    }
}

#[tokio::test]
async fn handle_file_event_modified_upserts_chunks() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    make_repo(repo.path());
    let store = store_at(db.path()).await;
    let wt = seed(&store, repo.path()).await;

    let before = store.get_chunks_for_worktree(wt).await.unwrap().len();

    // UNCOMMITTED edit adding a new function (the exact R06 scenario).
    std::fs::write(
        repo.path().join("a.ts"),
        "export function alphaOne() { return 1; }\nexport function betaTwo() { return 2; }\n",
    )
    .unwrap();

    let stats = handle_file_event(
        &store,
        wt,
        "fx",
        "main",
        repo.path(),
        &modified_event(repo.path().join("a.ts")),
    )
    .await
    .expect("handle_file_event");

    assert_eq!(stats.files_processed, 1, "one file submitted");
    let after = store.get_chunks_for_worktree(wt).await.unwrap().len();
    assert!(
        after > before,
        "real chunk rows must be written (before={before}, after={after}) — the R07 phantom wrote zero"
    );
}

#[tokio::test]
async fn handle_file_event_deleted_removes_mapping() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    make_repo(repo.path());
    let store = store_at(db.path()).await;
    let wt = seed(&store, repo.path()).await;
    assert!(worktree_relpaths(&store, wt).await.contains(&"a.ts".to_string()));

    std::fs::remove_file(repo.path().join("a.ts")).unwrap();
    let mut ev = modified_event(repo.path().join("a.ts"));
    ev.event_type = EventType::Deleted;

    let stats = handle_file_event(&store, wt, "fx", "main", repo.path(), &ev)
        .await
        .expect("handle_file_event deleted");
    assert_eq!(stats.files_processed, 1);
    assert!(
        !worktree_relpaths(&store, wt).await.contains(&"a.ts".to_string()),
        "deleted file's chunks must be unmapped from the worktree"
    );
}

#[tokio::test]
async fn handle_file_event_renamed_moves_mapping() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    make_repo(repo.path());
    let store = store_at(db.path()).await;
    let wt = seed(&store, repo.path()).await;

    std::fs::rename(repo.path().join("a.ts"), repo.path().join("b.ts")).unwrap();
    let ev = IndexingEvent {
        worktree_id: "main".to_string(),
        path: repo.path().join("b.ts"),
        event_type: EventType::Renamed,
        timestamp: SystemTime::now(),
        old_path: Some(repo.path().join("a.ts")),
    };

    let stats = handle_file_event(&store, wt, "fx", "main", repo.path(), &ev)
        .await
        .expect("handle_file_event renamed");
    assert_eq!(stats.files_processed, 2, "one removal + one upsert");
    let rels = worktree_relpaths(&store, wt).await;
    assert!(!rels.contains(&"a.ts".to_string()), "old mapping gone");
    assert!(rels.contains(&"b.ts".to_string()), "new mapping present");
}

#[tokio::test]
async fn incremental_update_indexes_added_files() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    make_repo(repo.path());
    let store = store_at(db.path()).await;
    let wt = seed(&store, repo.path()).await;

    // Establish index_state at commit A's tree.
    let tree_a = get_git_tree_sha(repo.path()).unwrap();
    store
        .update_index_state(wt, &tree_a, &UpdateStats::default())
        .await
        .unwrap();

    // Commit B adds a new file.
    std::fs::write(
        repo.path().join("c.ts"),
        "export function gammaThree() { return 3; }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "c.ts"]);
    git(repo.path(), &["commit", "-qm", "add c"]);
    let tree_b = get_git_tree_sha(repo.path()).unwrap();

    let stats = incremental_update(&store, wt, "fx", "main", repo.path())
        .await
        .expect("incremental_update");

    assert!(stats.files_processed >= 1, "diff entry processed");
    assert!(
        worktree_relpaths(&store, wt).await.contains(&"c.ts".to_string()),
        "commit-diff file must be REALLY indexed (R07: the old stub wrote zero rows)"
    );
    assert_eq!(
        store.get_last_indexed_tree(wt).await.unwrap(),
        tree_b,
        "tree_sha advances after verified work"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn incremental_update_no_advance_on_unreadable_file() {
    use std::os::unix::fs::PermissionsExt;

    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    make_repo(repo.path());
    let store = store_at(db.path()).await;
    let wt = seed(&store, repo.path()).await;

    let tree_a = get_git_tree_sha(repo.path()).unwrap();
    store
        .update_index_state(wt, &tree_a, &UpdateStats::default())
        .await
        .unwrap();

    // Commit B modifies a.ts, then the working-tree copy becomes unreadable.
    std::fs::write(
        repo.path().join("a.ts"),
        "export function alphaOne() { return 11; }\n",
    )
    .unwrap();
    git(repo.path(), &["add", "a.ts"]);
    git(repo.path(), &["commit", "-qm", "edit"]);
    std::fs::set_permissions(
        repo.path().join("a.ts"),
        std::fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    let res = incremental_update(&store, wt, "fx", "main", repo.path()).await;
    // Restore permissions before asserting (so TempDir cleanup works everywhere).
    std::fs::set_permissions(
        repo.path().join("a.ts"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    assert!(res.is_err(), "unreadable diff entry must be a hard error (R-WATCH-8)");
    assert_eq!(
        store.get_last_indexed_tree(wt).await.unwrap(),
        tree_a,
        "tree_sha MUST NOT advance when a diff entry could not be submitted (R08 invariant)"
    );
}

#[tokio::test]
async fn init_state_does_not_stamp_tree_sha() {
    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    make_repo(repo.path());
    let store = store_at(db.path()).await;

    // Fresh worktree with NO index_state and NO seed — first index is scan's job.
    let repo_id = store
        .get_or_create_repo("fx", repo.path().to_string_lossy().as_ref())
        .await
        .unwrap();
    let wt = store
        .get_or_create_worktree(repo_id, "main", repo.path().to_string_lossy().as_ref())
        .await
        .unwrap();

    let stats = incremental_update(&store, wt, "fx", "main", repo.path())
        .await
        .expect("init-state incremental_update is a no-op, not an error");
    assert_eq!(stats.files_processed, 0);
    assert_eq!(
        store.get_last_indexed_tree(wt).await.unwrap(),
        "init",
        "the init branch must NOT stamp tree_sha with zero work (R08: the old fall-through poisoned the next scan)"
    );
}

/// THE WIRING GUARD (R-WATCH-3): spawns the actual `maproom watch` binary and
/// asserts an UNCOMMITTED edit becomes searchable. This is the only test that
/// would catch a revert of the single watch-loop call-site swap back to the
/// tree-SHA-gated path (the exact shape of the original regression).
#[test]
fn watch_binary_indexes_uncommitted_edit() {
    let binary = {
        let mut p = std::env::current_exe().unwrap();
        p.pop(); // test binary name
        p.pop(); // deps/
        p.push("maproom");
        assert!(p.exists(), "maproom binary not built at {}", p.display());
        p
    };

    let repo = tempfile::TempDir::new().unwrap();
    let db = tempfile::TempDir::new().unwrap();
    make_repo(repo.path());
    let db_url = format!("sqlite://{}/w.db", db.path().display());

    // Scan baseline through the binary.
    let out = Command::new(&binary)
        .args(["scan", "--repo", "fx", "--path"])
        .arg(repo.path())
        .env("MAPROOM_DATABASE_URL", &db_url)
        .output()
        .unwrap();
    assert!(out.status.success(), "scan failed: {}", String::from_utf8_lossy(&out.stderr));

    // UNCOMMITTED edit.
    std::fs::write(
        repo.path().join("a.ts"),
        "export function alphaOne() { return 1; }\nexport function betaTwo() { return 2; }\n",
    )
    .unwrap();

    // Run watch long enough for the 3s poller to deliver + index the event.
    let mut watch = Command::new(&binary)
        .args(["watch", "--repo", "fx", "--json", "--path"])
        .arg(repo.path())
        .env("MAPROOM_DATABASE_URL", &db_url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(12));
    let _ = watch.kill();
    let _ = watch.wait();

    // The uncommitted symbol must now be searchable.
    let out = Command::new(&binary)
        .args([
            "search", "--repo", "fx", "--query", "betaTwo", "--format", "agent",
        ])
        .env("MAPROOM_DATABASE_URL", &db_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("betaTwo"),
        "uncommitted edit was not indexed by watch (R06); search output:\n{stdout}"
    );
}
