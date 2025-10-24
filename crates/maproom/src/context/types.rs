//! Core data structures for context assembly.

use serde::{Deserialize, Serialize};

/// A bundle of context items assembled for an LLM, respecting token budgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    /// Context items included in this bundle
    pub items: Vec<ContextItem>,
    /// Total token count across all items
    pub total_tokens: usize,
    /// Whether content was truncated to fit within budget
    pub truncated: bool,
}

impl ContextBundle {
    /// Create a new empty context bundle.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_tokens: 0,
            truncated: false,
        }
    }

    /// Add an item to the bundle and update the token count.
    pub fn add_item(&mut self, item: ContextItem) {
        self.total_tokens += item.tokens;
        self.items.push(item);
    }

    /// Check if adding an item would exceed the budget.
    pub fn would_exceed_budget(&self, item_tokens: usize, budget: usize) -> bool {
        self.total_tokens + item_tokens > budget
    }
}

impl Default for ContextBundle {
    fn default() -> Self {
        Self::new()
    }
}

/// A single item in a context bundle, representing a code section with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Relative path to the file
    pub relpath: String,
    /// Line range within the file
    pub range: LineRange,
    /// Role of this item in the context (e.g., "primary", "test", "caller")
    pub role: String,
    /// Explanation of why this item is included
    pub reason: String,
    /// The actual code content
    pub content: String,
    /// Token count for this content
    pub tokens: usize,
}

/// A line range within a file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineRange {
    /// Starting line number (1-indexed)
    pub start: i32,
    /// Ending line number (inclusive, 1-indexed)
    pub end: i32,
}

impl LineRange {
    /// Create a new line range.
    pub fn new(start: i32, end: i32) -> Self {
        Self { start, end }
    }

    /// Get the number of lines in this range.
    pub fn line_count(&self) -> usize {
        if self.end >= self.start {
            (self.end - self.start + 1) as usize
        } else {
            0
        }
    }
}

/// Options for expanding context beyond the primary chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandOptions {
    /// Include caller chunks (functions that call the primary chunk)
    pub callers: bool,
    /// Include callee chunks (functions called by the primary chunk)
    pub callees: bool,
    /// Include test chunks
    pub tests: bool,
    /// Include documentation chunks
    pub docs: bool,
    /// Include configuration files
    pub config: bool,
    /// Maximum depth for relationship traversal
    pub max_depth: i32,
}

impl Default for ExpandOptions {
    fn default() -> Self {
        Self {
            callers: false,
            callees: false,
            tests: false,
            docs: false,
            config: false,
            max_depth: 1,
        }
    }
}

impl ExpandOptions {
    /// Create options with all expansions disabled (primary chunk only).
    pub fn primary_only() -> Self {
        Self::default()
    }

    /// Create options with common expansions enabled (tests, one caller, one callee).
    pub fn with_common() -> Self {
        Self {
            callers: true,
            callees: true,
            tests: true,
            docs: false,
            config: false,
            max_depth: 1,
        }
    }

    /// Create options with all expansions enabled.
    pub fn with_all() -> Self {
        Self {
            callers: true,
            callees: true,
            tests: true,
            docs: true,
            config: true,
            max_depth: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_range_line_count() {
        let range = LineRange::new(10, 20);
        assert_eq!(range.line_count(), 11);

        let single_line = LineRange::new(5, 5);
        assert_eq!(single_line.line_count(), 1);

        let invalid = LineRange::new(20, 10);
        assert_eq!(invalid.line_count(), 0);
    }

    #[test]
    fn test_context_bundle_add_item() {
        let mut bundle = ContextBundle::new();
        assert_eq!(bundle.total_tokens, 0);

        let item = ContextItem {
            relpath: "test.rs".to_string(),
            range: LineRange::new(1, 10),
            role: "primary".to_string(),
            reason: "Target chunk".to_string(),
            content: "fn test() {}".to_string(),
            tokens: 5,
        };

        bundle.add_item(item);
        assert_eq!(bundle.total_tokens, 5);
        assert_eq!(bundle.items.len(), 1);
    }

    #[test]
    fn test_context_bundle_budget_check() {
        let mut bundle = ContextBundle::new();
        bundle.total_tokens = 100;

        assert!(!bundle.would_exceed_budget(50, 200));
        assert!(bundle.would_exceed_budget(150, 200));
    }

    #[test]
    fn test_expand_options_defaults() {
        let primary = ExpandOptions::primary_only();
        assert!(!primary.callers);
        assert!(!primary.callees);
        assert!(!primary.tests);

        let common = ExpandOptions::with_common();
        assert!(common.callers);
        assert!(common.callees);
        assert!(common.tests);
        assert!(!common.docs);
        assert!(!common.config);

        let all = ExpandOptions::with_all();
        assert!(all.callers);
        assert!(all.callees);
        assert!(all.tests);
        assert!(all.docs);
        assert!(all.config);
    }
}
