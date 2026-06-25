//! Live Phase-1 verification against a real pgvector instance (spec §10 Phase-1
//! exit criteria + §7 Phase-1 scenarios). Gated on `MAPROOM_TEST_PG_URL` and
//! `#[ignore]` so it never runs in the default suite.
//!
//!   MAPROOM_TEST_PG_URL=postgres://user:pw@host:5432/db \
//!     cargo test -p maproom --features postgres -- --ignored --test-threads=1

#![cfg(test)]

use std::collections::HashSet;

use sqlx::postgres::PgPoolOptions;

use super::PostgresStore;
use crate::db::index_state::UpdateStats;
use crate::db::traits::{
    StoreChunks, StoreCleanup, StoreCore, StoreEmbeddings, StoreEncoding, StoreIndexState,
    StoreMigration, StoreSearch,
};
use crate::db::{ChunkRecord, FileRecord};

fn test_url() -> Option<String> {
    std::env::var("MAPROOM_TEST_PG_URL").ok()
}

/// Reset to a clean schema and reconnect (so each run is isolated; connect()
/// re-applies migrations from scratch, recreating the pgvector extension).
async fn fresh_store(url: &str) -> PostgresStore {
    let pool = PgPoolOptions::new().connect(url).await.unwrap();
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    PostgresStore::connect(url).await.unwrap()
}

fn chunk(file_id: i64, worktree_id: i64, blob_sha: &str, start: i32, end: i32) -> ChunkRecord {
    ChunkRecord {
        file_id,
        blob_sha: blob_sha.to_string(),
        symbol_name: Some("doThing".to_string()),
        kind: "function".to_string(),
        signature: Some("fn doThing()".to_string()),
        docstring: None,
        start_line: start,
        end_line: end,
        preview: "fn doThing() {}".to_string(),
        ts_doc_text: "doThing function".to_string(),
        recency_score: 1.0,
        churn_score: 0.0,
        metadata: None,
        worktree_id,
    }
}

#[tokio::test]
#[ignore]
async fn phase1_live() {
    let Some(url) = test_url() else {
        eprintln!("skipping phase1_live: MAPROOM_TEST_PG_URL unset");
        return;
    };
    let store = fresh_store(&url).await;

    // ── Migrations auto-ran; integer-version adapter returns {1,2} (§5.2/§7) ──
    let applied = store.get_applied_migrations().await.unwrap();
    assert_eq!(applied, HashSet::from([1, 2]), "applied migration versions");

    // Idempotent: a second connect adds no tracking rows (§7 Migrations / R-MIG-2).
    let store2 = PostgresStore::connect(&url).await.unwrap();
    assert_eq!(
        store2.get_applied_migrations().await.unwrap(),
        HashSet::from([1, 2])
    );
    let mig_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM schema_migrations")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(mig_rows, 2, "schema_migrations stable across reconnect");

    // ── Schema shape (§7 Migrations) ──
    for t in [
        "repos",
        "worktrees",
        "commits",
        "files",
        "chunks",
        "chunk_worktrees",
        "chunk_edges",
        "code_embeddings",
        "index_state",
        "encoding_runs",
    ] {
        let reg: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
            .bind(format!("public.{t}"))
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert!(reg.is_some(), "required table {t} is missing");
    }
    let worktree_ids_col: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM information_schema.columns \
         WHERE table_name = 'chunks' AND column_name = 'worktree_ids'",
    )
    .fetch_optional(&store.pool)
    .await
    .unwrap();
    assert!(
        worktree_ids_col.is_none(),
        "chunks.worktree_ids must NOT exist (junction model)"
    );
    let vec_ext: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM pg_extension WHERE extname = 'vector'")
            .fetch_optional(&store.pool)
            .await
            .unwrap();
    assert!(vec_ext.is_some(), "pgvector extension present");
    assert!(
        store.has_vector_extension(),
        "has_vector_extension() cached true"
    );

    // ── StoreCore idempotency (§7 StoreCore) ──
    let repo = store
        .get_or_create_repo("acme/widget", "/src/widget")
        .await
        .unwrap();
    assert_eq!(
        repo,
        store
            .get_or_create_repo("acme/widget", "/src/widget")
            .await
            .unwrap()
    );
    let w1 = store
        .get_or_create_worktree(repo, "main", "/wt/main")
        .await
        .unwrap();
    assert_eq!(
        w1,
        store
            .get_or_create_worktree(repo, "main", "/wt/main")
            .await
            .unwrap()
    );
    let w2 = store
        .get_or_create_worktree(repo, "feature", "/wt/feature")
        .await
        .unwrap();
    let commit = store
        .get_or_create_commit(repo, "abc123", None)
        .await
        .unwrap();
    assert_eq!(
        commit,
        store
            .get_or_create_commit(repo, "abc123", None)
            .await
            .unwrap()
    );
    assert_eq!(store.list_worktrees(repo).await.unwrap().len(), 2);

    // ── StoreChunks: insert, junction idempotency, context (§7 StoreChunks) ──
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: w1,
            commit_id: commit,
            relpath: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "fhash".to_string(),
            size_bytes: 100,
            last_modified: None,
        })
        .await
        .unwrap();

    let c1 = store
        .insert_chunk(&chunk(file, w1, "BLOB1", 1, 5))
        .await
        .unwrap();
    // insert_chunk dedups on (file_id,start_line,end_line): same coords -> same id.
    let c1b = store
        .insert_chunk(&chunk(file, w1, "BLOB1b", 1, 5))
        .await
        .unwrap();
    assert_eq!(c1, c1b, "insert_chunk dedups on (file,start,end)");
    let _c2 = store
        .insert_chunk(&chunk(file, w1, "BLOB2", 6, 10))
        .await
        .unwrap();

    // add_chunk_to_worktree idempotent; get_chunk_worktrees returns all.
    store.add_chunk_to_worktree(c1, w2).await.unwrap();
    store.add_chunk_to_worktree(c1, w2).await.unwrap();
    let mut wts = store.get_chunk_worktrees(c1).await.unwrap();
    wts.sort_unstable();
    assert_eq!(wts, vec![w1, w2], "chunk mapped to both worktrees, no dup");

    // get_chunk_context: neighbors from same file, target excluded; None for missing.
    let ctx = store
        .get_chunk_context(c1, 1)
        .await
        .unwrap()
        .expect("context exists");
    assert_eq!(ctx.chunk.id, c1);
    assert!(ctx.surrounding_chunks.iter().all(|s| s.id != c1));
    assert!(store.get_chunk_context(999_999, 1).await.unwrap().is_none());

    // get_chunks_for_worktree returns (id, relpath) tuples.
    let for_w1 = store.get_chunks_for_worktree(w1).await.unwrap();
    assert!(for_w1
        .iter()
        .any(|(id, path)| *id == c1 && path == "src/lib.rs"));

    // ── Content-addressed counts/dedup (§7 StoreCore / dedup) ──
    // Two chunks (c1 BLOB1, c2 BLOB2) -> 2 distinct blob_sha.
    assert_eq!(
        store.get_global_chunk_count().await.unwrap(),
        2,
        "DISTINCT blob_sha count"
    );

    // ── StoreEmbeddings (§7 dedup) ──
    let emb: Vec<f32> = (0..768).map(|i| (i as f32) / 1000.0).collect();
    let e1 = store
        .upsert_embedding("BLOB1", &emb, "ollama")
        .await
        .unwrap();
    let e2 = store
        .upsert_embedding("BLOB1", &emb, "ollama")
        .await
        .unwrap();
    assert_eq!(e1, e2, "upsert_embedding stable id on re-upsert");
    assert!(store.has_embedding("BLOB1").await.unwrap());
    assert!(!store.has_embedding("NOPE").await.unwrap());

    // get_embedding round-trips within float4 precision (R-EMB-3).
    let got = store
        .get_embedding("BLOB1")
        .await
        .unwrap()
        .expect("embedding present");
    assert_eq!(got.len(), 768);
    for (a, b) in emb.iter().zip(got.iter()) {
        assert!((a - b).abs() <= 1e-5, "round-trip {a} vs {b}");
    }

    // Dimension validation (R-EMB-4): unsupported dim errors, listing supported.
    let bad: Vec<f32> = vec![0.1; 512];
    let err = store
        .upsert_embedding("BADDIM", &bad, "ollama")
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("512") && msg.contains("768") && msg.contains("1536"),
        "dim error: {msg}"
    );

    // fetch_chunks_needing_embeddings: incremental excludes embedded (BLOB1),
    // non-incremental returns all; dedup => only BLOB2 needs embedding now.
    let need_inc = store
        .fetch_chunks_needing_embeddings(true, None)
        .await
        .unwrap();
    assert!(
        need_inc.iter().all(|c| c.blob_sha != "BLOB1"),
        "incremental excludes already-embedded BLOB1"
    );
    assert!(
        need_inc.iter().any(|c| c.blob_sha == "BLOB2"),
        "BLOB2 still needs embedding"
    );
    let need_all = store
        .fetch_chunks_needing_embeddings(false, None)
        .await
        .unwrap();
    assert!(
        need_all.len() >= need_inc.len(),
        "non-incremental returns all"
    );
    assert_eq!(
        store.get_global_embedding_count().await.unwrap(),
        1,
        "one pooled embedding"
    );

    // Zero-recompute across worktrees: c1 (BLOB1) is in w1 AND w2; its embedding
    // is the single shared pool row, so it is never re-listed as needing one.
    assert!(need_inc.iter().all(|c| c.blob_sha != "BLOB1"));

    // ── StoreIndexState (§6.4) ──
    assert_eq!(
        store.get_last_indexed_tree(w1).await.unwrap(),
        "init",
        "init before indexing"
    );
    store
        .update_index_state(
            w1,
            "treesha1",
            &UpdateStats {
                files_processed: 1,
                chunks_processed: 2,
                embeddings_generated: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(store.get_last_indexed_tree(w1).await.unwrap(), "treesha1");

    eprintln!("phase1_live: all Phase-1 assertions passed");
}

/// Phase-2a FTS verification (spec §6.6, §7 FTS scenarios).
#[tokio::test]
#[ignore]
async fn fts_live() {
    let Some(url) = test_url() else {
        eprintln!("skipping fts_live: MAPROOM_TEST_PG_URL unset");
        return;
    };
    let store = fresh_store(&url).await;
    let repo = store
        .get_or_create_repo("acme/app", "/src/app")
        .await
        .unwrap();
    let wt = store
        .get_or_create_worktree(repo, "main", "/wt/main")
        .await
        .unwrap();
    let commit = store
        .get_or_create_commit(repo, "sha1", None)
        .await
        .unwrap();
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wt,
            commit_id: commit,
            relpath: "src/auth.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "h".to_string(),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();

    // helper to insert a chunk with explicit symbol/kind/ts_doc text
    let mk = |sym: &str, kind: &str, ts: &str, s: i32, e: i32, blob: &str| ChunkRecord {
        file_id: file,
        blob_sha: blob.to_string(),
        symbol_name: Some(sym.to_string()),
        kind: kind.to_string(),
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
    };
    store
        .insert_chunk(&mk(
            "validateProvider",
            "function",
            "validate provider authentication login",
            1,
            10,
            "B1",
        ))
        .await
        .unwrap();
    store
        .insert_chunk(&mk(
            "parseConfig",
            "function",
            "parse config yaml settings",
            11,
            20,
            "B2",
        ))
        .await
        .unwrap();
    store
        .insert_chunk(&mk(
            "AuthService",
            "class",
            "authentication service handler",
            21,
            30,
            "B3",
        ))
        .await
        .unwrap();

    // "authentication" matches the two auth chunks, not parseConfig.
    let (hits, total) = store
        .search_chunks_fts(
            "acme/app",
            Some("main"),
            "authentication",
            10,
            false,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(total, 2, "total_count of authentication matches");
    assert_eq!(hits.len(), 2);
    let syms: Vec<&str> = hits
        .iter()
        .filter_map(|h| h.symbol_name.as_deref())
        .collect();
    assert!(syms.contains(&"validateProvider") && syms.contains(&"AuthService"));
    assert!(!syms.contains(&"parseConfig"));

    // kind_filter restricts to the class (R-SEARCH-4); Some(&[]) == None.
    let (cls, ctotal) = store
        .search_chunks_fts(
            "acme/app",
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
            "acme/app",
            Some("main"),
            "authentication",
            10,
            false,
            Some(&[]),
            None,
        )
        .await
        .unwrap();
    assert_eq!(empty_kf.len(), 2, "Some(&[]) behaves like None");

    // Empty/all-special query -> Ok(empty), never error (R-SEARCH-1).
    let (none_hits, none_total) = store
        .search_chunks_fts("acme/app", Some("main"), "   ", 10, false, None, None)
        .await
        .unwrap();
    assert!(none_hits.is_empty() && none_total == 0);

    // Unknown repo/worktree -> empty.
    let (nr, _) = store
        .search_chunks_fts("nope", None, "authentication", 10, false, None, None)
        .await
        .unwrap();
    assert!(nr.is_empty());

    // search_fts_by_id stores exact_mult=3.0 when symbol normalizes to the query
    // (separate-pass model, R-SEARCH-9 — not folded into score).
    let by_id = store
        .search_fts_by_id(repo, Some(wt), "authentication", "validate_provider", 10)
        .await
        .unwrap();
    let vp = by_id
        .iter()
        .find(|h| h.symbol_name.as_deref() == Some("validateProvider"))
        .expect("vp present");
    assert_eq!(vp.exact_mult, Some(3.0));
    let asvc = by_id
        .iter()
        .find(|h| h.symbol_name.as_deref() == Some("AuthService"))
        .expect("authservice present");
    assert_eq!(asvc.exact_mult, Some(1.0));

    // get_chunks_metadata round-trips kind/symbol/recency.
    let ids: Vec<i64> = hits.iter().map(|h| h.chunk_id).collect();
    let meta = store.get_chunks_metadata(&ids).await.unwrap();
    assert_eq!(meta.len(), ids.len());

    eprintln!("fts_live: all FTS assertions passed");
}

/// Phase-2b vector + hybrid verification (spec §6.6, R-SEARCH-5/6/7).
#[tokio::test]
#[ignore]
async fn vector_hybrid_live() {
    let Some(url) = test_url() else {
        eprintln!("skipping vector_hybrid_live: MAPROOM_TEST_PG_URL unset");
        return;
    };
    use crate::db::types::{HybridWeights, SemanticRanking};
    let store = fresh_store(&url).await;
    let repo = store.get_or_create_repo("acme/vec", "/v").await.unwrap();
    let wt = store
        .get_or_create_worktree(repo, "main", "/wt")
        .await
        .unwrap();
    let commit = store.get_or_create_commit(repo, "s", None).await.unwrap();
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wt,
            commit_id: commit,
            relpath: "v.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "h".to_string(),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();

    let mk = |sym: &str, ts: &str, s: i32, e: i32, blob: &str| ChunkRecord {
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
    };
    let ca = store
        .insert_chunk(&mk("alpha", "code search alpha", 1, 5, "EA"))
        .await
        .unwrap();
    let cb = store
        .insert_chunk(&mk("beta", "code search beta", 6, 10, "EB"))
        .await
        .unwrap();
    let cc = store
        .insert_chunk(&mk("gamma", "code search gamma", 11, 15, "EC"))
        .await
        .unwrap();

    // Query Q=[1,0,..]; A identical (dist 0), B dist 0.5, C dist sqrt(2). Distinct order.
    let mut q = vec![0f32; 768];
    q[0] = 1.0;
    let a = q.clone();
    let mut b = vec![0f32; 768];
    b[0] = 0.5;
    let mut c = vec![0f32; 768];
    c[1] = 1.0;
    store.upsert_embedding("EA", &a, "m").await.unwrap();
    store.upsert_embedding("EB", &b, "m").await.unwrap();
    store.upsert_embedding("EC", &c, "m").await.unwrap();

    // Vector KNN ordered by ascending distance: A, B, C (R-SEARCH-5).
    let v = store
        .search_chunks_vector("acme/vec", Some("main"), &q, 10, false, None, None)
        .await
        .unwrap();
    let order: Vec<i64> = v.iter().map(|h| h.chunk_id).collect();
    assert_eq!(
        order,
        vec![ca, cb, cc],
        "vector order by ascending L2 distance"
    );
    assert!(
        v[0].score > v[1].score && v[1].score > v[2].score,
        "similarity descending"
    );
    assert!(
        (v[0].score - 1.0).abs() < 1e-6,
        "identical vector -> similarity 1.0"
    );

    // Degraded mode: a store with vec flag forced false returns empty (R-SEARCH-5).
    let degraded = PostgresStore::with_vec_available(store.pool.clone(), false);
    assert!(degraded
        .search_chunks_vector("acme/vec", Some("main"), &q, 10, false, None, None)
        .await
        .unwrap()
        .is_empty());

    // Hybrid: all three match FTS ("code") AND vector -> source "both" (R-SEARCH-6).
    let h = store
        .search_hybrid(
            "acme/vec",
            Some("main"),
            "code",
            &q,
            10,
            HybridWeights::default(),
        )
        .await
        .unwrap();
    assert_eq!(h.len(), 3);
    assert!(
        h.iter().all(|r| r.source == "both"),
        "fts+vector overlap -> both"
    );
    for w in h.windows(2) {
        assert!(w[0].score >= w[1].score, "hybrid sorted by RRF score desc");
    }

    // search_chunks_hybrid returns SearchHits; search_hybrid_ranked applies semantic ranking.
    let ch = store
        .search_chunks_hybrid("acme/vec", Some("main"), "code", &q, 10, false, None, None)
        .await
        .unwrap();
    assert_eq!(ch.len(), 3);
    let ranked = store
        .search_hybrid_ranked(
            "acme/vec",
            Some("main"),
            "code",
            &q,
            10,
            HybridWeights::default(),
            SemanticRanking::default(),
        )
        .await
        .unwrap();
    assert_eq!(ranked.len(), 3);
    for w in ranked.windows(2) {
        assert!(w[0].score >= w[1].score, "ranked sorted desc");
    }

    eprintln!("vector_hybrid_live: all vector+hybrid assertions passed");
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

/// Phase-3a verification: StoreEncoding lifecycle, StoreCleanup orphan-GC that
/// keeps embeddings + multi-worktree chunks, and batch embedding upsert.
#[tokio::test]
#[ignore]
async fn phase3a_live() {
    let Some(url) = test_url() else {
        eprintln!("skipping phase3a_live: MAPROOM_TEST_PG_URL unset");
        return;
    };
    use crate::db::types::EmbeddingRecord;
    let store = fresh_store(&url).await;

    // ── StoreEncoding lifecycle (§6.4, §5.3 timestamp format) ──
    let run = store
        .create_encoding_run(100, Some("ollama"), Some(768))
        .await
        .unwrap();
    let active = store
        .get_active_encoding_run()
        .await
        .unwrap()
        .expect("active run");
    assert_eq!(active.id, run);
    assert_eq!(active.status, "running");
    assert_eq!(active.total_chunks, 100);
    assert_eq!(active.provider.as_deref(), Some("ollama"));
    assert_eq!(active.dimension, Some(768));
    assert!(
        looks_like_ts(&active.started_at),
        "started_at format: {}",
        active.started_at
    );
    assert!(active.finished_at.is_none());
    store
        .update_encoding_run_progress(run, 42, Some(3.5))
        .await
        .unwrap();
    let active = store.get_active_encoding_run().await.unwrap().unwrap();
    assert_eq!(active.chunks_completed, 42);
    assert!(active
        .last_batch_at
        .as_deref()
        .map(looks_like_ts)
        .unwrap_or(false));
    store.complete_encoding_run(run, "completed").await.unwrap();
    assert!(
        store.get_active_encoding_run().await.unwrap().is_none(),
        "completed -> no active"
    );
    // mark_stale: a fresh running run gets failed.
    let r2 = store.create_encoding_run(5, None, None).await.unwrap();
    assert!(store.get_active_encoding_run().await.unwrap().is_some());
    store.mark_stale_runs_as_failed().await.unwrap();
    assert!(
        store.get_active_encoding_run().await.unwrap().is_none(),
        "stale running -> failed"
    );
    let _ = r2;

    // ── StoreCleanup: orphan GC keeps embeddings + multi-wt chunks (R-WT-4) ──
    let repo = store.get_or_create_repo("acme/clean", "/c").await.unwrap();
    let wa = store
        .get_or_create_worktree(repo, "A", "/wt/A")
        .await
        .unwrap();
    let wb = store
        .get_or_create_worktree(repo, "B", "/wt/B")
        .await
        .unwrap();
    let commit = store.get_or_create_commit(repo, "s", None).await.unwrap();
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wa,
            commit_id: commit,
            relpath: "c.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "h".to_string(),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    let shared = store
        .insert_chunk(&chunk(file, wa, "SHARED", 1, 5))
        .await
        .unwrap();
    store.add_chunk_to_worktree(shared, wb).await.unwrap();
    let only_a = store
        .insert_chunk(&chunk(file, wa, "ONLYA", 6, 10))
        .await
        .unwrap();
    let emb: Vec<f32> = vec![0.01; 768];
    store.upsert_embedding("SHARED", &emb, "m").await.unwrap();
    assert_eq!(store.get_global_embedding_count().await.unwrap(), 1);

    let res = store.delete_worktree_data(wa).await.unwrap();
    assert_eq!(res.embeddings_deleted, 0, "embeddings are kept (R-WT-4)");
    assert!(res.chunks_deleted >= 1, "orphan chunk (ONLYA) removed");
    // SHARED survives in B; ONLYA is gone; embedding pool intact.
    assert_eq!(
        store.get_chunk_worktrees(shared).await.unwrap(),
        vec![wb],
        "shared chunk kept in B"
    );
    assert!(
        store.get_chunk_by_id(only_a).await.unwrap().is_none(),
        "orphan chunk GC'd"
    );
    assert_eq!(
        store.get_global_embedding_count().await.unwrap(),
        1,
        "embedding pool persists"
    );

    // ── StoreCleanup: stale detection by disk existence ──
    let wmissing = store
        .get_or_create_worktree(repo, "missing", "/nonexistent/zzz-xyz")
        .await
        .unwrap();
    let wexists = store
        .get_or_create_worktree(repo, "exists", "/tmp")
        .await
        .unwrap();
    let stale = store.detect_stale_worktrees().await.unwrap();
    let stale_ids: Vec<i64> = stale.iter().map(|s| s.id).collect();
    assert!(stale_ids.contains(&wmissing), "missing path -> stale");
    assert!(!stale_ids.contains(&wexists), "/tmp exists -> not stale");
    assert!(stale.iter().all(|s| !s.exists));

    // ── batch embedding upsert (R-EMB-8) ──
    let batch = vec![
        EmbeddingRecord {
            blob_sha: "BB1".into(),
            embedding: vec![0.02; 768],
            model_version: "m".into(),
        },
        EmbeddingRecord {
            blob_sha: "BB2".into(),
            embedding: vec![0.03; 1024],
            model_version: "m".into(),
        },
    ];
    store.upsert_embeddings_batch_new(&batch).await.unwrap();
    assert!(store.has_embedding("BB1").await.unwrap() && store.has_embedding("BB2").await.unwrap());
    // bad dim fails the whole batch, naming the index.
    let bad = vec![EmbeddingRecord {
        blob_sha: "BAD".into(),
        embedding: vec![0.0; 512],
        model_version: "m".into(),
    }];
    let err = store
        .upsert_embeddings_batch_new(&bad)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("512") && err.contains("768"),
        "batch dim error: {err}"
    );
    assert!(
        !store.has_embedding("BAD").await.unwrap(),
        "bad batch not partially applied"
    );

    eprintln!("phase3a_live: encoding + cleanup + batch assertions passed");
}
