//! `StoreSearch` impl (spec §6.6).
//!
//! FTS is implemented (Phase 2a) via Postgres `tsvector`/`to_tsquery('simple')`
//! with the SAME token list as SQLite (reusing `sanitize_fts_term`), and the
//! exact-match multiplier stored in `SearchHit.exact_mult` exactly as SQLite's
//! `search_fts_by_id` does (separate-pass model, R-SEARCH-9 — NOT baked into the
//! rank). Vector/hybrid remain Phase-2b/3 stubs (`// PARITY-TODO`).
//!
//! Raw FTS scores differ from FTS5 by construction; parity is on membership +
//! deterministic ordering on all-distinct-score fixtures (§6.6), not raw scores.

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{PgPool, QueryBuilder, Row};

use super::PostgresStore;
use crate::db::traits::StoreSearch;
use crate::db::types::{
    ChunkMetadata, HybridResult, HybridWeights, RankedSearchHit, SemanticRanking,
};
use crate::db::SearchHit;
use crate::search::fts::normalize_for_exact_match;

/// Build a `to_tsquery('simple', …)` OR-of-prefix string with the SAME token
/// list SQLite's `build_fts_query` produces (reusing `sanitize_fts_term`), so
/// the query token set is identical across backends (R-SEARCH-1). Empty/all-
/// special input -> empty string (caller returns Ok(empty), never errors).
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

/// Resolve a repo by exact name or `%/name` suffix (mirrors `resolve_repo_id`).
async fn resolve_repo_id(pool: &PgPool, repo: &str) -> anyhow::Result<Option<i64>> {
    let id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM repos WHERE name = $1 OR name LIKE '%/' || $1 \
         ORDER BY (name = $1) DESC, id LIMIT 1",
    )
    .bind(repo)
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
fn non_empty<'a>(f: Option<&'a [String]>) -> Option<&'a [String]> {
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

#[allow(unused_variables)]
#[async_trait]
impl StoreSearch for PostgresStore {
    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        debug: bool,
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

        // Shared WHERE/JOIN builder so the count and the hits queries match.
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

        // total_count = all matches before LIMIT (DISTINCT chunk id). R-SEARCH-3.
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
        // PARITY-TODO(Phase 2b): pgvector <-> L2 KNN; FTS-only degrade when !has_vector_extension.
        Ok(Vec::new())
    }

    async fn search_vector_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query_embedding: &[f32],
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 2b).
        Ok(Vec::new())
    }

    async fn search_chunks_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        embedding: &[f32],
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 2b): RRF fuse FTS + vector positions.
        Ok(Vec::new())
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
        // PARITY-TODO(Phase 2b): reuse shared combine_results.
        Ok(Vec::new())
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
        // PARITY-TODO(Phase 2b): reuse shared apply_semantic_ranking.
        Ok(Vec::new())
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
