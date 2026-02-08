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

// Language provider functions used by per-language parser modules.
// Only include functions that are actually imported by a parser module.
// TS/JS/Markdown parsers call tree-sitter crates directly.

pub(crate) fn lang_python() -> Language {
    tree_sitter_python::language()
}

pub(crate) fn lang_rust() -> Language {
    tree_sitter_rust::language()
}

pub(crate) fn lang_go() -> Language {
    tree_sitter_go::language()
}

pub(crate) fn lang_ruby() -> Language {
    tree_sitter_ruby::language()
}

pub(crate) fn lang_csharp() -> Language {
    tree_sitter_c_sharp::language()
}

pub(crate) fn lang_java() -> Language {
    tree_sitter_java::language()
}

pub(crate) fn lang_cpp() -> Language {
    tree_sitter_cpp::language()
}

/// Extract visibility modifier from a node's children.
///
/// Iterates through node children looking for nodes whose kind matches one of the
/// provided `visibility_keywords`. Returns the first match found, or `default` if none found.
///
/// This is useful for C-family languages (C#, Java, C++) that use similar visibility systems.
///
/// # Arguments
/// - `node` - The AST node to inspect (typically a declaration node)
/// - `source` - The source code text
/// - `visibility_keywords` - List of node kinds that represent visibility modifiers
/// - `default` - Default visibility if no modifier found (e.g., "internal", "package", "private")
///
/// # Example
/// ```rust,ignore
/// // C# usage
/// let vis = extract_visibility_from_modifiers(
///     &node,
///     source,
///     &["public", "private", "protected", "internal"],
///     "internal"
/// );
/// ```
pub(crate) fn extract_visibility_from_modifiers(
    node: &Node,
    source: &str,
    visibility_keywords: &[&str],
    default: &str,
) -> String {
    let mut access_modifiers = Vec::new();

    // Iterate through children looking for modifier nodes
    for child in node.children(&mut node.walk()) {
        if child.kind() == "modifier" {
            if let Ok(modifier_text) = child.utf8_text(source.as_bytes()) {
                // Check if this modifier text is a visibility keyword
                if visibility_keywords.contains(&modifier_text) {
                    access_modifiers.push(modifier_text.to_string());
                }
            }
        }
    }

    if access_modifiers.is_empty() {
        default.to_string()
    } else {
        // Handle combined modifiers like "protected internal"
        access_modifiers.join(" ")
    }
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
