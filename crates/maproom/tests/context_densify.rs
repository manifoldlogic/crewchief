//! F81/F82 acceptance: `context` assembly is dense and symmetric by DEFAULT.
//!
//! Before this wave the flagship `maproom context` returned the primary
//! chunk only (flags default-off), capped expansion at ONE caller + ONE
//! callee (`.take(1)`), hard-coded depth 1 (silently ignoring --max-depth),
//! and never surfaced import edges. These tests pin the fixed behavior at
//! the strategy level (the exact code path both the CLI and the daemon
//! route through).

use std::sync::Arc;

use maproom::context::{AssemblyStrategy, DefaultAssemblyStrategy, ExpandOptions};
use maproom::db::sqlite::SqliteStore;
use maproom::db::traits::{StoreChunks, StoreCore, StoreGraph, StoreMigration};
use maproom::db::{ChunkRecord, FileRecord, Store};

/// Build a real on-disk worktree + store with a call graph around a primary
/// chunk `p`:
///
/// ```text
///   d ─calls→ a ─calls→ p ─calls→ x
///             b ─calls→ p ─calls→ y
///             c ─calls→ p ─calls→ z
///   m ─imports→ p
/// ```
///
/// Fan-in 3 (a,b,c) + one depth-2 caller (d); fan-out 3 (x,y,z); one
/// incoming import (m). The source file really exists on disk because the
/// assembler loads content through FileLoader.
async fn fixture() -> (tempfile::TempDir, Arc<dyn Store + Send + Sync>, i64) {
    let dir = tempfile::TempDir::new().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    // 100 numbered lines so every chunk range resolves to real content
    let content: String = (1..=100).map(|i| format!("// line {i} of fixture\n")).collect();
    std::fs::write(src_dir.join("lib.rs"), content).unwrap();

    let store = SqliteStore::connect(&format!("{}/ctx.db", dir.path().display()))
        .await
        .unwrap();
    store.migrate().await.unwrap();

    let repo = store
        .get_or_create_repo("acme/ctx", dir.path().to_str().unwrap())
        .await
        .unwrap();
    let wt = store
        .get_or_create_worktree(repo, "main", dir.path().to_str().unwrap())
        .await
        .unwrap();
    let commit = store.get_or_create_commit(repo, "c-ctx", None).await.unwrap();
    let file = store
        .upsert_file(&FileRecord {
            repo_id: repo,
            worktree_id: wt,
            commit_id: commit,
            relpath: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "h-ctx".to_string(),
            size_bytes: 1,
            last_modified: None,
        })
        .await
        .unwrap();

    let mk = |sym: &str, start: i32| ChunkRecord {
        file_id: file,
        worktree_id: wt,
        blob_sha: format!("B-{sym}"),
        symbol_name: Some(sym.to_string()),
        kind: "function".to_string(),
        signature: None,
        docstring: None,
        start_line: start,
        end_line: start + 3,
        preview: format!("fn {sym}() {{}}"),
        ts_doc_text: sym.to_string(),
        recency_score: 1.0,
        churn_score: 0.0,
        metadata: None,
    };
    let p = store.insert_chunk(&mk("primary_fn", 1)).await.unwrap();
    let a = store.insert_chunk(&mk("caller_a", 10)).await.unwrap();
    let b = store.insert_chunk(&mk("caller_b", 20)).await.unwrap();
    let c = store.insert_chunk(&mk("caller_c", 30)).await.unwrap();
    let d = store.insert_chunk(&mk("grand_caller_d", 40)).await.unwrap();
    let x = store.insert_chunk(&mk("callee_x", 50)).await.unwrap();
    let y = store.insert_chunk(&mk("callee_y", 60)).await.unwrap();
    let z = store.insert_chunk(&mk("callee_z", 70)).await.unwrap();
    let m = store.insert_chunk(&mk("importer_m", 80)).await.unwrap();

    for (src, dst) in [(a, p), (b, p), (c, p), (d, a), (p, x), (p, y), (p, z)] {
        store.insert_chunk_edge(src, dst, "calls").await.unwrap();
    }
    store.insert_chunk_edge(m, p, "imports").await.unwrap();

    (dir, Arc::new(store), p)
}

fn roles(bundle: &maproom::context::ContextBundle, role: &str) -> Vec<String> {
    bundle
        .items
        .iter()
        .filter(|i| i.role == role)
        .map(|i| i.reason.clone())
        .collect()
}

/// THE F81 HEADLINE: default options (no flags) return MULTIPLE callers AND
/// MULTIPLE callees AND the import relation — not the bare primary chunk.
#[tokio::test]
async fn default_options_are_dense_and_symmetric() {
    let (_dir, store, p) = fixture().await;
    let assembler = DefaultAssemblyStrategy::new(store);

    let bundle = assembler
        .assemble(p, 8000, ExpandOptions::default())
        .await
        .unwrap();

    let callers = roles(&bundle, "caller");
    let callees = roles(&bundle, "callee");
    let imports = roles(&bundle, "import");
    assert!(
        callers.len() >= 2,
        "default assembly must include MULTIPLE callers (was .take(1)); got {callers:?}"
    );
    assert!(
        callees.len() >= 2,
        "default assembly must include MULTIPLE callees (was .take(1)); got {callees:?}"
    );
    assert!(
        !imports.is_empty(),
        "default assembly must surface import relations (F82); items: {:?}",
        bundle.items.iter().map(|i| (&i.role, &i.reason)).collect::<Vec<_>>()
    );
    assert!(
        bundle.items.iter().any(|i| i.role == "primary"),
        "primary chunk always present"
    );
}

/// F81: --max-depth is honored — depth 2 reaches the caller-of-caller that
/// depth 1 cannot.
#[tokio::test]
async fn max_depth_is_honored() {
    let (_dir, store, p) = fixture().await;
    let assembler = DefaultAssemblyStrategy::new(store);

    let mut d1 = ExpandOptions::default();
    d1.max_depth = 1;
    let shallow = assembler.assemble(p, 8000, d1).await.unwrap();

    let mut d2 = ExpandOptions::default();
    d2.max_depth = 2;
    let deep = assembler.assemble(p, 8000, d2).await.unwrap();

    let shallow_callers = roles(&shallow, "caller");
    let deep_callers = roles(&deep, "caller");
    assert!(
        deep_callers.len() > shallow_callers.len(),
        "depth 2 must reach the transitive caller (grand_caller_d): depth1={shallow_callers:?} depth2={deep_callers:?}"
    );
    assert!(
        deep_callers.iter().any(|r| r.contains("grand_caller_d")),
        "the depth-2 caller must be labeled: {deep_callers:?}"
    );
}

/// F81: suppression flags work — callers can be turned OFF.
#[tokio::test]
async fn suppression_flags_work() {
    let (_dir, store, p) = fixture().await;
    let assembler = DefaultAssemblyStrategy::new(store);

    let mut opts = ExpandOptions::default();
    opts.callers = false;
    let bundle = assembler.assemble(p, 8000, opts).await.unwrap();

    assert!(
        roles(&bundle, "caller").is_empty(),
        "callers=false must suppress the segment"
    );
    assert!(
        roles(&bundle, "callee").len() >= 2,
        "other segments unaffected"
    );
}

/// primary_only() still means primary only (used by cache warm paths).
#[tokio::test]
async fn primary_only_stays_primary_only() {
    let (_dir, store, p) = fixture().await;
    let assembler = DefaultAssemblyStrategy::new(store);
    let bundle = assembler
        .assemble(p, 8000, ExpandOptions::primary_only())
        .await
        .unwrap();
    assert_eq!(bundle.items.len(), 1, "{:?}", bundle.items);
    assert_eq!(bundle.items[0].role, "primary");
}
