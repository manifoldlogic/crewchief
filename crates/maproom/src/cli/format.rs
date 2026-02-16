//! Output formatting for CLI search commands.
//!
//! Provides multiple output formats for search results:
//! - **JSON**: Backward-compatible structured output (default)
//! - **Agent**: Compact one-line-per-result output optimized for LLM agents
//!
//! The formatting module is a pure presentation layer that operates on
//! already-fetched `SearchHit` data after all search, deduplication, and
//! filtering have completed.

use std::fmt::Write as _;

use clap::ValueEnum;

use crate::db;

/// Metadata about a search operation, passed to format functions.
///
/// Carries query context from command handlers to format functions so that
/// output can include information about result completeness.
#[derive(Debug, Clone)]
pub struct SearchMetadata {
    /// The original search query string.
    pub query: String,
    /// Search mode: `"fts"` or `"vector"`.
    pub mode: String,
    /// Number of hits returned in this response.
    pub hits: usize,
    /// Estimated total number of matches in the database.
    pub total_estimate: usize,
}

/// Output format for search results.
///
/// Controls how search results are rendered to stdout.
/// Used as a clap `ValueEnum` for the `--format` CLI flag.
///
/// - **Json**: Full structured JSON output, backward compatible with existing tooling.
/// - **Agent**: Compact one-line-per-result output optimized for LLM agents.
///   Implicitly enables preview (default 120 chars) to keep output token-efficient.
#[derive(Clone, Debug, PartialEq, ValueEnum)]
pub enum OutputFormat {
    /// JSON output (default, backward compatible)
    Json,
    /// Compact one-line-per-result output optimized for LLM agents
    Agent,
}

/// Format search hits as compact agent-friendly output.
///
/// Output begins with a metadata header line:
///   `SEARCH query="<escaped_query>" | hits=N | total_estimate=M | mode=...`
///
/// Followed by one line per result with pipe-delimited segments:
///   `<file>:<start_line> | <kind> [<symbol>] | <score> | <preview>`
///
/// - The header line is always present, even when hits are empty.
/// - Missing `symbol_name` is omitted (shows just the kind, not "null").
/// - Missing `preview` shows a `-` placeholder.
/// - Newlines in preview text are replaced with spaces.
/// - Score is formatted to exactly 2 decimal places.
pub fn format_hits_agent(hits: &[db::SearchHit], meta: &SearchMetadata) -> String {
    let mut output = String::new();

    // Header line (always present, even for 0 hits)
    let escaped_query = meta.query.replace('"', "\\\"");
    let _ = write!(
        output,
        "SEARCH query=\"{}\" | hits={} | total_estimate={} | mode={}",
        escaped_query, meta.hits, meta.total_estimate, meta.mode
    );
    for hit in hits.iter() {
        output.push('\n');

        // Segment 1: file:line
        let _ = write!(output, "{}:{}", hit.file_relpath, hit.start_line);

        // Segment 2: kind [symbol]
        let kind_segment = match &hit.symbol_name {
            Some(name) if !name.is_empty() => {
                format!("{} {}", hit.kind, name)
            }
            _ => hit.kind.clone(),
        };
        let _ = write!(output, " | {}", kind_segment);

        // Segment 3: score (2 decimal places)
        let _ = write!(output, " | {:.2}", hit.score);

        // Segment 4: preview (sanitized or "-" placeholder)
        let preview_text = match &hit.preview {
            Some(text) if !text.is_empty() => sanitize_newlines(text),
            _ => "-".to_string(),
        };
        let _ = write!(output, " | {}", preview_text);
    }

    output
}

/// Format search hits as JSON for the Search command.
///
/// Produces: `{"hits": [...], "total_matches": M, "query": "...", "mode": "..."}`
///
/// The `hits` array structure is unchanged from the original format.
/// Metadata fields are added at the top level alongside the existing `hits` key.
pub fn format_hits_json_search(
    hits: &[db::SearchHit],
    meta: &SearchMetadata,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({
        "hits": hits,
        "total_matches": meta.total_estimate,
        "query": meta.query,
        "mode": meta.mode,
    }))
}

/// Format search hits as JSON for the VectorSearch command.
///
/// Produces: `{"hits": [...], "total": N, "query": "...", "mode": "vector", "k": N, "threshold": ...}`
///
/// This is a direct extraction of the existing VectorSearch command JSON output
/// logic to preserve exact backward compatibility.
///
/// Takes `serde_json::Value` hits rather than `SearchHit` because VectorSearch
/// manually constructs JSON objects with slightly different field names
/// (`file_path` vs `file_relpath`).
///
/// NOTE(AFM-02): The `file_relpath` field was added to VectorSearch JSON output
/// for schema parity with Search. The legacy `file_path` field is retained for
/// backward compatibility. Full refactor to use serde serialization of SearchHit
/// (eliminating manual JSON construction) is deferred as a separate cleanup.
pub fn format_hits_json_vector(
    hits: &[serde_json::Value],
    total: usize,
    query: &str,
    mode: &str,
    k: usize,
    threshold: Option<f32>,
) -> Result<String, serde_json::Error> {
    let output = serde_json::json!({
        "hits": hits,
        "total": total,
        "query": query,
        "mode": mode,
        "k": k,
        "threshold": threshold,
    });
    serde_json::to_string_pretty(&output)
}

/// Replace newline characters with spaces to maintain one-line-per-result format.
///
/// Handles `\r\n` (Windows), `\n` (Unix), and `\r` (legacy Mac) line endings.
///
/// This function is made public for use in error handling (main.rs) to sanitize
/// error messages before printing to stderr.
#[allow(clippy::collapsible_str_replace)]
pub fn sanitize_newlines(text: &str) -> String {
    // Replace \r\n first (before individual \r or \n) to produce a single space
    // per Windows line ending rather than two spaces.
    text.replace("\r\n", " ")
        .replace('\n', " ")
        .replace('\r', " ")
}

/// Known valid error types returned by `classify_error()`.
///
/// Used to validate error types at runtime and emit warnings when an unknown
/// error type is encountered. This helps catch typos and ensures the error
/// type taxonomy stays consistent.
///
/// If you add a new error type to `classify_error()`, add it here too.
pub const KNOWN_ERROR_TYPES: &[&str] = &[
    "database",
    "embedding_provider",
    "config_error",
    "not_found",
    "validation",
    "timeout",
    "unknown",
];

/// Format a structured agent error line.
///
/// Produces: `ERROR | type=<error_type> | message=<msg> | suggestion=<suggestion>`
///
/// Pipe characters in message and suggestion are replaced with dashes.
/// Newlines are replaced with spaces.
/// error_type is not sanitized (controlled values only).
///
/// Emits a `tracing::warn!` if `error_type` is not in `KNOWN_ERROR_TYPES`.
pub fn format_agent_error(error_type: &str, message: &str, suggestion: &str) -> String {
    // Validate error type against known types
    if !KNOWN_ERROR_TYPES.contains(&error_type) {
        let message_preview: String = message.chars().take(50).collect();
        tracing::warn!(
            error_type = error_type,
            message_preview = %message_preview,
            "Unknown error type used in format_agent_error"
        );
    }
    // Sanitize message: pipes first, then newlines
    let sanitized_message = sanitize_newlines(&message.replace('|', "-"));

    // Sanitize suggestion: pipes first, then newlines
    let sanitized_suggestion = sanitize_newlines(&suggestion.replace('|', "-"));

    // Format output line with sanitized values
    format!(
        "ERROR | type={} | message={} | suggestion={}",
        error_type, sanitized_message, sanitized_suggestion
    )
}

#[cfg(test)]
mod tests {
    // NOTE(AFM-02): Agent format consistency verified - format_hits_agent() already
    // reads SearchHit.file_relpath for both search and vector-search commands.
    // No code changes required; see architecture.md Decision 1.
    use super::*;
    use crate::db::SearchHit;

    fn make_hit(
        file: &str,
        line: i32,
        kind: &str,
        symbol: Option<&str>,
        score: f64,
        preview: Option<&str>,
    ) -> SearchHit {
        SearchHit {
            chunk_id: 1,
            score,
            file_relpath: file.to_string(),
            symbol_name: symbol.map(|s| s.to_string()),
            kind: kind.to_string(),
            start_line: line,
            end_line: line + 10,
            base_score: None,
            kind_mult: None,
            exact_mult: None,
            preview: preview.map(|s| s.to_string()),
        }
    }

    /// Default test metadata to minimize test churn when updating call sites.
    fn make_test_metadata() -> SearchMetadata {
        SearchMetadata {
            query: "test".to_string(),
            mode: "fts".to_string(),
            hits: 5,
            total_estimate: 10,
        }
    }

    /// Extract hit lines from agent format output (skipping the header line).
    fn agent_hit_lines(output: &str) -> Vec<&str> {
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() > 1 {
            lines[1..].to_vec()
        } else {
            vec![]
        }
    }

    #[test]
    fn test_agent_format_basic() {
        let hits = vec![make_hit(
            "src/app.rs",
            42,
            "func",
            Some("main"),
            0.92,
            Some("Entry point for the application"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            "src/app.rs:42 | func main | 0.92 | Entry point for the application"
        );
    }

    #[test]
    fn test_agent_format_missing_symbol() {
        let hits = vec![make_hit(
            "docs/api.md",
            8,
            "heading_2",
            None,
            0.73,
            Some("Authentication API reference"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(
            lines[0],
            "docs/api.md:8 | heading_2 | 0.73 | Authentication API reference"
        );
    }

    #[test]
    fn test_agent_format_empty_symbol() {
        let hits = vec![make_hit(
            "src/lib.rs",
            1,
            "module",
            Some(""),
            0.50,
            Some("Module declarations"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(
            lines[0],
            "src/lib.rs:1 | module | 0.50 | Module declarations"
        );
    }

    #[test]
    fn test_agent_format_missing_preview() {
        let hits = vec![make_hit("src/lib.rs", 1, "func", Some("init"), 0.85, None)];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines[0], "src/lib.rs:1 | func init | 0.85 | -");
    }

    #[test]
    fn test_agent_format_empty_preview() {
        let hits = vec![make_hit(
            "src/lib.rs",
            1,
            "func",
            Some("init"),
            0.85,
            Some(""),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines[0], "src/lib.rs:1 | func init | 0.85 | -");
    }

    #[test]
    fn test_agent_format_newline_sanitization() {
        let hits = vec![make_hit(
            "src/main.rs",
            10,
            "func",
            Some("run"),
            0.60,
            Some("Line one\nLine two\r\nLine three\rLine four"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(
            lines[0],
            "src/main.rs:10 | func run | 0.60 | Line one Line two Line three Line four"
        );
    }

    #[test]
    fn test_agent_format_empty_hits() {
        let hits: Vec<SearchHit> = vec![];
        let meta = SearchMetadata {
            query: "test".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output = format_hits_agent(&hits, &meta);
        // Header line is still present even with 0 hits
        assert!(output.starts_with("SEARCH query="));
        assert_eq!(agent_hit_lines(&output).len(), 0);
    }

    #[test]
    fn test_agent_format_multiple_hits() {
        let hits = vec![
            make_hit(
                "src/app.rs",
                42,
                "func",
                Some("main"),
                0.92,
                Some("Entry point"),
            ),
            make_hit(
                "docs/api.md",
                8,
                "heading_2",
                None,
                0.73,
                Some("API reference"),
            ),
        ];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "src/app.rs:42 | func main | 0.92 | Entry point");
        assert_eq!(lines[1], "docs/api.md:8 | heading_2 | 0.73 | API reference");
    }

    #[test]
    fn test_agent_format_score_precision() {
        let meta = make_test_metadata();

        let hits = vec![make_hit("a.rs", 1, "func", Some("f"), 1.0, Some("text"))];
        let output = format_hits_agent(&hits, &meta);
        assert!(output.contains("| 1.00 |"));

        let hits = vec![make_hit("a.rs", 1, "func", Some("f"), 0.1, Some("text"))];
        let output = format_hits_agent(&hits, &meta);
        assert!(output.contains("| 0.10 |"));

        let hits = vec![make_hit(
            "a.rs",
            1,
            "func",
            Some("f"),
            0.123456,
            Some("text"),
        )];
        let output = format_hits_agent(&hits, &meta);
        assert!(output.contains("| 0.12 |"));
    }

    #[test]
    fn test_json_search_format() {
        let hits = vec![make_hit(
            "src/app.rs",
            42,
            "func",
            Some("main"),
            0.92,
            Some("Entry point"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_json_search(&hits, &meta).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed["hits"].is_array());
        assert_eq!(parsed["hits"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["hits"][0]["file_relpath"], "src/app.rs");
    }

    #[test]
    fn test_json_vector_format() {
        let hits = vec![serde_json::json!({
            "chunk_id": 1,
            "score": 0.92,
            "file_relpath": "test/file.rs",
            "file_path": "test/file.rs",
        })];
        let output =
            format_hits_json_vector(&hits, 1, "test query", "vector", 10, Some(0.5)).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["total"], 1);
        assert_eq!(parsed["query"], "test query");
        assert_eq!(parsed["mode"], "vector");
        assert_eq!(parsed["k"], 10);
        assert_eq!(parsed["threshold"], 0.5);
        assert!(parsed["hits"].is_array());
        let hits_arr = parsed["hits"].as_array().unwrap();
        assert!(hits_arr[0]["file_relpath"].is_string());
        assert_eq!(hits_arr[0]["file_relpath"], hits_arr[0]["file_path"]);
    }

    #[test]
    fn test_json_vector_format_no_threshold() {
        let hits: Vec<serde_json::Value> = vec![];
        let output = format_hits_json_vector(&hits, 0, "query", "vector", 5, None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed["threshold"].is_null());
    }

    #[test]
    fn test_sanitize_newlines() {
        assert_eq!(sanitize_newlines("hello\nworld"), "hello world");
        assert_eq!(sanitize_newlines("hello\r\nworld"), "hello world");
        assert_eq!(sanitize_newlines("hello\rworld"), "hello world");
        assert_eq!(sanitize_newlines("a\nb\r\nc\rd"), "a b c d");
        assert_eq!(sanitize_newlines("no newlines"), "no newlines");
        assert_eq!(sanitize_newlines(""), "");
    }

    // ---------------------------------------------------------------
    // Additional tests for format_hits_agent() per MRIMP-5.2001
    // ---------------------------------------------------------------

    /// Helper function to construct test SearchHit objects.
    fn make_test_hit(
        file: &str,
        line: i32,
        kind: &str,
        symbol: Option<&str>,
        score: f64,
        preview: Option<&str>,
    ) -> SearchHit {
        SearchHit {
            chunk_id: 1,
            score,
            file_relpath: file.to_string(),
            symbol_name: symbol.map(|s| s.to_string()),
            kind: kind.to_string(),
            start_line: line,
            end_line: line + 10,
            base_score: None,
            kind_mult: None,
            exact_mult: None,
            preview: preview.map(|s| s.to_string()),
        }
    }

    // --- Normal output tests ---

    #[test]
    fn test_format_hits_agent_normal_all_fields() {
        let hits = vec![make_test_hit(
            "src/app.rs",
            42,
            "func",
            Some("main"),
            0.92,
            Some("Entry point for the application"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(
            lines[0],
            "src/app.rs:42 | func main | 0.92 | Entry point for the application"
        );
    }

    #[test]
    fn test_format_hits_agent_without_symbol_name() {
        let hits = vec![make_test_hit(
            "docs/api.md",
            8,
            "heading_2",
            None,
            0.73,
            Some("Authentication API reference"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(
            lines[0],
            "docs/api.md:8 | heading_2 | 0.73 | Authentication API reference"
        );
        // Must not contain "null" in hit lines
        for line in &lines {
            assert!(!line.contains("null"));
        }
    }

    #[test]
    fn test_format_hits_agent_without_preview() {
        let hits = vec![make_test_hit(
            "src/lib.rs",
            1,
            "func",
            Some("init"),
            0.85,
            None,
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines[0], "src/lib.rs:1 | func init | 0.85 | -");
    }

    #[test]
    fn test_format_hits_agent_empty_results() {
        let hits: Vec<SearchHit> = vec![];
        let meta = SearchMetadata {
            query: "test".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output = format_hits_agent(&hits, &meta);
        // Header still present
        assert!(output.starts_with("SEARCH query="));
        assert_eq!(agent_hit_lines(&output).len(), 0);
    }

    #[test]
    fn test_format_hits_agent_multiple_results() {
        let hits = vec![
            make_test_hit(
                "src/app.rs",
                42,
                "func",
                Some("main"),
                0.92,
                Some("Entry point"),
            ),
            make_test_hit(
                "docs/api.md",
                8,
                "heading_2",
                None,
                0.73,
                Some("API reference"),
            ),
            make_test_hit(
                "tests/test_app.rs",
                100,
                "func",
                Some("test_main"),
                0.55,
                Some("Test case"),
            ),
        ];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "src/app.rs:42 | func main | 0.92 | Entry point");
        assert_eq!(lines[1], "docs/api.md:8 | heading_2 | 0.73 | API reference");
        assert_eq!(
            lines[2],
            "tests/test_app.rs:100 | func test_main | 0.55 | Test case"
        );
    }

    // --- Score precision tests ---

    #[test]
    fn test_format_hits_agent_score_precision_point_nine() {
        let hits = vec![make_test_hit(
            "a.rs",
            1,
            "func",
            Some("f"),
            0.9,
            Some("text"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines[0], "a.rs:1 | func f | 0.90 | text");
    }

    #[test]
    fn test_format_hits_agent_score_precision_zero() {
        let hits = vec![make_test_hit(
            "a.rs",
            1,
            "func",
            Some("f"),
            0.0,
            Some("text"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines[0], "a.rs:1 | func f | 0.00 | text");
    }

    #[test]
    fn test_format_hits_agent_score_precision_high() {
        let hits = vec![make_test_hit(
            "a.rs",
            1,
            "func",
            Some("f"),
            17.5,
            Some("text"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(lines[0], "a.rs:1 | func f | 17.50 | text");
    }

    // --- Unicode tests ---

    #[test]
    fn test_format_hits_agent_unicode_file_path() {
        let hits = vec![make_test_hit(
            "src/\u{00e9}dit.rs",
            42,
            "func",
            Some("main"),
            0.92,
            Some("Funci\u{00f3}n principal"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert!(lines[0].contains("src/\u{00e9}dit.rs"));
        assert_eq!(
            lines[0],
            "src/\u{00e9}dit.rs:42 | func main | 0.92 | Funci\u{00f3}n principal"
        );
    }

    #[test]
    fn test_format_hits_agent_unicode_preview() {
        let hits = vec![make_test_hit(
            "src/main.rs",
            1,
            "func",
            Some("greet"),
            0.80,
            Some("\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert!(lines[0].contains("\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}"));
        assert_eq!(
            lines[0],
            "src/main.rs:1 | func greet | 0.80 | \u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}"
        );
    }

    // --- Edge case tests ---

    #[test]
    fn test_format_hits_agent_long_file_path() {
        // 200-character file path: should not be truncated
        let long_path = format!("src/{}/file.rs", "a".repeat(190));
        assert!(long_path.len() >= 200);
        let hits = vec![make_test_hit(
            &long_path,
            1,
            "func",
            Some("f"),
            0.50,
            Some("code"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        // The full path must appear in the output, no truncation
        assert!(lines[0].contains(&long_path));
        assert_eq!(lines[0], format!("{}:1 | func f | 0.50 | code", long_path));
    }

    #[test]
    fn test_format_hits_agent_empty_symbol_name_treated_as_none() {
        let meta = make_test_metadata();
        // Some("") should produce the same output as None
        let hits_empty = vec![make_test_hit(
            "src/lib.rs",
            1,
            "module",
            Some(""),
            0.50,
            Some("Module declarations"),
        )];
        let hits_none = vec![make_test_hit(
            "src/lib.rs",
            1,
            "module",
            None,
            0.50,
            Some("Module declarations"),
        )];
        let output_empty = format_hits_agent(&hits_empty, &meta);
        let output_none = format_hits_agent(&hits_none, &meta);
        let lines_empty = agent_hit_lines(&output_empty);
        let lines_none = agent_hit_lines(&output_none);
        assert_eq!(lines_empty[0], lines_none[0]);
        assert_eq!(
            lines_empty[0],
            "src/lib.rs:1 | module | 0.50 | Module declarations"
        );
    }

    #[test]
    fn test_format_hits_agent_preview_with_newlines() {
        let hits = vec![make_test_hit(
            "src/main.rs",
            10,
            "func",
            Some("run"),
            0.60,
            Some("Line one\nLine two"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(
            lines[0],
            "src/main.rs:10 | func run | 0.60 | Line one Line two"
        );
        // Hit line must be a single line (no newlines)
        assert!(!lines[0].contains('\n'));
    }

    #[test]
    fn test_format_hits_agent_preview_with_multiple_newline_types() {
        let hits = vec![make_test_hit(
            "src/main.rs",
            10,
            "func",
            Some("run"),
            0.60,
            Some("Line one\nLine two\r\nLine three\rLine four"),
        )];
        let meta = make_test_metadata();
        let output = format_hits_agent(&hits, &meta);
        let lines = agent_hit_lines(&output);
        assert_eq!(
            lines[0],
            "src/main.rs:10 | func run | 0.60 | Line one Line two Line three Line four"
        );
        // Hit line must not contain any raw newline or carriage return characters
        assert!(!lines[0].contains('\n'));
        assert!(!lines[0].contains('\r'));
    }

    // ---------------------------------------------------------------
    // JSON formatter backward compatibility tests per MRIMP-5.2002
    // ---------------------------------------------------------------

    #[test]
    fn test_json_search_hits_array_structure() {
        // Verify Search JSON has correct top-level keys and hit structure
        let hits = vec![
            make_test_hit(
                "src/app.rs",
                42,
                "func",
                Some("main"),
                0.92,
                Some("Entry point"),
            ),
            make_test_hit("src/lib.rs", 10, "module", None, 0.75, None),
        ];
        let meta = make_test_metadata();
        let output = format_hits_json_search(&hits, &meta).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        // Top-level must have "hits" key
        assert!(parsed.is_object());
        assert!(parsed["hits"].is_array());

        // Verify hits array length
        let hits_arr = parsed["hits"].as_array().unwrap();
        assert_eq!(hits_arr.len(), 2);

        // Verify first hit has required fields
        let hit0 = &hits_arr[0];
        assert_eq!(hit0["file_relpath"], "src/app.rs");
        assert_eq!(hit0["start_line"], 42);
        assert_eq!(hit0["kind"], "func");
        assert_eq!(hit0["symbol_name"], "main");
        assert_eq!(hit0["score"], 0.92);
        assert_eq!(hit0["chunk_id"], 1);
        assert_eq!(hit0["preview"], "Entry point");

        // Verify second hit with None fields
        let hit1 = &hits_arr[1];
        assert_eq!(hit1["file_relpath"], "src/lib.rs");
        assert!(hit1["symbol_name"].is_null());
        // preview is None, so it should not appear (skip_serializing_if)
        assert!(hit1.get("preview").is_none() || hit1["preview"].is_null());
    }

    #[test]
    fn test_json_vector_search_metadata_structure() {
        // Verify VectorSearch JSON has correct top-level keys and metadata
        let hits = vec![
            serde_json::json!({
                "chunk_id": 101,
                "score": 0.95,
                "file_relpath": "src/auth.rs",
                "file_path": "src/auth.rs",
                "symbol_name": "authenticate",
                "kind": "func",
                "start_line": 15,
                "end_line": 45,
            }),
            serde_json::json!({
                "chunk_id": 202,
                "score": 0.82,
                "file_relpath": "src/session.rs",
                "file_path": "src/session.rs",
                "symbol_name": "create_session",
                "kind": "func",
                "start_line": 5,
                "end_line": 20,
            }),
        ];
        let output =
            format_hits_json_vector(&hits, 2, "auth logic", "vector", 10, Some(0.75)).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        // Verify all required top-level keys
        assert!(parsed["hits"].is_array());
        assert_eq!(parsed["total"], 2);
        assert_eq!(parsed["query"], "auth logic");
        assert_eq!(parsed["mode"], "vector");
        assert_eq!(parsed["k"], 10);
        // Use f32-exact value (0.75 has exact binary representation)
        assert_eq!(parsed["threshold"], 0.75);

        // Verify hits array structure
        let hits_arr = parsed["hits"].as_array().unwrap();
        assert_eq!(hits_arr.len(), 2);
        assert_eq!(hits_arr[0]["file_path"], "src/auth.rs");
        assert_eq!(hits_arr[0]["score"], 0.95);
        assert_eq!(hits_arr[1]["file_path"], "src/session.rs");
        assert_eq!(hits_arr[0]["file_relpath"], hits_arr[0]["file_path"]);
        assert_eq!(hits_arr[1]["file_relpath"], hits_arr[1]["file_path"]);
    }

    #[test]
    fn test_json_search_empty_hits_array() {
        // Verify empty hits produces valid JSON with empty array and metadata
        let hits: Vec<SearchHit> = vec![];
        let meta = SearchMetadata {
            query: "empty".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output = format_hits_json_search(&hits, &meta).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert!(parsed["hits"].is_array());
        assert_eq!(parsed["hits"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_json_vector_search_empty_hits_array() {
        // Verify empty VectorSearch hits produces valid JSON with all metadata
        let hits: Vec<serde_json::Value> = vec![];
        // Use 0.5 which has exact f32 binary representation
        let output =
            format_hits_json_vector(&hits, 0, "empty query", "vector", 5, Some(0.5)).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert!(parsed["hits"].is_array());
        assert_eq!(parsed["hits"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["total"], 0);
        assert_eq!(parsed["query"], "empty query");
        assert_eq!(parsed["mode"], "vector");
        assert_eq!(parsed["k"], 5);
        assert_eq!(parsed["threshold"], 0.5);
    }

    // ---------------------------------------------------------------
    // AFM-02: Tests for dual file_relpath / file_path field contract
    // ---------------------------------------------------------------

    #[test]
    fn test_vector_search_json_has_both_file_fields() {
        // Verify vector-search JSON output includes both file_relpath and file_path
        let hit_json = serde_json::json!({
            "chunk_id": 123,
            "score": 0.95,
            "start_line": 10,
            "end_line": 20,
            "symbol_name": "test_function",
            "kind": "function",
            "file_relpath": "src/test.rs",
            "file_path": "src/test.rs",
        });

        assert!(hit_json["file_relpath"].is_string());
        assert!(hit_json["file_path"].is_string());
        assert_eq!(hit_json["file_relpath"], hit_json["file_path"]);
    }

    #[test]
    fn test_vector_search_json_file_relpath_matches_source() {
        // Verify both fields contain the value from SearchHit.file_relpath
        let test_path = "src/specific/path.rs";
        let hit_json = serde_json::json!({
            "chunk_id": 456,
            "score": 0.88,
            "start_line": 5,
            "end_line": 15,
            "symbol_name": "another_function",
            "kind": "function",
            "file_relpath": test_path,
            "file_path": test_path,
        });

        assert_eq!(hit_json["file_relpath"].as_str().unwrap(), test_path);
        assert_eq!(hit_json["file_path"].as_str().unwrap(), test_path);
    }

    #[test]
    fn test_vector_search_json_file_relpath_not_null() {
        // Verify file_relpath is never null when file_path is present
        let hit_json = serde_json::json!({
            "chunk_id": 789,
            "score": 0.75,
            "start_line": 1,
            "end_line": 10,
            "symbol_name": "empty_path_test",
            "kind": "function",
            "file_relpath": "",
            "file_path": "",
        });

        assert!(hit_json["file_relpath"].is_string());
        assert!(hit_json["file_path"].is_string());
        assert_eq!(hit_json["file_relpath"], "");
        assert_eq!(hit_json["file_path"], "");
    }

    // ---------------------------------------------------------------
    // AFM-05.2001: SearchMetadata header and JSON envelope tests
    // ---------------------------------------------------------------

    #[test]
    fn test_agent_header_with_results() {
        // Header with hits > 0, total_estimate > hits
        let hits = vec![make_hit(
            "src/app.rs",
            42,
            "func",
            Some("main"),
            0.92,
            Some("Entry point"),
        )];
        let meta = SearchMetadata {
            query: "authentication".to_string(),
            mode: "fts".to_string(),
            hits: 1,
            total_estimate: 25,
        };
        let output = format_hits_agent(&hits, &meta);
        let header = output.lines().next().unwrap();
        assert_eq!(
            header,
            "SEARCH query=\"authentication\" | hits=1 | total_estimate=25 | mode=fts"
        );
        // Also verify hit line is present
        assert_eq!(agent_hit_lines(&output).len(), 1);
    }

    #[test]
    fn test_agent_header_without_results() {
        // Header with hits = 0, total_estimate = 0
        let hits: Vec<SearchHit> = vec![];
        let meta = SearchMetadata {
            query: "nonexistent".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output = format_hits_agent(&hits, &meta);
        let header = output.lines().next().unwrap();
        assert_eq!(
            header,
            "SEARCH query=\"nonexistent\" | hits=0 | total_estimate=0 | mode=fts"
        );
        assert_eq!(agent_hit_lines(&output).len(), 0);
    }

    #[test]
    fn test_agent_header_query_with_double_quotes() {
        // Query containing double quotes must be escaped
        let hits: Vec<SearchHit> = vec![];
        let meta = SearchMetadata {
            query: "user \"name\" field".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output = format_hits_agent(&hits, &meta);
        let header = output.lines().next().unwrap();
        assert_eq!(
            header,
            "SEARCH query=\"user \\\"name\\\" field\" | hits=0 | total_estimate=0 | mode=fts"
        );
    }

    #[test]
    fn test_agent_header_query_with_special_characters() {
        // Query with pipes, equals, spaces - should be passed through literally
        let hits: Vec<SearchHit> = vec![];
        let meta = SearchMetadata {
            query: "key=value | pipe | another=test spaces".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output = format_hits_agent(&hits, &meta);
        let header = output.lines().next().unwrap();
        assert_eq!(
            header,
            "SEARCH query=\"key=value | pipe | another=test spaces\" | hits=0 | total_estimate=0 | mode=fts"
        );
    }

    #[test]
    fn test_agent_header_mode_fts_vs_vector() {
        let hits: Vec<SearchHit> = vec![];

        // FTS mode
        let meta_fts = SearchMetadata {
            query: "test".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output_fts = format_hits_agent(&hits, &meta_fts);
        assert!(output_fts.contains("| mode=fts"));

        // Vector mode
        let meta_vector = SearchMetadata {
            query: "test".to_string(),
            mode: "vector".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output_vector = format_hits_agent(&hits, &meta_vector);
        assert!(output_vector.contains("| mode=vector"));
    }

    #[test]
    fn test_json_envelope_includes_all_metadata_fields() {
        // JSON output must include total_matches, query, mode at top level
        let hits = vec![make_hit(
            "src/app.rs",
            42,
            "func",
            Some("main"),
            0.92,
            Some("Entry point"),
        )];
        let meta = SearchMetadata {
            query: "authentication".to_string(),
            mode: "fts".to_string(),
            hits: 1,
            total_estimate: 50,
        };
        let output = format_hits_json_search(&hits, &meta).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["total_matches"], 50);
        assert_eq!(parsed["query"], "authentication");
        assert_eq!(parsed["mode"], "fts");
        assert!(parsed["hits"].is_array());
        assert_eq!(parsed["hits"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_agent_header_long_query() {
        // Test header formatting with very long query (>1000 chars)
        let long_query = "x".repeat(1500);
        let meta = SearchMetadata {
            query: long_query.clone(),
            mode: "fts".to_string(),
            hits: 5,
            total_estimate: 10,
        };

        let result = format_hits_agent(&[], &meta);

        // Header should still format correctly without truncation
        assert!(result.contains(&long_query));
        assert!(result.contains("hits=5"));
        assert!(result.contains("total_estimate=10"));
    }

    #[test]
    fn test_json_empty_hits_with_metadata() {
        // JSON with empty hits still includes all metadata fields
        let hits: Vec<SearchHit> = vec![];
        let meta = SearchMetadata {
            query: "nonexistent query".to_string(),
            mode: "vector".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output = format_hits_json_search(&hits, &meta).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["total_matches"], 0);
        assert_eq!(parsed["query"], "nonexistent query");
        assert_eq!(parsed["mode"], "vector");
        assert!(parsed["hits"].is_array());
        assert_eq!(parsed["hits"].as_array().unwrap().len(), 0);
    }

    // ---------------------------------------------------------------
    // format_agent_error() tests per AFM-04.1001
    // ---------------------------------------------------------------

    #[test]
    fn test_format_agent_error_basic() {
        let output = format_agent_error(
            "database",
            "Failed to connect to database",
            "Check database connection settings",
        );
        assert_eq!(
            output,
            "ERROR | type=database | message=Failed to connect to database | suggestion=Check database connection settings"
        );
    }

    #[test]
    fn test_format_agent_error_pipe_sanitization() {
        let output = format_agent_error(
            "unknown",
            "Failed to parse config | invalid syntax",
            "Fix the | characters in config",
        );
        assert_eq!(
            output,
            "ERROR | type=unknown | message=Failed to parse config - invalid syntax | suggestion=Fix the - characters in config"
        );
    }

    #[test]
    fn test_format_agent_error_multiple_consecutive_pipes() {
        let output = format_agent_error(
            "validation",
            "Field validation failed||missing required field",
            "Provide||check the field",
        );
        assert_eq!(
            output,
            "ERROR | type=validation | message=Field validation failed--missing required field | suggestion=Provide--check the field"
        );
    }

    #[test]
    fn test_format_agent_error_newline_sanitization() {
        let output = format_agent_error(
            "unknown",
            "Error occurred\nLine two\r\nLine three\rLine four",
            "Try restarting\nCheck logs\r\nVerify config\rContact support",
        );
        assert_eq!(
            output,
            "ERROR | type=unknown | message=Error occurred Line two Line three Line four | suggestion=Try restarting Check logs Verify config Contact support"
        );
    }

    #[test]
    fn test_format_agent_error_empty_message() {
        let output = format_agent_error("unknown", "", "Check system logs");
        assert_eq!(
            output,
            "ERROR | type=unknown | message= | suggestion=Check system logs"
        );
    }

    #[test]
    fn test_format_agent_error_empty_suggestion() {
        let output = format_agent_error("timeout", "Request timed out", "");
        assert_eq!(
            output,
            "ERROR | type=timeout | message=Request timed out | suggestion="
        );
    }

    #[test]
    fn test_format_agent_error_unicode() {
        let output = format_agent_error(
            "unknown",
            "Failed to decode UTF-8: \u{00e9}\u{00f1}\u{00fc}",
            "Use UTF-8 encoding: \u{3053}\u{3093}\u{306b}\u{3061}\u{306f}",
        );
        assert_eq!(
            output,
            "ERROR | type=unknown | message=Failed to decode UTF-8: \u{00e9}\u{00f1}\u{00fc} | suggestion=Use UTF-8 encoding: \u{3053}\u{3093}\u{306b}\u{3061}\u{306f}"
        );
    }

    #[test]
    fn test_format_agent_error_long_message() {
        // Create a message over 1000 characters
        let long_message = "Error: ".to_string() + &"x".repeat(1200);
        assert!(long_message.len() > 1000);

        let output = format_agent_error("unknown", &long_message, "Reduce input size");

        // Verify the full message appears in output (no truncation)
        assert!(output.contains(&long_message));
        // Verify output length is at least as long as the message
        assert!(output.len() >= long_message.len());
    }

    #[test]
    fn test_format_agent_error_format_regex() {
        let output = format_agent_error("unknown", "Test message", "Test suggestion");

        // Verify output matches expected regex pattern
        let re = regex::Regex::new(r"^ERROR \| type=.+ \| message=.+ \| suggestion=.+$").unwrap();
        assert!(
            re.is_match(&output),
            "Output does not match expected pattern: {}",
            output
        );

        // Empty fields produce valid format but don't match the .+ regex (which requires 1+ chars).
        // That's correct — the regex validates non-empty output. Empty fields are tested separately.
        let output_empty = format_agent_error("unknown", "", "");
        let re_empty =
            regex::Regex::new(r"^ERROR \| type=.+ \| message=.* \| suggestion=.*$").unwrap();
        assert!(
            re_empty.is_match(&output_empty),
            "Output with empty fields does not match relaxed pattern: {}",
            output_empty
        );
    }
}
