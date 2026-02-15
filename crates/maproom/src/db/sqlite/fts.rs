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
/// * `kind_filter` - Optional filter for chunk kinds (e.g., "function", "class")
/// * `lang_filter` - Optional filter for file languages (e.g., "rust", "typescript")
///
/// # Returns
/// Vector of FtsResult with chunk_ids, ranks, and positions
pub fn search_fts(
    conn: &Connection,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    limit: usize,
    kind_filter: Option<&[String]>,
    lang_filter: Option<&[String]>,
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

    // Build dynamic SQL with filter conditions
    // Base params: ?1 = fts_query, ?2 = repo_id
    // With worktree: ?3 = worktree_id, then filters, then LIMIT
    // Without worktree: filters start at ?3, then LIMIT
    let mut param_idx: usize = if worktree_id.is_some() { 4 } else { 3 };
    let mut filter_conditions = Vec::new();

    if let Some(kinds) = kind_filter {
        if !kinds.is_empty() {
            let placeholders = (0..kinds.len())
                .map(|i| format!("?{}", param_idx + i))
                .collect::<Vec<_>>()
                .join(", ");
            filter_conditions.push(format!("c.kind IN ({})", placeholders));
            param_idx += kinds.len();
        }
    }

    if let Some(langs) = lang_filter {
        if !langs.is_empty() {
            let placeholders = (0..langs.len())
                .map(|i| format!("?{}", param_idx + i))
                .collect::<Vec<_>>()
                .join(", ");
            filter_conditions.push(format!("f.language IN ({})", placeholders));
            param_idx += langs.len();
        }
    }

    let filter_clause = if filter_conditions.is_empty() {
        String::new()
    } else {
        format!(" AND {}", filter_conditions.join(" AND "))
    };

    let limit_placeholder = format!("?{}", param_idx);

    let sql = if worktree_id.is_some() {
        format!(
            r#"
            SELECT c.id, fts_chunks.rank
            FROM fts_chunks
            JOIN chunks c ON c.id = fts_chunks.rowid
            JOIN files f ON f.id = c.file_id
            JOIN chunk_worktrees cw ON cw.chunk_id = c.id
            WHERE fts_chunks MATCH ?1
              AND f.repo_id = ?2
              AND cw.worktree_id = ?3
              {}
            ORDER BY fts_chunks.rank ASC
            LIMIT {}
        "#,
            filter_clause, limit_placeholder
        )
    } else {
        format!(
            r#"
            SELECT DISTINCT c.id, fts_chunks.rank
            FROM fts_chunks
            JOIN chunks c ON c.id = fts_chunks.rowid
            JOIN files f ON f.id = c.file_id
            WHERE fts_chunks MATCH ?1
              AND f.repo_id = ?2
              {}
            ORDER BY fts_chunks.rank ASC
            LIMIT {}
        "#,
            filter_clause, limit_placeholder
        )
    };

    // Build dynamic parameter list
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    param_values.push(Box::new(fts_query));
    param_values.push(Box::new(repo_id));

    if let Some(wid) = worktree_id {
        param_values.push(Box::new(wid));
    }

    if let Some(kinds) = kind_filter {
        for kind in kinds {
            param_values.push(Box::new(kind.clone()));
        }
    }

    if let Some(langs) = lang_filter {
        for lang in langs {
            param_values.push(Box::new(lang.clone()));
        }
    }

    param_values.push(Box::new(limit as i64));

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let mut results = Vec::new();

    let rows = stmt.query_map(params_refs.as_slice(), |row| {
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

    // Set position (0-indexed rank in result set)
    for (i, result) in results.iter_mut().enumerate() {
        result.position = i;
    }

    Ok(results)
}

/// Count the total number of FTS matches without the LIMIT constraint.
///
/// Executes a COUNT query using the same WHERE clause (MATCH, repo_id, worktree_id,
/// kind_filter, lang_filter) as [`search_fts`] but without LIMIT or ORDER BY.
/// The no-worktree path uses `COUNT(DISTINCT c.id)` to match the main query's
/// `SELECT DISTINCT`.
///
/// Returns 0 for empty queries.
pub fn count_fts_matches(
    conn: &Connection,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    kind_filter: Option<&[String]>,
    lang_filter: Option<&[String]>,
) -> Result<usize> {
    let fts_query = build_fts_query(query);
    if fts_query.is_empty() {
        return Ok(0);
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

    // Build dynamic filter conditions (same logic as search_fts)
    let mut param_idx: usize = if worktree_id.is_some() { 4 } else { 3 };
    let mut filter_conditions = Vec::new();

    if let Some(kinds) = kind_filter {
        if !kinds.is_empty() {
            let placeholders = (0..kinds.len())
                .map(|i| format!("?{}", param_idx + i))
                .collect::<Vec<_>>()
                .join(", ");
            filter_conditions.push(format!("c.kind IN ({})", placeholders));
            param_idx += kinds.len();
        }
    }

    if let Some(langs) = lang_filter {
        if !langs.is_empty() {
            let placeholders = (0..langs.len())
                .map(|i| format!("?{}", param_idx + i))
                .collect::<Vec<_>>()
                .join(", ");
            filter_conditions.push(format!("f.language IN ({})", placeholders));
            let _ = param_idx; // suppress unused assignment warning
        }
    }

    let filter_clause = if filter_conditions.is_empty() {
        String::new()
    } else {
        format!(" AND {}", filter_conditions.join(" AND "))
    };

    let count_sql = if worktree_id.is_some() {
        format!(
            r#"
            SELECT COUNT(*)
            FROM fts_chunks
            JOIN chunks c ON c.id = fts_chunks.rowid
            JOIN files f ON f.id = c.file_id
            JOIN chunk_worktrees cw ON cw.chunk_id = c.id
            WHERE fts_chunks MATCH ?1
              AND f.repo_id = ?2
              AND cw.worktree_id = ?3
              {}
            "#,
            filter_clause
        )
    } else {
        format!(
            r#"
            SELECT COUNT(DISTINCT c.id)
            FROM fts_chunks
            JOIN chunks c ON c.id = fts_chunks.rowid
            JOIN files f ON f.id = c.file_id
            WHERE fts_chunks MATCH ?1
              AND f.repo_id = ?2
              {}
            "#,
            filter_clause
        )
    };

    // Build parameter list (same as search_fts but without LIMIT)
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    param_values.push(Box::new(fts_query));
    param_values.push(Box::new(repo_id));

    if let Some(wid) = worktree_id {
        param_values.push(Box::new(wid));
    }

    if let Some(kinds) = kind_filter {
        for kind in kinds {
            param_values.push(Box::new(kind.clone()));
        }
    }

    if let Some(langs) = lang_filter {
        for lang in langs {
            param_values.push(Box::new(lang.clone()));
        }
    }

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let total_count: i64 = conn.query_row(&count_sql, params_refs.as_slice(), |row| row.get(0))?;

    Ok(total_count as usize)
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

    // ==================== Filter Generation / search_fts Tests ====================

    /// Create a minimal in-memory SQLite database with the schema needed for search_fts.
    fn setup_fts_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE repos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                root_path TEXT NOT NULL
            );
            CREATE TABLE worktrees (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id INTEGER NOT NULL REFERENCES repos(id),
                name TEXT NOT NULL,
                abs_path TEXT NOT NULL,
                UNIQUE(repo_id, name)
            );
            CREATE TABLE commits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id INTEGER NOT NULL REFERENCES repos(id),
                sha TEXT NOT NULL,
                committed_at DATETIME,
                UNIQUE(repo_id, sha)
            );
            CREATE TABLE files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id INTEGER NOT NULL REFERENCES repos(id),
                worktree_id INTEGER NOT NULL REFERENCES worktrees(id),
                commit_id INTEGER NOT NULL REFERENCES commits(id),
                relpath TEXT NOT NULL,
                language TEXT,
                content_hash TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                last_modified DATETIME,
                UNIQUE(commit_id, relpath, content_hash)
            );
            CREATE TABLE chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER NOT NULL REFERENCES files(id),
                blob_sha TEXT NOT NULL,
                symbol_name TEXT,
                kind TEXT NOT NULL,
                signature TEXT,
                docstring TEXT,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                preview TEXT NOT NULL,
                ts_doc_text TEXT,
                recency_score REAL NOT NULL,
                churn_score REAL NOT NULL,
                metadata JSON,
                UNIQUE(file_id, start_line, end_line)
            );
            CREATE TABLE chunk_worktrees (
                chunk_id INTEGER NOT NULL REFERENCES chunks(id),
                worktree_id INTEGER NOT NULL REFERENCES worktrees(id),
                PRIMARY KEY (chunk_id, worktree_id)
            );
            CREATE VIRTUAL TABLE fts_chunks USING fts5(
                content,
                docstring,
                symbol_name,
                content='chunks',
                content_rowid='id'
            );
            ",
        )
        .unwrap();
        conn
    }

    /// Insert a chunk with a specific kind, language, and searchable text into the test DB.
    /// Returns the chunk_id.
    fn insert_test_chunk(
        conn: &Connection,
        repo_id: i64,
        worktree_id: i64,
        commit_id: i64,
        relpath: &str,
        language: &str,
        kind: &str,
        symbol_name: &str,
        preview: &str,
        start_line: i32,
    ) -> i64 {
        // Upsert file (may already exist for same relpath)
        let content_hash = format!("hash_{}_{}", relpath, start_line);
        conn.execute(
            "INSERT OR IGNORE INTO files (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![repo_id, worktree_id, commit_id, relpath, language, content_hash, 100],
        )
        .unwrap();

        let file_id: i64 = conn
            .query_row(
                "SELECT id FROM files WHERE relpath = ?1 AND commit_id = ?2",
                params![relpath, commit_id],
                |row| row.get(0),
            )
            .unwrap();

        let blob_sha = format!("blob_{}_{}", symbol_name, start_line);
        let end_line = start_line + 10;

        conn.execute(
            "INSERT INTO chunks (file_id, blob_sha, symbol_name, kind, start_line, end_line, preview, ts_doc_text, recency_score, churn_score)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![file_id, blob_sha, symbol_name, kind, start_line, end_line, preview, preview, 1.0, 0.5],
        )
        .unwrap();

        let chunk_id: i64 = conn
            .query_row(
                "SELECT id FROM chunks WHERE file_id = ?1 AND start_line = ?2 AND end_line = ?3",
                params![file_id, start_line, end_line],
                |row| row.get(0),
            )
            .unwrap();

        // Insert into chunk_worktrees junction
        conn.execute(
            "INSERT OR IGNORE INTO chunk_worktrees (chunk_id, worktree_id) VALUES (?1, ?2)",
            params![chunk_id, worktree_id],
        )
        .unwrap();

        // Insert into FTS
        conn.execute(
            "INSERT OR REPLACE INTO fts_chunks(rowid, content, docstring, symbol_name) VALUES (?1, ?2, ?3, ?4)",
            params![chunk_id, preview, preview, symbol_name],
        )
        .unwrap();

        chunk_id
    }

    /// Set up a test database with diverse test data for filter testing.
    /// Returns (conn, repo_id, worktree_id, commit_id).
    fn setup_filter_test_data() -> (Connection, i64, i64, i64) {
        let conn = setup_fts_test_db();

        conn.execute(
            "INSERT INTO repos (name, root_path) VALUES ('test-repo', '/tmp/test')",
            [],
        )
        .unwrap();
        let repo_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO worktrees (repo_id, name, abs_path) VALUES (?1, 'main', '/tmp/test')",
            params![repo_id],
        )
        .unwrap();
        let worktree_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO commits (repo_id, sha) VALUES (?1, 'abc123')",
            params![repo_id],
        )
        .unwrap();
        let commit_id: i64 = conn.last_insert_rowid();

        // Insert diverse chunks:
        // Python functions
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "src/auth.py",
            "py",
            "func",
            "authenticate_user",
            "def authenticate_user(): pass",
            1,
        );
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "src/auth.py",
            "py",
            "class",
            "AuthManager",
            "class AuthManager: authenticate logic",
            20,
        );

        // TypeScript functions and classes
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "src/user.ts",
            "ts",
            "func",
            "getUser",
            "function getUser() authenticate fetch",
            1,
        );
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "src/user.ts",
            "ts",
            "class",
            "UserService",
            "class UserService authenticate",
            20,
        );
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "src/user.ts",
            "ts",
            "method",
            "findById",
            "method findById authenticate search",
            40,
        );

        // Rust functions
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "src/main.rs",
            "rs",
            "func",
            "main_authenticate",
            "fn main_authenticate() authenticate",
            1,
        );
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "src/main.rs",
            "rs",
            "import",
            "use_auth",
            "use auth authenticate module",
            20,
        );

        // Markdown headings
        insert_test_chunk(
            &conn,
            repo_id,
            worktree_id,
            commit_id,
            "docs/auth.md",
            "md",
            "heading_2",
            "auth_docs",
            "authenticate documentation guide",
            1,
        );

        (conn, repo_id, worktree_id, commit_id)
    }

    #[test]
    fn test_filter_generation_no_filters() {
        let (conn, _, _, _) = setup_filter_test_data();

        // No filters should return all matching results
        let results = search_fts(&conn, "test-repo", None, "authenticate", 50, None, None).unwrap();

        // All 8 chunks mention "authenticate" in their content
        assert!(
            results.len() >= 6,
            "No filters should return many results, got {}",
            results.len()
        );
    }

    #[test]
    fn test_filter_generation_kind_only_single() {
        let (conn, _, _, _) = setup_filter_test_data();

        let kind_filter = vec!["func".to_string()];
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            Some(&kind_filter),
            None,
        )
        .unwrap();

        // Should only return func chunks
        for result in &results {
            // Verify all returned chunks have kind == "func"
            let kind: String = conn
                .query_row(
                    "SELECT kind FROM chunks WHERE id = ?1",
                    params![result.chunk_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(kind, "func", "Expected kind 'func', got '{}'", kind);
        }
        assert!(!results.is_empty(), "Should find at least one func chunk");
    }

    #[test]
    fn test_filter_generation_kind_only_multiple() {
        let (conn, _, _, _) = setup_filter_test_data();

        let kind_filter = vec![
            "func".to_string(),
            "class".to_string(),
            "method".to_string(),
        ];
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            Some(&kind_filter),
            None,
        )
        .unwrap();

        // Should return func, class, and method chunks
        for result in &results {
            let kind: String = conn
                .query_row(
                    "SELECT kind FROM chunks WHERE id = ?1",
                    params![result.chunk_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(
                kind == "func" || kind == "class" || kind == "method",
                "Expected kind in [func, class, method], got '{}'",
                kind,
            );
        }
        assert!(
            results.len() >= 3,
            "Should find multiple chunk kinds, got {}",
            results.len()
        );
    }

    #[test]
    fn test_filter_generation_lang_only_single() {
        let (conn, _, _, _) = setup_filter_test_data();

        let lang_filter = vec!["py".to_string()];
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            None,
            Some(&lang_filter),
        )
        .unwrap();

        // Should only return chunks from Python files
        for result in &results {
            let language: String = conn
                .query_row(
                    "SELECT f.language FROM chunks c JOIN files f ON f.id = c.file_id WHERE c.id = ?1",
                    params![result.chunk_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(language, "py", "Expected language 'py', got '{}'", language);
        }
        assert!(!results.is_empty(), "Should find at least one py chunk");
    }

    #[test]
    fn test_filter_generation_lang_only_multiple() {
        let (conn, _, _, _) = setup_filter_test_data();

        let lang_filter = vec!["py".to_string(), "ts".to_string()];
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            None,
            Some(&lang_filter),
        )
        .unwrap();

        for result in &results {
            let language: String = conn
                .query_row(
                    "SELECT f.language FROM chunks c JOIN files f ON f.id = c.file_id WHERE c.id = ?1",
                    params![result.chunk_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(
                language == "py" || language == "ts",
                "Expected language in [py, ts], got '{}'",
                language,
            );
        }
        assert!(
            results.len() >= 3,
            "Should find results from py and ts files, got {}",
            results.len()
        );
    }

    #[test]
    fn test_filter_generation_both_filters() {
        let (conn, _, _, _) = setup_filter_test_data();

        let kind_filter = vec!["func".to_string()];
        let lang_filter = vec!["py".to_string()];
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            Some(&kind_filter),
            Some(&lang_filter),
        )
        .unwrap();

        // Should only return func chunks from py files (AND semantics)
        for result in &results {
            let (kind, language): (String, String) = conn
                .query_row(
                    "SELECT c.kind, f.language FROM chunks c JOIN files f ON f.id = c.file_id WHERE c.id = ?1",
                    params![result.chunk_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(kind, "func", "Expected kind 'func', got '{}'", kind);
            assert_eq!(language, "py", "Expected language 'py', got '{}'", language);
        }
        // We know there is exactly 1 Python func: authenticate_user
        assert_eq!(
            results.len(),
            1,
            "Should find exactly 1 py func chunk, got {}",
            results.len()
        );
    }

    #[test]
    fn test_filter_generation_empty_array_treated_as_none() {
        let (conn, _, _, _) = setup_filter_test_data();

        // Empty arrays should behave the same as None
        let results_none =
            search_fts(&conn, "test-repo", None, "authenticate", 50, None, None).unwrap();

        let empty_kind: Vec<String> = vec![];
        let empty_lang: Vec<String> = vec![];
        let results_empty = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            Some(&empty_kind),
            Some(&empty_lang),
        )
        .unwrap();

        assert_eq!(
            results_none.len(),
            results_empty.len(),
            "Empty filter arrays should return same results as None. None={}, Empty={}",
            results_none.len(),
            results_empty.len(),
        );
    }

    #[test]
    fn test_parameter_index_calculation() {
        // Test with worktree specified + both filters to validate param index arithmetic
        let (conn, _, _, _) = setup_filter_test_data();

        let kind_filter = vec!["func".to_string(), "class".to_string()];
        let lang_filter = vec!["py".to_string(), "ts".to_string(), "rs".to_string()];

        // With worktree (adds worktree_id param, shifting indices)
        let results = search_fts(
            &conn,
            "test-repo",
            Some("main"),
            "authenticate",
            50,
            Some(&kind_filter),
            Some(&lang_filter),
        )
        .unwrap();

        // Should return func and class chunks from py, ts, or rs files
        for result in &results {
            let (kind, language): (String, String) = conn
                .query_row(
                    "SELECT c.kind, f.language FROM chunks c JOIN files f ON f.id = c.file_id WHERE c.id = ?1",
                    params![result.chunk_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert!(
                kind == "func" || kind == "class",
                "Expected kind in [func, class], got '{}'",
                kind,
            );
            assert!(
                language == "py" || language == "ts" || language == "rs",
                "Expected language in [py, ts, rs], got '{}'",
                language,
            );
        }
        assert!(
            !results.is_empty(),
            "Should find results with combined filters"
        );
    }

    #[test]
    fn test_filter_nonexistent_kind_returns_empty() {
        let (conn, _, _, _) = setup_filter_test_data();

        let kind_filter = vec!["nonexistent_kind".to_string()];
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            Some(&kind_filter),
            None,
        )
        .unwrap();

        assert!(
            results.is_empty(),
            "Nonexistent kind should return empty results, got {}",
            results.len(),
        );
    }

    #[test]
    fn test_filter_nonexistent_lang_returns_empty() {
        let (conn, _, _, _) = setup_filter_test_data();

        let lang_filter = vec!["nonexistent_lang".to_string()];
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            None,
            Some(&lang_filter),
        )
        .unwrap();

        assert!(
            results.is_empty(),
            "Nonexistent lang should return empty results, got {}",
            results.len(),
        );
    }

    #[test]
    fn test_filter_long_kind_list() {
        let (conn, _, _, _) = setup_filter_test_data();

        // Test with 10+ kind values to ensure SQL generation handles long lists
        let kind_filter: Vec<String> = vec![
            "func",
            "class",
            "method",
            "import",
            "heading_2",
            "variable",
            "constant",
            "interface",
            "enum",
            "trait",
            "module",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            50,
            Some(&kind_filter),
            None,
        )
        .unwrap();

        // Should still work and return results for the kinds that match
        assert!(
            !results.is_empty(),
            "Long kind list should still return results"
        );
    }

    // ==================== count_fts_matches Tests ====================

    #[test]
    fn test_count_matches_actual_when_k_exceeds_results() {
        let (conn, _, _, _) = setup_filter_test_data();

        // Use a large limit (k=100) so all results are returned
        let results =
            search_fts(&conn, "test-repo", None, "authenticate", 100, None, None).unwrap();
        let count =
            count_fts_matches(&conn, "test-repo", None, "authenticate", None, None).unwrap();

        // When k > total matches, count should equal results.len()
        assert_eq!(
            count,
            results.len(),
            "Count ({}) should match actual results ({}) when k > total matches",
            count,
            results.len()
        );
    }

    #[test]
    fn test_count_exceeds_k_when_truncated() {
        let (conn, _, _, _) = setup_filter_test_data();

        // First, get total count with no limit
        let total_count =
            count_fts_matches(&conn, "test-repo", None, "authenticate", None, None).unwrap();
        assert!(
            total_count > 2,
            "Need more than 2 results for this test, got {}",
            total_count
        );

        // Now search with k=2 to force truncation
        let results = search_fts(&conn, "test-repo", None, "authenticate", 2, None, None).unwrap();

        assert_eq!(results.len(), 2, "Should return exactly k=2 results");
        assert!(
            total_count > results.len(),
            "Total count ({}) should exceed truncated results ({})",
            total_count,
            results.len()
        );
    }

    #[test]
    fn test_count_respects_kind_filter() {
        let (conn, _, _, _) = setup_filter_test_data();

        // Get unfiltered count
        let unfiltered_count =
            count_fts_matches(&conn, "test-repo", None, "authenticate", None, None).unwrap();

        // Get count with kind=["func"] filter
        let kind_filter = vec!["func".to_string()];
        let filtered_count = count_fts_matches(
            &conn,
            "test-repo",
            None,
            "authenticate",
            Some(&kind_filter),
            None,
        )
        .unwrap();

        // Filtered count should be less than unfiltered (we have classes, imports, etc.)
        assert!(
            filtered_count < unfiltered_count,
            "Kind-filtered count ({}) should be less than unfiltered count ({})",
            filtered_count,
            unfiltered_count
        );
        assert!(
            filtered_count > 0,
            "Kind-filtered count should be > 0 (we have func chunks)"
        );

        // Verify count matches actual search results
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            100,
            Some(&kind_filter),
            None,
        )
        .unwrap();
        assert_eq!(
            filtered_count,
            results.len(),
            "Filtered count ({}) should match filtered results ({})",
            filtered_count,
            results.len()
        );
    }

    #[test]
    fn test_count_respects_lang_filter() {
        let (conn, _, _, _) = setup_filter_test_data();

        // Get unfiltered count
        let unfiltered_count =
            count_fts_matches(&conn, "test-repo", None, "authenticate", None, None).unwrap();

        // Get count with lang=["py"] filter
        let lang_filter = vec!["py".to_string()];
        let filtered_count = count_fts_matches(
            &conn,
            "test-repo",
            None,
            "authenticate",
            None,
            Some(&lang_filter),
        )
        .unwrap();

        // Filtered count should be less than unfiltered (we have ts, rs, md chunks)
        assert!(
            filtered_count < unfiltered_count,
            "Lang-filtered count ({}) should be less than unfiltered count ({})",
            filtered_count,
            unfiltered_count
        );
        assert!(
            filtered_count > 0,
            "Lang-filtered count should be > 0 (we have py chunks)"
        );

        // Verify count matches actual search results
        let results = search_fts(
            &conn,
            "test-repo",
            None,
            "authenticate",
            100,
            None,
            Some(&lang_filter),
        )
        .unwrap();
        assert_eq!(
            filtered_count,
            results.len(),
            "Filtered count ({}) should match filtered results ({})",
            filtered_count,
            results.len()
        );
    }

    #[test]
    fn test_count_respects_worktree_filter() {
        let conn = setup_fts_test_db();

        // Create repo with two worktrees
        conn.execute(
            "INSERT INTO repos (name, root_path) VALUES ('wt-repo', '/tmp/wt')",
            [],
        )
        .unwrap();
        let repo_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO worktrees (repo_id, name, abs_path) VALUES (?1, 'main', '/tmp/wt/main')",
            params![repo_id],
        )
        .unwrap();
        let wt_main: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO worktrees (repo_id, name, abs_path) VALUES (?1, 'feature', '/tmp/wt/feature')",
            params![repo_id],
        )
        .unwrap();
        let wt_feature: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO commits (repo_id, sha) VALUES (?1, 'sha1')",
            params![repo_id],
        )
        .unwrap();
        let commit_id: i64 = conn.last_insert_rowid();

        // Insert 3 chunks in main worktree
        for i in 0..3 {
            insert_test_chunk(
                &conn,
                repo_id,
                wt_main,
                commit_id,
                &format!("main_{}.rs", i),
                "rs",
                "func",
                &format!("main_fn_{}", i),
                "searchable main function",
                i * 20,
            );
        }

        // Insert 1 chunk in feature worktree
        insert_test_chunk(
            &conn,
            repo_id,
            wt_feature,
            commit_id,
            "feature_0.rs",
            "rs",
            "func",
            "feature_fn_0",
            "searchable feature function",
            0,
        );

        // Count for main worktree should be 3
        let main_count =
            count_fts_matches(&conn, "wt-repo", Some("main"), "searchable", None, None).unwrap();
        assert_eq!(main_count, 3, "Main worktree should have 3 matches");

        // Count for feature worktree should be 1
        let feature_count =
            count_fts_matches(&conn, "wt-repo", Some("feature"), "searchable", None, None).unwrap();
        assert_eq!(feature_count, 1, "Feature worktree should have 1 match");

        // Count without worktree should be >= 4 (all chunks across worktrees)
        let all_count =
            count_fts_matches(&conn, "wt-repo", None, "searchable", None, None).unwrap();
        assert!(
            all_count >= 4,
            "All worktrees count ({}) should be >= 4",
            all_count
        );
    }

    #[test]
    fn test_count_with_zero_k() {
        // Test FTS search with k=0 (should return empty Vec but count may be non-zero)
        let (conn, _, _, _) = setup_filter_test_data();

        let hits = search_fts(&conn, "test-repo", None, "authenticate", 0, None, None).unwrap();
        assert_eq!(hits.len(), 0, "k=0 should return empty results");

        // total_count may be > 0 if matches exist
        let total_count =
            count_fts_matches(&conn, "test-repo", None, "authenticate", None, None).unwrap();
        assert!(
            total_count > 0,
            "COUNT should find matches even when k=0, got {}",
            total_count
        );
    }

    #[test]
    fn test_count_empty_query_returns_zero() {
        let (conn, _, _, _) = setup_filter_test_data();

        let count = count_fts_matches(&conn, "test-repo", None, "", None, None).unwrap();
        assert_eq!(count, 0, "Empty query should return count of 0");

        let count = count_fts_matches(&conn, "test-repo", None, "   ", None, None).unwrap();
        assert_eq!(count, 0, "Whitespace-only query should return count of 0");
    }
}
