//! Cross-backend parity harness (spec §9.4): asserts `SqliteStore` and
//! `PostgresStore` are equivalent through the backend-agnostic
//! `Arc<dyn Store + Send + Sync>` handle.
//!
//! Each `Scenario` from §7 is a `check_*(store: &(dyn Store + Send + Sync))`
//! function run against EVERY backend in `backends()`:
//!   - `sqlite` (`:memory:`) — always present.
//!   - `postgres` — only when built `--features postgres` AND `MAPROOM_TEST_PG_URL`
//!     is set (otherwise skipped with a notice).
//!
//! Run command (matches §11 DoD):
//!   MAPROOM_TEST_PG_URL=postgres://maproom:maproom@localhost:5432/maproom_test \
//!     cargo test -p maproom --features postgres --test store_parity \
//!     -- --ignored --test-threads=1
//!
//! State isolation without a schema reset (so the test crate needs no `sqlx`
//! dev-dep, which would otherwise leak into `cargo tree`): every seeded entity is
//! namespaced by a process-unique token, and GLOBAL counters
//! (`get_global_chunk_count` / `get_global_embedding_count`) are asserted as
//! DELTAS around the seeding, so a shared (non-reset) Postgres database and
//! repeated runs stay correct. The Postgres-only degraded-vector branch
//! (R-SEARCH-5 option b) is covered by the in-crate `db::postgres::tests` suite,
//! not here (it needs the concrete `PostgresStore`).
#![allow(clippy::bool_assert_comparison)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// `&dyn Store` exposes every sub-trait method with only `Store` in scope; the one
// concrete call is `SqliteStore::migrate()`, which needs `StoreMigration`.
use maproom::db::index_state::UpdateStats;
use maproom::db::sqlite::SqliteStore;
use maproom::db::traits::StoreMigration;
use maproom::db::types::ImportDirection;
use maproom::db::{ChunkRecord, FileRecord, Store};

/// Process-unique base so names never collide across checks or re-runs.
fn unique_base() -> u64 {
    static SEED: AtomicU64 = AtomicU64::new(0);
    if SEED.load(Ordering::Relaxed) == 0 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        SEED.store(nanos, Ordering::Relaxed);
    }
    SEED.fetch_add(1, Ordering::Relaxed)
}

/// Build the backend vector: sqlite always, postgres when feature+env present.
async fn backends() -> Vec<(&'static str, Arc<dyn Store + Send + Sync>)> {
    let mut v: Vec<(&'static str, Arc<dyn Store + Send + Sync>)> = Vec::new();

    // Shared-cache in-memory URL (not plain ":memory:"): SqliteStore pools
    // connections via r2d2, and plain `:memory:` gives EACH pooled connection its
    // own independent database — so the connection that ran migrations (and built
    // the `vec_code_*` virtual tables) differs from the one a later query lands on,
    // surfacing as "no such table: vec_code_768". `mode=memory&cache=shared` with a
    // unique name makes every pooled connection share one isolated DB (the same
    // pattern the in-crate SQLite tests use).
    let mem = format!(
        "file:memdb_parity_{}?mode=memory&cache=shared",
        unique_base()
    );
    let sqlite = SqliteStore::connect(&mem).await.expect("sqlite connect");
    sqlite.migrate().await.expect("sqlite migrate");
    v.push(("sqlite", Arc::new(sqlite)));

    #[cfg(feature = "postgres")]
    {
        if let Ok(url) = std::env::var("MAPROOM_TEST_PG_URL") {
            let pg = maproom::db::postgres::PostgresStore::connect(&url)
                .await
                .expect("postgres connect");
            v.push(("postgres", Arc::new(pg)));
        } else {
            eprintln!("store_parity: MAPROOM_TEST_PG_URL unset — postgres backend skipped");
        }
    }
    v
}

/// Run one check against every backend, prefixing panics with the backend name.
async fn for_each<F, Fut>(f: F)
where
    F: Fn(&'static str, Arc<dyn Store + Send + Sync>) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    for (name, store) in backends().await {
        eprintln!("  [{name}] running parity check");
        f(name, store).await;
    }
}

/// Seed a repo/worktree/commit/file with run-unique names; returns the ids.
struct Seed {
    repo: i64,
    wt: i64,
    #[allow(dead_code)]
    commit: i64,
    file: i64,
    repo_name: String,
}

async fn seed(store: &(dyn Store + Send + Sync), tag: &str) -> Seed {
    let b = unique_base();
    let repo_name = format!("acme/{tag}-{b}");
    let repo = store
        .get_or_create_repo(&repo_name, &format!("/src/{tag}-{b}"))
        .await
        .unwrap();
    let wt = store
        .get_or_create_worktree(repo, "main", &format!("/wt/{tag}-{b}"))
        .await
        .unwrap();
    let commit = store
        .get_or_create_commit(repo, &format!("sha-{b}"), None)
        .await
        .unwrap();
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wt,
            commit_id: commit,
            relpath: format!("src/{tag}_{b}.rs"),
            language: Some("rust".to_string()),
            content_hash: format!("h-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    Seed {
        repo,
        wt,
        commit,
        file,
        repo_name,
    }
}

fn chunk(file: i64, wt: i64, blob: &str, sym: &str, ts: &str, s: i32, e: i32) -> ChunkRecord {
    ChunkRecord {
        file_id: file,
        blob_sha: blob.to_string(),
        symbol_name: Some(sym.to_string()),
        kind: "function".to_string(),
        signature: None,
        docstring: None,
        start_line: s,
        end_line: e,
        preview: ts.to_string(),
        ts_doc_text: ts.to_string(),
        recency_score: 1.0,
        churn_score: 0.0,
        metadata: None,
        worktree_id: wt,
    }
}

// ── Exact-equality scenarios (§9.4) ─────────────────────────────────────────

async fn check_core_idempotency(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let name = format!("acme/idem-{b}");
    let repo = store.get_or_create_repo(&name, "/p").await.unwrap();
    assert_eq!(repo, store.get_or_create_repo(&name, "/p").await.unwrap());
    let w1 = store
        .get_or_create_worktree(repo, "main", "/wt/m")
        .await
        .unwrap();
    assert_eq!(
        w1,
        store
            .get_or_create_worktree(repo, "main", "/wt/m")
            .await
            .unwrap()
    );
    let _w2 = store
        .get_or_create_worktree(repo, "feat", "/wt/f")
        .await
        .unwrap();
    let c = store.get_or_create_commit(repo, "abc", None).await.unwrap();
    assert_eq!(
        c,
        store.get_or_create_commit(repo, "abc", None).await.unwrap()
    );
    assert_eq!(store.list_worktrees(repo).await.unwrap().len(), 2);
}

async fn check_distinct_chunk_count(store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "dcc").await;
    let before = store.get_global_chunk_count().await.unwrap();
    let b = unique_base();
    let (b1, b2) = (format!("DCCA{b}"), format!("DCCB{b}"));
    // Two distinct blob_sha -> +2; a third chunk re-using b1 -> still +2 (DISTINCT).
    store
        .insert_chunk(&chunk(s.file, s.wt, &b1, "f1", "one", 1, 5))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(s.file, s.wt, &b2, "f2", "two", 6, 10))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(s.file, s.wt, &b1, "f3", "three", 11, 15))
        .await
        .unwrap();
    let after = store.get_global_chunk_count().await.unwrap();
    assert_eq!(after - before, 2, "DISTINCT blob_sha delta");
}

async fn check_chunk_dedup_and_worktree_mapping(store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "map").await;
    let b = unique_base();
    let blob = format!("MAP{b}");
    let c1 = store
        .insert_chunk(&chunk(s.file, s.wt, &blob, "x", "x", 1, 5))
        .await
        .unwrap();
    // Same (file,start,end) coords dedup to the same id, even with a new blob.
    let c1b = store
        .insert_chunk(&chunk(s.file, s.wt, &format!("{blob}b"), "x", "x", 1, 5))
        .await
        .unwrap();
    assert_eq!(c1, c1b, "insert_chunk dedups on (file,start,end)");

    let w2 = store
        .get_or_create_worktree(s.repo, "feat", "/wt/feat2")
        .await
        .unwrap();
    // add_chunk_to_worktree is idempotent.
    store.add_chunk_to_worktree(c1, w2).await.unwrap();
    store.add_chunk_to_worktree(c1, w2).await.unwrap();
    let mut wts = store.get_chunk_worktrees(c1).await.unwrap();
    wts.sort_unstable();
    let mut expect = vec![s.wt, w2];
    expect.sort_unstable();
    assert_eq!(wts, expect, "chunk mapped to both worktrees, no dup");

    // get_chunk_context: neighbor from same file, target excluded; None for missing.
    store
        .insert_chunk(&chunk(s.file, s.wt, &format!("{blob}n"), "n", "n", 6, 10))
        .await
        .unwrap();
    let ctx = store
        .get_chunk_context(c1, 1)
        .await
        .unwrap()
        .expect("context");
    assert_eq!(ctx.chunk.id, c1);
    assert!(ctx.surrounding_chunks.iter().all(|x| x.id != c1));
    assert!(store
        .get_chunk_context(99_999_999, 1)
        .await
        .unwrap()
        .is_none());
}

async fn check_embeddings(store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "emb").await;
    let b = unique_base();
    let blob = format!("EMB{b}");
    store
        .insert_chunk(&chunk(s.file, s.wt, &blob, "e", "e", 1, 5))
        .await
        .unwrap();

    let v: Vec<f32> = (0..768).map(|i| (i as f32) / 1000.0).collect();
    let e1 = store.upsert_embedding(&blob, &v, "ollama").await.unwrap();
    let e2 = store.upsert_embedding(&blob, &v, "ollama").await.unwrap();
    assert_eq!(e1, e2, "upsert_embedding stable id");
    assert!(store.has_embedding(&blob).await.unwrap());
    assert!(!store.has_embedding(&format!("NOPE{b}")).await.unwrap());

    let got = store.get_embedding(&blob).await.unwrap().expect("present");
    assert_eq!(got.len(), 768);
    for (a, c) in v.iter().zip(got.iter()) {
        assert!((a - c).abs() <= 1e-5, "round-trip {a} vs {c}");
    }

    // dim validation: 512 errors, listing supported dims.
    let bad: Vec<f32> = vec![0.1; 512];
    let msg = store
        .upsert_embedding(&format!("BAD{b}"), &bad, "ollama")
        .await
        .unwrap_err()
        .to_string();
    assert!(
        msg.contains("512") && msg.contains("768") && msg.contains("1536"),
        "dim err: {msg}"
    );

    // incremental excludes already-embedded blob; non-incremental returns >= incremental.
    let inc = store
        .fetch_chunks_needing_embeddings(true, None)
        .await
        .unwrap();
    assert!(
        inc.iter().all(|c| c.blob_sha != blob),
        "incremental excludes embedded"
    );
    let all = store
        .fetch_chunks_needing_embeddings(false, None)
        .await
        .unwrap();
    assert!(all.len() >= inc.len(), "non-incremental returns all");
}

async fn check_index_state(store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "idx").await;
    assert_eq!(
        store.get_last_indexed_tree(s.wt).await.unwrap(),
        "init",
        "init default"
    );
    store
        .update_index_state(
            s.wt,
            "treesha-xyz",
            &UpdateStats {
                files_processed: 1,
                chunks_processed: 2,
                embeddings_generated: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        store.get_last_indexed_tree(s.wt).await.unwrap(),
        "treesha-xyz"
    );
}

fn looks_like_ts(s: &str) -> bool {
    let b = s.as_bytes();
    s.len() == 19
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b' '
        && b[13] == b':'
        && b[16] == b':'
        && s.char_indices()
            .all(|(i, c)| matches!(i, 4 | 7 | 10 | 13 | 16) || c.is_ascii_digit())
}

async fn check_encoding_lifecycle(store: &(dyn Store + Send + Sync)) {
    let run = store
        .create_encoding_run(100, Some("ollama"), Some(768))
        .await
        .unwrap();
    let active = store
        .get_active_encoding_run()
        .await
        .unwrap()
        .expect("active");
    assert_eq!(active.id, run);
    assert_eq!(active.status, "running");
    assert_eq!(active.total_chunks, 100);
    assert_eq!(active.provider.as_deref(), Some("ollama"));
    assert_eq!(active.dimension, Some(768));
    assert!(
        looks_like_ts(&active.started_at),
        "started_at: {}",
        active.started_at
    );
    assert!(active.finished_at.is_none());
    store
        .update_encoding_run_progress(run, 42, Some(3.5))
        .await
        .unwrap();
    let active = store.get_active_encoding_run().await.unwrap().unwrap();
    assert_eq!(active.chunks_completed, 42);
    store.complete_encoding_run(run, "completed").await.unwrap();
    // get_active returns the most-recent running run; ensure THIS one is no longer it.
    let still = store.get_active_encoding_run().await.unwrap();
    assert!(
        still.map(|r| r.id != run).unwrap_or(true),
        "completed run not active"
    );
}

async fn check_cleanup_orphan_gc(name: &str, store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "cln").await;
    let b = unique_base();
    let wb = store
        .get_or_create_worktree(s.repo, "B", &format!("/wt/B-{b}"))
        .await
        .unwrap();
    let shared = store
        .insert_chunk(&chunk(s.file, s.wt, &format!("SH{b}"), "sh", "sh", 1, 5))
        .await
        .unwrap();
    store.add_chunk_to_worktree(shared, wb).await.unwrap();
    let only_a = store
        .insert_chunk(&chunk(s.file, s.wt, &format!("OA{b}"), "oa", "oa", 6, 10))
        .await
        .unwrap();
    store
        .upsert_embedding(&format!("SH{b}"), &vec![0.01; 768], "m")
        .await
        .unwrap();
    let emb_before = store.get_global_embedding_count().await.unwrap();

    let res = store.delete_worktree_data(s.wt).await.unwrap();
    // Shared parity (both backends): the worktree-A-only orphan chunk is deleted
    // and the result reports a non-zero chunk count.
    assert!(res.chunks_deleted >= 1, "orphan chunk removed");
    assert!(
        store.get_chunk_by_id(only_a).await.unwrap().is_none(),
        "orphan GC'd"
    );

    // Junction-survival is a BOTH-backend property since review [07]: GC is
    // by the `chunk_worktrees` junction, not file ownership (R-WT-1/R-WT-4),
    // so a chunk still mapped to another worktree survives — and on SQLite
    // its shared blob's embedding survives too (only last-reference blobs
    // are GC'd, for vec_code/ANN consistency).
    assert_eq!(
        store.get_chunk_worktrees(shared).await.unwrap(),
        vec![wb],
        "shared kept in B"
    );
    assert_eq!(
        res.embeddings_deleted, 0,
        "shared blob's embedding must survive"
    );
    // Pool persistence beyond live references is the PG-only divergence
    // (persistent content-addressed pool, R-WT-4; SQLite GCs unreferenced
    // blobs deliberately).
    if name == "postgres" {
        assert_eq!(
            store.get_global_embedding_count().await.unwrap(),
            emb_before,
            "embedding pool persists"
        );
    }
}

async fn check_stale_detection(store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "stale").await;
    let b = unique_base();
    let missing = store
        .get_or_create_worktree(s.repo, "missing", &format!("/nonexistent/zz-{b}"))
        .await
        .unwrap();
    let exists = store
        .get_or_create_worktree(s.repo, "exists", "/tmp")
        .await
        .unwrap();
    let stale = store.detect_stale_worktrees().await.unwrap();
    let ids: Vec<i64> = stale.iter().map(|x| x.id).collect();
    assert!(ids.contains(&missing), "missing path -> stale");
    assert!(!ids.contains(&exists), "/tmp exists -> not stale");
    assert!(stale.iter().all(|x| !x.exists));
}

/// Regression coverage for divergences fixed after the §7 review: methods whose
/// SQL previously differed between backends (ordering, scoping, count metric) now
/// produce identical, engineered-to-be-unambiguous results on both. Fixtures are
/// built so the OLD (divergent) behavior would have failed these assertions.
async fn check_method_parity_regression(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();

    // find_chunk_by_symbol: two chunks share a symbol -> highest id (ORDER BY id
    // DESC), scoped by file ownership. (PG previously returned the LOWEST id.)
    let s = seed(store, "reg").await;
    let dup = format!("DUP{b}");
    let _first = store
        .insert_chunk(&chunk(s.file, s.wt, &format!("{b}fa"), &dup, "a", 1, 5))
        .await
        .unwrap();
    let second = store
        .insert_chunk(&chunk(s.file, s.wt, &format!("{b}fb"), &dup, "b", 6, 10))
        .await
        .unwrap();
    assert_eq!(
        store
            .find_chunk_by_symbol(s.repo, Some(s.wt), &dup, None)
            .await
            .unwrap(),
        Some(second),
        "find_chunk_by_symbol -> highest id (DESC), file-ownership scoped",
    );

    // get_chunk_context: `surrounding` neighbours on EACH side by start line (NOT
    // nearest-by-line-distance). Lines [1,10,11,12], target=10, surrounding=1 ->
    // per-side window {1, 11}; nearest-by-distance would give {11, 12}.
    let cs = seed(store, "ctx").await;
    let cb = format!("CTX{b}");
    store
        .insert_chunk(&chunk(cs.file, cs.wt, &format!("{cb}1"), "c1", "c1", 1, 4))
        .await
        .unwrap();
    let target = store
        .insert_chunk(&chunk(
            cs.file,
            cs.wt,
            &format!("{cb}2"),
            "c2",
            "c2",
            10,
            10,
        ))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(
            cs.file,
            cs.wt,
            &format!("{cb}3"),
            "c3",
            "c3",
            11,
            11,
        ))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(
            cs.file,
            cs.wt,
            &format!("{cb}4"),
            "c4",
            "c4",
            12,
            12,
        ))
        .await
        .unwrap();
    let ctx = store
        .get_chunk_context(target, 1)
        .await
        .unwrap()
        .expect("ctx");
    let mut lines: Vec<i32> = ctx
        .surrounding_chunks
        .iter()
        .map(|c| c.start_line)
        .collect();
    lines.sort_unstable();
    assert_eq!(
        lines,
        vec![1, 11],
        "get_chunk_context: per-side neighbours by start line"
    );

    // get_chunks_by_blob_sha: ORDER BY start_line (NOT id). Insert higher line
    // first (lower id), then lower line (higher id) -> ascending by start_line.
    let sh = format!("SBS{b}");
    store
        .insert_chunk(&chunk(cs.file, cs.wt, &sh, "hi", "hi", 200, 205))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(cs.file, cs.wt, &sh, "lo", "lo", 20, 25))
        .await
        .unwrap();
    let order: Vec<i32> = store
        .get_chunks_by_blob_sha(&sh)
        .await
        .unwrap()
        .iter()
        .map(|c| c.start_line)
        .collect();
    assert_eq!(
        order,
        vec![20, 200],
        "get_chunks_by_blob_sha ordered by start_line"
    );

    // get_repo_chunk_count: DISTINCT blob_sha via the chunk_worktrees junction,
    // resolvable by exact name AND escaped '%/name' suffix. (PG previously counted
    // DISTINCT c.id via files.) 2 distinct blobs (one reused) -> 2.
    let rc = seed(store, "rcc").await;
    let (rb1, rb2) = (format!("RC1{b}"), format!("RC2{b}"));
    store
        .insert_chunk(&chunk(rc.file, rc.wt, &rb1, "r1", "r1", 1, 5))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(rc.file, rc.wt, &rb2, "r2", "r2", 6, 10))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(rc.file, rc.wt, &rb1, "r3", "r3", 11, 15))
        .await
        .unwrap();
    assert_eq!(
        store.get_repo_chunk_count(&rc.repo_name).await.unwrap(),
        2,
        "get_repo_chunk_count (exact name) = DISTINCT blob_sha",
    );
    let suffix = rc.repo_name.split('/').next_back().unwrap().to_string();
    assert_eq!(
        store.get_repo_chunk_count(&suffix).await.unwrap(),
        2,
        "get_repo_chunk_count (suffix match)",
    );

    // get_worktree_embedding_count: DISTINCT chunk_id (NOT blob_sha) -> two chunks
    // sharing one embedded blob count as 2. (PG previously counted DISTINCT blob_sha.)
    let we = seed(store, "wec").await;
    let eb = format!("WEC{b}");
    store
        .insert_chunk(&chunk(we.file, we.wt, &eb, "e1", "e1", 1, 5))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(we.file, we.wt, &eb, "e2", "e2", 6, 10))
        .await
        .unwrap();
    store
        .upsert_embedding(&eb, &vec![0.01f32; 768], "m")
        .await
        .unwrap();
    assert_eq!(
        store.get_worktree_embedding_count(we.wt).await.unwrap(),
        2,
        "get_worktree_embedding_count = DISTINCT chunk_id",
    );

    // A non-finite (NaN) embedding of a valid dimension is rejected on BOTH
    // backends (pgvector rejects it; a NaN also poisons distance ordering).
    let mut nan = vec![0.0f32; 768];
    nan[3] = f32::NAN;
    assert!(
        store
            .upsert_embedding(&format!("NAN{b}"), &nan, "m")
            .await
            .is_err(),
        "non-finite embedding rejected on write",
    );
}

// ── Driver tests (one per scenario; each gets fresh backends) ────────────────

#[tokio::test]
#[ignore]
async fn parity_core_idempotency() {
    for_each(|_n, s| async move { check_core_idempotency(s.as_ref()).await }).await;
}

/// F34: get_chunks_by_ids is a real batched fetch on BOTH backends —
/// found rows carry full enrichment, missing ids are simply absent.
async fn check_get_chunks_by_ids(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "byids").await;
    let c1 = store
        .insert_chunk(&chunk(
            s.file,
            s.wt,
            &format!("BI1{b}"),
            "byids_one",
            "one",
            1,
            4,
        ))
        .await
        .unwrap();
    let c2 = store
        .insert_chunk(&chunk(
            s.file,
            s.wt,
            &format!("BI2{b}"),
            "byids_two",
            "two",
            6,
            9,
        ))
        .await
        .unwrap();

    let got = store
        .get_chunks_by_ids(&[c1, c2, 987_654_321])
        .await
        .unwrap();
    assert_eq!(got.len(), 2, "two found, phantom id absent: {got:?}");
    let one = got.iter().find(|c| c.id == c1).expect("c1 present");
    assert_eq!(one.symbol_name.as_deref(), Some("byids_one"));
    assert!(
        one.file_path.contains("src/"),
        "relpath joined: {}",
        one.file_path
    );
    assert!(!one.preview.is_empty());
    assert!(store.get_chunks_by_ids(&[]).await.unwrap().is_empty());
}

/// F15: an unknown REPO is an error carrying StoreError on BOTH backends
/// (PG used to silently return the empty set — indistinguishable from a
/// healthy zero-hit query).
async fn check_search_unknown_repo_errors(store: &(dyn Store + Send + Sync)) {
    let err = store
        .search_chunks_fts("no-such-repo-zz", None, "anything", 5, false, None, None)
        .await
        .expect_err("unknown repo must error, not return empty");
    assert!(
        err.downcast_ref::<maproom::db::StoreError>().is_some(),
        "error must be the typed StoreError, got: {err:#}"
    );
    assert!(err.to_string().contains("Repository not found"), "{err}");

    let verr = store
        .search_chunks_vector(
            "no-such-repo-zz",
            None,
            &vec![0.01f32; 768],
            5,
            false,
            None,
            None,
        )
        .await
        .expect_err("vector: unknown repo must error");
    assert!(verr.to_string().contains("Repository not found"), "{verr}");
}

/// F76: per-dimension embedded-chunk breakdown, ordered by dim, on BOTH
/// backends. Two chunks at 768 + one at 1024 -> [(768,2),(1024,1)].
async fn check_embedding_dim_breakdown(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "dims").await;
    for (i, dim) in [(0usize, 768usize), (1, 768), (2, 1024)] {
        let blob = format!("DIM{i}{b}");
        store
            .insert_chunk(&chunk(
                s.file,
                s.wt,
                &blob,
                &format!("dim_fn{i}"),
                "d",
                (i as i32) * 10 + 1,
                (i as i32) * 10 + 5,
            ))
            .await
            .unwrap();
        store
            .upsert_embedding(&blob, &vec![0.01f32; dim], "m")
            .await
            .unwrap();
    }
    let breakdown = store
        .get_worktree_embedding_dim_breakdown(s.wt)
        .await
        .unwrap();
    assert_eq!(
        breakdown,
        vec![(768, 2), (1024, 1)],
        "ordered per-dim counts"
    );
}

#[tokio::test]
#[ignore]
async fn parity_embedding_dim_breakdown() {
    for_each(|_n, s| async move { check_embedding_dim_breakdown(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_search_unknown_repo_errors() {
    for_each(|_n, s| async move { check_search_unknown_repo_errors(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_get_chunks_by_ids() {
    for_each(|_n, s| async move { check_get_chunks_by_ids(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_method_regression() {
    for_each(|_n, s| async move { check_method_parity_regression(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_distinct_chunk_count() {
    for_each(|_n, s| async move { check_distinct_chunk_count(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_chunk_dedup_worktree_mapping() {
    for_each(|_n, s| async move { check_chunk_dedup_and_worktree_mapping(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_embeddings() {
    for_each(|_n, s| async move { check_embeddings(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_index_state() {
    for_each(|_n, s| async move { check_index_state(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_encoding_lifecycle() {
    for_each(|_n, s| async move { check_encoding_lifecycle(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_cleanup_orphan_gc() {
    for_each(|n, s| async move { check_cleanup_orphan_gc(n, s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_stale_detection() {
    for_each(|_n, s| async move { check_stale_detection(s.as_ref()).await }).await;
}

// ── Search parity (§6.6) ────────────────────────────────────────────────────

async fn check_fts(store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "fts").await;
    let b = unique_base();
    let mk = |sym: &str, kind: &str, ts: &str, n: i32, blob: &str| ChunkRecord {
        file_id: s.file,
        blob_sha: blob.to_string(),
        symbol_name: Some(sym.to_string()),
        kind: kind.to_string(),
        signature: None,
        docstring: None,
        start_line: n,
        end_line: n + 4,
        preview: ts.to_string(),
        ts_doc_text: ts.to_string(),
        recency_score: 1.0,
        churn_score: 0.0,
        metadata: None,
        worktree_id: s.wt,
    };
    store
        .insert_chunk(&mk(
            "validateProvider",
            "function",
            "validate provider authentication login",
            1,
            &format!("F1{b}"),
        ))
        .await
        .unwrap();
    store
        .insert_chunk(&mk(
            "parseConfig",
            "function",
            "parse config yaml settings",
            6,
            &format!("F2{b}"),
        ))
        .await
        .unwrap();
    store
        .insert_chunk(&mk(
            "AuthService",
            "class",
            "authentication service handler",
            11,
            &format!("F3{b}"),
        ))
        .await
        .unwrap();

    // "authentication" matches the two auth chunks; ASCII fixture -> match-set equality.
    let (hits, total) = store
        .search_chunks_fts(
            &s.repo_name,
            Some("main"),
            "authentication",
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(total, 2, "total_count of matches");
    let mut syms: Vec<String> = hits.iter().filter_map(|h| h.symbol_name.clone()).collect();
    syms.sort();
    assert_eq!(
        syms,
        vec!["AuthService".to_string(), "validateProvider".to_string()]
    );

    // kind_filter restricts to the class; Some(&[]) behaves like None.
    let (cls, ctotal) = store
        .search_chunks_fts(
            &s.repo_name,
            Some("main"),
            "authentication",
            10,
            false,
            Some(&["class".to_string()]),
            None,
        )
        .await
        .unwrap();
    assert_eq!(ctotal, 1);
    assert_eq!(cls.len(), 1);
    assert_eq!(cls[0].symbol_name.as_deref(), Some("AuthService"));
    let (empty_kf, _) = store
        .search_chunks_fts(
            &s.repo_name,
            Some("main"),
            "authentication",
            10,
            false,
            Some(&[]),
            None,
        )
        .await
        .unwrap();
    assert_eq!(empty_kf.len(), 2, "Some(&[]) == None");

    // Empty / all-special query -> Ok(empty), never error.
    let (none_hits, none_total) = store
        .search_chunks_fts(&s.repo_name, Some("main"), "   ", 10, false, None, None)
        .await
        .unwrap();
    assert!(none_hits.is_empty() && none_total == 0);

    // OR-prefix multi-term: "authentication login" still matches validateProvider.
    let (multi, _) = store
        .search_chunks_fts(
            &s.repo_name,
            Some("main"),
            "authentication login",
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(multi
        .iter()
        .any(|h| h.symbol_name.as_deref() == Some("validateProvider")));
}

/// Insert N chunks whose embeddings sit at strictly increasing L2 distance from a
/// query vector, plus matching FTS text. Returns (repo_name, ordered chunk ids,
/// query vector). chunk[i] is the i-th closest (i=0 nearest).
async fn seed_ranked(store: &(dyn Store + Send + Sync)) -> (String, Vec<i64>, Vec<f32>) {
    let s = seed(store, "rank").await;
    let b = unique_base();
    // Three documents with a LARGE term-frequency gap (10/5/1 "code") so FTS rank
    // is unambiguously A>B>C on BOTH BM25 and ts_rank, AND decreasing vector
    // proximity. Both signals agree, so the RRF-fused scores are strictly distinct
    // and the fused order is total (§9.4) — no ties that could flip run-to-run.
    let docs = [
        (
            "alpha",
            "code code code code code code code code code code search alpha",
            0.0f32,
        ),
        ("beta", "code code code code code search beta", 0.5f32),
        ("gamma", "code search gamma", 1.0f32),
    ];
    let mut ids = Vec::new();
    for (i, (sym, ts, off)) in docs.iter().enumerate() {
        let blob = format!("R{i}{b}");
        let id = store
            .insert_chunk(&ChunkRecord {
                file_id: s.file,
                blob_sha: blob.clone(),
                symbol_name: Some(sym.to_string()),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
                start_line: (i as i32) * 5 + 1,
                end_line: (i as i32) * 5 + 4,
                preview: ts.to_string(),
                ts_doc_text: ts.to_string(),
                recency_score: 1.0,
                churn_score: 0.0,
                metadata: None,
                worktree_id: s.wt,
            })
            .await
            .unwrap();
        // Embedding: dim 0 = 1-off (nearer for smaller off), dim 1 = off.
        let mut v = vec![0f32; 768];
        v[0] = 1.0 - off;
        v[1] = *off;
        store.upsert_embedding(&blob, &v, "m").await.unwrap();
        ids.push(id);
    }
    // Build the ANN index. On SQLite this populates the `vec_code_768` virtual
    // table that vector/hybrid search reads (without it, vector search degrades to
    // empty and hybrid errors on the missing table); on Postgres it is a no-op
    // (the `embedding` column IS the ANN index). This is the legitimate
    // cross-backend write-path difference the harness must drive.
    store.sync_all_embeddings_to_vec().await.unwrap();
    let mut q = vec![0f32; 768];
    q[0] = 1.0;
    (s.repo_name, ids, q)
}

async fn check_vector(name: &str, store: &(dyn Store + Send + Sync)) {
    // Vector search requires an ANN backend (pgvector / sqlite-vec). When the
    // backend reports none (e.g. sqlite-vec not statically available in this test
    // binary), the design is graceful degradation — skip the order assertions for
    // that backend rather than assert on an empty/erroring path.
    if !store.has_vector_extension() {
        eprintln!("  [{name}] no vector extension — skipping vector parity");
        return;
    }
    let (repo, ids, q) = seed_ranked(store).await;
    let v = store
        .search_chunks_vector(&repo, Some("main"), &q, 10, false, None, None)
        .await
        .unwrap();
    let order: Vec<i64> = v.iter().map(|h| h.chunk_id).collect();
    // SQLite ranks by L2 distance, Postgres by cosine distance; the engineered
    // fixtures (A=[1,0], B=[.5,.5], C=[0,1] vs Q=[1,0]) give the SAME order A,B,C and
    // identical-vector similarity 1.0 under both metrics — that's the parity contract.
    assert_eq!(order, ids, "[{name}] vector order by nearest-distance");
    assert!(
        v[0].score > v[1].score && v[1].score > v[2].score,
        "[{name}] similarity descending"
    );
    assert!(
        (v[0].score - 1.0).abs() < 1e-6,
        "[{name}] identical vector -> similarity 1.0"
    );
}

async fn check_hybrid(name: &str, store: &(dyn Store + Send + Sync)) {
    use maproom::db::HybridWeights;
    // Hybrid needs the vector half; skip on a backend without an ANN extension
    // (see check_vector). search_hybrid does not degrade — it errors on the
    // missing vec table — so guard BEFORE calling it.
    if !store.has_vector_extension() {
        eprintln!("  [{name}] no vector extension — skipping hybrid parity");
        return;
    }
    let (repo, ids, q) = seed_ranked(store).await;
    let h = store
        .search_hybrid(
            &repo,
            Some("main"),
            "code",
            &q,
            10,
            HybridWeights::default(),
        )
        .await
        .unwrap();
    assert_eq!(h.len(), 3, "[{name}] all three fused");
    assert!(
        h.iter().all(|r| r.source == "both"),
        "[{name}] fts+vector overlap -> both"
    );
    let order: Vec<i64> = h.iter().map(|r| r.chunk_id).collect();
    assert_eq!(
        order, ids,
        "[{name}] hybrid RRF order (all-distinct fused scores)"
    );
    for w in h.windows(2) {
        // Strictly decreasing: the engineered fixture has all-distinct fused
        // scores (§9.4), so there are no ties that could flip the order.
        assert!(
            w[0].score > w[1].score,
            "[{name}] hybrid strictly sorted desc"
        );
    }

    // search_chunks_hybrid returns SearchHits in the same fused order.
    let ch = store
        .search_chunks_hybrid(&repo, Some("main"), "code", &q, 10, false, None, None)
        .await
        .unwrap();
    assert_eq!(
        ch.iter().map(|h| h.chunk_id).collect::<Vec<_>>(),
        ids,
        "[{name}] chunks_hybrid order"
    );
}

// ── Graph parity (§6.9) ─────────────────────────────────────────────────────

async fn check_graph(name: &str, store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "graph").await;
    let b = unique_base();
    // A length-12 call chain c[0]->c[1]->...->c[12] (13 nodes).
    let mut c = Vec::new();
    for i in 0..=12 {
        let id = store
            .insert_chunk(&chunk(
                s.file,
                s.wt,
                &format!("G{i}_{b}"),
                &format!("g{i}"),
                "g",
                i * 5 + 1,
                i * 5 + 4,
            ))
            .await
            .unwrap();
        c.push(id);
    }
    for i in 0..12 {
        store
            .insert_chunk_edge(c[i as usize], c[(i + 1) as usize], "calls")
            .await
            .unwrap();
    }

    // Default depth 3: callees of c0 are c1,c2,c3 (depths 1,2,3), ordered.
    let d3 = store.find_callees(c[0], None).await.unwrap();
    assert_eq!(
        d3.iter().map(|g| g.chunk_id).collect::<Vec<_>>(),
        vec![c[1], c[2], c[3]],
        "[{name}] default depth 3"
    );
    assert!(d3.iter().all(|g| g.edge_type == "calls"));

    // Some(100) clamps to the hard cap 10 (R-NFR-2): c1..c10, never c11/c12.
    let capped = store.find_callees(c[0], Some(100)).await.unwrap();
    let maxd = capped.iter().map(|g| g.depth).max().unwrap();
    assert_eq!(maxd, 10, "[{name}] depth clamped to 10");
    assert!(
        !capped
            .iter()
            .any(|g| g.chunk_id == c[11] || g.chunk_id == c[12]),
        "[{name}] no beyond-cap nodes"
    );

    // find_callers backward over 'calls'.
    let callers = store.find_callers(c[3], Some(1)).await.unwrap();
    assert_eq!(
        callers.iter().map(|g| g.chunk_id).collect::<Vec<_>>(),
        vec![c[2]],
        "[{name}] direct caller"
    );

    // get_direct_edges: depth-1 only.
    let direct = store
        .get_direct_edges(c[0], ImportDirection::Outgoing)
        .await
        .unwrap();
    assert!(direct.iter().all(|g| g.depth == 1));
    assert_eq!(
        direct.iter().map(|g| g.chunk_id).collect::<Vec<_>>(),
        vec![c[1]]
    );

    // imports + extends directionality.
    store
        .insert_chunk_edge(c[0], c[5], "imports")
        .await
        .unwrap();
    store
        .insert_chunk_edge(c[6], c[7], "extends")
        .await
        .unwrap();
    let imp = store
        .find_imports(c[0], ImportDirection::Outgoing, None)
        .await
        .unwrap();
    assert_eq!(
        imp.iter().map(|g| g.chunk_id).collect::<Vec<_>>(),
        vec![c[5]],
        "[{name}] outgoing import"
    );
    assert!(imp.iter().all(|g| g.edge_type == "imports"));
    let ext = store
        .find_extensions(c[7], ImportDirection::Incoming, None)
        .await
        .unwrap();
    assert_eq!(
        ext.iter().map(|g| g.chunk_id).collect::<Vec<_>>(),
        vec![c[6]],
        "[{name}] subclass via extends"
    );

    // Cycle termination: close the chain and ensure traversal still returns.
    store.insert_chunk_edge(c[12], c[0], "calls").await.unwrap();
    let after_cycle = store.find_callees(c[0], None).await.unwrap();
    assert!(
        after_cycle.len() >= 3,
        "[{name}] cycle terminates, still finds chain"
    );
}

// ── Search + graph driver tests ─────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn parity_fts() {
    for_each(|_n, s| async move { check_fts(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_vector() {
    for_each(|n, s| async move { check_vector(n, s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_hybrid() {
    for_each(|n, s| async move { check_hybrid(n, s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_graph() {
    for_each(|n, s| async move { check_graph(n, s.as_ref()).await }).await;
}

// ── Edge-depth (F-B) parity: batch insert, cross-file callers, test_of ───────

/// Spec DoD §1: `insert_chunk_edges_batch`, cross-file `find_callers`, `test_of`
/// round-trip, and `list_symbols_for_worktree` behave identically on both backends.
async fn check_edge_batch_and_test_of(name: &str, store: &(dyn Store + Send + Sync)) {
    let s = seed(store, "edgebatch").await;
    let b = unique_base();

    // A second file so caller -> callee is genuinely cross-file.
    let file_b = store
        .upsert_file(&FileRecord {
            repo_id: s.repo,
            worktree_id: s.wt,
            commit_id: s.commit,
            relpath: format!("src/other_{b}.rs"),
            language: Some("rust".to_string()),
            content_hash: format!("hb-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();

    let caller = store
        .insert_chunk(&chunk(s.file, s.wt, &format!("CA{b}"), "caller", "c", 1, 5))
        .await
        .unwrap();
    let callee = store
        .insert_chunk(&chunk(file_b, s.wt, &format!("CB{b}"), "callee", "c", 1, 5))
        .await
        .unwrap();
    let test = store
        .insert_chunk(&chunk(
            s.file,
            s.wt,
            &format!("CT{b}"),
            "test_caller",
            "c",
            7,
            11,
        ))
        .await
        .unwrap();

    // B4: batch insert cross-file calls + a test_of edge in one transaction.
    store
        .insert_chunk_edges_batch(&[
            (caller, callee, "calls".to_string()),
            (test, callee, "calls".to_string()),
            (test, callee, "test_of".to_string()),
        ])
        .await
        .unwrap();

    // Cross-file find_callers: both callers found, all via `calls` (test_of purity).
    let callers = store.find_callers(callee, Some(1)).await.unwrap();
    let caller_ids: Vec<i64> = callers.iter().map(|g| g.chunk_id).collect();
    assert!(
        caller_ids.contains(&caller) && caller_ids.contains(&test),
        "[{name}] cross-file callers must include both, got {caller_ids:?}"
    );
    assert!(
        callers.iter().all(|g| g.edge_type == "calls"),
        "[{name}] test_of must not leak into find_callers"
    );

    // test_of round-trip: only the test chunk targets callee via test_of.
    let incoming = store
        .get_direct_edges(callee, ImportDirection::Incoming)
        .await
        .unwrap();
    let mut test_of_srcs: Vec<i64> = incoming
        .iter()
        .filter(|e| e.edge_type == "test_of")
        .map(|e| e.chunk_id)
        .collect();
    test_of_srcs.sort_unstable();
    assert_eq!(test_of_srcs, vec![test], "[{name}] test_of round-trip");

    // list_symbols_for_worktree returns the worktree's named chunks with relpath+kind.
    let symbols = store.list_symbols_for_worktree(s.wt).await.unwrap();
    assert!(
        symbols.iter().any(|(id, sym, rel, kind)| *id == callee
            && sym == "callee"
            && rel.contains("other_")
            && kind == "function"),
        "[{name}] list_symbols must return the callee row, got {symbols:?}"
    );
    assert!(
        symbols.iter().any(|(id, _, _, _)| *id == caller),
        "[{name}] list_symbols must return the caller row"
    );
}

#[tokio::test]
#[ignore]
async fn parity_edge_batch_and_test_of() {
    for_each(|n, s| async move { check_edge_batch_and_test_of(n, s.as_ref()).await }).await;
}

// ── Wave B (fix spec R09/R05/R18/R03) parity checks ─────────────────────────

/// R09 / R-GC-1..4: unmapping superseded generations keeps only the surviving
/// file generation's chunks and reports the junction rows removed.
async fn check_unmap_superseded_file_chunks(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "gc").await;
    let relpath = format!("src/gc_{b}.rs");

    // Generation A: file row + two chunks.
    let file_a = store
        .upsert_file(&FileRecord {
            repo_id: s.repo,
            worktree_id: s.wt,
            commit_id: s.commit,
            relpath: relpath.clone(),
            language: Some("rust".to_string()),
            content_hash: format!("genA-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(
            file_a,
            s.wt,
            &format!("A1{b}"),
            "old_fn_one",
            "old one",
            1,
            5,
        ))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(
            file_a,
            s.wt,
            &format!("A2{b}"),
            "old_fn_two",
            "old two",
            6,
            9,
        ))
        .await
        .unwrap();

    // Generation B: new commit, same relpath, new content hash -> new file row.
    let commit_b = store
        .get_or_create_commit(s.repo, &format!("shaB-{b}"), None)
        .await
        .unwrap();
    let file_b = store
        .upsert_file(&FileRecord {
            repo_id: s.repo,
            worktree_id: s.wt,
            commit_id: commit_b,
            relpath: relpath.clone(),
            language: Some("rust".to_string()),
            content_hash: format!("genB-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    assert_ne!(
        file_a, file_b,
        "distinct generations must be distinct file rows"
    );
    store
        .insert_chunk(&chunk(
            file_b,
            s.wt,
            &format!("B1{b}"),
            "new_fn",
            "new fn",
            1,
            4,
        ))
        .await
        .unwrap();

    let removed = store
        .unmap_superseded_file_chunks(s.wt, &relpath, Some(file_b))
        .await
        .unwrap();
    assert_eq!(removed, 2, "both generation-A junction rows removed");

    let rels: Vec<(i64, String)> = store
        .get_chunks_for_worktree(s.wt)
        .await
        .unwrap()
        .into_iter()
        .filter(|(_, r)| r == &relpath)
        .collect();
    assert_eq!(
        rels.len(),
        1,
        "only the surviving generation's chunk remains: {rels:?}"
    );
}

/// R09: keep_file_id = None removes every generation (file deleted).
async fn check_unmap_superseded_none_removes_all_generations(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "gcnone").await;
    let relpath = format!("src/gcnone_{b}.rs");
    let file_a = store
        .upsert_file(&FileRecord {
            repo_id: s.repo,
            worktree_id: s.wt,
            commit_id: s.commit,
            relpath: relpath.clone(),
            language: Some("rust".to_string()),
            content_hash: format!("gA-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(
            file_a,
            s.wt,
            &format!("N1{b}"),
            "gone_fn",
            "gone",
            1,
            3,
        ))
        .await
        .unwrap();

    let removed = store
        .unmap_superseded_file_chunks(s.wt, &relpath, None)
        .await
        .unwrap();
    assert_eq!(removed, 1);
    let remaining = store
        .get_chunks_for_worktree(s.wt)
        .await
        .unwrap()
        .into_iter()
        .filter(|(_, r)| r == &relpath)
        .count();
    assert_eq!(remaining, 0, "no generation may survive keep_file_id=None");
}

/// R09 / CC-2 (R-WT-4): a second worktree's mapping AND the content-addressed
/// embeddings pool survive another worktree's re-index. This parity test is
/// the mechanical guard that no embedding deletes crept into the GC path.
async fn check_unmap_superseded_preserves_shared_and_pool(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "gcshare").await;
    let relpath = format!("src/gcshare_{b}.rs");
    let wt2 = store
        .get_or_create_worktree(s.repo, "feature", &format!("/wt/gcshare2-{b}"))
        .await
        .unwrap();

    let file_a = store
        .upsert_file(&FileRecord {
            repo_id: s.repo,
            worktree_id: s.wt,
            commit_id: s.commit,
            relpath: relpath.clone(),
            language: Some("rust".to_string()),
            content_hash: format!("shA-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    let blob = format!("SHARED{b}");
    // Same chunk mapped to BOTH worktrees (insert_chunk upserts + maps).
    store
        .insert_chunk(&chunk(file_a, s.wt, &blob, "shared_fn", "shared", 1, 5))
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(file_a, wt2, &blob, "shared_fn", "shared", 1, 5))
        .await
        .unwrap();
    // Pool entry for the shared blob.
    store
        .upsert_embedding(&blob, &vec![0.01f32; 768], "test-model")
        .await
        .unwrap();
    let pool_before = store.get_global_embedding_count().await.unwrap();

    // wt1 re-indexes: generation B supersedes A *for wt1 only*.
    let commit_b = store
        .get_or_create_commit(s.repo, &format!("shB-{b}"), None)
        .await
        .unwrap();
    let file_b = store
        .upsert_file(&FileRecord {
            repo_id: s.repo,
            worktree_id: s.wt,
            commit_id: commit_b,
            relpath: relpath.clone(),
            language: Some("rust".to_string()),
            content_hash: format!("shB-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(
            file_b,
            s.wt,
            &format!("SB{b}"),
            "shared_fn_v2",
            "v2",
            1,
            6,
        ))
        .await
        .unwrap();
    store
        .unmap_superseded_file_chunks(s.wt, &relpath, Some(file_b))
        .await
        .unwrap();

    // wt2's mapping to the generation-A chunk survives. Probed via FTS
    // search scoped to wt2: search joins the chunk_worktrees JUNCTION on
    // BOTH backends (get_chunks_for_worktree is file-ownership-based on
    // SQLite — a pre-existing backend divergence, unsuitable here).
    let _ = wt2; // id used via the worktree NAME below
                 // Query on the ts-doc word ("shared") — tokenization of symbol names
                 // differs across backends; the ts text matches on both.
    let (wt2_hits, _) = store
        .search_chunks_fts(
            &s.repo_name,
            Some("feature"),
            "shared",
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(
        !wt2_hits.is_empty(),
        "the other worktree's chunk mapping must survive (R-WT-4)"
    );
    // The embeddings pool is untouched.
    let pool_after = store.get_global_embedding_count().await.unwrap();
    assert_eq!(
        pool_before, pool_after,
        "code_embeddings pool must never shrink on re-index"
    );
}

/// R05 / R-STALE-1: delete_worktree_data removes the worktree REGISTRATION so
/// stale detection converges instead of re-reporting forever.
async fn check_delete_worktree_data_removes_registration(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "stalereg").await;
    store
        .insert_chunk(&chunk(
            s.file,
            s.wt,
            &format!("SR{b}"),
            "stale_fn",
            "stale",
            1,
            3,
        ))
        .await
        .unwrap();

    store.delete_worktree_data(s.wt).await.unwrap();

    let names: Vec<String> = store
        .list_worktrees(s.repo)
        .await
        .unwrap()
        .into_iter()
        .map(|w| w.name)
        .collect();
    assert!(
        names.is_empty(),
        "worktree registration must be deleted (was re-reported forever): {names:?}"
    );
    let stale = store.detect_stale_worktrees().await.unwrap();
    assert!(
        !stale.iter().any(|sw| sw.id == s.wt),
        "deleted worktree must not be re-reported as stale"
    );
}

/// R18 / R-WTF-2: an unknown worktree name yields the EMPTY set on all three
/// search paths (it used to silently drop the filter on SQLite).
async fn check_search_unknown_worktree_returns_empty(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "wtf").await;
    store
        .insert_chunk(&chunk(
            s.file,
            s.wt,
            &format!("WT{b}"),
            "wtf_probe_fn",
            "wtf probe fn",
            1,
            3,
        ))
        .await
        .unwrap();

    let (hits, total) = store
        .search_chunks_fts(
            &s.repo_name,
            Some("no-such-wt"),
            "wtf_probe_fn",
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(
        hits.is_empty(),
        "fts: unknown worktree must be empty, got {hits:?}"
    );
    assert_eq!(total, 0);

    let vhits = store
        .search_chunks_vector(
            &s.repo_name,
            Some("no-such-wt"),
            &vec![0.01f32; 768],
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(vhits.is_empty(), "vector: unknown worktree must be empty");

    let hhits = store
        .search_chunks_hybrid(
            &s.repo_name,
            Some("no-such-wt"),
            "wtf_probe_fn",
            &vec![0.01f32; 768],
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(hhits.is_empty(), "hybrid: unknown worktree must be empty");

    // Control: the KNOWN worktree still finds the probe.
    let (known_hits, _) = store
        .search_chunks_fts(
            &s.repo_name,
            Some("main"),
            "wtf_probe_fn",
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(!known_hits.is_empty(), "control: known worktree must match");
}

/// R03 / R-VER-2/3: a fresh store on each backend verifies clean against its
/// OWN required-table list (SQLite 12 incl. context_cache; PG 11 core).
async fn check_verify_schema_clean(store: &(dyn Store + Send + Sync)) {
    let missing = store.verify_schema().await.unwrap();
    assert_eq!(
        missing,
        Vec::<String>::new(),
        "fresh schema must verify clean"
    );
}

#[tokio::test]
#[ignore]
async fn parity_unmap_superseded_file_chunks() {
    for_each(|_n, s| async move { check_unmap_superseded_file_chunks(s.as_ref()).await }).await;
}

#[tokio::test]
#[ignore]
async fn parity_unmap_superseded_none_removes_all_generations() {
    for_each(|_n, s| async move {
        check_unmap_superseded_none_removes_all_generations(s.as_ref()).await
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn parity_unmap_superseded_preserves_shared_and_pool() {
    for_each(
        |_n, s| async move { check_unmap_superseded_preserves_shared_and_pool(s.as_ref()).await },
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn parity_delete_worktree_data_removes_registration() {
    for_each(
        |_n, s| async move { check_delete_worktree_data_removes_registration(s.as_ref()).await },
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn parity_search_unknown_worktree_returns_empty() {
    for_each(|_n, s| async move { check_search_unknown_worktree_returns_empty(s.as_ref()).await })
        .await;
}

#[tokio::test]
#[ignore]
async fn parity_verify_schema_clean() {
    for_each(|_n, s| async move { check_verify_schema_clean(s.as_ref()).await }).await;
}

/// R10 / R-PREV-1/2: the main FTS path carries a non-empty preview on BOTH
/// backends (the PG SELECT omitted the column and its mapper hardcoded None).
async fn check_fts_hit_includes_preview(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "prev").await;
    store
        .insert_chunk(&chunk(
            s.file,
            s.wt,
            &format!("PV{b}"),
            "preview_probe_fn",
            "preview probe body",
            1,
            4,
        ))
        .await
        .unwrap();
    let (hits, _) = store
        .search_chunks_fts(&s.repo_name, None, "preview", 10, false, None, None)
        .await
        .unwrap();
    assert!(!hits.is_empty(), "probe must match");
    let p = hits[0].preview.as_deref().unwrap_or("");
    assert!(
        !p.is_empty(),
        "FTS hits must carry the stored preview (R10; PG used to hardcode None)"
    );
}

#[tokio::test]
#[ignore]
async fn parity_fts_hit_includes_preview() {
    for_each(|_n, s| async move { check_fts_hit_includes_preview(s.as_ref()).await }).await;
}

/// Review H4: get_chunks_for_worktree is JUNCTION-scoped on BOTH backends —
/// a chunk mapped to worktree B via chunk_worktrees is visible for B even
/// when its files row belongs to worktree A (the branch-switch flow).
async fn check_get_chunks_for_worktree_junction_scoped(store: &(dyn Store + Send + Sync)) {
    let b = unique_base();
    let s = seed(store, "junc").await;
    let relpath = format!("src/junc_{b}.rs");
    let wt2 = store
        .get_or_create_worktree(s.repo, "feature", &format!("/wt/junc2-{b}"))
        .await
        .unwrap();
    let file_a = store
        .upsert_file(&FileRecord {
            repo_id: s.repo,
            worktree_id: s.wt, // files row OWNED by wt1
            commit_id: s.commit,
            relpath: relpath.clone(),
            language: Some("rust".to_string()),
            content_hash: format!("jc-{b}"),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    store
        .insert_chunk(&chunk(
            file_a,
            s.wt,
            &format!("J1{b}"),
            "junc_fn",
            "junc fn",
            1,
            3,
        ))
        .await
        .unwrap();
    // Map the SAME chunk to wt2 via the junction (no wt2-owned files row).
    store
        .insert_chunk(&chunk(
            file_a,
            wt2,
            &format!("J1{b}"),
            "junc_fn",
            "junc fn",
            1,
            3,
        ))
        .await
        .unwrap();

    let wt2_rels: Vec<String> = store
        .get_chunks_for_worktree(wt2)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .filter(|r| r == &relpath)
        .collect();
    assert_eq!(
        wt2_rels.len(),
        1,
        "junction-mapped chunk must be visible for wt2 (H4; ownership-scoping missed it)"
    );
}

#[tokio::test]
#[ignore]
async fn parity_get_chunks_for_worktree_junction_scoped() {
    for_each(
        |_n, s| async move { check_get_chunks_for_worktree_junction_scoped(s.as_ref()).await },
    )
    .await;
}
