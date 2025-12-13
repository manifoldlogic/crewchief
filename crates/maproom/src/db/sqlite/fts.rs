//! FTS5 full-text search module for SQLite backend
//!
//! Provides FTS5-based keyword search with rank normalization for hybrid search integration.
//! FTS5 ranks are normalized to 0-1 scale for Reciprocal Rank Fusion with vector search.

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};

use super::resolve_repo_id;

/// Result from FTS5 search
#[derive(Debug, Clone)]
pub struct FtsResult {
    /// Chunk ID in the chunks table
    pub chunk_id: i64,
    /// Original FTS5 rank (negative, more negative = better)
    pub rank: f64,
    /// Normalized rank 0-1 (higher = better)
    pub normalized_rank: f64,
    /// Position in result set (0-indexed, used for RRF)
    pub position: usize,
}

/// Normalize FTS5 rank to 0-1 scale
///
/// FTS5 rank is negative where more negative = better match.
/// This converts to 0-1 scale where 1 = best match.
///
/// Formula: 1 / (1 + abs(rank))
pub fn normalize_fts_rank(rank: f64) -> f64 {
    1.0 / (1.0 + rank.abs())
}

static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\p{L}\p{N}_\s]").unwrap());

/// Sanitize a search term for FTS5 queries by replacing special characters with spaces.
/// Uses Unicode categories `[^\p{L}\p{N}_\s]` to preserve letters and numbers from any language
/// while removing FTS5 special characters.
///
/// # Examples
/// ```
/// # use crewchief_maproom::db::sqlite::fts::sanitize_fts_term;
/// assert_eq!(sanitize_fts_term("package.json").trim(), "package json");
/// assert_eq!(sanitize_fts_term("src/main.rs").trim(), "src main rs");
/// assert_eq!(sanitize_fts_term("array[0]").trim(), "array 0");
/// ```
pub fn sanitize_fts_term(term: &str) -> String {
    SPECIAL_CHAR_REGEX.replace_all(term, " ").to_string()
}

/// Build FTS5 query from user input
///
/// Sanitizes special FTS5 characters and builds an OR query with prefix matching.
/// Returns empty string if query is effectively empty after sanitization.
///
/// # FTS5 Syntax Notes
/// - `term*` enables prefix matching (e.g., "func*" matches "function", "func")
/// - OR between terms broadens the search
/// - Special characters like `"`, `'`, `*`, `(`, `)`, `-`, `:` must be removed/escaped
pub fn build_fts_query(query: &str) -> String {
    let words: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            // Sanitize: remove FTS5 special characters
            let clean = sanitize_fts_term(t);
            clean.trim().to_string()
        })
        .filter(|t| !t.is_empty())
        .collect();

    if words.is_empty() {
        return String::new();
    }

    // Build OR query with prefix matching
    words
        .iter()
        .flat_map(|w| w.split_whitespace()) // Handle any embedded spaces from replacement
        .filter(|w| !w.is_empty())
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// Search chunks using FTS5 full-text search
///
/// This is the core FTS implementation that returns chunk_ids with ranks.
/// The caller should join with chunks table to get full chunk data.
///
/// # Arguments
/// * `conn` - SQLite connection
/// * `repo` - Repository name to filter by
/// * `worktree` - Optional worktree name to filter by
/// * `query` - User's search query
/// * `limit` - Maximum number of results
///
/// # Returns
/// Vector of FtsResult with chunk_ids, ranks, and positions
pub fn search_fts(
    conn: &Connection,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    limit: usize,
) -> Result<Vec<FtsResult>> {
    let fts_query = build_fts_query(query);
    if fts_query.is_empty() {
        return Ok(vec![]);
    }

    // Resolve repo_id with fuzzy matching
    let repo_id = resolve_repo_id(conn, repo)?;

    // Resolve worktree_id if specified
    let worktree_id: Option<i64> = if let Some(w) = worktree {
        conn.query_row(
            "SELECT id FROM worktrees WHERE repo_id = ?1 AND name = ?2",
            params![repo_id, w],
            |row| row.get(0),
        )
        .optional()?
    } else {
        None
    };

    // Build SQL based on worktree filter
    let sql = if worktree_id.is_some() {
        r#"
            SELECT c.id, fts_chunks.rank
            FROM fts_chunks
            JOIN chunks c ON c.id = fts_chunks.rowid
            JOIN files f ON f.id = c.file_id
            JOIN chunk_worktrees cw ON cw.chunk_id = c.id
            WHERE fts_chunks MATCH ?1
              AND f.repo_id = ?2
              AND cw.worktree_id = ?3
            ORDER BY fts_chunks.rank ASC
            LIMIT ?4
        "#
    } else {
        r#"
            SELECT DISTINCT c.id, fts_chunks.rank
            FROM fts_chunks
            JOIN chunks c ON c.id = fts_chunks.rowid
            JOIN files f ON f.id = c.file_id
            WHERE fts_chunks MATCH ?1
              AND f.repo_id = ?2
            ORDER BY fts_chunks.rank ASC
            LIMIT ?3
        "#
    };

    let mut stmt = conn.prepare(sql)?;
    let mut results = Vec::new();

    if let Some(wid) = worktree_id {
        let rows = stmt.query_map(params![fts_query, repo_id, wid, limit as i64], |row| {
            let chunk_id: i64 = row.get(0)?;
            let rank: f64 = row.get(1)?;
            Ok(FtsResult {
                chunk_id,
                rank,
                normalized_rank: normalize_fts_rank(rank),
                position: 0, // Will be set after collecting
            })
        })?;

        for result in rows {
            results.push(result?);
        }
    } else {
        let rows = stmt.query_map(params![fts_query, repo_id, limit as i64], |row| {
            let chunk_id: i64 = row.get(0)?;
            let rank: f64 = row.get(1)?;
            Ok(FtsResult {
                chunk_id,
                rank,
                normalized_rank: normalize_fts_rank(rank),
                position: 0, // Will be set after collecting
            })
        })?;

        for result in rows {
            results.push(result?);
        }
    }

    // Set position (0-indexed rank in result set)
    for (i, result) in results.iter_mut().enumerate() {
        result.position = i;
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_fts_rank_best_match() {
        // FTS5 rank of 0 (best possible) should normalize to 1.0
        let normalized = normalize_fts_rank(0.0);
        assert!(
            (normalized - 1.0).abs() < 1e-6,
            "Rank 0 should normalize to 1.0"
        );
    }

    #[test]
    fn test_normalize_fts_rank_negative() {
        // FTS5 rank of -1.0 should normalize to 0.5
        let normalized = normalize_fts_rank(-1.0);
        assert!(
            (normalized - 0.5).abs() < 1e-6,
            "Rank -1.0 should normalize to 0.5"
        );
    }

    #[test]
    fn test_normalize_fts_rank_large_negative() {
        // Large negative rank should give low normalized score
        let normalized = normalize_fts_rank(-10.0);
        assert!(
            normalized < 0.1,
            "Large negative rank should give low score"
        );
        assert!(normalized > 0.0, "Normalized rank should be positive");
    }

    #[test]
    fn test_normalize_fts_rank_monotonic() {
        // More negative rank = worse match = lower normalized score
        let rank0 = normalize_fts_rank(0.0);
        let rank1 = normalize_fts_rank(-1.0);
        let rank5 = normalize_fts_rank(-5.0);

        assert!(rank0 > rank1, "Rank 0 should be better than -1");
        assert!(rank1 > rank5, "Rank -1 should be better than -5");
    }

    #[test]
    fn test_normalize_fts_rank_range() {
        // All normalized ranks should be in (0, 1]
        for rank in [0.0, -0.5, -1.0, -5.0, -100.0] {
            let normalized = normalize_fts_rank(rank);
            assert!(
                normalized > 0.0 && normalized <= 1.0,
                "Normalized rank should be in (0, 1], got {} for rank {}",
                normalized,
                rank
            );
        }
    }

    #[test]
    fn test_build_fts_query_simple() {
        let query = build_fts_query("hello");
        assert_eq!(query, "hello*");
    }

    #[test]
    fn test_build_fts_query_multiple_words() {
        let query = build_fts_query("hello world");
        assert_eq!(query, "hello* OR world*");
    }

    #[test]
    fn test_build_fts_query_sanitize_quotes() {
        let query = build_fts_query("\"hello\" 'world'");
        assert_eq!(query, "hello* OR world*");
    }

    #[test]
    fn test_build_fts_query_sanitize_wildcards() {
        let query = build_fts_query("hello* world*");
        assert_eq!(query, "hello* OR world*");
    }

    #[test]
    fn test_build_fts_query_sanitize_parens() {
        let query = build_fts_query("(hello) (world)");
        assert_eq!(query, "hello* OR world*");
    }

    #[test]
    fn test_build_fts_query_empty() {
        let query = build_fts_query("");
        assert!(query.is_empty());
    }

    #[test]
    fn test_build_fts_query_only_special_chars() {
        let query = build_fts_query("\"\" '*' ()");
        assert!(query.is_empty());
    }

    #[test]
    fn test_build_fts_query_hyphen_handling() {
        // Hyphen should be treated as word separator
        let query = build_fts_query("some-function");
        assert_eq!(query, "some* OR function*");
    }

    #[test]
    fn test_build_fts_query_colon_handling() {
        // Colon should be treated as word separator
        let query = build_fts_query("module:function");
        assert_eq!(query, "module* OR function*");
    }

    #[test]
    fn test_build_fts_query_comprehensive_sanitization() {
        // Dots (file extensions)
        let query = build_fts_query("package.json");
        assert_eq!(query, "package* OR json*");

        // Slashes (file paths)
        let query = build_fts_query("src/main.rs");
        assert_eq!(query, "src* OR main* OR rs*");

        // Brackets (array syntax)
        let query = build_fts_query("array[0]");
        assert_eq!(query, "array* OR 0*");

        // Braces (template syntax)
        let query = build_fts_query("template{value}");
        assert_eq!(query, "template* OR value*");

        // At sign (email/decorators)
        let query = build_fts_query("user@email.com");
        assert_eq!(query, "user* OR email* OR com*");

        // Backslash (Windows paths)
        let query = build_fts_query("path\\to\\file");
        assert_eq!(query, "path* OR to* OR file*");

        // Mixed special characters
        let query = build_fts_query("src/main@v2.rs");
        assert_eq!(query, "src* OR main* OR v2* OR rs*");

        // Operators
        let query = build_fts_query("a+b=c");
        assert_eq!(query, "a* OR b* OR c*");
    }

    #[test]
    fn test_build_fts_query_whitespace() {
        let query = build_fts_query("  hello   world  ");
        assert_eq!(query, "hello* OR world*");
    }
}
