//! Phase 1/2 verification: the SQLite export reader + NDJSON artifact.

use super::*;
use crate::db::sqlite::SqliteStore;
use crate::db::traits::{StoreChunks, StoreCore, StoreEmbeddings, StoreMigration};
use crate::db::{ChunkRecord, FileRecord};

/// Shared-cache in-memory URL so every pooled connection shares one DB (plain
/// `:memory:` gives each pooled connection its own — see store_parity.rs).
async fn mem_store(tag: &str) -> SqliteStore {
    let url = format!("file:memdb_transfer_{tag}?mode=memory&cache=shared");
    let store = SqliteStore::connect(&url).await.expect("sqlite connect");
    store.migrate().await.expect("sqlite migrate");
    store
}

fn parse_records(bytes: &[u8]) -> Vec<Record> {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Record>(l).expect("parse NDJSON record"))
        .collect()
}

/// Spec §S1 Gherkin: the export reader yields reinsertable rows for every core
/// entity, with full pool metadata, and vectors round-trip bit-exactly.
#[tokio::test]
async fn export_sqlite_full_roundtrip() {
    let store = mem_store("export_roundtrip").await;

    // Seed a minimal but complete graph.
    let repo = store.get_or_create_repo("acme/widget", "/w").await.unwrap();
    let wt = store
        .get_or_create_worktree(repo, "main", "/w/main")
        .await
        .unwrap();
    let commit = store
        .get_or_create_commit(repo, "deadbeef", None)
        .await
        .unwrap();
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wt,
            commit_id: commit,
            relpath: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "ch1".to_string(),
            size_bytes: 42,
            last_modified: None,
        })
        .await
        .unwrap();

    let mk = |sym: &str, s: i32, e: i32, blob: &str| ChunkRecord {
        file_id: file,
        blob_sha: blob.to_string(),
        symbol_name: Some(sym.to_string()),
        kind: "function".to_string(),
        signature: Some(format!("fn {sym}()")),
        docstring: None,
        start_line: s,
        end_line: e,
        preview: format!("fn {sym}() {{}}"),
        ts_doc_text: format!("{sym} function"),
        recency_score: 1.0,
        churn_score: 0.0,
        metadata: None,
        worktree_id: wt,
    };
    let ca = store.insert_chunk(&mk("alpha", 1, 5, "BA")).await.unwrap();
    let cb = store.insert_chunk(&mk("beta", 6, 10, "BB")).await.unwrap();
    store.insert_chunk_edge(ca, cb, "calls").await.unwrap();

    // Embeddings at a supported dim, with a value not exactly representable in short
    // decimal (0.1) to prove the base64/LE-bytes round-trip is bit-exact, not decimal.
    let mut va = vec![0.0f32; 768];
    va[0] = 0.1;
    va[1] = -0.3;
    va[767] = 0.5;
    let mut vb = vec![0.0f32; 768];
    vb[0] = 0.2;
    store.upsert_embedding("BA", &va, "model-x").await.unwrap();
    store.upsert_embedding("BB", &vb, "model-x").await.unwrap();

    // Export.
    let (buf, stats) = export_sqlite(&store, Vec::<u8>::new()).await.unwrap();
    let recs = parse_records(&buf);

    // Header first, correct version + backend.
    match &recs[0] {
        Record::Header(h) => {
            assert_eq!(h.format_version, FORMAT_VERSION);
            assert_eq!(h.source_backend, "sqlite");
            assert!(!h.minimized);
        }
        other => panic!("first record must be Header, got {other:?}"),
    }

    // Counts.
    let expected = TransferStats {
        repos: 1,
        worktrees: 1,
        commits: 1,
        files: 1,
        chunks: 2,
        embeddings: 2,
        chunk_worktrees: 2,
        chunk_edges: 1,
        index_state: 0,
        encoding_runs: 0,
    };
    assert_eq!(stats, expected, "export stats");

    // Cross-check stats against the parsed stream.
    let count = |pred: &dyn Fn(&Record) -> bool| recs.iter().filter(|r| pred(r)).count() as u64;
    assert_eq!(count(&|r| matches!(r, Record::Repo(_))), 1);
    assert_eq!(count(&|r| matches!(r, Record::Chunk(_))), 2);
    assert_eq!(count(&|r| matches!(r, Record::Embedding(_))), 2);
    assert_eq!(count(&|r| matches!(r, Record::ChunkWorktree(_))), 2);
    assert_eq!(count(&|r| matches!(r, Record::ChunkEdge(_))), 1);

    // The edge references the two chunk source-ids (import re-resolves these).
    let edge = recs
        .iter()
        .find_map(|r| match r {
            Record::ChunkEdge(e) => Some(e),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        (
            edge.src_chunk_id,
            edge.dst_chunk_id,
            edge.edge_type.as_str()
        ),
        (ca, cb, "calls")
    );

    // Bit-exact embedding round-trip for blob BA.
    let ea = recs
        .iter()
        .find_map(|r| match r {
            Record::Embedding(e) if e.blob_sha == "BA" => Some(e),
            _ => None,
        })
        .unwrap();
    assert_eq!(ea.embedding_dim, 768);
    assert_eq!(ea.model_version, "model-x");
    let decoded = decode_embedding(&ea.embedding_b64).unwrap();
    assert_eq!(decoded.len(), 768);
    assert_eq!(
        decoded, va,
        "vector must round-trip bit-exactly through base64/LE bytes"
    );
    // Explicit bit check on the non-decimal-exact lane.
    assert_eq!(decoded[0].to_bits(), 0.1f32.to_bits());
}

/// Empty index exports just a header with all-zero counts.
#[tokio::test]
async fn export_empty_index() {
    let store = mem_store("export_empty").await;
    let (buf, stats) = export_sqlite(&store, Vec::<u8>::new()).await.unwrap();
    let recs = parse_records(&buf);
    assert_eq!(recs.len(), 1);
    assert!(matches!(recs[0], Record::Header(_)));
    assert_eq!(stats, TransferStats::default());
}

/// Spec §S3 / §S5.3 (PG-gated): a full SQLite → export → import → Postgres round-trip
/// preserves entity counts and the embedding vector, with NO recompute. Gated on
/// `MAPROOM_TEST_PG_URL`; run with `--features postgres -- --ignored --test-threads=1`.
#[cfg(feature = "postgres")]
#[tokio::test]
#[ignore]
async fn sqlite_to_postgres_roundtrip() {
    use crate::db::postgres::PostgresStore;
    use sqlx::postgres::PgPoolOptions;

    let Ok(pg_url) = std::env::var("MAPROOM_TEST_PG_URL") else {
        eprintln!("skipping sqlite_to_postgres_roundtrip: MAPROOM_TEST_PG_URL unset");
        return;
    };

    // 1. Build a SQLite source index with two chunks, an edge, and embeddings.
    let src = mem_store("roundtrip_src").await;
    let repo = src.get_or_create_repo("acme/rt", "/rt").await.unwrap();
    let wt = src
        .get_or_create_worktree(repo, "main", "/rt/main")
        .await
        .unwrap();
    let commit = src
        .get_or_create_commit(repo, "cafe1234", None)
        .await
        .unwrap();
    let file = src
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wt,
            commit_id: commit,
            relpath: "src/m.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "chz".to_string(),
            size_bytes: 10,
            last_modified: None,
        })
        .await
        .unwrap();
    let mk = |sym: &str, s: i32, e: i32, blob: &str| ChunkRecord {
        file_id: file,
        blob_sha: blob.to_string(),
        symbol_name: Some(sym.to_string()),
        kind: "function".to_string(),
        signature: None,
        docstring: None,
        start_line: s,
        end_line: e,
        preview: format!("fn {sym}"),
        ts_doc_text: sym.to_string(),
        recency_score: 1.0,
        churn_score: 0.0,
        metadata: None,
        worktree_id: wt,
    };
    let ca = src.insert_chunk(&mk("aa", 1, 3, "RBA")).await.unwrap();
    let cb = src.insert_chunk(&mk("bb", 4, 6, "RBB")).await.unwrap();
    src.insert_chunk_edge(ca, cb, "calls").await.unwrap();
    let mut va = vec![0.0f32; 768];
    va[0] = 0.1;
    va[5] = -0.7;
    src.upsert_embedding("RBA", &va, "m").await.unwrap();
    src.upsert_embedding("RBB", &vec![0.25f32; 768], "m")
        .await
        .unwrap();

    let (buf, ex_stats) = export_sqlite(&src, Vec::<u8>::new()).await.unwrap();

    // 2. Fresh Postgres target (drop + reconnect re-applies migrations, incl. 0004).
    let pool = PgPoolOptions::new().connect(&pg_url).await.unwrap();
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let pg = PostgresStore::connect(&pg_url).await.unwrap();

    // 3. Import.
    let report = super::import::import_postgres(&pg, std::io::Cursor::new(buf))
        .await
        .unwrap();

    // 4. Counts match the export exactly (nothing dropped, nothing recomputed).
    assert_eq!(report.stats, ex_stats, "import stats == export stats");
    assert_eq!(report.skipped_bad_dim, 0);
    assert_eq!(pg.get_global_chunk_count().await.unwrap(), 2);
    assert_eq!(pg.get_global_embedding_count().await.unwrap(), 2);

    // 5. The migrated embedding is preserved (f32-exact in practice; tight tolerance
    // guards against any pgvector text-format edge). The artifact itself is bit-exact
    // (see export_sqlite_full_roundtrip).
    let got = pg.get_embedding("RBA").await.unwrap().unwrap();
    assert_eq!(got.len(), 768);
    for (a, b) in va.iter().zip(&got) {
        assert!((a - b).abs() < 1e-6, "embedding preserved: {a} vs {b}");
    }

    // 6. Idempotent: a second import of the same artifact adds no duplicate rows.
    let (buf2, _) = export_sqlite(&src, Vec::<u8>::new()).await.unwrap();
    super::import::import_postgres(&pg, std::io::Cursor::new(buf2))
        .await
        .unwrap();
    assert_eq!(
        pg.get_global_chunk_count().await.unwrap(),
        2,
        "re-import is idempotent"
    );
    assert_eq!(pg.get_global_embedding_count().await.unwrap(), 2);

    eprintln!("sqlite_to_postgres_roundtrip: counts + vector preserved, idempotent");
}
