//! `StoreSearch` impl (spec §6.6) — FTS + vector + hybrid parity (Phase 2).
//!
//! - FTS: Postgres `tsvector`/`to_tsquery('simple')` with the SAME token list as
//!   SQLite (reusing `sanitize_fts_term`); `exact_mult` stored separately
//!   (R-SEARCH-9), `total_count` before LIMIT (R-SEARCH-3), filters (R-SEARCH-4).
//! - Vector: pgvector `<->` (L2) KNN, `similarity = 1/(1+distance)`
//!   (`distance_to_similarity`, R-SEARCH-5); degrades to empty when
//!   `has_vector_extension()` is false; scoped to matching `embedding_dim`.
//! - Hybrid: reuses SQLite's `combine_results` (RRF) and `apply_semantic_ranking`
//!   UNMODIFIED (R-SEARCH-6/7) — fed FTS/vector position orderings.
//!
//! Raw FTS/vector scores differ from SQLite by construction; parity is on
//! membership + deterministic all-distinct-score ordering (§6.6), not raw scores.

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{PgPool, QueryBuilder, Row};

use super::PostgresStore;
use crate::db::sqlite::fts::FtsResult;
use crate::db::sqlite::hybrid::{apply_semantic_ranking, combine_results};
use crate::db::sqlite::vector::{distance_to_similarity, VectorResult};
use crate::db::traits::StoreSearch;
use crate::db::types::{
    ChunkMetadata, HybridResult, HybridWeights, RankedSearchHit, SemanticRanking,
};
use crate::db::SearchHit;
use crate::search::fts::normalize_for_exact_match;

/// Build a `to_tsquery('simple', …)` OR-of-prefix string with the SAME token
/// list SQLite's `build_fts_query` produces (reusing `sanitize_fts_term`), so
/// the query token set is identical across backends (R-SEARCH-1). Empty input
/// -> empty string (caller returns Ok(empty), never errors).
fn build_tsquery(query: &str) -> String {
    let words: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            crate::db::sqlite::fts::sanitize_fts_term(t)
                .trim()
                .to_string()
        })
        .filter(|t| !t.is_empty())
        .collect();
    words
        .iter()
        .flat_map(|w| w.split_whitespace())
        .filter(|w| !w.is_empty())
        .map(|w| format!("{w}:*"))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// pgvector text literal `[a,b,c]` for binding with `$N::vector`.
fn vector_literal(v: &[f32]) -> String {
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

/// Recover an L2 distance from a `1/(1+d)` similarity (inverse of
/// `distance_to_similarity`), for feeding `VectorResult`.
fn similarity_to_distance(sim: f64) -> f64 {
    if sim <= 0.0 {
        f64::INFINITY
    } else {
        (1.0 / sim) - 1.0
    }
}

async fn resolve_repo_id(pool: &PgPool, repo: &str) -> anyhow::Result<Option<i64>> {
    // Suffix-match `<owner>/<repo>` by name, but escape LIKE metacharacters in the
    // user-supplied repo so `%`/`_`/`\` are matched literally, not as wildcards.
    let escaped = repo
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let suffix = format!("%/{escaped}");
    // ILIKE (not LIKE) for the fuzzy suffix so matching is case-insensitive,
    // matching SQLite's LIKE (which is ASCII case-insensitive by default).
    let id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM repos WHERE name = $1 OR name ILIKE $2 ESCAPE '\\' \
         ORDER BY (name = $1) DESC, id LIMIT 1",
    )
    .bind(repo)
    .bind(suffix)
    .fetch_optional(pool)
    .await?;
    Ok(id)
}

async fn resolve_worktree_id(
    pool: &PgPool,
    repo_id: i64,
    worktree: &str,
) -> anyhow::Result<Option<i64>> {
    let id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM worktrees WHERE repo_id = $1 AND name = $2")
            .bind(repo_id)
            .bind(worktree)
            .fetch_optional(pool)
            .await?;
    Ok(id)
}

/// Treat `Some(&[])` as `None` (R-SEARCH-4).
fn non_empty(f: Option<&[String]>) -> Option<&[String]> {
    f.filter(|s| !s.is_empty())
}

fn row_to_fts_hit(r: &sqlx::postgres::PgRow, normalized_query: &str) -> SearchHit {
    let symbol_name: Option<String> = r.get("symbol_name");
    let exact_mult = symbol_name
        .as_ref()
        .map(|s| {
            if normalize_for_exact_match(s).to_lowercase() == normalized_query.to_lowercase() {
                3.0
            } else {
                1.0
            }
        })
        .unwrap_or(1.0);
    SearchHit {
        chunk_id: r.get("id"),
        start_line: r.get("start_line"),
        end_line: r.get("end_line"),
        symbol_name,
        kind: r.get("kind"),
        file_relpath: r.get("relpath"),
        score: r.get::<f32, _>("score") as f64,
        base_score: None,
        kind_mult: None,
        exact_mult: Some(exact_mult),
        preview: None,
    }
}

/// Detail columns needed to (re)build a `SearchHit` (which isn't `Clone`).
#[derive(Clone)]
struct HitDetail {
    start_line: i32,
    end_line: i32,
    symbol_name: Option<String>,
    kind: String,
    file_relpath: String,
    preview: Option<String>,
}

impl PostgresStore {
    /// Run a built KNN query without the session `statement_timeout`.
    ///
    /// The exact nearest-neighbour scan over `code_embeddings` has no ANN index
    /// yet (Phase-2; see `migrations_pg/0002`), so on a non-trivial corpus it can
    /// exceed the `statement_timeout` `tuned_pool` sets on every connection and be
    /// killed mid-scan. Running it in a read-only transaction with
    /// `SET LOCAL statement_timeout = 0` lets the scan complete; `SET LOCAL` is
    /// scoped to the transaction and reverts on commit, so the pooled connection
    /// keeps its normal timeout for every other query.
    async fn fetch_knn_rows(
        &self,
        mut qb: QueryBuilder<'_, sqlx::Postgres>,
    ) -> anyhow::Result<Vec<sqlx::postgres::PgRow>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL statement_timeout = 0")
            .execute(&mut *tx)
            .await?;
        let rows = qb.build().fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows)
    }

    /// FTS results as RRF inputs (chunk_id + 0-indexed position), no filters —
    /// the hybrid path matches SQLite's `search_fts`/`search_vector` helpers.
    async fn fts_result_list(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<FtsResult>> {
        let (hits, _) = self
            .search_chunks_fts(repo, worktree, query, limit, false, None, None)
            .await?;
        Ok(hits
            .iter()
            .enumerate()
            .map(|(i, h)| FtsResult {
                chunk_id: h.chunk_id,
                rank: h.score,
                normalized_rank: h.score,
                position: i,
            })
            .collect())
    }

    async fn vector_result_list(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        limit: i64,
    ) -> anyhow::Result<Vec<VectorResult>> {
        let hits = self
            .search_chunks_vector(repo, worktree, embedding, limit, false, None, None)
            .await?;
        Ok(hits
            .iter()
            .map(|h| VectorResult {
                chunk_id: h.chunk_id,
                distance: similarity_to_distance(h.score),
                similarity: h.score,
            })
            .collect())
    }
}

#[async_trait]
impl StoreSearch for PostgresStore {
    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        _debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<(Vec<SearchHit>, usize)> {
        let tsq = build_tsquery(query);
        if tsq.is_empty() {
            return Ok((Vec::new(), 0));
        }
        let Some(repo_id) = resolve_repo_id(&self.pool, repo).await? else {
            return Ok((Vec::new(), 0));
        };
        let wt_id = match worktree {
            Some(w) => match resolve_worktree_id(&self.pool, repo_id, w).await? {
                Some(id) => Some(id),
                None => return Ok((Vec::new(), 0)),
            },
            None => None,
        };
        let kinds = non_empty(kind_filter);
        let langs = non_empty(lang_filter);

        let push_from_where = |qb: &mut QueryBuilder<'_, sqlx::Postgres>| {
            qb.push(" FROM chunks c JOIN files f ON f.id = c.file_id");
            if wt_id.is_some() {
                qb.push(" JOIN chunk_worktrees cw ON cw.chunk_id = c.id");
            }
            qb.push(" WHERE c.ts_doc @@ to_tsquery('simple', ")
                .push_bind(tsq.clone())
                .push(") AND f.repo_id = ")
                .push_bind(repo_id);
            if let Some(wid) = wt_id {
                qb.push(" AND cw.worktree_id = ").push_bind(wid);
            }
            if let Some(kinds) = kinds {
                qb.push(" AND c.kind = ANY(")
                    .push_bind(kinds.to_vec())
                    .push(")");
            }
            if let Some(langs) = langs {
                qb.push(" AND f.language = ANY(")
                    .push_bind(langs.to_vec())
                    .push(")");
            }
        };

        let mut count_qb = QueryBuilder::<sqlx::Postgres>::new("SELECT count(DISTINCT c.id)");
        push_from_where(&mut count_qb);
        let total: i64 = count_qb.build_query_scalar().fetch_one(&self.pool).await?;

        let mut qb = QueryBuilder::<sqlx::Postgres>::new(
            "SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, \
             ts_rank(c.ts_doc, to_tsquery('simple', ",
        );
        qb.push_bind(tsq.clone()).push(")) AS score");
        push_from_where(&mut qb);
        qb.push(" ORDER BY score DESC, c.id LIMIT ").push_bind(k);
        let rows = qb.build().fetch_all(&self.pool).await?;
        let normalized = normalize_for_exact_match(query);
        let hits = rows
            .iter()
            .map(|r| row_to_fts_hit(r, &normalized))
            .collect();
        Ok((hits, total as usize))
    }

    async fn search_fts_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query: &str,
        normalized_query: &str,
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let tsq = build_tsquery(query);
        if tsq.is_empty() {
            return Ok(Vec::new());
        }
        let rows = if let Some(wid) = worktree_id {
            sqlx::query(
                "SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, \
                    ts_rank(c.ts_doc, to_tsquery('simple', $1)) AS score \
                 FROM chunks c JOIN files f ON f.id = c.file_id \
                 JOIN chunk_worktrees cw ON cw.chunk_id = c.id \
                 WHERE c.ts_doc @@ to_tsquery('simple', $1) AND f.repo_id = $2 AND cw.worktree_id = $3 \
                 ORDER BY score DESC, c.id LIMIT $4",
            )
            .bind(&tsq)
            .bind(repo_id)
            .bind(wid)
            .bind(k)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, \
                    ts_rank(c.ts_doc, to_tsquery('simple', $1)) AS score \
                 FROM chunks c JOIN files f ON f.id = c.file_id \
                 WHERE c.ts_doc @@ to_tsquery('simple', $1) AND f.repo_id = $2 \
                 ORDER BY score DESC, c.id LIMIT $3",
            )
            .bind(&tsq)
            .bind(repo_id)
            .bind(k)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows
            .iter()
            .map(|r| row_to_fts_hit(r, normalized_query))
            .collect())
    }

    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // Degrade to empty (FTS-only at the caller) when pgvector is absent — BEFORE
        // validation, so a missing extension degrades gracefully even for a
        // malformed embedding (parity with SQLite; R-SEARCH-5/R-TRAIT-3).
        if !self.has_vector_extension() {
            return Ok(Vec::new());
        }
        // pgvector IS present: reject unsupported dims and non-finite values (the
        // SQLite vector path errors on a bad dim once the extension is active).
        super::embeddings::validate_embedding(embedding)?;
        let Some(repo_id) = resolve_repo_id(&self.pool, repo).await? else {
            return Ok(Vec::new());
        };
        let wt_id = match worktree {
            Some(w) => match resolve_worktree_id(&self.pool, repo_id, w).await? {
                Some(id) => Some(id),
                None => return Ok(Vec::new()),
            },
            None => None,
        };
        let lit = vector_literal(embedding);
        let mut qb = QueryBuilder::<sqlx::Postgres>::new(
            "SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, c.preview, \
             (e.embedding <-> ",
        );
        qb.push_bind(lit).push(
            "::vector) AS distance \
             FROM code_embeddings e JOIN chunks c ON c.blob_sha = e.blob_sha \
             JOIN files f ON f.id = c.file_id",
        );
        if wt_id.is_some() {
            qb.push(" JOIN chunk_worktrees cw ON cw.chunk_id = c.id");
        }
        qb.push(" WHERE f.repo_id = ")
            .push_bind(repo_id)
            .push(" AND e.embedding_dim = ")
            .push_bind(embedding.len() as i32);
        if let Some(wid) = wt_id {
            qb.push(" AND cw.worktree_id = ").push_bind(wid);
        }
        if let Some(kinds) = non_empty(kind_filter) {
            qb.push(" AND c.kind = ANY(")
                .push_bind(kinds.to_vec())
                .push(")");
        }
        if let Some(langs) = non_empty(lang_filter) {
            qb.push(" AND f.language = ANY(")
                .push_bind(langs.to_vec())
                .push(")");
        }
        qb.push(" ORDER BY distance ASC, c.id LIMIT ").push_bind(k);
        let rows = self.fetch_knn_rows(qb).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let distance: f64 = r.get("distance");
                let similarity = distance_to_similarity(distance);
                SearchHit {
                    chunk_id: r.get("id"),
                    start_line: r.get("start_line"),
                    end_line: r.get("end_line"),
                    symbol_name: r.get("symbol_name"),
                    kind: r.get("kind"),
                    file_relpath: r.get("relpath"),
                    score: similarity,
                    base_score: if debug { Some(similarity) } else { None },
                    kind_mult: None,
                    exact_mult: None,
                    preview: r.get::<Option<String>, _>("preview"),
                }
            })
            .collect())
    }

    async fn search_vector_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query_embedding: &[f32],
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // Degrade BEFORE validation so a missing extension yields Ok(empty) even
        // for a malformed embedding (parity with SQLite; R-SEARCH-5).
        if !self.has_vector_extension() {
            return Ok(Vec::new());
        }
        super::embeddings::validate_embedding(query_embedding)?;
        let lit = vector_literal(query_embedding);
        let mut qb = QueryBuilder::<sqlx::Postgres>::new(
            "SELECT c.id, c.start_line, c.end_line, c.symbol_name, c.kind, f.relpath, c.preview, \
             (e.embedding <-> ",
        );
        qb.push_bind(lit).push(
            "::vector) AS distance \
             FROM code_embeddings e JOIN chunks c ON c.blob_sha = e.blob_sha \
             JOIN files f ON f.id = c.file_id",
        );
        if worktree_id.is_some() {
            qb.push(" JOIN chunk_worktrees cw ON cw.chunk_id = c.id");
        }
        qb.push(" WHERE f.repo_id = ")
            .push_bind(repo_id)
            .push(" AND e.embedding_dim = ")
            .push_bind(query_embedding.len() as i32);
        if let Some(wid) = worktree_id {
            qb.push(" AND cw.worktree_id = ").push_bind(wid);
        }
        qb.push(" ORDER BY distance ASC, c.id LIMIT ").push_bind(k);
        let rows = self.fetch_knn_rows(qb).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let distance: f64 = r.get("distance");
                SearchHit {
                    chunk_id: r.get("id"),
                    start_line: r.get("start_line"),
                    end_line: r.get("end_line"),
                    symbol_name: r.get("symbol_name"),
                    kind: r.get("kind"),
                    file_relpath: r.get("relpath"),
                    score: distance_to_similarity(distance),
                    base_score: None,
                    kind_mult: None,
                    exact_mult: None,
                    preview: r.get::<Option<String>, _>("preview"),
                }
            })
            .collect())
    }

    async fn search_chunks_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        embedding: &[f32],
        k: i64,
        _debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let fetch = k.saturating_mul(3).max(k);
        let (fts_hits, _) = self
            .search_chunks_fts(
                repo,
                worktree,
                query,
                fetch,
                false,
                kind_filter,
                lang_filter,
            )
            .await?;
        let vec_hits = self
            .search_chunks_vector(
                repo,
                worktree,
                embedding,
                fetch,
                false,
                kind_filter,
                lang_filter,
            )
            .await?;

        let fts_r: Vec<FtsResult> = fts_hits
            .iter()
            .enumerate()
            .map(|(i, h)| FtsResult {
                chunk_id: h.chunk_id,
                rank: h.score,
                normalized_rank: h.score,
                position: i,
            })
            .collect();
        let vec_r: Vec<VectorResult> = vec_hits
            .iter()
            .map(|h| VectorResult {
                chunk_id: h.chunk_id,
                distance: similarity_to_distance(h.score),
                similarity: h.score,
            })
            .collect();

        // Detail map (SearchHit isn't Clone) — consume the source hits.
        let mut detail: HashMap<i64, HitDetail> = HashMap::new();
        for h in fts_hits.into_iter().chain(vec_hits) {
            detail.entry(h.chunk_id).or_insert(HitDetail {
                start_line: h.start_line,
                end_line: h.end_line,
                symbol_name: h.symbol_name,
                kind: h.kind,
                file_relpath: h.file_relpath,
                preview: h.preview,
            });
        }

        let combined = combine_results(&fts_r, &vec_r, &HybridWeights::default(), k as usize);
        Ok(combined
            .into_iter()
            .filter_map(|hr| {
                let d = detail.get(&hr.chunk_id)?;
                Some(SearchHit {
                    chunk_id: hr.chunk_id,
                    start_line: d.start_line,
                    end_line: d.end_line,
                    symbol_name: d.symbol_name.clone(),
                    kind: d.kind.clone(),
                    file_relpath: d.file_relpath.clone(),
                    score: hr.score,
                    base_score: None,
                    kind_mult: None,
                    exact_mult: None,
                    preview: d.preview.clone(),
                })
            })
            .collect())
    }

    async fn search_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
    ) -> anyhow::Result<Vec<HybridResult>> {
        let fetch = (limit.saturating_mul(3)) as i64;
        let fts_r = self.fts_result_list(repo, worktree, query, fetch).await?;
        let vec_r = self
            .vector_result_list(repo, worktree, query_embedding, fetch)
            .await?;
        Ok(combine_results(&fts_r, &vec_r, &weights, limit))
    }

    async fn search_hybrid_ranked(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
        ranking: SemanticRanking,
    ) -> anyhow::Result<Vec<RankedSearchHit>> {
        let fetch = limit.saturating_mul(2);
        let hits = self
            .search_hybrid(repo, worktree, query, query_embedding, fetch, weights)
            .await?;
        if hits.is_empty() {
            return Ok(Vec::new());
        }
        let chunk_ids: Vec<i64> = hits.iter().map(|h| h.chunk_id).collect();
        let metadata = self.get_chunks_metadata(&chunk_ids).await?;
        let mut ranked: Vec<RankedSearchHit> = hits
            .into_iter()
            .filter_map(|h| {
                let meta = metadata.get(&h.chunk_id)?;
                Some(RankedSearchHit {
                    chunk_id: h.chunk_id,
                    score: h.score,
                    fts_rank: h.fts_rank,
                    vector_rank: h.vector_rank,
                    kind: meta.kind.clone(),
                    symbol_name: meta.symbol_name.clone(),
                    recency_score: meta.recency_score,
                    source: h.source,
                })
            })
            .collect();
        apply_semantic_ranking(&mut ranked, query, &ranking);
        ranked.truncate(limit);
        Ok(ranked)
    }

    async fn get_chunks_metadata(
        &self,
        chunk_ids: &[i64],
    ) -> anyhow::Result<HashMap<i64, ChunkMetadata>> {
        if chunk_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            "SELECT id, kind, symbol_name, recency_score FROM chunks WHERE id = ANY($1)",
        )
        .bind(chunk_ids.to_vec())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| {
                (
                    r.get::<i64, _>("id"),
                    ChunkMetadata {
                        kind: r.get("kind"),
                        symbol_name: r.get("symbol_name"),
                        recency_score: r.get::<f32, _>("recency_score") as f64,
                    },
                )
            })
            .collect())
    }
}
