//! Markdown parser implementation

use regex::Regex;
use tree_sitter::{Node, Parser};

use super::common::HierarchyTracker;
use crate::indexer::SymbolChunk;
use crate::profile_scope;

// Language provider
#[allow(dead_code)]
fn lang_markdown() -> tree_sitter::Language {
    tree_sitter_md::language()
}

/// Extract chunks from Markdown source code
pub(super) fn extract_markdown_chunks(source: &str) -> Vec<SymbolChunk> {
    profile_scope!("extract_markdown_chunks");

    let mut parser = Parser::new();
    parser.set_language(&lang_markdown()).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut chunks = Vec::new();
    let root = tree.root_node();
    let mut hierarchy = HierarchyTracker::new();

    // Walk the tree and extract headings and code blocks
    walk_markdown_nodes(source, root, &mut chunks, &mut hierarchy);

    // Extract links using regex (tree-sitter-md limitation workaround)
    // See MD_ENHANCE-1001 ticket lines 114-118 for context
    extract_markdown_links(source, &mut chunks);

    chunks
}

fn walk_markdown_nodes(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    hierarchy: &mut HierarchyTracker,
) {
    let kind = node.kind();

    match kind {
        "atx_heading" => {
            extract_heading(source, node, chunks, hierarchy);
        }
        "fenced_code_block" => {
            extract_code_block(source, node, chunks, hierarchy);
        }
        "pipe_table" => {
            extract_table(source, node, chunks);
        }
        "list" => {
            extract_list(source, node, chunks);
        }
        // Note: tree-sitter-md does not provide structured link nodes
        // Links are parsed as individual punctuation tokens within inline content
        // Link extraction will be handled in a future ticket (MD_ENHANCE-3002)
        // using regex or alternative approach
        _ => {}
    }

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_markdown_nodes(source, child, chunks, hierarchy);
        }
    }
}

fn extract_heading(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    hierarchy: &mut HierarchyTracker,
) {
    // Get heading level by checking the marker
    let mut level = 0;
    let mut heading_text = String::new();

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "atx_h1_marker" => level = 1,
                "atx_h2_marker" => level = 2,
                "atx_h3_marker" => level = 3,
                "atx_h4_marker" => level = 4,
                "atx_h5_marker" => level = 5,
                "atx_h6_marker" => level = 6,
                "inline" => {
                    // The heading text is in the inline node
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        heading_text = text.trim().to_string();
                    }
                }
                _ => {}
            }
        }
    }

    if level > 0 && !heading_text.is_empty() {
        let start_line = (node.start_position().row + 1) as i32;

        // Find section end (next heading of same or higher level, or EOF)
        let end_line = find_section_end(source, node, level);

        // Update hierarchy and get parent path
        let parent_path = hierarchy.enter_heading(level as u8, heading_text.clone());

        let kind = format!("heading_{}", level);

        chunks.push(SymbolChunk {
            symbol_name: Some(heading_text),
            kind,
            signature: None,
            docstring: None,
            start_line,
            end_line,
            metadata: Some(serde_json::json!({
                "level": level,
                "parent_path": parent_path
            })),
        });
    }
}

fn find_section_end(source: &str, heading_node: Node, heading_level: usize) -> i32 {
    let start_row = heading_node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();

    // Start searching from the next line after the heading
    let mut end_idx = start_row + 1;
    let mut in_code_block = false;

    while end_idx < lines.len() {
        let line = lines[end_idx];

        // Track code blocks
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            end_idx += 1;
            continue;
        }

        // Only check for headings outside of code blocks
        if !in_code_block {
            if let Some(next_level) = get_heading_level_from_line(line) {
                if next_level <= heading_level {
                    // Found a heading of same or higher level - section ends here
                    return end_idx as i32;
                }
            }
        }

        end_idx += 1;
    }

    // Section goes to end of file
    lines.len() as i32
}

fn get_heading_level_from_line(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }

    let mut level = 0;
    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
            if level > 6 {
                return None; // Not a valid heading
            }
        } else if ch == ' ' {
            // Valid heading must have space after #
            return Some(level);
        } else {
            // Not a valid heading (e.g., "#tag" without space)
            return None;
        }
    }
    None
}

fn extract_code_block(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    hierarchy: &HierarchyTracker,
) {
    let mut language: Option<String> = None;
    let mut code_lines_count = 0;

    // Extract language from info_string if present
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "info_string" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        // Extract just the language name (first word) from info_string
                        // This handles cases like "typescript {1-3}" or "rust copy"
                        let lang_text = text.split_whitespace().next().unwrap_or(text.trim());
                        language = Some(lang_text.to_string());
                    }
                }
                "code_fence_content" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        code_lines_count = text.lines().count();
                    }
                }
                _ => {}
            }
        }
    }

    let start_line = (node.start_position().row + 1) as i32;
    let end_line = (node.end_position().row + 1) as i32;

    // Get parent heading path for linking code block to its section
    let parent_path = hierarchy.get_current_path();

    let symbol_name = if let Some(ref lang) = language {
        format!("Code: {}", lang)
    } else {
        "Code: plain".to_string()
    };

    chunks.push(SymbolChunk {
        symbol_name: Some(symbol_name),
        kind: "code_block".to_string(),
        signature: None,
        docstring: None,
        start_line,
        end_line,
        metadata: Some(serde_json::json!({
            "language": language.unwrap_or_else(|| "plain".to_string()),
            "parent_path": parent_path,
            "lines_of_code": code_lines_count
        })),
    });
}

fn extract_table(_source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let start_line = (node.start_position().row + 1) as i32;
    let end_line = (node.end_position().row + 1) as i32;

    // Count rows and columns
    let mut row_count = 0;
    let mut column_count = 0;
    let mut has_header = false;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "pipe_table_header" => {
                    has_header = true;
                    row_count += 1;
                    // Count cells in header to determine column count
                    for j in 0..child.child_count() {
                        if let Some(cell) = child.child(j) {
                            if cell.kind() == "pipe_table_cell" {
                                column_count += 1;
                            }
                        }
                    }
                }
                "pipe_table_row" => {
                    row_count += 1;
                }
                _ => {}
            }
        }
    }

    chunks.push(SymbolChunk {
        symbol_name: Some(format!("Table {}x{}", row_count, column_count)),
        kind: "markdown_section".to_string(),
        signature: None,
        docstring: None,
        start_line,
        end_line,
        metadata: Some(serde_json::json!({
            "section_type": "table",
            "rows": row_count,
            "columns": column_count,
            "has_header": has_header
        })),
    });
}

fn extract_list(_source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let start_line = (node.start_position().row + 1) as i32;
    let end_line = (node.end_position().row + 1) as i32;

    // Determine list type and count items
    let mut list_type = "unordered";
    let mut item_count = 0;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "list_item" {
                item_count += 1;

                // Check first list item to determine type
                if item_count == 1 {
                    for j in 0..child.child_count() {
                        if let Some(marker) = child.child(j) {
                            if marker.kind() == "list_marker_dot" {
                                list_type = "ordered";
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    chunks.push(SymbolChunk {
        symbol_name: Some(format!("List ({} items)", item_count)),
        kind: "markdown_section".to_string(),
        signature: None,
        docstring: None,
        start_line,
        end_line,
        metadata: Some(serde_json::json!({
            "list_type": list_type,
            "item_count": item_count
        })),
    });
}

/// Extract markdown links using regex patterns.
/// This is a workaround for tree-sitter-md limitation where links are parsed
/// as individual punctuation tokens rather than structured nodes.
/// See MD_ENHANCE-1001 ticket lines 114-118 for context.
///
/// Extracts three types of links:
/// 1. Regular links: [text](url)
/// 2. Image links: ![alt](url)
/// 3. Both extract the link text/alt and the target URL
fn extract_markdown_links(source: &str, chunks: &mut Vec<SymbolChunk>) {
    // Regex patterns for markdown links
    // Regular link: [text](url) - captures text in group 1, url in group 2
    // Image link: ![alt](url) - captures alt in group 1, url in group 2
    let link_pattern = Regex::new(r"(?m)(!?)\[([^\]]*)\]\(([^)]+)\)").unwrap();

    for cap in link_pattern.captures_iter(source) {
        let is_image = cap.get(1).is_some_and(|m| m.as_str() == "!");
        let link_text = cap.get(2).map_or("", |m| m.as_str());
        let target = cap.get(3).map_or("", |m| m.as_str());

        // Skip empty targets
        if target.trim().is_empty() {
            continue;
        }

        // Classify the link type
        let link_type = classify_link(target);

        // Find the line number where this link appears
        let full_match = cap.get(0).unwrap();
        let link_position = full_match.start();
        let line_number = find_line_number(source, link_position);

        // Create chunk metadata
        let metadata = serde_json::json!({
            "link_type": link_type,
            "target": target,
            "link_text": link_text,
            "is_image": is_image,
        });

        // Create a link chunk
        let kind = if is_image { "image_link" } else { "link" };
        let symbol_name = if !link_text.is_empty() {
            Some(link_text.to_string())
        } else {
            Some(target.to_string())
        };

        chunks.push(SymbolChunk {
            symbol_name,
            kind: kind.to_string(),
            signature: Some(target.to_string()),
            docstring: None,
            start_line: line_number as i32,
            end_line: line_number as i32,
            metadata: Some(metadata),
        });
    }
}

/// Classify a link target into one of: "external", "anchor", "relative", or "absolute"
fn classify_link(target: &str) -> String {
    if target.starts_with("http://") || target.starts_with("https://") {
        "external".to_string()
    } else if target.starts_with('#') {
        "anchor".to_string()
    } else if target.starts_with('/') {
        "absolute".to_string()
    } else {
        "relative".to_string()
    }
}

/// Find the line number (1-indexed) where a character position appears in the source
fn find_line_number(source: &str, position: usize) -> usize {
    let mut current_pos = 0;
    for (line_idx, line) in source.lines().enumerate() {
        let line_len = line.len() + 1; // +1 for newline
        if current_pos + line_len > position {
            return line_idx + 1; // 1-indexed
        }
        current_pos += line_len;
    }
    1 // Default to line 1 if not found
}
