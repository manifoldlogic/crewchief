//! R-EMB-9 — real-write-path embedding dedup against PostgresStore.
//!
//! The §9.4 parity harness inserts `ChunkRecord`s with a caller-supplied
//! `blob_sha`, so it verifies the dedup *queries* but NOT that the real indexer
//! computes `blob_sha` consistently (writer/reader hash agreement). This test
//! closes that gap: it indexes byte-identical file content into TWO worktrees
//! through the actual indexer (`incremental/processor.rs`) against a real
//! `PostgresStore`, then asserts the content-addressed pool deduplicates —
//! exactly one `code_embeddings` row per content, and the second worktree's
//! chunk is excluded from `fetch_chunks_needing_embeddings(true, None)` once the
//! shared content is embedded (zero recompute, R-WT-2/R-EMB-9).
//!
//! Gated on `--features postgres` AND `MAPROOM_TEST_PG_URL`, `#[ignore]`.
#![cfg(feature = "postgres")]

use std::sync::Arc;

use maproom::db::postgres::PostgresStore;
use maproom::db::Store;
use maproom::incremental::{ChangeType, FileHasher, IncrementalProcessor, Trigger, UpdateTask};

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1)
}

#[tokio::test]
#[ignore]
async fn pg_real_indexer_dedup() {
    let Ok(url) = std::env::var("MAPROOM_TEST_PG_URL") else {
        eprintln!("skipping pg_real_indexer_dedup: MAPROOM_TEST_PG_URL unset");
        return;
    };
    let store: Arc<dyn Store + Send + Sync> =
        Arc::new(PostgresStore::connect(&url).await.expect("pg connect"));

    let id = nanos();
    let marker = format!("dedup_{id}");

    // A unique, chunkable Rust source file (unique symbol/comment => run-unique
    // blob_sha, so assertions hold on a shared, non-reset Postgres database).
    let content = format!(
        "//! module {marker}\n\n\
         /// Doc for {marker}.\n\
         pub fn {marker}(x: i32) -> i32 {{\n    x + 1\n}}\n"
    );

    let tmp = tempfile::tempdir().expect("tempdir");
    let repo_root = tmp.path().to_path_buf();
    let src_dir = repo_root.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let file_path = src_dir.join(format!("{marker}.rs"));
    std::fs::write(&file_path, &content).unwrap();
    let hash = FileHasher::hash_bytes(content.as_bytes());

    // Repo + two worktrees + a commit, all backed by the same file content.
    let repo_id = store
        .get_or_create_repo(&format!("acme/{marker}"), repo_root.to_str().unwrap())
        .await
        .unwrap();
    let wt_a = store
        .get_or_create_worktree(repo_id, "A", repo_root.to_str().unwrap())
        .await
        .unwrap();
    let wt_b = store
        .get_or_create_worktree(repo_id, "B", repo_root.to_str().unwrap())
        .await
        .unwrap();
    // Distinct commits per worktree (as real branches have), so each worktree
    // gets its OWN file row — `upsert_file` dedups on (commit_id, relpath,
    // content_hash). This yields two chunk rows that SHARE a content-addressed
    // blob_sha, which is exactly the cross-worktree dedup R-EMB-9 targets.
    let commit_a = store
        .get_or_create_commit(repo_id, &format!("sha-a-{id}"), None)
        .await
        .unwrap();
    let commit_b = store
        .get_or_create_commit(repo_id, &format!("sha-b-{id}"), None)
        .await
        .unwrap();

    // Index the SAME file content through the REAL indexer into BOTH worktrees.
    for (wt, commit) in [(wt_a, commit_a), (wt_b, commit_b)] {
        let processor =
            IncrementalProcessor::new(store.clone(), repo_root.clone(), repo_id, wt, commit);
        processor
            .process(UpdateTask::new(
                file_path.clone(),
                ChangeType::New(hash),
                Trigger::Auto,
            ))
            .await
            .expect("processor.process");
    }

    // Chunks of MY content that still need an embedding (filter by the marker).
    let before = store
        .fetch_chunks_needing_embeddings(true, None)
        .await
        .unwrap();
    let mine: Vec<_> = before
        .iter()
        .filter(|c| c.preview.contains(&marker))
        .collect();
    assert!(
        !mine.is_empty(),
        "real indexer produced chunks for the test file"
    );

    // Content addressing: the same content in two worktrees yields more chunk
    // ROWS than distinct blob_shas (each content shared across both worktrees).
    let distinct: std::collections::HashSet<&str> =
        mine.iter().map(|c| c.blob_sha.as_str()).collect();
    assert!(
        mine.len() > distinct.len(),
        "two worktrees share content-addressed blob_shas (rows {} > distinct {})",
        mine.len(),
        distinct.len()
    );

    // Embed each distinct content ONCE (simulating worktree A's embedding pass).
    let emb = vec![0.01f32; 768];
    for blob in &distinct {
        store
            .upsert_embedding(blob, &emb, "test-model")
            .await
            .unwrap();
        // UNIQUE(blob_sha): existence implies exactly one code_embeddings row.
        assert!(store.has_embedding(blob).await.unwrap());
    }

    // Zero recompute (R-EMB-9): with the shared content embedded once, NEITHER
    // worktree's chunk is reported as needing an embedding.
    let after = store
        .fetch_chunks_needing_embeddings(true, None)
        .await
        .unwrap();
    let mine_after = after.iter().filter(|c| c.preview.contains(&marker)).count();
    assert_eq!(
        mine_after, 0,
        "after embedding the shared content once, the second worktree's chunk is excluded"
    );

    eprintln!(
        "pg_real_indexer_dedup: {} chunk rows across 2 worktrees deduped to {} content-addressed embedding(s)",
        mine.len(),
        distinct.len()
    );
}
