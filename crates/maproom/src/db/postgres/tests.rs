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
use crate::db::traits::{StoreChunks, StoreCore, StoreEmbeddings, StoreIndexState, StoreMigration};
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
