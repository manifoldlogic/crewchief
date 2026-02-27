use criterion::{black_box, criterion_group, criterion_main, Criterion};
use maproom::db::sqlite::SqliteStore;
use maproom::db::{ChunkRecord, FileRecord, VectorStore};
use tempfile::NamedTempFile;
use tokio::runtime::Runtime;

async fn setup_store() -> SqliteStore {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap();
    let url = format!("sqlite://{}", path);

    let store = SqliteStore::connect(&url).await.unwrap();
    store.migrate().await.unwrap();

    // Keep temp_file alive by leaking or returning it?
    // NamedTempFile deletes on drop. We need to keep it alive.
    // For benchmark, we just assume it lives long enough or we use a persistent path.
    // Let's use a deterministic path in /tmp/
    std::mem::forget(temp_file);
    store
}

fn benchmark_indexing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("sqlite_insert_100_chunks", |b| {
        b.to_async(&rt).iter(|| async {
            // We setup a new store for each iteration to measure insert cost?
            // Or measure batch insert?
            // Let's measure batch insert into existing store.
            let store = setup_store().await;

            let chunks: Vec<ChunkRecord> = (0..100)
                .map(|i| ChunkRecord {
                    file_id: 1,
                    blob_sha: format!("blob{}", i),
                    symbol_name: Some(format!("symbol{}", i)),
                    kind: "function".to_string(),
                    signature: None,
                    docstring: None,
                    start_line: i,
                    end_line: i + 10,
                    preview: "code...".to_string(),
                    ts_doc_text: "search me".to_string(),
                    recency_score: 1.0,
                    churn_score: 0.0,
                    metadata: None,
                    worktree_id: 1,
                })
                .collect();

            // Mock file insert first
            store
                .upsert_file(&FileRecord {
                    repo_id: 1,
                    worktree_id: 1,
                    commit_id: 1,
                    relpath: "test.rs".to_string(),
                    language: None,
                    content_hash: "h".to_string(),
                    size_bytes: 100,
                    last_modified: None,
                })
                .await
                .ok();

            black_box(store.insert_chunks_batch(&chunks).await.unwrap());
        })
    });
}

criterion_group!(benches, benchmark_indexing);
criterion_main!(benches);
