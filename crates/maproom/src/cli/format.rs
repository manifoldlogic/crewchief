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
/// Each result is a single line with pipe-delimited segments:
///   `<file>:<start_line> | <kind> [<symbol>] | <score> | <preview>`
///
/// - Empty results produce an empty string.
/// - Missing `symbol_name` is omitted (shows just the kind, not "null").
/// - Missing `preview` shows a `-` placeholder.
/// - Newlines in preview text are replaced with spaces.
/// - Score is formatted to exactly 2 decimal places.
pub fn format_hits_agent(hits: &[db::SearchHit]) -> String {
    if hits.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    for (i, hit) in hits.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }

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
/// Produces: `{"hits": [...]}`
///
/// This is a direct extraction of the existing Search command JSON output
/// logic to preserve exact backward compatibility.
pub fn format_hits_json_search(hits: &[db::SearchHit]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({"hits": hits}))
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
/// TODO: Future refactor could normalize JSON schemas between Search and
/// VectorSearch commands. This asymmetry is accepted technical debt for
/// backward compatibility (see architecture.md Decision 5).
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
        let location = format!("{}:{}-{}", item.relpath, item.range.start, item.range.end);

        if item.role == "primary" {
            // Content preview: first 3 lines, sanitized, capped at 200 chars
            let preview_lines: Vec<&str> = item.content.lines().take(3).collect();
            let preview_joined = preview_lines.join(" ");
            let preview_sanitized = sanitize_newlines(&preview_joined);
            let preview_capped: String = preview_sanitized.chars().take(200).collect();

            let _ = writeln!(
                output,
                "{} | {} | {} | {} | {}",
                item.role, location, item.tokens, item.reason, preview_capped
            );
        } else {
            let _ = writeln!(
                output,
                "{} | {} | {} | {}",
                item.role, location, item.tokens, item.reason
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
#[allow(clippy::collapsible_str_replace)]
fn sanitize_newlines(text: &str) -> String {
    // Replace \r\n first (before individual \r or \n) to produce a single space
    // per Windows line ending rather than two spaces.
    text.replace("\r\n", " ")
        .replace('\n', " ")
        .replace('\r', " ")
}

#[cfg(test)]
mod tests {
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
        let output = format_hits_agent(&hits);
        assert_eq!(
            output,
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
        let output = format_hits_agent(&hits);
        assert_eq!(
            output,
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
        let output = format_hits_agent(&hits);
        assert_eq!(output, "src/lib.rs:1 | module | 0.50 | Module declarations");
    }

    #[test]
    fn test_agent_format_missing_preview() {
        let hits = vec![make_hit("src/lib.rs", 1, "func", Some("init"), 0.85, None)];
        let output = format_hits_agent(&hits);
        assert_eq!(output, "src/lib.rs:1 | func init | 0.85 | -");
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
        let output = format_hits_agent(&hits);
        assert_eq!(output, "src/lib.rs:1 | func init | 0.85 | -");
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
        let output = format_hits_agent(&hits);
        assert_eq!(
            output,
            "src/main.rs:10 | func run | 0.60 | Line one Line two Line three Line four"
        );
    }

    #[test]
    fn test_agent_format_empty_hits() {
        let hits: Vec<SearchHit> = vec![];
        let output = format_hits_agent(&hits);
        assert_eq!(output, "");
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
        let output = format_hits_agent(&hits);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "src/app.rs:42 | func main | 0.92 | Entry point");
        assert_eq!(lines[1], "docs/api.md:8 | heading_2 | 0.73 | API reference");
    }

    #[test]
    fn test_agent_format_score_precision() {
        let hits = vec![make_hit("a.rs", 1, "func", Some("f"), 1.0, Some("text"))];
        let output = format_hits_agent(&hits);
        assert!(output.contains("| 1.00 |"));

        let hits = vec![make_hit("a.rs", 1, "func", Some("f"), 0.1, Some("text"))];
        let output = format_hits_agent(&hits);
        assert!(output.contains("| 0.10 |"));

        let hits = vec![make_hit(
            "a.rs",
            1,
            "func",
            Some("f"),
            0.123456,
            Some("text"),
        )];
        let output = format_hits_agent(&hits);
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
        let output = format_hits_json_search(&hits).unwrap();
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
            "file_path": "src/app.rs",
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
        let output = format_hits_agent(&hits);
        assert_eq!(
            output,
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
        let output = format_hits_agent(&hits);
        assert_eq!(
            output,
            "docs/api.md:8 | heading_2 | 0.73 | Authentication API reference"
        );
        // Must not contain "null" anywhere
        assert!(!output.contains("null"));
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
        let output = format_hits_agent(&hits);
        assert_eq!(output, "src/lib.rs:1 | func init | 0.85 | -");
    }

    #[test]
    fn test_format_hits_agent_empty_results() {
        let hits: Vec<SearchHit> = vec![];
        let output = format_hits_agent(&hits);
        assert_eq!(output, "");
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
        let output = format_hits_agent(&hits);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "src/app.rs:42 | func main | 0.92 | Entry point");
        assert_eq!(lines[1], "docs/api.md:8 | heading_2 | 0.73 | API reference");
        assert_eq!(
            lines[2],
            "tests/test_app.rs:100 | func test_main | 0.55 | Test case"
        );
        // Verify newline separators between lines
        assert_eq!(output.matches('\n').count(), 2);
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
        let output = format_hits_agent(&hits);
        assert_eq!(output, "a.rs:1 | func f | 0.90 | text");
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
        let output = format_hits_agent(&hits);
        assert_eq!(output, "a.rs:1 | func f | 0.00 | text");
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
        let output = format_hits_agent(&hits);
        assert_eq!(output, "a.rs:1 | func f | 17.50 | text");
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
        let output = format_hits_agent(&hits);
        assert!(output.contains("src/\u{00e9}dit.rs"));
        assert_eq!(
            output,
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
        let output = format_hits_agent(&hits);
        assert!(output.contains("\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}"));
        assert_eq!(
            output,
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
        let output = format_hits_agent(&hits);
        // The full path must appear in the output, no truncation
        assert!(output.contains(&long_path));
        assert_eq!(output, format!("{}:1 | func f | 0.50 | code", long_path));
    }

    #[test]
    fn test_format_hits_agent_empty_symbol_name_treated_as_none() {
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
        let output_empty = format_hits_agent(&hits_empty);
        let output_none = format_hits_agent(&hits_none);
        assert_eq!(output_empty, output_none);
        assert_eq!(
            output_empty,
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
        let output = format_hits_agent(&hits);
        assert_eq!(
            output,
            "src/main.rs:10 | func run | 0.60 | Line one Line two"
        );
        // Must be a single line (no newlines in the output for this hit)
        assert_eq!(output.lines().count(), 1);
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
        let output = format_hits_agent(&hits);
        assert_eq!(
            output,
            "src/main.rs:10 | func run | 0.60 | Line one Line two Line three Line four"
        );
        // Must be a single line
        assert_eq!(output.lines().count(), 1);
        // Must not contain any raw newline or carriage return characters
        assert!(!output.contains('\n'));
        assert!(!output.contains('\r'));
    }

    // ---------------------------------------------------------------
    // JSON formatter backward compatibility tests per MRIMP-5.2002
    // ---------------------------------------------------------------

    #[test]
    fn test_json_search_hits_array_structure() {
        // Verify Search JSON has correct top-level key and hit structure
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
        let output = format_hits_json_search(&hits).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        // Top-level must have "hits" key only
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
                "file_path": "src/auth.rs",
                "symbol_name": "authenticate",
                "kind": "func",
                "start_line": 15,
                "end_line": 45,
            }),
            serde_json::json!({
                "chunk_id": 202,
                "score": 0.82,
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
    }

    #[test]
    fn test_json_search_empty_hits_array() {
        // Verify empty hits produces valid JSON with empty array
        let hits: Vec<SearchHit> = vec![];
        let output = format_hits_json_search(&hits).unwrap();
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
    fn test_context_agent_mixed_items() {
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

        assert_eq!(lines.len(), 4);
        // Header: total tokens = 150 + 80 + 100 = 330
        assert_eq!(
            lines[0],
            "CONTEXT chunk_id=555 | tokens=330/6000 | items=3 | truncated=no"
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
            "test | tests/auth_test.rs:1-25 | 100 | Tests authenticate()"
        );
    }

    #[test]
    fn test_context_agent_truncated_flag() {
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
    fn test_context_agent_preview_capped_at_200_chars() {
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
    fn test_context_agent_preview_three_lines_only() {
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
    fn test_context_agent_empty_content_preview() {
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
    fn test_context_agent_multiple_primaries_fr8() {
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
    fn test_context_agent_unicode_preview_cap() {
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
}
