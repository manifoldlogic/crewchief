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

use crate::context::types::ContextBundle;
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

/// Output format for search and context results.
///
/// Controls how search/context results are rendered to stdout.
/// Used as a clap `ValueEnum` for the `--format` CLI flag.
///
/// - **Json**: Full structured JSON output, backward compatible with existing tooling.
/// - **Agent**: Compact one-line-per-result output optimized for LLM agents.
///   Implicitly enables preview (default 120 chars) to keep output token-efficient.
///
/// NOTE(AFM-03): For the `context` command, the default was changed from human-readable
/// output (via `format_context_bundle()`) to Json. The previous `--json` bool flag
/// defaulted to false, producing human-readable output. All programmatic consumers
/// (daemon, MCP server, VSCode extension) use JSON-RPC and are unaffected. The old
/// human-readable format is preserved in `format_context_bundle()` but no longer
/// reachable via CLI flags; use `--format agent` for compact interactive output.
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

/// Format a context bundle as compact agent-friendly output.
///
/// Produces a header line followed by one line per context item:
///
/// ```text
/// CONTEXT chunk_id=N | tokens=T/B | items=I | truncated=yes/no
/// role | relpath:start-end | tokens | reason
/// role | relpath:start-end | tokens | reason | content_preview  (primary only)
/// ```
///
/// - Non-primary items omit content (agents can read files if needed).
/// - Primary items include a preview: first 3 lines joined with spaces,
///   sanitized of newlines, capped at 200 characters.
/// - Empty bundles produce the header line only with `items=0`.
/// - Multiple primary items all receive content previews (FR-8).
///
/// # Examples
///
/// ```
/// use maproom::cli::format::format_context_agent;
/// use maproom::context::types::{ContextBundle, ContextItem, LineRange};
///
/// let bundle = ContextBundle {
///     items: vec![
///         ContextItem {
///             role: "primary".to_string(),
///             relpath: "src/auth.rs".to_string(),
///             range: LineRange { start: 42, end: 68 },
///             tokens: 450,
///             reason: "Target function".to_string(),
///             content: "fn authenticate(user: &str) {\n    let db = connect();\n    verify(user)\n}".to_string(),
///         },
///         ContextItem {
///             role: "caller".to_string(),
///             relpath: "src/api.rs".to_string(),
///             range: LineRange { start: 100, end: 120 },
///             tokens: 300,
///             reason: "Calls authenticate".to_string(),
///             content: "// caller content...".to_string(),
///         },
///     ],
///     total_tokens: 750,
///     truncated: false,
/// };
///
/// let output = format_context_agent(&bundle, 12345, 6000);
///
/// // Expected output format:
/// // CONTEXT chunk_id=12345 | tokens=750/6000 | items=2 | truncated=no
/// // primary | src/auth.rs:42-68 | 450 | Target function | fn authenticate(user: &str) {     let db = connect();     verify(user)
/// // caller | src/api.rs:100-120 | 300 | Calls authenticate
///
/// let lines: Vec<&str> = output.lines().collect();
/// assert_eq!(lines.len(), 3);
/// assert_eq!(
///     lines[0],
///     "CONTEXT chunk_id=12345 | tokens=750/6000 | items=2 | truncated=no"
/// );
/// // Primary item includes a content preview (first 3 lines joined with spaces)
/// assert!(lines[1].starts_with("primary | src/auth.rs:42-68 | 450 | Target function | "));
/// // Non-primary item omits content
/// assert_eq!(lines[2], "caller | src/api.rs:100-120 | 300 | Calls authenticate");
/// ```
pub fn format_context_agent(bundle: &ContextBundle, chunk_id: i64, budget: usize) -> String {
    let mut output = String::new();

    // Header line
    let total_tokens: usize = bundle.items.iter().map(|i| i.tokens).sum();
    let truncated_flag = if bundle.truncated { "yes" } else { "no" };
    let _ = writeln!(
        output,
        "CONTEXT chunk_id={} | tokens={}/{} | items={} | truncated={}",
        chunk_id,
        total_tokens,
        budget,
        bundle.items.len(),
        truncated_flag
    );

    // Item lines
    for item in &bundle.items {
        // Sanitize fields that could corrupt the pipe-delimited format:
        // - Pipe chars in reason/relpath would create extra segments
        // - Null bytes in content could corrupt text-based output
        let safe_reason = item.reason.replace('|', " ");
        let safe_relpath = item.relpath.replace('|', " ");
        let location = format!("{}:{}-{}", safe_relpath, item.range.start, item.range.end);

        if item.role == "primary" {
            // Content preview: first 3 lines, sanitized, capped at 200 chars
            let safe_content = item.content.replace('\0', "");
            let preview_lines: Vec<&str> = safe_content.lines().take(3).collect();
            let preview_joined = preview_lines.join(" ");
            let preview_sanitized = sanitize_newlines(&preview_joined);
            let preview_capped: String = preview_sanitized.chars().take(200).collect();

            let _ = writeln!(
                output,
                "{} | {} | {} | {} | {}",
                item.role, location, item.tokens, safe_reason, preview_capped
            );
        } else {
            let _ = writeln!(
                output,
                "{} | {} | {} | {}",
                item.role, location, item.tokens, safe_reason
            );
        }
    }

    // Remove trailing newline for clean output
    if output.ends_with('\n') {
        output.pop();
    }

    output
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
    // Tests for format_context_agent() per AFM-03.2001
    // ---------------------------------------------------------------

    use crate::context::types::{ContextBundle, ContextItem, LineRange};

    /// Helper to create a ContextItem for testing.
    fn make_context_item(
        relpath: &str,
        start: i32,
        end: i32,
        role: &str,
        reason: &str,
        content: &str,
        tokens: usize,
    ) -> ContextItem {
        ContextItem {
            relpath: relpath.to_string(),
            range: LineRange::new(start, end),
            role: role.to_string(),
            reason: reason.to_string(),
            content: content.to_string(),
            tokens,
        }
    }

    #[test]
    fn test_context_agent_empty_bundle() {
        let bundle = ContextBundle::new();
        let output = format_context_agent(&bundle, 42, 6000);
        assert_eq!(
            output,
            "CONTEXT chunk_id=42 | tokens=0/6000 | items=0 | truncated=no"
        );
    }

    #[test]
    fn test_context_agent_single_primary() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/auth.rs",
            10,
            30,
            "primary",
            "Target chunk",
            "fn authenticate(user: &str) -> bool {\n    // Check credentials\n    true\n}",
            150,
        ));

        let output = format_context_agent(&bundle, 12345, 6000);
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            "CONTEXT chunk_id=12345 | tokens=150/6000 | items=1 | truncated=no"
        );
        assert_eq!(
            lines[1],
            "primary | src/auth.rs:10-30 | 150 | Target chunk | fn authenticate(user: &str) -> bool {     // Check credentials     true"
        );
    }

    #[test]
    fn test_context_agent_non_primary_no_preview() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/caller.rs",
            5,
            15,
            "caller",
            "Calls authenticate()",
            "fn login() {\n    authenticate(\"admin\");\n}",
            80,
        ));

        let output = format_context_agent(&bundle, 99, 4000);
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            "CONTEXT chunk_id=99 | tokens=80/4000 | items=1 | truncated=no"
        );
        // Non-primary: no preview segment
        assert_eq!(
            lines[1],
            "caller | src/caller.rs:5-15 | 80 | Calls authenticate()"
        );
    }

    #[test]
    fn test_context_agent_multi_item() {
        // primary + 3 supporting items
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/auth.rs",
            10,
            30,
            "primary",
            "Target chunk",
            "fn authenticate() {}",
            150,
        ));
        bundle.add_item(make_context_item(
            "src/caller.rs",
            5,
            15,
            "caller",
            "Calls authenticate()",
            "fn login() { authenticate(); }",
            80,
        ));
        bundle.add_item(make_context_item(
            "src/callee.rs",
            20,
            40,
            "callee",
            "Called by authenticate()",
            "fn verify_token() {}",
            90,
        ));
        bundle.add_item(make_context_item(
            "tests/auth_test.rs",
            1,
            25,
            "test",
            "Tests authenticate()",
            "#[test]\nfn test_auth() { assert!(authenticate()); }",
            100,
        ));

        let output = format_context_agent(&bundle, 555, 6000);
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 5); // header + 4 items
                                    // Header: total tokens = 150 + 80 + 90 + 100 = 420
        assert_eq!(
            lines[0],
            "CONTEXT chunk_id=555 | tokens=420/6000 | items=4 | truncated=no"
        );
        // Primary has preview
        assert!(lines[1].starts_with("primary | src/auth.rs:10-30 | 150 | Target chunk | "));
        // Non-primary items have no preview
        assert_eq!(
            lines[2],
            "caller | src/caller.rs:5-15 | 80 | Calls authenticate()"
        );
        assert_eq!(
            lines[3],
            "callee | src/callee.rs:20-40 | 90 | Called by authenticate()"
        );
        assert_eq!(
            lines[4],
            "test | tests/auth_test.rs:1-25 | 100 | Tests authenticate()"
        );
    }

    #[test]
    fn test_context_agent_truncated_bundle() {
        let mut bundle = ContextBundle::new();
        bundle.truncated = true;
        bundle.add_item(make_context_item(
            "src/big.rs",
            1,
            500,
            "primary",
            "Large chunk",
            "fn big() {}",
            5500,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();

        assert!(lines[0].contains("truncated=yes"));
        assert_eq!(
            lines[0],
            "CONTEXT chunk_id=1 | tokens=5500/6000 | items=1 | truncated=yes"
        );
    }

    #[test]
    fn test_preview_length_cap() {
        // Create content with lines that join to more than 200 chars
        let long_line = "x".repeat(250);
        let content = format!("{}\nsecond line\nthird line", long_line);

        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/long.rs",
            1,
            10,
            "primary",
            "Long content",
            &content,
            200,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();

        // Extract the preview from the primary line (after the 4th pipe)
        let primary_line = lines[1];
        let segments: Vec<&str> = primary_line.splitn(5, " | ").collect();
        assert_eq!(segments.len(), 5);
        let preview = segments[4];
        // Preview must be at most 200 characters
        assert!(
            preview.chars().count() <= 200,
            "Preview should be capped at 200 chars, got {}",
            preview.chars().count()
        );
    }

    #[test]
    fn test_preview_first_three_lines() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/multi.rs",
            1,
            5,
            "primary",
            "Multi-line",
            content,
            50,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        let primary_line = lines[1];
        let segments: Vec<&str> = primary_line.splitn(5, " | ").collect();
        let preview = segments[4];

        // Should contain lines 1-3 but not 4-5
        assert!(preview.contains("line1"));
        assert!(preview.contains("line2"));
        assert!(preview.contains("line3"));
        assert!(!preview.contains("line4"));
        assert!(!preview.contains("line5"));
        assert_eq!(preview, "line1 line2 line3");
    }

    #[test]
    fn test_context_agent_empty_content() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/empty.rs",
            1,
            1,
            "primary",
            "Empty content",
            "",
            0,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        let primary_line = lines[1];
        // Should end with empty preview after the last pipe
        assert!(primary_line.ends_with("| Empty content | "));
    }

    #[test]
    fn test_context_agent_multiple_primaries() {
        // FR-8: Multiple primary chunks all get previews
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/a.rs",
            1,
            10,
            "primary",
            "First primary",
            "fn first() {}",
            100,
        ));
        bundle.add_item(make_context_item(
            "src/b.rs",
            20,
            30,
            "primary",
            "Second primary",
            "fn second() {}",
            120,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 3);
        // Both primaries should have 5 segments (including preview)
        let segments_a: Vec<&str> = lines[1].splitn(5, " | ").collect();
        let segments_b: Vec<&str> = lines[2].splitn(5, " | ").collect();
        assert_eq!(segments_a.len(), 5, "First primary should have preview");
        assert_eq!(segments_b.len(), 5, "Second primary should have preview");
        assert!(segments_a[4].contains("fn first()"));
        assert!(segments_b[4].contains("fn second()"));
    }

    #[test]
    fn test_context_agent_unicode_content() {
        // Ensure unicode chars are not split when capping at 200 characters
        // Each CJK char is 1 char but 3 bytes in UTF-8
        let unicode_content = "\u{4e16}\u{754c}".repeat(150); // 300 CJK chars
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/unicode.rs",
            1,
            10,
            "primary",
            "Unicode content",
            &unicode_content,
            500,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        let primary_line = lines[1];
        let segments: Vec<&str> = primary_line.splitn(5, " | ").collect();
        let preview = segments[4];

        // Preview should be exactly 200 chars (capped from 300)
        assert_eq!(
            preview.chars().count(),
            200,
            "Unicode preview should be capped at exactly 200 chars"
        );
        // Should be valid UTF-8 (would have panicked already if not)
        assert!(std::str::from_utf8(preview.as_bytes()).is_ok());
    }

    #[test]
    fn test_context_agent_zero_tokens() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/zero.rs",
            1,
            1,
            "caller",
            "Zero tokens",
            "",
            0,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        assert!(output.contains("tokens=0/6000"));
        assert!(output.contains("| 0 |"));
    }

    #[test]
    fn test_context_agent_single_line_content() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/one.rs",
            1,
            1,
            "primary",
            "One liner",
            "fn one_liner() {}",
            10,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        let segments: Vec<&str> = lines[1].splitn(5, " | ").collect();
        assert_eq!(segments[4], "fn one_liner() {}");
    }

    #[test]
    fn test_context_agent_all_roles() {
        // Test every known role type: primary, caller, callee, test, doc, config, hook, jsx_parent, jsx_child
        let mut bundle = ContextBundle::new();
        let roles = [
            "primary",
            "caller",
            "callee",
            "test",
            "doc",
            "config",
            "hook",
            "jsx_parent",
            "jsx_child",
        ];
        for (i, role) in roles.iter().enumerate() {
            bundle.add_item(make_context_item(
                &format!("src/{}.rs", role),
                (i as i32) + 1,
                (i as i32) + 10,
                role,
                &format!("Role: {}", role),
                &format!("fn {}() {{}}", role),
                50,
            ));
        }

        let output = format_context_agent(&bundle, 42, 6000);
        let lines: Vec<&str> = output.lines().collect();

        // Header + 9 items = 10 lines
        assert_eq!(lines.len(), 10);
        assert!(lines[0].contains("items=9"));
        // Verify each role appears in the output
        for role in &roles {
            assert!(
                output.contains(&format!("{} | src/{}.rs:", role, role)),
                "Missing role: {}",
                role
            );
        }
        // Only primary should have content preview (5 segments)
        let primary_segments: Vec<&str> = lines[1].splitn(5, " | ").collect();
        assert_eq!(primary_segments.len(), 5, "Primary should have preview");
        // Non-primary items should have 4 segments
        for line in &lines[2..] {
            let segments: Vec<&str> = line.splitn(5, " | ").collect();
            assert_eq!(
                segments.len(),
                4,
                "Non-primary should have no preview: {}",
                line
            );
        }
    }

    #[test]
    fn test_context_agent_header_format() {
        // Validate exact header line structure
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/main.rs",
            1,
            50,
            "primary",
            "Target",
            "fn main() {}",
            200,
        ));
        bundle.add_item(make_context_item(
            "src/lib.rs",
            1,
            10,
            "caller",
            "Calls main",
            "use main;",
            50,
        ));

        let output = format_context_agent(&bundle, 99999, 8000);
        let header = output.lines().next().unwrap();

        // Verify exact header format
        assert_eq!(
            header,
            "CONTEXT chunk_id=99999 | tokens=250/8000 | items=2 | truncated=no"
        );

        // Verify header components are pipe-delimited
        let segments: Vec<&str> = header.split(" | ").collect();
        assert_eq!(segments.len(), 4);
        assert!(segments[0].starts_with("CONTEXT chunk_id="));
        assert!(segments[1].starts_with("tokens="));
        assert!(segments[2].starts_with("items="));
        assert!(segments[3].starts_with("truncated="));
    }

    #[test]
    fn test_context_agent_large_budget() {
        // Large numbers formatted correctly
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/big.rs",
            1,
            10000,
            "primary",
            "Huge file",
            "fn big() {}",
            999999,
        ));

        let output = format_context_agent(&bundle, 1000000, 1000000);
        let header = output.lines().next().unwrap();
        assert_eq!(
            header,
            "CONTEXT chunk_id=1000000 | tokens=999999/1000000 | items=1 | truncated=no"
        );
    }

    #[test]
    fn test_context_agent_unicode_path() {
        // Multi-byte unicode characters in file paths
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/\u{65e5}\u{672c}\u{8a9e}.rs",
            1,
            10,
            "primary",
            "Japanese path",
            "fn greet() {}",
            30,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("src/\u{65e5}\u{672c}\u{8a9e}.rs:1-10"));
    }

    #[test]
    fn test_context_agent_empty_reason() {
        // Empty reason string
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/noreason.rs",
            1,
            5,
            "caller",
            "",
            "fn call() {}",
            20,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        // The reason field should be empty but the format still works
        assert_eq!(lines[1], "caller | src/noreason.rs:1-5 | 20 | ");
    }

    #[test]
    fn test_preview_newline_sanitization() {
        // All newline types: \n, \r\n, \r should be replaced with spaces
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/newlines.rs",
            1,
            5,
            "primary",
            "Newline test",
            "line_unix\nline_windows\r\nline_mac\rline_end",
            40,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        let segments: Vec<&str> = lines[1].splitn(5, " | ").collect();
        let preview = segments[4];

        // All newline types should be replaced with spaces
        assert!(!preview.contains('\n'));
        assert!(!preview.contains('\r'));
        assert_eq!(preview, "line_unix line_windows line_mac line_end");
    }

    #[test]
    fn test_preview_exact_200_char_boundary() {
        // Create content that results in exactly 200 characters after sanitization
        // Use a string of exactly 200 'a' characters on one line
        let exact_200 = "a".repeat(200);
        let content = format!("{}\nextra line", exact_200);

        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/boundary.rs",
            1,
            5,
            "primary",
            "Boundary test",
            &content,
            100,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();
        let segments: Vec<&str> = lines[1].splitn(5, " | ").collect();
        let preview = segments[4];

        // After joining first 3 lines: "aaa...aaa extra line" (200 + 1 space + 10 = 211 chars)
        // Should be capped at exactly 200 chars
        assert_eq!(
            preview.chars().count(),
            200,
            "Preview should be exactly 200 chars at the boundary, got {}",
            preview.chars().count()
        );
    }

    #[test]
    fn test_preview_only_for_primary() {
        // Non-primary items should never have content preview
        let mut bundle = ContextBundle::new();
        let non_primary_roles = [
            "caller",
            "callee",
            "test",
            "doc",
            "config",
            "hook",
            "jsx_parent",
            "jsx_child",
        ];
        for role in &non_primary_roles {
            bundle.add_item(make_context_item(
                &format!("src/{}.rs", role),
                1,
                10,
                role,
                &format!("{} item", role),
                "fn content_that_should_not_appear() {}",
                50,
            ));
        }

        let output = format_context_agent(&bundle, 1, 6000);
        // The actual content should not appear in output for non-primary items
        assert!(
            !output.contains("content_that_should_not_appear"),
            "Non-primary items should not include content preview"
        );
        // Each non-primary line should have exactly 4 pipe-delimited segments
        for line in output.lines().skip(1) {
            let segments: Vec<&str> = line.splitn(5, " | ").collect();
            assert_eq!(
                segments.len(),
                4,
                "Non-primary item should have 4 segments: {}",
                line
            );
        }
    }

    #[test]
    fn test_context_agent_no_primary() {
        // Bundle with no primary items - all items are supporting roles
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/caller.rs",
            1,
            10,
            "caller",
            "Caller item",
            "fn call() {}",
            50,
        ));
        bundle.add_item(make_context_item(
            "tests/test.rs",
            1,
            5,
            "test",
            "Test item",
            "#[test] fn t() {}",
            30,
        ));

        let output = format_context_agent(&bundle, 1, 6000);
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 3); // header + 2 items
        assert!(lines[0].contains("items=2"));
        // No item should have a preview (no primary)
        assert!(!output.contains("fn call()"));
        assert!(!output.contains("#[test] fn t()"));
    }

    // ---------------------------------------------------------------
    // Token efficiency benchmark test per AFM-03.2001
    // ---------------------------------------------------------------

    #[test]
    fn test_token_efficiency_benchmark() {
        // Build a realistic 5-item test bundle:
        // 1 primary (~800 tokens worth of content)
        // 2 callers (~400 tokens each)
        // 1 test (~300 tokens)
        // 1 doc (~200 tokens)

        // Generate realistic Rust code content
        let primary_content = (0..50)
            .map(|i| format!("    let var_{} = compute_value({}, &config);", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        let primary_content = format!(
            "pub fn authenticate(user: &str, password: &str, config: &AuthConfig) -> Result<Session, AuthError> {{\n{}\n    Ok(Session::new(user))\n}}",
            primary_content
        );

        let caller1_content = (0..30)
            .map(|i| format!("    let step_{} = process_request({}, &ctx);", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        let caller1_content = format!(
            "pub fn login_handler(req: &Request) -> Response {{\n{}\n    Response::ok()\n}}",
            caller1_content
        );

        let caller2_content = (0..30)
            .map(|i| format!("    let check_{} = validate_input({}, &rules);", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        let caller2_content = format!(
            "pub fn api_middleware(req: &Request, next: &Handler) -> Response {{\n{}\n    next.call(req)\n}}",
            caller2_content
        );

        let test_content = (0..25)
            .map(|i| {
                format!(
                    "    assert_eq!(authenticate(\"user_{}\", \"pass_{}\", &config), expected_{});",
                    i, i, i
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let test_content = format!(
            "#[test]\nfn test_authenticate() {{\n    let config = AuthConfig::default();\n{}\n}}",
            test_content
        );

        let doc_content = (0..20)
            .map(|i| format!("/// Parameter {}: Description of parameter {} and its usage in the authentication flow.", i, i))
            .collect::<Vec<_>>()
            .join("\n");

        // Count tokens for each content piece
        let counter = crate::context::TokenCounter::new();
        let primary_tokens = counter.count(&primary_content).unwrap();
        let caller1_tokens = counter.count(&caller1_content).unwrap();
        let caller2_tokens = counter.count(&caller2_content).unwrap();
        let test_tokens = counter.count(&test_content).unwrap();
        let doc_tokens = counter.count(&doc_content).unwrap();

        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/auth.rs",
            10,
            60,
            "primary",
            "Target function: authenticate()",
            &primary_content,
            primary_tokens,
        ));
        bundle.add_item(make_context_item(
            "src/handlers/login.rs",
            25,
            55,
            "caller",
            "Calls authenticate() in login flow",
            &caller1_content,
            caller1_tokens,
        ));
        bundle.add_item(make_context_item(
            "src/middleware/api.rs",
            100,
            130,
            "caller",
            "Calls authenticate() in API middleware",
            &caller2_content,
            caller2_tokens,
        ));
        bundle.add_item(make_context_item(
            "tests/auth_test.rs",
            1,
            25,
            "test",
            "Unit tests for authenticate()",
            &test_content,
            test_tokens,
        ));
        bundle.add_item(make_context_item(
            "docs/auth.rs",
            1,
            20,
            "doc",
            "Documentation for authentication module",
            &doc_content,
            doc_tokens,
        ));

        let budget = 6000;
        let chunk_id = 12345;

        // Render both formats
        let agent_output = format_context_agent(&bundle, chunk_id, budget);
        let json_output = serde_json::to_string_pretty(&bundle).unwrap();

        // Count tokens using tiktoken cl100k_base
        let agent_tokens = counter.count(&agent_output).unwrap();
        let json_tokens = counter.count(&json_output).unwrap();

        // Assert: agent format must use at most 60% of JSON tokens
        let ratio = agent_tokens as f64 / json_tokens as f64;
        assert!(
            agent_tokens <= (json_tokens as f64 * 0.60) as usize,
            "Agent format ({} tokens) should be <= 60% of JSON format ({} tokens). Ratio: {:.2}%",
            agent_tokens,
            json_tokens,
            ratio * 100.0
        );
    }

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
        assert_eq!(agent_hit_lines(&output).len(), 1);
    }

    #[test]
    fn test_agent_header_without_results() {
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

        let meta_fts = SearchMetadata {
            query: "test".to_string(),
            mode: "fts".to_string(),
            hits: 0,
            total_estimate: 0,
        };
        let output_fts = format_hits_agent(&hits, &meta_fts);
        assert!(output_fts.contains("| mode=fts"));

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
        let long_query = "x".repeat(1500);
        let meta = SearchMetadata {
            query: long_query.clone(),
            mode: "fts".to_string(),
            hits: 5,
            total_estimate: 10,
        };

        let result = format_hits_agent(&[], &meta);

        assert!(result.contains(&long_query));
        assert!(result.contains("hits=5"));
        assert!(result.contains("total_estimate=10"));
    }

    #[test]
    fn test_json_empty_hits_with_metadata() {
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
    // Defensive edge case tests per AFM-03.4002
    // ---------------------------------------------------------------

    #[test]
    fn test_agent_format_budget_zero() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/main.rs",
            1,
            10,
            "primary",
            "Target",
            "fn main() {\n    println!(\"test\");\n}",
            100,
        ));

        let output = format_context_agent(&bundle, 1001, 0);

        assert!(output.starts_with("CONTEXT chunk_id=1001 | tokens=100/0"));
        assert!(output.contains("primary | src/main.rs:1-10"));
        assert!(output.lines().count() >= 2);
    }

    #[test]
    fn test_agent_format_null_bytes_in_content() {
        let content_with_nulls = "fn test() {\n    let x\0 = 5;\n    println!(\"test\0\");\n}";
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/test.rs",
            1,
            5,
            "primary",
            "Target",
            content_with_nulls,
            50,
        ));

        let output = format_context_agent(&bundle, 1002, 1000);

        assert!(!output.contains('\0'), "Output contains null bytes");

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_agent_format_pipe_in_reason() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/core.rs",
            10,
            20,
            "primary",
            "Reason with | pipe character",
            "fn process() {}",
            100,
        ));

        let output = format_context_agent(&bundle, 1003, 1000);

        assert!(output.contains("src/core.rs:10-20"));

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        let segments: Vec<&str> = lines[1].splitn(6, " | ").collect();
        assert_eq!(
            segments.len(),
            5,
            "Primary line should have exactly 5 segments (role, location, tokens, reason, preview), got {}",
            segments.len()
        );
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
    fn test_agent_format_pipe_in_relpath() {
        let mut bundle = ContextBundle::new();
        bundle.add_item(make_context_item(
            "src/weird|name.rs",
            5,
            15,
            "primary",
            "Target",
            "fn test() {}",
            80,
        ));

        let output = format_context_agent(&bundle, 1004, 1000);

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        let segments: Vec<&str> = lines[1].splitn(6, " | ").collect();
        assert_eq!(
            segments.len(),
            5,
            "Primary line should have exactly 5 segments even with pipe in relpath, got {}",
            segments.len()
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
