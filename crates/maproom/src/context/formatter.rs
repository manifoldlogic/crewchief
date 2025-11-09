//! Content formatting utilities for transforming chunks into ContextItems.
//!
//! This module provides the ContentFormatter which takes raw chunk metadata
//! and formats it into semantically annotated ContextItem structures with:
//! - Role labels (primary, test, caller, callee, config, hook, route)
//! - Human-readable reason explanations
//! - Intelligent truncation when content exceeds budgets
//! - Accurate token counting

use anyhow::Result;
use tracing::debug;

use super::token_counter::TokenCounter;
use super::truncation::{CodeTruncator, TruncationStrategy};
use super::types::{ContextItem, LineRange};

/// Content formatter that transforms chunks into annotated ContextItems.
///
/// The formatter is responsible for:
/// - Adding semantic role labels
/// - Generating explanatory reason text
/// - Truncating content when it exceeds token budgets
/// - Counting tokens accurately
///
/// # Example
///
/// ```
/// use crewchief_maproom::context::formatter::ContentFormatter;
/// use crewchief_maproom::context::types::LineRange;
///
/// let formatter = ContentFormatter::new();
/// let content = "fn test() { println!(\"hello\"); }";
///
/// let item = formatter.format(
///     "src/test.rs",
///     LineRange::new(1, 1),
///     "primary",
///     None,
///     content,
///     Some(100)
/// ).unwrap();
///
/// assert_eq!(item.role, "primary");
/// assert!(item.tokens > 0);
/// ```
pub struct ContentFormatter {
    token_counter: TokenCounter,
    truncator: CodeTruncator,
}

impl ContentFormatter {
    /// Create a new content formatter.
    pub fn new() -> Self {
        Self {
            token_counter: TokenCounter::new(),
            truncator: CodeTruncator::new(),
        }
    }

    /// Format a chunk into a ContextItem with metadata.
    ///
    /// # Arguments
    ///
    /// * `relpath` - Relative path to the file
    /// * `range` - Line range of the chunk
    /// * `role` - Semantic role (e.g., "primary", "test", "caller")
    /// * `symbol_name` - Optional name of the symbol (function, class, etc.)
    /// * `content` - The actual code content
    /// * `max_tokens` - Optional budget limit; if exceeded, content will be truncated
    ///
    /// # Returns
    ///
    /// A ContextItem with role, reason, content (possibly truncated), and token count.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::formatter::ContentFormatter;
    /// use crewchief_maproom::context::types::LineRange;
    ///
    /// let formatter = ContentFormatter::new();
    /// let item = formatter.format(
    ///     "src/main.rs",
    ///     LineRange::new(10, 20),
    ///     "primary",
    ///     Some("main"),
    ///     "fn main() { }\n",
    ///     None
    /// ).unwrap();
    ///
    /// assert_eq!(item.relpath, "src/main.rs");
    /// assert_eq!(item.role, "primary");
    /// ```
    pub fn format(
        &self,
        relpath: &str,
        range: LineRange,
        role: &str,
        symbol_name: Option<&str>,
        content: &str,
        max_tokens: Option<usize>,
    ) -> Result<ContextItem> {
        // Generate reason based on role
        let reason = self.get_reason(role, symbol_name, relpath, range);

        // Handle truncation if max_tokens is specified
        let (final_content, tokens) = if let Some(budget) = max_tokens {
            let current_tokens = self.token_counter.count(content)?;
            if current_tokens > budget {
                debug!(
                    "Truncating {} from {} to {} tokens",
                    relpath, current_tokens, budget
                );
                let result = self.truncator.truncate(
                    content,
                    budget,
                    TruncationStrategy::PreserveSignature,
                )?;
                (result.content, result.tokens)
            } else {
                (content.to_string(), current_tokens)
            }
        } else {
            let tokens = self.token_counter.count(content)?;
            (content.to_string(), tokens)
        };

        Ok(ContextItem {
            relpath: relpath.to_string(),
            range,
            role: role.to_string(),
            reason,
            content: final_content,
            tokens,
        })
    }

    /// Generate a human-readable reason explaining why this chunk is included.
    ///
    /// The reason varies based on the role:
    /// - `primary`: The main chunk being examined
    /// - `test`: Test file for this code
    /// - `caller`: Code that calls this function
    /// - `callee`: Code called by this function
    /// - `config`: Configuration file
    /// - `hook`: React hook or lifecycle method
    /// - `route`: Route definition
    ///
    /// # Arguments
    ///
    /// * `role` - Semantic role of the chunk
    /// * `symbol_name` - Optional symbol name for context
    /// * `relpath` - File path for additional context
    /// * `range` - Line range for location info
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::formatter::ContentFormatter;
    /// use crewchief_maproom::context::types::LineRange;
    ///
    /// let formatter = ContentFormatter::new();
    /// let reason = formatter.get_reason(
    ///     "test",
    ///     Some("testMain"),
    ///     "src/main.test.ts",
    ///     LineRange::new(10, 20)
    /// );
    ///
    /// assert!(reason.contains("test"));
    /// ```
    pub fn get_reason(
        &self,
        role: &str,
        symbol_name: Option<&str>,
        relpath: &str,
        range: LineRange,
    ) -> String {
        let location = format!("{}:{}-{}", relpath, range.start, range.end);

        match role {
            "primary" => {
                if let Some(name) = symbol_name {
                    format!("Primary implementation: {} at {}", name, location)
                } else {
                    format!("Primary implementation at {}", location)
                }
            }
            "test" => {
                if let Some(name) = symbol_name {
                    format!("Tests '{}' behavior at {}", name, location)
                } else {
                    format!("Test file at {}", location)
                }
            }
            "caller" => {
                format!("Calls this code from {}", location)
            }
            "callee" => {
                if let Some(name) = symbol_name {
                    format!("Called by this code: {} at {}", name, location)
                } else {
                    format!("Called by this code at {}", location)
                }
            }
            "config" => {
                format!("Configuration file at {}", location)
            }
            "hook" => {
                if let Some(name) = symbol_name {
                    format!("React hook: {} at {}", name, location)
                } else {
                    format!("React hook at {}", location)
                }
            }
            "route" => {
                if let Some(name) = symbol_name {
                    format!("Route definition: {} at {}", name, location)
                } else {
                    format!("Route definition at {}", location)
                }
            }
            "neighbor" => {
                if let Some(name) = symbol_name {
                    format!("Related code: {} at {}", name, location)
                } else {
                    format!("Related code at {}", location)
                }
            }
            "import" => {
                if let Some(name) = symbol_name {
                    format!("Imported by this code: {} at {}", name, location)
                } else {
                    format!("Imported at {}", location)
                }
            }
            "export" => {
                if let Some(name) = symbol_name {
                    format!("Exports from this code: {} at {}", name, location)
                } else {
                    format!("Exported at {}", location)
                }
            }
            "doc" => {
                format!("Documentation at {}", location)
            }
            _ => {
                // Unknown role - provide generic reason
                format!("Related code ({}) at {}", role, location)
            }
        }
    }

    /// Count tokens in content using the internal token counter.
    ///
    /// This is a convenience method for counting tokens without formatting.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::formatter::ContentFormatter;
    ///
    /// let formatter = ContentFormatter::new();
    /// let tokens = formatter.count_tokens("fn test() {}").unwrap();
    /// assert!(tokens > 0);
    /// ```
    pub fn count_tokens(&self, content: &str) -> Result<usize> {
        self.token_counter.count(content)
    }

    /// Format content with automatic truncation to fit a specific budget.
    ///
    /// This is a specialized version of `format()` that always applies
    /// truncation if needed. Useful when you have a strict budget constraint.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::formatter::ContentFormatter;
    /// use crewchief_maproom::context::types::LineRange;
    ///
    /// let formatter = ContentFormatter::new();
    /// let long_content = "fn test() {\n".to_string() + &"    // line\n".repeat(100) + "}";
    ///
    /// let item = formatter.format_with_truncation(
    ///     "src/test.rs",
    ///     LineRange::new(1, 100),
    ///     "primary",
    ///     Some("test"),
    ///     &long_content,
    ///     50,
    ///     super::truncation::TruncationStrategy::PreserveSignature
    /// ).unwrap();
    ///
    /// assert!(item.tokens <= 50);
    /// ```
    pub fn format_with_truncation(
        &self,
        relpath: &str,
        range: LineRange,
        role: &str,
        symbol_name: Option<&str>,
        content: &str,
        budget: usize,
        strategy: TruncationStrategy,
    ) -> Result<ContextItem> {
        let reason = self.get_reason(role, symbol_name, relpath, range);

        // Apply truncation
        let result = self.truncator.truncate(content, budget, strategy)?;

        Ok(ContextItem {
            relpath: relpath.to_string(),
            range,
            role: role.to_string(),
            reason,
            content: result.content,
            tokens: result.tokens,
        })
    }
}

impl Default for ContentFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_basic() {
        let formatter = ContentFormatter::new();
        let content = "fn test() { println!(\"hello\"); }";

        let item = formatter
            .format(
                "src/test.rs",
                LineRange::new(1, 1),
                "primary",
                Some("test"),
                content,
                None,
            )
            .unwrap();

        assert_eq!(item.relpath, "src/test.rs");
        assert_eq!(item.range.start, 1);
        assert_eq!(item.range.end, 1);
        assert_eq!(item.role, "primary");
        assert!(item.reason.contains("Primary implementation"));
        assert!(item.reason.contains("test"));
        assert_eq!(item.content, content);
        assert!(item.tokens > 0);
    }

    #[test]
    fn test_format_with_truncation_needed() {
        let formatter = ContentFormatter::new();

        // Create long content that will definitely exceed budget
        let mut content = String::from("fn long_function() {\n");
        for i in 0..100 {
            content.push_str(&format!("    let x{} = {};\n", i, i));
        }
        content.push_str("}");

        let item = formatter
            .format(
                "src/long.rs",
                LineRange::new(1, 102),
                "primary",
                Some("long_function"),
                &content,
                Some(100), // Small budget
            )
            .unwrap();

        assert!(item.tokens <= 100);
        assert!(item.content.contains("fn long_function()"));
        assert!(item.content.contains("[truncated]"));
    }

    #[test]
    fn test_format_no_truncation() {
        let formatter = ContentFormatter::new();
        let content = "fn short() { return 42; }";

        let item = formatter
            .format(
                "src/short.rs",
                LineRange::new(1, 1),
                "primary",
                Some("short"),
                content,
                Some(1000), // Large budget
            )
            .unwrap();

        assert_eq!(item.content, content);
        assert!(item.tokens < 1000);
    }

    #[test]
    fn test_get_reason_primary() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "primary",
            Some("main"),
            "src/main.rs",
            LineRange::new(10, 20),
        );

        assert!(reason.contains("Primary implementation"));
        assert!(reason.contains("main"));
        assert!(reason.contains("src/main.rs:10-20"));
    }

    #[test]
    fn test_get_reason_test() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "test",
            Some("testMain"),
            "src/main.test.ts",
            LineRange::new(5, 15),
        );

        assert!(reason.contains("Tests"));
        assert!(reason.contains("testMain"));
        assert!(reason.contains("behavior"));
    }

    #[test]
    fn test_get_reason_caller() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "caller",
            Some("parentFunction"),
            "src/parent.rs",
            LineRange::new(1, 10),
        );

        assert!(reason.contains("Calls this code"));
        assert!(reason.contains("src/parent.rs:1-10"));
    }

    #[test]
    fn test_get_reason_callee() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "callee",
            Some("helper"),
            "src/helper.rs",
            LineRange::new(20, 30),
        );

        assert!(reason.contains("Called by this code"));
        assert!(reason.contains("helper"));
    }

    #[test]
    fn test_get_reason_config() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason("config", None, "tsconfig.json", LineRange::new(1, 50));

        assert!(reason.contains("Configuration file"));
        assert!(reason.contains("tsconfig.json"));
    }

    #[test]
    fn test_get_reason_hook() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "hook",
            Some("useAuth"),
            "src/hooks/useAuth.ts",
            LineRange::new(1, 30),
        );

        assert!(reason.contains("React hook"));
        assert!(reason.contains("useAuth"));
    }

    #[test]
    fn test_get_reason_route() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "route",
            Some("/api/users"),
            "src/routes/users.ts",
            LineRange::new(10, 50),
        );

        assert!(reason.contains("Route definition"));
        assert!(reason.contains("/api/users"));
    }

    #[test]
    fn test_get_reason_neighbor() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "neighbor",
            Some("relatedFunc"),
            "src/utils.rs",
            LineRange::new(5, 10),
        );

        assert!(reason.contains("Related code"));
        assert!(reason.contains("relatedFunc"));
    }

    #[test]
    fn test_get_reason_unknown_role() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "unknown_role",
            Some("something"),
            "src/file.rs",
            LineRange::new(1, 5),
        );

        assert!(reason.contains("Related code"));
        assert!(reason.contains("unknown_role"));
    }

    #[test]
    fn test_get_reason_no_symbol_name() {
        let formatter = ContentFormatter::new();

        let reason =
            formatter.get_reason("primary", None, "src/anonymous.rs", LineRange::new(1, 10));

        assert!(reason.contains("Primary implementation"));
        assert!(!reason.contains("<unknown>"));
    }

    #[test]
    fn test_count_tokens() {
        let formatter = ContentFormatter::new();

        let tokens = formatter.count_tokens("fn test() {}").unwrap();
        assert!(tokens > 0);

        let tokens_empty = formatter.count_tokens("").unwrap();
        assert_eq!(tokens_empty, 0);
    }

    #[test]
    fn test_format_with_truncation_strategy() {
        let formatter = ContentFormatter::new();

        let mut content = String::new();
        for i in 0..50 {
            content.push_str(&format!("line {}\n", i));
        }

        let item = formatter
            .format_with_truncation(
                "src/test.rs",
                LineRange::new(1, 50),
                "primary",
                Some("test"),
                &content,
                50,
                TruncationStrategy::PreserveSignature,
            )
            .unwrap();

        assert!(item.tokens <= 50);
        assert!(item.content.contains("[truncated]"));
    }

    #[test]
    fn test_format_all_role_types() {
        let formatter = ContentFormatter::new();
        let content = "fn test() {}";
        let range = LineRange::new(1, 1);

        let roles = vec![
            "primary", "test", "caller", "callee", "config", "hook", "route", "neighbor", "import",
            "export", "doc",
        ];

        for role in roles {
            let item = formatter
                .format("src/test.rs", range, role, Some("test"), content, None)
                .unwrap();

            assert_eq!(item.role, role);
            assert!(!item.reason.is_empty());
            assert!(item.tokens > 0);
        }
    }

    #[test]
    fn test_format_preserves_content_when_under_budget() {
        let formatter = ContentFormatter::new();
        let content = "fn small() { return 1; }";

        let item = formatter
            .format(
                "src/small.rs",
                LineRange::new(1, 1),
                "primary",
                Some("small"),
                content,
                Some(1000),
            )
            .unwrap();

        // Content should be unchanged
        assert_eq!(item.content, content);
        assert!(item.tokens < 1000);
    }

    #[test]
    fn test_format_empty_content() {
        let formatter = ContentFormatter::new();

        let item = formatter
            .format(
                "src/empty.rs",
                LineRange::new(1, 1),
                "primary",
                None,
                "",
                None,
            )
            .unwrap();

        assert_eq!(item.content, "");
        assert_eq!(item.tokens, 0);
    }

    #[test]
    fn test_format_multiline_content() {
        let formatter = ContentFormatter::new();

        let content = r#"fn fibonacci(n: u32) -> u32 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}"#;

        let item = formatter
            .format(
                "src/math.rs",
                LineRange::new(10, 16),
                "primary",
                Some("fibonacci"),
                content,
                None,
            )
            .unwrap();

        assert_eq!(item.content, content);
        assert!(item.tokens > 20);
    }

    #[test]
    fn test_reason_location_formatting() {
        let formatter = ContentFormatter::new();

        let reason = formatter.get_reason(
            "primary",
            Some("test"),
            "src/deeply/nested/file.rs",
            LineRange::new(100, 200),
        );

        assert!(reason.contains("src/deeply/nested/file.rs:100-200"));
    }

    #[test]
    fn test_format_with_special_characters() {
        let formatter = ContentFormatter::new();
        let content = "fn test() { println!(\"Hello 世界 👋\"); }";

        let item = formatter
            .format(
                "src/unicode.rs",
                LineRange::new(1, 1),
                "primary",
                Some("test"),
                content,
                None,
            )
            .unwrap();

        assert_eq!(item.content, content);
        assert!(item.tokens > 0);
    }
}
