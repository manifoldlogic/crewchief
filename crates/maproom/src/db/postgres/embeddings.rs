//! `StoreEmbeddings` impl — content-addressed embedding pool (§6.5).
//!
//! Phase-1 real: `upsert_embedding`, `has_embedding`, `get_embedding`,
//! `fetch_chunks_needing_embeddings` (+ trivial count / sync no-ops / copy).
//! `upsert_embeddings_batch_new` is a Phase-3 stub.
//!
//! Vectors cross the wire as pgvector's text form `[a,b,c]` via `$N::vector` on
//! write and `embedding::text` on read (the crate omits the pgvector crate / the
//! sqlx json+chrono features — see Cargo.toml).

use async_trait::async_trait;
use sqlx::{QueryBuilder, Row};

use super::PostgresStore;
use crate::db::traits::StoreEmbeddings;
use crate::db::types::EmbeddingRecord;
use crate::db::ChunkForEmbedding;

/// Embedding dimensions maproom supports (mirrors `SUPPORTED_DIMENSIONS`).
const SUPPORTED_DIMENSIONS: [usize; 3] = [768, 1024, 1536];

fn validate_dim(dim: usize) -> anyhow::Result<()> {
    if !SUPPORTED_DIMENSIONS.contains(&dim) {
        anyhow::bail!(
            "unsupported embedding dimension {dim}; supported dimensions: 768, 1024, 1536"
        );
    }
    Ok(())
}

/// Validate an embedding's dimension AND that every component is finite. pgvector
/// rejects `NaN`/`±inf` on its `::vector` cast, and a `NaN` would poison `<->`
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

/// Parse pgvector's `embedding::text` output `[a,b,c]` back into a float vec.
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
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version) \
             VALUES ($1, $2::vector, $3, $4) \
             ON CONFLICT (blob_sha) DO UPDATE SET \
                 embedding = EXCLUDED.embedding, \
                 embedding_dim = EXCLUDED.embedding_dim, \
                 model_version = EXCLUDED.model_version \
             RETURNING id",
        )
        .bind(blob_sha)
        .bind(format_vector(embedding))
        .bind(embedding.len() as i32)
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
        let mut qb = QueryBuilder::<sqlx::Postgres>::new(
            "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version) VALUES ",
        );
        let mut first = true;
        for e in embeddings {
            if !first {
                qb.push(", ");
            }
            first = false;
            qb.push("(")
                .push_bind(e.blob_sha.clone())
                .push(", ")
                .push_bind(format_vector(&e.embedding))
                .push("::vector, ")
                .push_bind(e.embedding.len() as i32)
                .push(", ")
                .push_bind(e.model_version.clone())
                .push(")");
        }
        qb.push(
            " ON CONFLICT (blob_sha) DO UPDATE SET \
             embedding = EXCLUDED.embedding, \
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
        let text: Option<String> =
            sqlx::query_scalar("SELECT embedding::text FROM code_embeddings WHERE blob_sha = $1")
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
        // No-op: in pgvector the `embedding` column IS the ANN-searchable column,
        // so the SQLite vec0-sync step collapses (§5.4).
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
