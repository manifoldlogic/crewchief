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

/// Output format for search results.
///
/// Controls how search results are rendered to stdout.
/// Used as a clap `ValueEnum` for the `--format` CLI flag.
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
}
