//! Cross-repo search tests (R5 / spec §D-8)
//!
//! Validates that `search_fts_multi_repo` works correctly for both SQLite and
//! Postgres backends with one/many/all repo scopes.
//!
//! SQLite backend: always runs (`:memory:` shared-cache).
//! Postgres backend: gated on `--features postgres` AND `MAPROOM_TEST_PG_URL`.
//!
//! Run:
//!   # SQLite only:
//!   cargo test -p maproom --test cross_repo_search
//!
//!   # Both backends:
//!   MAPROOM_TEST_PG_URL=postgres://maproom:maproom@localhost:5433/maproom_test \
//!     cargo test -p maproom --features postgres --test cross_repo_search

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use maproom::db::sqlite::SqliteStore;
use maproom::db::traits::StoreMigration;
use maproom::db::{ChunkRecord, FileRecord, Store};

// ── Helpers ──────────────────────────────────────────────────────────────────

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

async fn backends() -> Vec<(&'static str, Arc<dyn Store + Send + Sync>)> {
    let mut v: Vec<(&'static str, Arc<dyn Store + Send + Sync>)> = Vec::new();

    let b = unique_base();
    let mem = format!("file:cross_repo_{}?mode=memory&cache=shared", b);
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
            eprintln!("cross_repo_search: MAPROOM_TEST_PG_URL unset — postgres backend skipped");
        }
    }
    v
}

fn make_chunk(file_id: i64, worktree_id: i64, blob: &str, sym: &str, text: &str) -> ChunkRecord {
    ChunkRecord {
        file_id,
        blob_sha: blob.to_string(),
        symbol_name: Some(sym.to_string()),
        kind: "function".to_string(),
        signature: None,
        docstring: None,
        start_line: 1,
        end_line: 10,
        preview: text.to_string(),
        ts_doc_text: text.to_string(),
        recency_score: 1.0,
        churn_score: 0.0,
        metadata: None,
        worktree_id,
    }
}

/// Seed one repo with one worktree/file and one indexed chunk.
/// Returns (repo_id, chunk symbol_name).
async fn seed_repo(
    store: &(dyn Store + Send + Sync),
    repo_name: &str,
    blob: &str,
    sym: &str,
    fts_text: &str,
) -> i64 {
    let b = unique_base();
    let repo_id = store
        .get_or_create_repo(repo_name, &format!("/src/{b}"))
        .await
        .expect("get_or_create_repo");
    let wt_id = store
        .get_or_create_worktree(repo_id, "main", &format!("/wt/{b}"))
        .await
        .expect("get_or_create_worktree");
    let commit_id = store
        .get_or_create_commit(repo_id, &format!("sha-{b}"), None)
        .await
        .expect("get_or_create_commit");
    let file_id = store
        .upsert_file(&FileRecord {
            repo_id,
            worktree_id: wt_id,
            commit_id,
            relpath: format!("src/{sym}_{b}.rs"),
            language: Some("rust".to_string()),
            content_hash: format!("hash-{blob}-{b}"),
            size_bytes: 100,
            last_modified: None,
        })
        .await
        .expect("upsert_file");
    store
        .insert_chunk(&make_chunk(file_id, wt_id, blob, sym, fts_text))
        .await
        .expect("insert_chunk");
    repo_id
}

// ── Test: single-repo scope via search_fts_multi_repo ────────────────────────

#[tokio::test]
async fn test_single_repo_scope() {
    for (backend, store) in backends().await {
        let b = unique_base();
        let repo_a = seed_repo(
            store.as_ref(),
            &format!("acme/alpha-{b}"),
            &format!("blob-a-{b}"),
            &format!("uniquewordalphabeta{b}"),
            &format!("uniquewordalphabeta{b} is a function in alpha"),
        )
        .await;

        // Search targeting only repo_a
        let hits = store
            .search_fts_multi_repo(
                &[repo_a],
                &format!("uniquewordalphabeta{b}"),
                10,
                None,
                None,
            )
            .await
            .unwrap_or_else(|e| panic!("[{backend}] search_fts_multi_repo failed: {e}"));

        assert!(
            !hits.is_empty(),
            "[{backend}] expected at least one hit for single-repo scope"
        );
        eprintln!("[{backend}] test_single_repo_scope: {} hit(s)", hits.len());
    }
}

// ── Test: multi-repo (list) scope — hits from both repos ─────────────────────

#[tokio::test]
async fn test_multi_repo_list_scope() {
    for (backend, store) in backends().await {
        let b = unique_base();
        let shared_word = format!("crossrepoterm{b}");

        let repo_a = seed_repo(
            store.as_ref(),
            &format!("acme/multi-a-{b}"),
            &format!("blob-ma-{b}"),
            &format!("fn_a_{b}"),
            &format!("{shared_word} lives in repo alpha"),
        )
        .await;
        let repo_b = seed_repo(
            store.as_ref(),
            &format!("acme/multi-b-{b}"),
            &format!("blob-mb-{b}"),
            &format!("fn_b_{b}"),
            &format!("{shared_word} also lives in repo beta"),
        )
        .await;

        let hits = store
            .search_fts_multi_repo(&[repo_a, repo_b], &shared_word, 10, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{backend}] multi-repo search failed: {e}"));

        // Both repos should contribute a hit
        assert!(
            hits.len() >= 2,
            "[{backend}] expected >=2 hits (one per repo), got {}",
            hits.len()
        );

        // Verify hits are ordered by repo_id groups (D-8b)
        // Group consecutive hits: each repo's hits should be contiguous.
        // (We can't assert repo_id directly from SearchHit, but chunk_ids are distinct.)
        eprintln!(
            "[{backend}] test_multi_repo_list_scope: {} hit(s)",
            hits.len()
        );
    }
}

// ── Test: all-repos scope — hits across every repo in the store ───────────────

#[tokio::test]
async fn test_all_repos_scope() {
    for (backend, store) in backends().await {
        let b = unique_base();
        let shared_word = format!("globalterm{b}");

        let repo_a = seed_repo(
            store.as_ref(),
            &format!("acme/all-a-{b}"),
            &format!("blob-aa-{b}"),
            &format!("gfn_a_{b}"),
            &format!("{shared_word} in repo one"),
        )
        .await;
        let repo_b = seed_repo(
            store.as_ref(),
            &format!("acme/all-b-{b}"),
            &format!("blob-ab-{b}"),
            &format!("gfn_b_{b}"),
            &format!("{shared_word} in repo two"),
        )
        .await;

        // Fetch all repo_ids from the store
        let all_repos = store.list_repos().await.expect("list_repos");
        let all_ids: Vec<i64> = all_repos.iter().map(|r| r.id).collect();
        assert!(
            all_ids.contains(&repo_a) && all_ids.contains(&repo_b),
            "[{backend}] seeded repos not found in list_repos"
        );

        let hits = store
            .search_fts_multi_repo(&all_ids, &shared_word, 10, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{backend}] all-repos search failed: {e}"));

        assert!(
            hits.len() >= 2,
            "[{backend}] expected >=2 hits (one per seeded repo), got {}",
            hits.len()
        );
        eprintln!("[{backend}] test_all_repos_scope: {} hit(s)", hits.len());
    }
}

// ── Test: k-per-repo cap (D-8c) ──────────────────────────────────────────────

#[tokio::test]
async fn test_k_per_repo_cap() {
    for (backend, store) in backends().await {
        let b = unique_base();
        let word = format!("cappedword{b}");

        // Seed repo_a with 3 chunks all matching `word`
        let b2 = unique_base();
        let repo_id = store
            .get_or_create_repo(&format!("acme/cap-{b}"), &format!("/src/cap-{b}"))
            .await
            .unwrap();
        let wt_id = store
            .get_or_create_worktree(repo_id, "main", &format!("/wt/cap-{b}"))
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, &format!("sha-cap-{b2}"), None)
            .await
            .unwrap();
        let file_id = store
            .upsert_file(&FileRecord {
                repo_id,
                worktree_id: wt_id,
                commit_id,
                relpath: format!("src/cap_{b}.rs"),
                language: Some("rust".to_string()),
                content_hash: format!("hash-cap-{b2}"),
                size_bytes: 100,
                last_modified: None,
            })
            .await
            .unwrap();

        for i in 0..3u64 {
            store
                .insert_chunk(&make_chunk(
                    file_id,
                    wt_id,
                    &format!("blob-cap-{b}-{i}"),
                    &format!("fn_cap_{b}_{i}"),
                    &format!("{word} chunk number {i}"),
                ))
                .await
                .unwrap();
        }

        // k=2: should return at most 2 hits from this repo
        let hits = store
            .search_fts_multi_repo(&[repo_id], &word, 2, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{backend}] k-per-repo cap test failed: {e}"));

        assert!(
            hits.len() <= 2,
            "[{backend}] k=2 cap exceeded: got {} hits",
            hits.len()
        );
        eprintln!(
            "[{backend}] test_k_per_repo_cap: {} hit(s) (capped at k=2)",
            hits.len()
        );
    }
}

// ── Test: empty repo_ids returns empty result ─────────────────────────────────

#[tokio::test]
async fn test_empty_repo_ids_returns_empty() {
    for (backend, store) in backends().await {
        let hits = store
            .search_fts_multi_repo(&[], "anything", 10, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{backend}] empty repo_ids test failed: {e}"));

        assert!(
            hits.is_empty(),
            "[{backend}] expected empty result for empty repo_ids"
        );
    }
}

// ── Test: scope isolation — cross-repo contamination ─────────────────────────

#[tokio::test]
async fn test_scope_isolation() {
    for (backend, store) in backends().await {
        let b = unique_base();
        let word_a = format!("onlyinrepoalpha{b}");
        let word_b = format!("onlyinrepobeta{b}");

        let repo_a = seed_repo(
            store.as_ref(),
            &format!("acme/iso-a-{b}"),
            &format!("blob-ia-{b}"),
            &format!("iso_fn_a_{b}"),
            &format!("{word_a} is exclusive to alpha"),
        )
        .await;
        let repo_b = seed_repo(
            store.as_ref(),
            &format!("acme/iso-b-{b}"),
            &format!("blob-ib-{b}"),
            &format!("iso_fn_b_{b}"),
            &format!("{word_b} is exclusive to beta"),
        )
        .await;

        // Search word_a restricted to repo_a only — should NOT see beta chunks
        let hits_a = store
            .search_fts_multi_repo(&[repo_a], &word_a, 10, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{backend}] isolation test (a) failed: {e}"));
        assert!(
            !hits_a.is_empty(),
            "[{backend}] expected hits for word_a in repo_a"
        );

        // Search word_b restricted to repo_b only — should NOT see alpha chunks
        let hits_b = store
            .search_fts_multi_repo(&[repo_b], &word_b, 10, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{backend}] isolation test (b) failed: {e}"));
        assert!(
            !hits_b.is_empty(),
            "[{backend}] expected hits for word_b in repo_b"
        );

        // Search word_a but restrict to repo_b — should be empty
        let hits_cross = store
            .search_fts_multi_repo(&[repo_b], &word_a, 10, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{backend}] isolation test (cross) failed: {e}"));
        assert!(
            hits_cross.is_empty(),
            "[{backend}] cross-repo contamination: word_a leaked into repo_b results"
        );

        eprintln!("[{backend}] test_scope_isolation: passed");
    }
}
