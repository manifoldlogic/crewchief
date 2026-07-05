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
    let wt = store.get_or_create_worktree(repo, "main", "/w/main").await.unwrap();
    let commit = store.get_or_create_commit(repo, "deadbeef", None).await.unwrap();
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
    assert_eq!((edge.src_chunk_id, edge.dst_chunk_id, edge.edge_type.as_str()), (ca, cb, "calls"));

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
    assert_eq!(decoded, va, "vector must round-trip bit-exactly through base64/LE bytes");
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
