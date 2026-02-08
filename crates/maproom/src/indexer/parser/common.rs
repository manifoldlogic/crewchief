//! Common utilities and types shared across all language parsers

use tree_sitter::{Language, Node};

use crate::indexer::SymbolChunk;

/// Heading hierarchy tracking for markdown parent paths
pub(crate) struct HeadingNode {
    pub(crate) level: u8,
    pub(crate) text: String,
}

pub(crate) struct HierarchyTracker {
    pub(crate) stack: Vec<HeadingNode>,
}

impl HierarchyTracker {
    pub(crate) fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Update the hierarchy stack when entering a new heading
    /// Returns the parent path (breadcrumb) for this heading
    pub(crate) fn enter_heading(&mut self, level: u8, text: String) -> String {
        // Pop stack until we're at the appropriate parent level
        // If new level <= current level, we need to pop to find the parent
        while let Some(top) = self.stack.last() {
            if top.level >= level {
                self.stack.pop();
            } else {
                break;
            }
        }

        // Generate parent path from remaining stack
        let parent_path = if self.stack.is_empty() {
            String::new()
        } else {
            self.stack
                .iter()
                .map(|node| node.text.as_str())
                .collect::<Vec<_>>()
                .join(" > ")
        };

        // Push the new heading onto the stack
        self.stack.push(HeadingNode { level, text });

        parent_path
    }

    /// Get the current heading path (full breadcrumb including current heading)
    /// Used by code blocks and other elements to link to their parent section
    pub(crate) fn get_current_path(&self) -> String {
        if self.stack.is_empty() {
            String::new()
        } else {
            self.stack
                .iter()
                .map(|node| node.text.as_str())
                .collect::<Vec<_>>()
                .join(" > ")
        }
    }
}

// Use the safe language providers exposed by the crates
pub(crate) fn lang_typescript() -> Language {
    tree_sitter_typescript::language_typescript()
}

pub(crate) fn lang_tsx() -> Language {
    tree_sitter_typescript::language_tsx()
}

pub(crate) fn lang_javascript() -> Language {
    tree_sitter_javascript::language()
}

pub(crate) fn lang_python() -> Language {
    tree_sitter_python::language()
}

pub(crate) fn lang_rust() -> Language {
    tree_sitter_rust::language()
}

pub(crate) fn lang_go() -> Language {
    tree_sitter_go::language()
}

pub(crate) fn lang_markdown() -> Language {
    tree_sitter_md::language()
}

pub(crate) fn lang_ruby() -> Language {
    tree_sitter_ruby::language()
}

pub(crate) fn lang_csharp() -> Language {
    tree_sitter_c_sharp::language()
}

pub(crate) fn lang_cpp() -> Language {
    tree_sitter_cpp::language()
}

/// Helper function to push a chunk with node position
pub(crate) fn push_chunk(
    source: &str,
    node: Node,
    name: Option<String>,
    kind: &str,
    out: &mut Vec<SymbolChunk>,
) {
    let start = node.start_position();
    let end = node.end_position();
    let start_line = (start.row + 1) as i32;
    let end_line = (end.row + 1) as i32;
    let _preview = extract_preview(source, start_line, end_line);
    out.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature: None,
        docstring: None,
        start_line,
        end_line,
        metadata: None,
    });
}

/// Extract a preview from source between line ranges
pub(crate) fn extract_preview(source: &str, start_line: i32, end_line: i32) -> String {
    let start = start_line.max(1) as usize - 1;
    let end = end_line.max(start_line) as usize;
    source
        .lines()
        .skip(start)
        .take(end - start)
        .take(60)
        .collect::<Vec<_>>()
        .join("\n")
}
