//! Portable data transfer between maproom backends (F47).
//!
//! A versioned NDJSON artifact that `maproom db export` writes from a SQLite index
//! and `maproom db import` loads into a Postgres backend, moving the
//! content-addressed embedding pool WITHOUT recompute (the adoption wall the tool
//! removes). Export runs from the shipped (SQLite-only) binary; import is
//! `--features postgres`-gated (using the Postgres backend already requires that
//! build, so this is no new burden — spec S5.2).
//!
//! Wire format: one JSON object per line (NDJSON), internally tagged by `t`. Rows
//! are emitted in FK-dependency order so a streaming importer can insert as it
//! reads. Embedding vectors are carried as base64 of the raw source bytes — for
//! SQLite that is the little-endian f32 BLOB verbatim — so a round-trip is
//! bit-exact (no decimal-text drift). The record structs are artifact-local (NOT
//! the internal DB types) so the on-disk schema is stable across internal refactors;
//! bump `FORMAT_VERSION` on any breaking change.

use std::io::Write;

use anyhow::{Context, Result};
use base64ct::{Base64, Encoding};
use serde::{Deserialize, Serialize};

use crate::db::sqlite::SqliteStore;

/// The Postgres importer (`db import`). Gated: using the Postgres backend already
/// requires this build, so import being postgres-only is no new burden (§S5.2).
#[cfg(feature = "postgres")]
pub mod import;

/// Artifact format version. Bump on any breaking change to the record schema.
pub const FORMAT_VERSION: u32 = 1;

/// One line of the export artifact, internally tagged by `t`. Emitted in
/// FK-dependency order: Header → repos → worktrees → commits → files → chunks →
/// embeddings → chunk_worktrees → chunk_edges → index_state → encoding_runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Record {
    Header(Header),
    Repo(RepoRow),
    Worktree(WorktreeRow),
    Commit(CommitRow),
    File(FileRow),
    Chunk(ChunkRow),
    Embedding(EmbeddingRow),
    ChunkWorktree(ChunkWorktreeRow),
    ChunkEdge(ChunkEdgeRow),
    IndexState(IndexStateRow),
    EncodingRun(EncodingRunRow),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub format_version: u32,
    pub source_backend: String,
    pub maproom_version: String,
    /// True when the source index was in don't-store-content mode (F48): content
    /// fields will be absent. Import records this so it never assumes content.
    #[serde(default)]
    pub minimized: bool,
}

/// Source ids are carried so import can build source→dest id maps; import remaps
/// via natural keys (§S3.1) and MUST NOT insert these ids verbatim into Postgres.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRow {
    pub id: i64,
    pub name: String,
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeRow {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub abs_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRow {
    pub id: i64,
    pub repo_id: i64,
    pub sha: String,
    pub committed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRow {
    pub id: i64,
    pub repo_id: i64,
    pub worktree_id: i64,
    pub commit_id: i64,
    pub relpath: String,
    pub language: Option<String>,
    pub content_hash: String,
    pub size_bytes: i64,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRow {
    pub id: i64,
    pub file_id: i64,
    pub blob_sha: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i64,
    pub end_line: i64,
    pub preview: String,
    pub ts_doc_text: Option<String>,
    pub recency_score: f64,
    pub churn_score: f64,
    /// `chunks.metadata` JSON, carried as its raw text.
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRow {
    pub blob_sha: String,
    /// Base64 of the raw source embedding bytes (SQLite: little-endian f32 BLOB,
    /// verbatim) — bit-exact, no decimal-text drift.
    pub embedding_b64: String,
    pub embedding_dim: i64,
    pub model_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkWorktreeRow {
    pub chunk_id: i64,
    pub worktree_id: i64,
}

/// `chunk_edges` carries source chunk ids; import RE-RESOLVES them through the
/// chunk id map (never copies verbatim — §S3.1). The edge `id` is not carried.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkEdgeRow {
    pub src_chunk_id: i64,
    pub dst_chunk_id: i64,
    pub edge_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStateRow {
    pub worktree_id: i64,
    pub tree_sha: String,
    pub chunks_processed: i64,
    pub embeddings_generated: i64,
    pub last_indexed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingRunRow {
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub total_chunks: i64,
    pub chunks_completed: i64,
    pub chunks_per_second: Option<f64>,
    pub last_batch_at: Option<String>,
    pub provider: Option<String>,
    pub dimension: Option<i64>,
}

/// Per-entity counts written, for the CLI report + the round-trip parity check.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferStats {
    pub repos: u64,
    pub worktrees: u64,
    pub commits: u64,
    pub files: u64,
    pub chunks: u64,
    pub embeddings: u64,
    pub chunk_worktrees: u64,
    pub chunk_edges: u64,
    pub index_state: u64,
    pub encoding_runs: u64,
}

/// Decode an `EmbeddingRow`'s base64 payload back to little-endian f32 lanes.
/// Errors if the byte length is not a multiple of 4. Bit-exact inverse of the
/// SQLite BLOB encoding.
pub fn decode_embedding(b64: &str) -> Result<Vec<f32>> {
    let bytes = Base64::decode_vec(b64).map_err(|e| anyhow::anyhow!("bad base64: {e}"))?;
    if bytes.len() % 4 != 0 {
        anyhow::bail!("embedding byte length {} is not a multiple of 4", bytes.len());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

fn write_rec<W: Write>(w: &mut W, rec: &Record) -> Result<()> {
    serde_json::to_writer(&mut *w, rec).context("serialize export record")?;
    w.write_all(b"\n").context("write export record")?;
    Ok(())
}

/// Export a SQLite index to the NDJSON artifact `w`, streaming row-by-row through a
/// single connection (bounded memory, §S1.3). Returns `w` (so a caller can flush an
/// owned writer) plus per-entity counts. Runs on the shipped SQLite-only binary.
pub async fn export_sqlite<W: Write + Send + 'static>(
    store: &SqliteStore,
    mut w: W,
) -> Result<(W, TransferStats)> {
    let maproom_version = env!("CARGO_PKG_VERSION").to_string();
    store
        .run(move |conn| {
            let mut s = TransferStats::default();

            write_rec(
                &mut w,
                &Record::Header(Header {
                    format_version: FORMAT_VERSION,
                    source_backend: "sqlite".to_string(),
                    maproom_version,
                    minimized: false,
                }),
            )?;

            {
                let mut stmt = conn.prepare("SELECT id, name, root_path FROM repos ORDER BY id")?;
                let rows = stmt.query_map([], |r| {
                    Ok(RepoRow { id: r.get(0)?, name: r.get(1)?, root_path: r.get(2)? })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::Repo(row?))?;
                    s.repos += 1;
                }
            }
            {
                let mut stmt = conn
                    .prepare("SELECT id, repo_id, name, abs_path FROM worktrees ORDER BY id")?;
                let rows = stmt.query_map([], |r| {
                    Ok(WorktreeRow {
                        id: r.get(0)?,
                        repo_id: r.get(1)?,
                        name: r.get(2)?,
                        abs_path: r.get(3)?,
                    })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::Worktree(row?))?;
                    s.worktrees += 1;
                }
            }
            {
                let mut stmt = conn
                    .prepare("SELECT id, repo_id, sha, committed_at FROM commits ORDER BY id")?;
                let rows = stmt.query_map([], |r| {
                    Ok(CommitRow {
                        id: r.get(0)?,
                        repo_id: r.get(1)?,
                        sha: r.get(2)?,
                        committed_at: r.get(3)?,
                    })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::Commit(row?))?;
                    s.commits += 1;
                }
            }
            {
                let mut stmt = conn.prepare(
                    "SELECT id, repo_id, worktree_id, commit_id, relpath, language, \
                     content_hash, size_bytes, last_modified FROM files ORDER BY id",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok(FileRow {
                        id: r.get(0)?,
                        repo_id: r.get(1)?,
                        worktree_id: r.get(2)?,
                        commit_id: r.get(3)?,
                        relpath: r.get(4)?,
                        language: r.get(5)?,
                        content_hash: r.get(6)?,
                        size_bytes: r.get(7)?,
                        last_modified: r.get(8)?,
                    })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::File(row?))?;
                    s.files += 1;
                }
            }
            {
                let mut stmt = conn.prepare(
                    "SELECT id, file_id, blob_sha, symbol_name, kind, signature, docstring, \
                     start_line, end_line, preview, ts_doc_text, recency_score, churn_score, \
                     metadata FROM chunks ORDER BY id",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok(ChunkRow {
                        id: r.get(0)?,
                        file_id: r.get(1)?,
                        blob_sha: r.get(2)?,
                        symbol_name: r.get(3)?,
                        kind: r.get(4)?,
                        signature: r.get(5)?,
                        docstring: r.get(6)?,
                        start_line: r.get(7)?,
                        end_line: r.get(8)?,
                        preview: r.get(9)?,
                        ts_doc_text: r.get(10)?,
                        recency_score: r.get(11)?,
                        churn_score: r.get(12)?,
                        metadata: r.get(13)?,
                    })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::Chunk(row?))?;
                    s.chunks += 1;
                }
            }
            {
                let mut stmt = conn.prepare(
                    "SELECT blob_sha, embedding, embedding_dim, model_version \
                     FROM code_embeddings ORDER BY id",
                )?;
                let rows = stmt.query_map([], |r| {
                    let blob: Option<Vec<u8>> = r.get(1)?;
                    Ok((r.get::<_, String>(0)?, blob, r.get::<_, i64>(2)?, r.get::<_, String>(3)?))
                })?;
                for row in rows {
                    let (blob_sha, blob, embedding_dim, model_version) = row?;
                    // Skip pool rows with no stored vector (nothing to move).
                    let Some(bytes) = blob else { continue };
                    write_rec(
                        &mut w,
                        &Record::Embedding(EmbeddingRow {
                            blob_sha,
                            embedding_b64: Base64::encode_string(&bytes),
                            embedding_dim,
                            model_version,
                        }),
                    )?;
                    s.embeddings += 1;
                }
            }
            {
                let mut stmt = conn
                    .prepare("SELECT chunk_id, worktree_id FROM chunk_worktrees ORDER BY chunk_id")?;
                let rows = stmt.query_map([], |r| {
                    Ok(ChunkWorktreeRow { chunk_id: r.get(0)?, worktree_id: r.get(1)? })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::ChunkWorktree(row?))?;
                    s.chunk_worktrees += 1;
                }
            }
            {
                let mut stmt = conn.prepare(
                    "SELECT src_chunk_id, dst_chunk_id, \"type\" FROM chunk_edges ORDER BY id",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok(ChunkEdgeRow {
                        src_chunk_id: r.get(0)?,
                        dst_chunk_id: r.get(1)?,
                        edge_type: r.get(2)?,
                    })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::ChunkEdge(row?))?;
                    s.chunk_edges += 1;
                }
            }
            {
                let mut stmt = conn.prepare(
                    "SELECT worktree_id, tree_sha, chunks_processed, embeddings_generated, \
                     last_indexed FROM index_state ORDER BY worktree_id",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok(IndexStateRow {
                        worktree_id: r.get(0)?,
                        tree_sha: r.get(1)?,
                        chunks_processed: r.get(2)?,
                        embeddings_generated: r.get(3)?,
                        last_indexed: r.get(4)?,
                    })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::IndexState(row?))?;
                    s.index_state += 1;
                }
            }
            {
                let mut stmt = conn.prepare(
                    "SELECT started_at, finished_at, status, total_chunks, chunks_completed, \
                     chunks_per_second, last_batch_at, provider, dimension \
                     FROM encoding_runs ORDER BY id",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok(EncodingRunRow {
                        started_at: r.get(0)?,
                        finished_at: r.get(1)?,
                        status: r.get(2)?,
                        total_chunks: r.get(3)?,
                        chunks_completed: r.get(4)?,
                        chunks_per_second: r.get(5)?,
                        last_batch_at: r.get(6)?,
                        provider: r.get(7)?,
                        dimension: r.get(8)?,
                    })
                })?;
                for row in rows {
                    write_rec(&mut w, &Record::EncodingRun(row?))?;
                    s.encoding_runs += 1;
                }
            }

            w.flush().context("flush export artifact")?;
            Ok((w, s))
        })
        .await
}

#[cfg(test)]
mod tests;
