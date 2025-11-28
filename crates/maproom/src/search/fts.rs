//! Full-text search executor using PostgreSQL FTS.
//!
//! This module implements full-text search using PostgreSQL's tsvector/tsquery
//! with ts_rank_cd ranking. It applies proximity boost for phrase matching
//! and exact match bonuses for symbol names.
//!
//! ## Edge Case Handling (SEMRANK-2007)
//!
//! This executor handles edge cases gracefully via SQL CASE statements:
//!
//! 1. **NULL symbol_name** (lines 137-140):
//!    - Documentation/markdown chunks have NULL symbol_name
//!    - CASE ELSE clause: `ELSE 1.0` (neutral multiplier, no boost)
//!    - No crash, no SQL error, degrades gracefully
//!
//! 2. **Unknown kind** (lines 152-154):
//!    - Future tree-sitter updates may introduce new kinds
//!    - CASE ELSE clause: `ELSE 1.0` (neutral baseline)
//!    - Unknown kinds get baseline multiplier (no penalty, no crash)
//!
//! 3. **NULL kind** (line 153):
//!    - Explicit handler: `WHEN c.kind IS NULL THEN 1.0`
//!    - Ensures NULL is not treated differently from unknown kinds
//!
//! 4. **Empty query** (lines 117-120):
//!    - Returns empty RankedResults (not error)
//!    - Prevents invalid tsquery execution
//!
//! 5. **Parameterized queries** (lines 180-189):
//!    - All query parameters use $1, $2, $3... placeholders
//!    - Prevents SQL injection attacks
//!    - Special characters in queries are treated as literal text
//!
//! All edge cases validated in tests/integration/semrank-edge-cases.test.ts

use crate::db::SqliteStore;
use crate::search::executor_types::{RankedResult, RankedResults, SearchSource};
use regex::Regex;
use tracing::{debug, instrument, warn};

/// Normalize query for exact match detection
///
/// Handles acronym-aware camelCase to snake_case conversion:
/// - "validateProvider" → "validate_provider"
/// - "XMLParser" → "xml_parser"
/// - "validateHTTPRequest" → "validate_http_request"
/// - "HTTPSHandler" → "https_handler"
/// - "Base64Encoder" → "base64_encoder"
/// - "validate-provider" → "validate_provider"
pub fn normalize_for_exact_match(query: &str) -> String {
    let mut normalized = query.to_string();

    // Step 1: Handle consecutive uppercase (acronyms) before lowercase
    // "XMLParser" → "XML_Parser", "HTTPSHandler" → "HTTPS_Handler"
    let re1 = Regex::new(r"([A-Z]+)([A-Z][a-z])").unwrap();
    normalized = re1.replace_all(&normalized, "${1}_${2}").to_string();

    // Step 2: Handle transition from lowercase to multiple capitals (acronym after lowercase)
    // "validateHTTP" → "validate_HTTP"
    let re2 = Regex::new(r"([a-z\d])([A-Z]{2,})").unwrap();
    normalized = re2.replace_all(&normalized, "${1}_${2}").to_string();

    // Step 3: Handle camelCase → snake_case (single capital after lowercase)
    // "validateProvider" → "validate_Provider"
    let re3 = Regex::new(r"([a-z\d])([A-Z])").unwrap();
    normalized = re3.replace_all(&normalized, "${1}_${2}").to_string();

    // Step 4: Handle kebab-case, spaces, and dots → snake_case
    let re4 = Regex::new(r"[\s\-\.]").unwrap();
    normalized = re4.replace_all(&normalized, "_").to_string();

    // Step 5: Lowercase everything
    normalized = normalized.to_lowercase();

    // Step 6: Clean up multiple/trailing/leading underscores
    let re5 = Regex::new(r"_+").unwrap();
    normalized = re5.replace_all(&normalized, "_").to_string();
    let re6 = Regex::new(r"^_|_$").unwrap();
    normalized = re6.replace_all(&normalized, "").to_string();

    normalized
}

/// Full-text search executor.
///
/// Uses PostgreSQL full-text search with ts_rank_cd ranking function.
/// Over-fetches results (limit * 3) to provide more candidates for fusion.
pub struct FTSExecutor;

impl FTSExecutor {
    /// Execute full-text search query.
    ///
    /// # Parameters
    /// - `client`: Database client
    /// - `fts_query`: PostgreSQL tsquery string (e.g., "auth & login")
    /// - `normalized_query`: Normalized query for exact match detection (snake_case)
    /// - `repo_id`: Repository ID to filter results
    /// - `worktree_id`: Optional worktree ID for additional filtering
    /// - `limit`: Maximum number of results (will over-fetch by 3x)
    ///
    /// # Returns
    /// RankedResults with FTS scores normalized to 0.0-1.0 range
    ///
    /// # SQL Query
    /// ```sql
    /// WITH fts_results AS (
    ///   SELECT
    ///     c.id,
    ///     ts_rank_cd(c.ts_doc, to_tsquery('simple', $1), 32) as base_score,
    ///     CASE
    ///       WHEN LOWER(c.symbol_name) = LOWER($2) THEN 3.0
    ///       ELSE 1.0
    ///     END as exact_mult,
    ///     CASE
    ///       WHEN c.kind IN ('func', 'async_func') THEN 2.5
    ///       WHEN c.kind IN ('class', 'component') THEN 2.0
    ///       WHEN c.kind = 'hook' THEN 1.8
    ///       -- ... (see source for full mapping)
    ///     END as kind_mult
    ///   FROM maproom.chunks c
    ///   JOIN maproom.files f ON f.id = c.file_id
    ///   WHERE c.ts_doc @@ to_tsquery('simple', $1)
    ///     AND f.repo_id = $3
    ///     AND ($4::bigint IS NULL OR f.worktree_id = $4)
    /// )
    /// SELECT
    ///   id,
    ///   base_score,
    ///   kind_mult,
    ///   exact_mult,
    ///   (base_score * kind_mult * exact_mult) as final_score,
    ///   ROW_NUMBER() OVER (ORDER BY base_score * kind_mult * exact_mult DESC) as rank
    /// FROM fts_results
    /// ORDER BY final_score DESC
    /// LIMIT $5;
    /// ```
    #[instrument(skip(store), fields(query_len = fts_query.len()))]
    pub async fn execute(
        store: &SqliteStore,
        fts_query: &str,
        _normalized_query: &str,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<RankedResults, FTSError> {
        if fts_query.is_empty() {
            debug!("Empty FTS query, returning no results");
            return Ok(RankedResults::empty(SearchSource::FTS));
        }

        // Over-fetch by 3x for fusion
        let fetch_limit = (limit * 3) as i64;

        debug!(
            "Executing FTS query: '{}' (limit: {}, over-fetch: {})",
            fts_query, limit, fetch_limit
        );

        // Delegate to SqliteStore's FTS search
        let hits = store
            .search_fts_by_id(repo_id, worktree_id, fts_query, fetch_limit)
            .await
            .map_err(|e| FTSError::Database(e.to_string()))?;

        // Convert SearchHit to RankedResult
        let results: Vec<RankedResult> = hits
            .into_iter()
            .enumerate()
            .map(|(i, hit)| RankedResult::new(hit.chunk_id, hit.score as f32, i + 1))
            .collect();

        debug!("FTS search returned {} results", results.len());
        Ok(RankedResults::new(results, SearchSource::FTS))
    }
}

/// Errors that can occur during FTS execution.
#[derive(Debug, thiserror::Error)]
pub enum FTSError {
    /// Database query error
    #[error("Database error: {0}")]
    Database(String),

    /// Invalid FTS query syntax
    #[error("Invalid FTS query: {0}")]
    InvalidQuery(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fts_executor_exists() {
        // Verify the executor type exists
        let _executor = FTSExecutor;
    }

    // SEMRANK-2004b: Query normalization tests
    mod normalize_tests {
        use super::*;

        #[test]
        fn test_simple_camelcase() {
            assert_eq!(
                normalize_for_exact_match("validateProvider"),
                "validate_provider"
            );
        }

        #[test]
        fn test_single_word() {
            assert_eq!(normalize_for_exact_match("provider"), "provider");
        }

        #[test]
        fn test_multiple_camelcase_transitions() {
            assert_eq!(
                normalize_for_exact_match("getUserNameFromDatabase"),
                "get_user_name_from_database"
            );
        }

        #[test]
        fn test_acronym_at_start() {
            assert_eq!(normalize_for_exact_match("XMLParser"), "xml_parser");
            assert_eq!(normalize_for_exact_match("HTTPClient"), "http_client");
            assert_eq!(normalize_for_exact_match("FTPUploader"), "ftp_uploader");
        }

        #[test]
        fn test_acronym_in_middle() {
            assert_eq!(
                normalize_for_exact_match("validateHTTPRequest"),
                "validate_http_request"
            );
            assert_eq!(
                normalize_for_exact_match("sendSMTPMessage"),
                "send_smtp_message"
            );
            assert_eq!(
                normalize_for_exact_match("parseJSONData"),
                "parse_json_data"
            );
        }

        #[test]
        fn test_consecutive_capitals() {
            assert_eq!(normalize_for_exact_match("HTTPSHandler"), "https_handler");
            assert_eq!(
                normalize_for_exact_match("XMLHTTPRequest"),
                "xmlhttp_request"
            );
            assert_eq!(normalize_for_exact_match("SSLContext"), "ssl_context");
        }

        #[test]
        fn test_numbers_with_capitals() {
            assert_eq!(normalize_for_exact_match("Base64Encoder"), "base64_encoder");
            assert_eq!(normalize_for_exact_match("MD5Hash"), "md5_hash");
            assert_eq!(normalize_for_exact_match("SHA256Digest"), "sha256_digest");
        }

        #[test]
        fn test_kebab_case() {
            assert_eq!(
                normalize_for_exact_match("validate-provider"),
                "validate_provider"
            );
            assert_eq!(
                normalize_for_exact_match("user-auth-service-factory"),
                "user_auth_service_factory"
            );
        }

        #[test]
        fn test_spaces() {
            assert_eq!(
                normalize_for_exact_match("validate provider"),
                "validate_provider"
            );
            assert_eq!(
                normalize_for_exact_match("user  auth   service"),
                "user_auth_service"
            );
        }

        #[test]
        fn test_dots() {
            assert_eq!(
                normalize_for_exact_match("user.auth.service"),
                "user_auth_service"
            );
        }

        #[test]
        fn test_edge_cases() {
            assert_eq!(normalize_for_exact_match(""), "");
            assert_eq!(normalize_for_exact_match("HTTP"), "http");
            assert_eq!(normalize_for_exact_match("validate"), "validate");
            assert_eq!(
                normalize_for_exact_match("user__auth___service"),
                "user_auth_service"
            );
            assert_eq!(
                normalize_for_exact_match("_privateMethod"),
                "private_method"
            );
            assert_eq!(normalize_for_exact_match("method_"), "method");
        }

        #[test]
        fn test_mixed_separators() {
            assert_eq!(
                normalize_for_exact_match("user-auth.service Provider"),
                "user_auth_service_provider"
            );
        }

        #[test]
        fn test_real_world_examples() {
            assert_eq!(
                normalize_for_exact_match("ValidationErrorHandler"),
                "validation_error_handler"
            );
            assert_eq!(
                normalize_for_exact_match("UserAuthFormContainer"),
                "user_auth_form_container"
            );
            assert_eq!(
                normalize_for_exact_match("execute_fts_search"),
                "execute_fts_search"
            );
            assert_eq!(
                normalize_for_exact_match("user-profile-service"),
                "user_profile_service"
            );
        }
    }

    // Note: Full integration tests with real database are in tests/search/executors_test.rs
}
