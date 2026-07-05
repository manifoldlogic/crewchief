//! `StoreEmbeddings` impl — content-addressed embedding pool (§6.5).
//!
//! Phase-1 real: `upsert_embedding`, `has_embedding`, `get_embedding`,
//! `fetch_chunks_needing_embeddings` (+ trivial count / sync no-ops / copy).
//! `upsert_embeddings_batch_new` is a Phase-3 stub.
//!
//! Vectors cross the wire as pgvector's text form `[a,b,c]` via `$N::vector(dim)`
//! on write and `<col>::text` on read (the crate omits the pgvector crate / the
//! sqlx json+chrono features — see Cargo.toml). Storage is dimension-typed
//! (spec §4/F04): each row lives in exactly one of `embedding_768`/`embedding_1024`/
//! `embedding_1536` (the column matching its dim), the others NULL — so pgvector's
//! per-dim cosine HNSW index (migration 0004) can serve the KNN scan.

use async_trait::async_trait;
use sqlx::{QueryBuilder, Row};

use super::PostgresStore;
use crate::db::traits::StoreEmbeddings;
use crate::db::types::EmbeddingRecord;
use crate::db::ChunkForEmbedding;

/// Embedding dimensions maproom supports (mirrors SQLite's `SUPPORTED_DIMENSIONS`).
/// Single source of truth for the pool's dim registry (spec S1.5): insert routing,
/// read, and the search path all derive their typed column from it, so adding a dim
/// is one edit here + one typed column & index in a new migration.
const SUPPORTED_DIMENSIONS: [usize; 3] = [768, 1024, 1536];

fn validate_dim(dim: usize) -> anyhow::Result<()> {
    if !SUPPORTED_DIMENSIONS.contains(&dim) {
        anyhow::bail!(
            "unsupported embedding dimension {dim}; supported dimensions: 768, 1024, 1536"
        );
    }
    Ok(())
}

/// The typed vector column a given dim's embeddings live in (spec S1.1/S1.5).
/// `pub(super)` so `search.rs` targets the same column the write path populated.
/// Returns a `&'static str` from a fixed, validated set — safe to inline into SQL
/// (never user-derived), which the dynamic column name requires (Postgres has no
/// bind-param for identifiers).
pub(super) fn embedding_column_for_dim(dim: usize) -> anyhow::Result<&'static str> {
    match dim {
        768 => Ok("embedding_768"),
        1024 => Ok("embedding_1024"),
        1536 => Ok("embedding_1536"),
        _ => anyhow::bail!(
            "unsupported embedding dimension {dim}; supported dimensions: 768, 1024, 1536"
        ),
    }
}

/// Validate an embedding's dimension AND that every component is finite. pgvector
/// rejects `NaN`/`±inf` on its `::vector` cast, and a `NaN` would poison `<=>`
/// distance ordering, so non-finite values are caught here on both the write and
/// the search paths rather than surfacing as an opaque DB error. `pub(super)` so
/// `search.rs` reuses it.
pub(super) fn validate_embedding(embedding: &[f32]) -> anyhow::Result<()> {
    validate_dim(embedding.len())?;
    if let Some(pos) = embedding.iter().position(|x| !x.is_finite()) {
        anyhow::bail!("embedding contains a non-finite value (NaN/inf) at index {pos}");
    }
    Ok(())
}

/// Render a float slice as pgvector's text form, e.g. `[1,2.5,3]`.
fn format_vector(v: &[f32]) -> String {
    let mut s = String::with_capacity(v.len() * 8 + 2);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&x.to_string());
    }
    s.push(']');
    s
}

/// Push the three typed-vector column slots for one row of a multi-row INSERT: the
/// column matching the embedding's dim gets the `::vector(N)` literal, the other two
/// get `NULL` — so every pool row has exactly one non-null typed column (the storage
/// invariant that keeps read/search unambiguous). The dim is assumed already
/// validated (unsupported dims never reach here).
fn push_typed_vector_slots(qb: &mut QueryBuilder<'_, sqlx::Postgres>, embedding: &[f32]) {
    let d = embedding.len();
    let lit = format_vector(embedding);
    for (i, dim) in [768usize, 1024, 1536].into_iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        if d == dim {
            qb.push_bind(lit.clone()).push(format!("::vector({dim})"));
        } else {
            qb.push("NULL");
        }
    }
}

/// Parse pgvector's `<col>::text` output `[a,b,c]` back into a float vec.
fn parse_vector(text: &str) -> anyhow::Result<Vec<f32>> {
    let inner = text
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|t| t.trim().parse::<f32>().map_err(anyhow::Error::from))
        .collect()
}

#[async_trait]
impl StoreEmbeddings for PostgresStore {
    async fn upsert_embedding(
        &self,
        blob_sha: &str,
        embedding: &[f32],
        model_version: &str,
    ) -> anyhow::Result<i64> {
        validate_embedding(embedding)?;
        let dim = embedding.len();
        let col = embedding_column_for_dim(dim)?;
        // Route into the typed column for this dim. `col`/`dim` are from the fixed,
        // validated registry — safe to inline (identifiers/typmods take no bind
        // param). ON CONFLICT copies ALL THREE typed columns from EXCLUDED (which
        // has only `col` populated, the others NULL), so re-embedding the same
        // blob_sha at a different dim clears the stale column — preserving the
        // exactly-one-non-null invariant.
        let sql = format!(
            "INSERT INTO code_embeddings (blob_sha, {col}, embedding_dim, model_version) \
             VALUES ($1, $2::vector({dim}), $3, $4) \
             ON CONFLICT (blob_sha) DO UPDATE SET \
                 embedding_768 = EXCLUDED.embedding_768, \
                 embedding_1024 = EXCLUDED.embedding_1024, \
                 embedding_1536 = EXCLUDED.embedding_1536, \
                 embedding_dim = EXCLUDED.embedding_dim, \
                 model_version = EXCLUDED.model_version \
             RETURNING id"
        );
        let id: i64 = sqlx::query_scalar(&sql)
            .bind(blob_sha)
            .bind(format_vector(embedding))
            .bind(dim as i32)
            .bind(model_version)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    async fn upsert_embeddings_batch_new(
        &self,
        embeddings: &[EmbeddingRecord],
    ) -> anyhow::Result<()> {
        if embeddings.is_empty() {
            return Ok(());
        }
        // Validate ALL embeddings first (dimension + finiteness); fail the whole
        // batch on any bad one, naming the offending index (R-EMB-8).
        for (i, e) in embeddings.iter().enumerate() {
            validate_embedding(&e.embedding)
                .map_err(|err| anyhow::anyhow!("embedding {i}: {err}"))?;
        }
        // One multi-row INSERT … ON CONFLICT — a single statement is atomic, so no
        // held Transaction (which would trip the async_trait Send/Executor check).
        // Each row routes its vector into the typed column for its dim (the other
        // two slots NULL, via push_typed_vector_slots), so a mixed-dim batch is
        // still one statement — no per-dim grouping needed.
        let mut qb = QueryBuilder::<sqlx::Postgres>::new(
            "INSERT INTO code_embeddings \
             (blob_sha, embedding_768, embedding_1024, embedding_1536, embedding_dim, model_version) \
             VALUES ",
        );
        let mut first = true;
        for e in embeddings {
            if !first {
                qb.push(", ");
            }
            first = false;
            qb.push("(").push_bind(e.blob_sha.clone()).push(", ");
            push_typed_vector_slots(&mut qb, &e.embedding);
            qb.push(", ")
                .push_bind(e.embedding.len() as i32)
                .push(", ")
                .push_bind(e.model_version.clone())
                .push(")");
        }
        // Copy all three typed columns from EXCLUDED so a re-embed at a different
        // dim clears the stale column (same invariant as single upsert).
        qb.push(
            " ON CONFLICT (blob_sha) DO UPDATE SET \
             embedding_768 = EXCLUDED.embedding_768, \
             embedding_1024 = EXCLUDED.embedding_1024, \
             embedding_1536 = EXCLUDED.embedding_1536, \
             embedding_dim = EXCLUDED.embedding_dim, \
             model_version = EXCLUDED.model_version",
        );
        qb.build().execute(&self.pool).await?;
        Ok(())
    }

    async fn has_embedding(&self, blob_sha: &str) -> anyhow::Result<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM code_embeddings WHERE blob_sha = $1)")
                .bind(blob_sha)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }

    async fn get_embedding(&self, blob_sha: &str) -> anyhow::Result<Option<Vec<f32>>> {
        // Read the one non-null typed column (each row populates exactly one).
        // COALESCE over the per-dim columns returns whichever is set, as text.
        let text: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE(embedding_768::text, embedding_1024::text, embedding_1536::text) \
             FROM code_embeddings WHERE blob_sha = $1",
        )
        .bind(blob_sha)
        .fetch_optional(&self.pool)
        .await?;
        match text {
            Some(t) => Ok(Some(parse_vector(&t)?)),
            None => Ok(None),
        }
    }

    async fn sync_embedding_to_vec(
        &self,
        _embedding_id: i64,
        _embedding: &[f32],
    ) -> anyhow::Result<()> {
        // No-op: in pgvector the typed `embedding_<dim>` column IS the ANN-searchable
        // column (indexed by HNSW), so the SQLite vec0-sync step collapses (§5.4).
        Ok(())
    }

    async fn sync_all_embeddings_to_vec(&self) -> anyhow::Result<usize> {
        // No-op (see sync_embedding_to_vec); nothing to backfill.
        Ok(0)
    }

    async fn get_chunks_needing_embeddings_count(&self) -> anyhow::Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM chunks \
             WHERE blob_sha NOT IN (SELECT blob_sha FROM code_embeddings)",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    async fn copy_existing_embeddings_from_cache(&self) -> anyhow::Result<i64> {
        // Default per R-EMB-7 (SQLite returns a no-op i64).
        Ok(0)
    }

    async fn fetch_chunks_needing_embeddings(
        &self,
        incremental: bool,
        sample_size: Option<usize>,
    ) -> anyhow::Result<Vec<ChunkForEmbedding>> {
        // SQL parity (R-EMB-2): base query over all chunks; when incremental,
        // exclude chunks whose blob_sha already has an embedding; sample_size -> LIMIT.
        let mut sql = String::from(
            "SELECT c.id, c.blob_sha, c.signature, c.docstring, c.preview FROM chunks c",
        );
        if incremental {
            sql.push_str(" WHERE c.blob_sha NOT IN (SELECT blob_sha FROM code_embeddings)");
        }
        sql.push_str(" ORDER BY c.id");
        if let Some(n) = sample_size {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| ChunkForEmbedding {
                id: r.get("id"),
                blob_sha: r.get("blob_sha"),
                signature: r.get("signature"),
                docstring: r.get("docstring"),
                preview: r.get("preview"),
            })
            .collect())
    }
}
