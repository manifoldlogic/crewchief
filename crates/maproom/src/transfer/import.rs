//! Postgres importer for the F47 migration artifact (`db import`).
//!
//! Streams the NDJSON artifact (in FK-dependency order) into a Postgres backend,
//! remapping source ids to destination ids via NATURAL KEYS (repo name, worktree
//! `(repo,name)`, commit sha, file `(commit,relpath,content_hash)`, chunk
//! `(file,start,end)`, embedding `blob_sha`) — source ids are `AUTOINCREMENT` and
//! must never be inserted verbatim into Postgres `GENERATED ALWAYS AS IDENTITY`
//! columns (§S3.1). The content-addressed embedding pool is merged verbatim by
//! `blob_sha` WITHOUT recompute (§S3.2), deduped against rows other teams may have
//! already imported. `chunk_edges` are re-resolved through the chunk id map.
//! Postgres-gated: using the Postgres backend already requires this build (§S5.2).

use std::collections::HashMap;
use std::io::BufRead;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

use super::{decode_embedding, Record, TransferStats, FORMAT_VERSION};
use crate::db::postgres::PostgresStore;
use crate::db::traits::{StoreChunks, StoreCore, StoreEmbeddings};
use crate::db::types::EmbeddingRecord;
use crate::db::{ChunkRecord, FileRecord};

/// Dims the Postgres pool can store (migration 0004 `RAISE`s otherwise). Off-registry
/// embeddings are skipped-with-report rather than aborting the import (§S3.3).
const SUPPORTED_DIMS: [i64; 3] = [768, 1024, 1536];

/// Flush the embedding buffer to the pool in batches of this size.
const EMBED_BATCH: usize = 500;

#[derive(Debug, Default)]
pub struct ImportReport {
    pub stats: TransferStats,
    /// Embeddings skipped because their dim is not in the Postgres registry.
    pub skipped_bad_dim: u64,
    /// True if the source index was in don't-store-content mode.
    pub source_minimized: bool,
}

/// Parse a SQLite datetime string best-effort for the trait methods that take
/// `Option<DateTime<Utc>>`; `None` if absent or unparseable (these are metadata, not
/// keys — a parse miss must not fail a migration).
fn parse_dt(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|ndt| Utc.from_utc_datetime(&ndt))
}

/// Import a maproom export artifact into a Postgres backend.
pub async fn import_postgres<R: BufRead>(store: &PostgresStore, reader: R) -> Result<ImportReport> {
    let mut repo_map: HashMap<i64, i64> = HashMap::new();
    let mut wt_map: HashMap<i64, i64> = HashMap::new();
    let mut commit_map: HashMap<i64, i64> = HashMap::new();
    let mut file_map: HashMap<i64, i64> = HashMap::new();
    // src_file_id -> src_worktree_id (a chunk's insert worktree = its file's worktree;
    // additional chunk_worktrees mappings are added idempotently below).
    let mut file_src_wt: HashMap<i64, i64> = HashMap::new();
    let mut chunk_map: HashMap<i64, i64> = HashMap::new();
    let mut embed_buf: Vec<EmbeddingRecord> = Vec::new();

    let mut report = ImportReport::default();
    let mut header_seen = false;

    for line in reader.lines() {
        let line = line.context("read artifact line")?;
        if line.trim().is_empty() {
            continue;
        }
        let rec: Record = serde_json::from_str(&line).context("parse artifact record")?;
        match rec {
            Record::Header(h) => {
                if h.format_version != FORMAT_VERSION {
                    bail!(
                        "unsupported artifact format_version {} (this build reads {FORMAT_VERSION})",
                        h.format_version
                    );
                }
                report.source_minimized = h.minimized;
                header_seen = true;
                // If the source was minimized, make the destination sticky-minimized
                // too (and suppress content on every insert below via the cached flag),
                // so the policy carries across the migration.
                if h.minimized {
                    store.set_content_minimized().await?;
                }
            }
            Record::Repo(r) => {
                let dest = store.get_or_create_repo(&r.name, &r.root_path).await?;
                repo_map.insert(r.id, dest);
                report.stats.repos += 1;
            }
            Record::Worktree(w) => {
                let repo = *repo_map.get(&w.repo_id).with_context(|| {
                    format!("worktree {} references unknown repo {}", w.id, w.repo_id)
                })?;
                let dest = store
                    .get_or_create_worktree(repo, &w.name, &w.abs_path)
                    .await?;
                wt_map.insert(w.id, dest);
                report.stats.worktrees += 1;
            }
            Record::Commit(c) => {
                let repo = *repo_map.get(&c.repo_id).with_context(|| {
                    format!("commit {} references unknown repo {}", c.id, c.repo_id)
                })?;
                let ts = c.committed_at.as_deref().and_then(parse_dt);
                let dest = store.get_or_create_commit(repo, &c.sha, ts).await?;
                commit_map.insert(c.id, dest);
                report.stats.commits += 1;
            }
            Record::File(f) => {
                let repo_id = *repo_map
                    .get(&f.repo_id)
                    .context("file references unknown repo")?;
                let worktree_id = *wt_map
                    .get(&f.worktree_id)
                    .context("file references unknown worktree")?;
                let commit_id = *commit_map
                    .get(&f.commit_id)
                    .context("file references unknown commit")?;
                let dest = store
                    .upsert_file(&FileRecord {
                        repo_id,
                        worktree_id,
                        commit_id,
                        relpath: f.relpath,
                        language: f.language,
                        content_hash: f.content_hash,
                        size_bytes: f.size_bytes as i32,
                        last_modified: f.last_modified.as_deref().and_then(parse_dt),
                    })
                    .await?;
                file_map.insert(f.id, dest);
                file_src_wt.insert(f.id, f.worktree_id);
                report.stats.files += 1;
            }
            Record::Chunk(c) => {
                let file_id = *file_map
                    .get(&c.file_id)
                    .context("chunk references unknown file")?;
                let src_wt = *file_src_wt
                    .get(&c.file_id)
                    .context("chunk's file has no recorded worktree")?;
                let worktree_id = *wt_map
                    .get(&src_wt)
                    .context("chunk's file worktree not mapped")?;
                let metadata = match c.metadata.as_deref() {
                    Some(s) => Some(serde_json::from_str(s).context("parse chunk metadata JSON")?),
                    None => None,
                };
                let dest = store
                    .insert_chunk(&ChunkRecord {
                        file_id,
                        blob_sha: c.blob_sha,
                        symbol_name: c.symbol_name,
                        kind: c.kind,
                        signature: c.signature,
                        docstring: c.docstring,
                        start_line: c.start_line as i32,
                        end_line: c.end_line as i32,
                        preview: c.preview,
                        ts_doc_text: c.ts_doc_text.unwrap_or_default(),
                        recency_score: c.recency_score as f32,
                        churn_score: c.churn_score as f32,
                        metadata,
                        worktree_id,
                    })
                    .await?;
                chunk_map.insert(c.id, dest);
                report.stats.chunks += 1;
            }
            Record::Embedding(e) => {
                // Skip-with-report keys off the DECODED length (the byte payload is the
                // source of truth), not the stated `embedding_dim` — a mismatch, a
                // non-multiple-of-4 payload, or an off-registry dim must skip, never
                // abort the whole migration (upsert would `bail!` on a bad length).
                let embedding = match decode_embedding(&e.embedding_b64) {
                    Ok(v) if SUPPORTED_DIMS.contains(&(v.len() as i64)) => v,
                    _ => {
                        report.skipped_bad_dim += 1;
                        continue;
                    }
                };
                embed_buf.push(EmbeddingRecord {
                    blob_sha: e.blob_sha,
                    embedding,
                    model_version: e.model_version,
                });
                report.stats.embeddings += 1;
                if embed_buf.len() >= EMBED_BATCH {
                    store.upsert_embeddings_batch_new(&embed_buf).await?;
                    embed_buf.clear();
                }
            }
            Record::ChunkWorktree(cw) => {
                let chunk_id = *chunk_map
                    .get(&cw.chunk_id)
                    .context("chunk_worktree references unknown chunk")?;
                let worktree_id = *wt_map
                    .get(&cw.worktree_id)
                    .context("chunk_worktree references unknown worktree")?;
                // Idempotent: insert_chunk already mapped the chunk to its file's
                // worktree; this adds any additional (content-shared) worktrees.
                sqlx::query(
                    "INSERT INTO chunk_worktrees (chunk_id, worktree_id) VALUES ($1, $2) \
                     ON CONFLICT DO NOTHING",
                )
                .bind(chunk_id)
                .bind(worktree_id)
                .execute(&store.pool)
                .await?;
                report.stats.chunk_worktrees += 1;
            }
            Record::ChunkEdge(ed) => {
                // Re-resolve source ids through the chunk map (§S3.1) — never verbatim.
                let src = *chunk_map
                    .get(&ed.src_chunk_id)
                    .context("edge references unknown src chunk")?;
                let dst = *chunk_map
                    .get(&ed.dst_chunk_id)
                    .context("edge references unknown dst chunk")?;
                store.insert_chunk_edge(src, dst, &ed.edge_type).await?;
                report.stats.chunk_edges += 1;
            }
            Record::IndexState(is) => {
                let worktree_id = *wt_map
                    .get(&is.worktree_id)
                    .context("index_state references unknown worktree")?;
                sqlx::query(
                    // SQLite datetimes are naive UTC; interpret them as UTC (not the
                    // PG session TimeZone) so migrated timestamps stay consistent.
                    "INSERT INTO index_state \
                     (worktree_id, tree_sha, chunks_processed, embeddings_generated, last_indexed) \
                     VALUES ($1, $2, $3, $4, ($5::timestamp AT TIME ZONE 'UTC')) \
                     ON CONFLICT (worktree_id) DO UPDATE SET \
                         tree_sha = EXCLUDED.tree_sha, \
                         chunks_processed = EXCLUDED.chunks_processed, \
                         embeddings_generated = EXCLUDED.embeddings_generated, \
                         last_indexed = EXCLUDED.last_indexed",
                )
                .bind(worktree_id)
                .bind(&is.tree_sha)
                .bind(is.chunks_processed as i32)
                .bind(is.embeddings_generated as i32)
                .bind(&is.last_indexed)
                .execute(&store.pool)
                .await?;
                report.stats.index_state += 1;
            }
            Record::EncodingRun(er) => {
                sqlx::query(
                    "INSERT INTO encoding_runs \
                     (started_at, finished_at, status, total_chunks, chunks_completed, \
                      chunks_per_second, last_batch_at, provider, dimension) \
                     VALUES (($1::timestamp AT TIME ZONE 'UTC'), ($2::timestamp AT TIME ZONE 'UTC'), \
                      $3, $4, $5, $6, ($7::timestamp AT TIME ZONE 'UTC'), $8, $9)",
                )
                .bind(&er.started_at)
                .bind(er.finished_at.as_deref())
                .bind(&er.status)
                .bind(er.total_chunks)
                .bind(er.chunks_completed)
                .bind(er.chunks_per_second)
                .bind(er.last_batch_at.as_deref())
                .bind(er.provider.as_deref())
                .bind(er.dimension.map(|d| d as i32))
                .execute(&store.pool)
                .await?;
                report.stats.encoding_runs += 1;
            }
        }
    }

    if !header_seen {
        bail!("artifact is missing its header record");
    }
    if !embed_buf.is_empty() {
        store.upsert_embeddings_batch_new(&embed_buf).await?;
    }
    Ok(report)
}
