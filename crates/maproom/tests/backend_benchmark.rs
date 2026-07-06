//! F75 — SQLite-vs-Postgres search benchmark harness.
//!
//! Runs ONE engineered corpus through BOTH backends and scores each backend's search
//! results with the existing evaluation metric machinery (`calculate_all_metrics`),
//! then emits a colorblind-safe side-by-side quality + latency comparison. This
//! replaces the hollow `golden_test.rs` `execute_search_query -> vec![]` stub with a
//! real dual-backend runner.
//!
//! Test/bench-only: touches no production read/write path. The SQLite arm runs in the
//! default suite; the Postgres arm runs under `--features postgres` + `MAPROOM_TEST_PG_URL`
//! (exactly like `store_parity`). Vector modes self-skip on a backend without an ANN
//! extension (graceful degradation, like `store_parity`'s `check_vector`).
//!
//! Ground truth is by NATURAL KEY (`symbol_name`), never a raw autoincrement chunk_id —
//! the exact fabrication the golden harness got wrong.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use maproom::db::sqlite::SqliteStore;
// `dyn Store` exposes every supertrait method (StoreCore/StoreChunks/StoreEmbeddings/
// StoreSearch); only the concrete `sqlite.migrate()` needs StoreMigration in scope.
use maproom::db::traits::{Store, StoreMigration};
use maproom::db::{ChunkRecord, FileRecord, SearchHit};
use maproom::evaluation::{calculate_all_metrics, EvaluationMetrics, RankedResult};

const DIM: usize = 768;
const TOPICS: &[&str] = &[
    "authenticate",
    "database",
    "parser",
    "network",
    "cache",
    "encryption",
];
const K_VALUES: &[usize] = &[1, 3, 5, 10];
const TOP_K: i64 = 10;

/// A seeded corpus document. Relevance is known by construction and keyed by `sym`.
struct Doc {
    sym: String,
    ts: String,
    emb: Vec<f32>,
}

/// A benchmark query with construction-derived ground truth (`sym -> relevance grade`).
struct Query {
    text: String,
    emb: Vec<f32>,
    relevant: HashMap<String, u8>,
}

fn onehot(i: usize, val: f32) -> Vec<f32> {
    let mut v = vec![0.0f32; DIM];
    v[i] = val;
    v
}

/// Engineered corpus: 6 topics × (4 strong grade-3 + 2 weak grade-1) chunks + 8
/// distractors. Strong chunks carry the topic keyword 3× (FTS) and sit exactly on the
/// topic's one-hot centroid (vector); weak chunks carry it 1× and sit off-centroid;
/// distractors carry no keyword and live in a far dimension. Each query targets one
/// topic; its relevant set is that topic's strong+weak chunks.
fn build_corpus() -> (Vec<Doc>, Vec<Query>) {
    let mut docs = Vec::new();
    let mut queries = Vec::new();
    let n = TOPICS.len();
    for (ti, topic) in TOPICS.iter().enumerate() {
        let centroid = onehot(ti, 1.0);
        let mut relevant = HashMap::new();
        for j in 0..4 {
            let sym = format!("{topic}_strong_{j}");
            docs.push(Doc {
                sym: sym.clone(),
                ts: format!("{topic} {topic} {topic} function implementation"),
                emb: centroid.clone(),
            });
            relevant.insert(sym, 3u8);
        }
        for j in 0..2 {
            let sym = format!("{topic}_weak_{j}");
            let mut e = onehot(ti, 0.7);
            e[(ti + 1) % n] = 0.3;
            docs.push(Doc {
                sym: sym.clone(),
                ts: format!("{topic} helper utility"),
                emb: e,
            });
            relevant.insert(sym, 1u8);
        }
        queries.push(Query {
            text: topic.to_string(),
            emb: centroid,
            relevant,
        });
    }
    for j in 0..8 {
        docs.push(Doc {
            sym: format!("distractor_{j}"),
            ts: "miscellaneous unrelated code block".to_string(),
            emb: onehot(100 + j, 1.0),
        });
    }
    (docs, queries)
}

fn unique_base() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(0);
    format!("{}", C.fetch_add(1, Ordering::SeqCst))
}

/// The proven dual-backend fixture (copied from `store_parity`): SQLite always; Postgres
/// under `--features postgres` + `MAPROOM_TEST_PG_URL`.
async fn backends() -> Vec<(&'static str, Arc<dyn Store + Send + Sync>)> {
    let mut v: Vec<(&'static str, Arc<dyn Store + Send + Sync>)> = Vec::new();
    let mem = format!("file:memdb_bench_{}?mode=memory&cache=shared", unique_base());
    let sqlite = SqliteStore::connect(&mem).await.expect("sqlite connect");
    sqlite.migrate().await.expect("sqlite migrate");
    v.push(("sqlite", Arc::new(sqlite)));

    #[cfg(feature = "postgres")]
    {
        if let Ok(url) = std::env::var("MAPROOM_TEST_PG_URL") {
            // Fresh schema so the benchmark corpus is isolated from other rows.
            let pool = sqlx::postgres::PgPoolOptions::new()
                .connect(&url)
                .await
                .expect("pg pool");
            sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
                .execute(&pool)
                .await
                .expect("reset pg schema");
            pool.close().await;
            let pg = maproom::db::postgres::PostgresStore::connect(&url)
                .await
                .expect("postgres connect");
            v.push(("postgres", Arc::new(pg)));
        } else {
            eprintln!("backend_benchmark: MAPROOM_TEST_PG_URL unset — postgres arm skipped");
        }
    }
    v
}

async fn seed_corpus(store: &(dyn Store + Send + Sync), docs: &[Doc]) -> String {
    let repo_name = format!("bench/corpus-{}", unique_base());
    let repo = store
        .get_or_create_repo(&repo_name, "/bench")
        .await
        .unwrap();
    let wt = store
        .get_or_create_worktree(repo, "main", "/bench/main")
        .await
        .unwrap();
    let commit = store.get_or_create_commit(repo, "benchsha", None).await.unwrap();
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wt,
            commit_id: commit,
            relpath: "bench.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "bh".to_string(),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();
    for (i, d) in docs.iter().enumerate() {
        let blob = format!("blob_{}", d.sym);
        store
            .insert_chunk(&ChunkRecord {
                file_id: file,
                blob_sha: blob.clone(),
                symbol_name: Some(d.sym.clone()),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
                start_line: (i * 10 + 1) as i32,
                end_line: (i * 10 + 5) as i32,
                preview: d.ts.clone(),
                ts_doc_text: d.ts.clone(),
                recency_score: 1.0,
                churn_score: 0.0,
                metadata: None,
                worktree_id: wt,
            })
            .await
            .unwrap();
        store.upsert_embedding(&blob, &d.emb, "bench-model").await.unwrap();
    }
    // Populate the SQLite vec0 tables (no-op on Postgres).
    store.sync_all_embeddings_to_vec().await.unwrap();
    repo_name
}

/// Score one backend's ranked hits against a query's natural-key ground truth.
fn score(hits: &[SearchHit], relevant: &HashMap<String, u8>) -> EvaluationMetrics {
    let ranked: Vec<RankedResult> = hits
        .iter()
        .map(|h| {
            let grade = h
                .symbol_name
                .as_deref()
                .and_then(|s| relevant.get(s))
                .copied()
                .unwrap_or(0);
            RankedResult {
                id: h.chunk_id,
                relevant: grade > 0,
                relevance_grade: grade,
            }
        })
        .collect();
    calculate_all_metrics(&ranked, relevant.len(), K_VALUES)
}

fn avg_metrics(ms: &[EvaluationMetrics]) -> EvaluationMetrics {
    let n = ms.len().max(1) as f64;
    let mut p = HashMap::new();
    let mut r = HashMap::new();
    let mut nd = HashMap::new();
    for &k in K_VALUES {
        p.insert(k, ms.iter().map(|m| m.precision_at_k[&k]).sum::<f64>() / n);
        r.insert(k, ms.iter().map(|m| m.recall_at_k[&k]).sum::<f64>() / n);
        nd.insert(k, ms.iter().map(|m| m.ndcg_at_k[&k]).sum::<f64>() / n);
    }
    let mrr = ms.iter().map(|m| m.mrr).sum::<f64>() / n;
    EvaluationMetrics {
        precision_at_k: p,
        recall_at_k: r,
        ndcg_at_k: nd,
        mrr,
    }
}

fn pct(mut lat: Vec<Duration>, q: f64) -> Duration {
    if lat.is_empty() {
        return Duration::ZERO;
    }
    lat.sort_unstable();
    let idx = ((lat.len() as f64 - 1.0) * q).round() as usize;
    lat[idx]
}

/// One (backend, mode) result row.
struct Row {
    backend: &'static str,
    mode: &'static str,
    metrics: EvaluationMetrics,
    p50: Duration,
    p95: Duration,
    n_queries: usize,
}

/// Run FTS + vector (if the backend has an ANN extension) over all queries.
async fn run_backend(
    name: &'static str,
    store: &(dyn Store + Send + Sync),
    repo: &str,
    queries: &[Query],
) -> Vec<Row> {
    let mut rows = Vec::new();

    // FTS
    let (mut fts_ms, mut fts_lat) = (Vec::new(), Vec::new());
    for q in queries {
        let t = Instant::now();
        let (hits, _) = store
            .search_chunks_fts(repo, Some("main"), &q.text, TOP_K, false, None, None)
            .await
            .unwrap();
        fts_lat.push(t.elapsed());
        fts_ms.push(score(&hits, &q.relevant));
    }
    rows.push(Row {
        backend: name,
        mode: "fts",
        metrics: avg_metrics(&fts_ms),
        p50: pct(fts_lat.clone(), 0.5),
        p95: pct(fts_lat, 0.95),
        n_queries: queries.len(),
    });

    // Vector — skip if the backend has no ANN extension (graceful degradation).
    if store.has_vector_extension() {
        let (mut v_ms, mut v_lat) = (Vec::new(), Vec::new());
        for q in queries {
            let t = Instant::now();
            let hits = store
                .search_chunks_vector(repo, Some("main"), &q.emb, TOP_K, false, None, None)
                .await
                .unwrap();
            v_lat.push(t.elapsed());
            v_ms.push(score(&hits, &q.relevant));
        }
        rows.push(Row {
            backend: name,
            mode: "vector",
            metrics: avg_metrics(&v_ms),
            p50: pct(v_lat.clone(), 0.5),
            p95: pct(v_lat, 0.95),
            n_queries: queries.len(),
        });
    } else {
        eprintln!("  [{name}] vector: SKIPPED (no ANN extension)");
    }

    rows
}

fn render_report(rows: &[Row], pg_present: bool) {
    eprintln!("\n===== F75 backend search benchmark (engineered corpus) =====");
    eprintln!(
        "corpus: {} topics × (4 strong + 2 weak) + 8 distractors; {} queries; k={:?}",
        TOPICS.len(),
        rows.first().map(|r| r.n_queries).unwrap_or(0),
        K_VALUES
    );
    if !pg_present {
        eprintln!("NOTE: Postgres arm SKIPPED (MAPROOM_TEST_PG_URL unset / non-postgres build).");
    }
    eprintln!(
        "\n{:<10} {:<8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>9} {:>9}",
        "backend", "mode", "P@1", "P@5", "R@5", "nDCG@5", "MRR", "lat_p50", "lat_p95"
    );
    eprintln!("{}", "-".repeat(84));
    for r in rows {
        eprintln!(
            "{:<10} {:<8} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>7.1?} {:>7.1?}",
            r.backend,
            r.mode,
            r.metrics.precision_at_k[&1],
            r.metrics.precision_at_k[&5],
            r.metrics.recall_at_k[&5],
            r.metrics.ndcg_at_k[&5],
            r.metrics.mrr,
            r.p50,
            r.p95,
        );
    }

    // Side-by-side deltas (Postgres − SQLite) per mode.
    if pg_present {
        eprintln!("\n----- quality delta (Postgres − SQLite), per mode -----");
        for mode in ["fts", "vector"] {
            let sq = rows.iter().find(|r| r.backend == "sqlite" && r.mode == mode);
            let pg = rows.iter().find(|r| r.backend == "postgres" && r.mode == mode);
            if let (Some(s), Some(p)) = (sq, pg) {
                eprintln!(
                    "  {:<8} nDCG@5 Δ {:+.3}   MRR Δ {:+.3}   R@5 Δ {:+.3}",
                    mode,
                    p.metrics.ndcg_at_k[&5] - s.metrics.ndcg_at_k[&5],
                    p.metrics.mrr - s.metrics.mrr,
                    p.metrics.recall_at_k[&5] - s.metrics.recall_at_k[&5],
                );
            }
        }
    }
    eprintln!("============================================================\n");
}

/// F75 — run the corpus through both backends, report the comparison, and assert
/// tolerant quality thresholds (never exact-order on the approximate PG vector path).
#[tokio::test]
async fn backend_search_benchmark() {
    let (docs, queries) = build_corpus();
    let bes = backends().await;
    let pg_present = bes.iter().any(|(n, _)| *n == "postgres");

    let mut rows = Vec::new();
    for (name, store) in &bes {
        let repo = seed_corpus(store.as_ref(), &docs).await;
        rows.extend(run_backend(name, store.as_ref(), &repo, &queries).await);
    }

    render_report(&rows, pg_present);

    // ── Assertions (O1 = report + TOLERANT thresholds; no exact-order on PG). ──
    let get = |backend: &str, mode: &str| rows.iter().find(|r| r.backend == backend && r.mode == mode);

    // SQLite FTS must rank the keyword-bearing topic chunks well.
    let sq_fts = get("sqlite", "fts").expect("sqlite fts row");
    assert!(
        sq_fts.metrics.recall_at_k[&10] > 0.9,
        "SQLite FTS should retrieve nearly all relevant chunks in top-10 (got R@10={:.3})",
        sq_fts.metrics.recall_at_k[&10]
    );
    assert!(
        sq_fts.metrics.ndcg_at_k[&5] > 0.7,
        "SQLite FTS nDCG@5 should be high on the engineered corpus (got {:.3})",
        sq_fts.metrics.ndcg_at_k[&5]
    );

    // SQLite vector (if the extension is present) ranks the on-centroid strong chunks top.
    if let Some(sq_vec) = get("sqlite", "vector") {
        assert!(
            sq_vec.metrics.ndcg_at_k[&5] > 0.8,
            "SQLite vector nDCG@5 should be high (strong chunks sit on the centroid; got {:.3})",
            sq_vec.metrics.ndcg_at_k[&5]
        );
    }

    // Postgres, when present, must be WITHIN TOLERANCE of SQLite (membership/quality, not
    // exact order — PG vector is approximate by the documented HNSW design).
    if pg_present {
        const TOL: f64 = 0.15;
        for mode in ["fts", "vector"] {
            if let (Some(s), Some(p)) = (get("sqlite", mode), get("postgres", mode)) {
                assert!(
                    p.metrics.ndcg_at_k[&5] >= s.metrics.ndcg_at_k[&5] - TOL,
                    "Postgres {mode} nDCG@5 ({:.3}) must be within {TOL} of SQLite ({:.3})",
                    p.metrics.ndcg_at_k[&5],
                    s.metrics.ndcg_at_k[&5]
                );
                assert!(
                    p.metrics.recall_at_k[&10] >= s.metrics.recall_at_k[&10] - TOL,
                    "Postgres {mode} R@10 ({:.3}) must be within {TOL} of SQLite ({:.3})",
                    p.metrics.recall_at_k[&10],
                    s.metrics.recall_at_k[&10]
                );
            }
        }
    }
}

/// Metric-machinery sanity pin (deterministic, backend-independent, DoD #4): an ideal
/// ranking scores nDCG 1.0 and MRR 1.0.
#[test]
fn metric_machinery_ideal_ranking() {
    let ideal = vec![
        RankedResult { id: 1, relevant: true, relevance_grade: 3 },
        RankedResult { id: 2, relevant: true, relevance_grade: 2 },
        RankedResult { id: 3, relevant: true, relevance_grade: 1 },
        RankedResult { id: 4, relevant: false, relevance_grade: 0 },
    ];
    let m = calculate_all_metrics(&ideal, 3, K_VALUES);
    assert!((m.ndcg_at_k[&5] - 1.0).abs() < 1e-9, "ideal ranking scores nDCG 1.0");
    assert!((m.mrr - 1.0).abs() < 1e-9, "top-ranked relevant → MRR 1.0");
    assert!((m.recall_at_k[&5] - 1.0).abs() < 1e-9, "all 3 relevant retrieved → recall 1.0");
}
