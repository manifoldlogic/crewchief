//! Ruby parser

use tree_sitter::{Node, Parser};

use super::common::lang_ruby;
use crate::indexer::SymbolChunk;

pub(super) fn extract_ruby_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_ruby())
        .expect("Failed to set Ruby language");
    let tree = parser.parse(source, None);
    let mut chunks = Vec::new();

    if let Some(tree) = tree {
        let root = tree.root_node();
        let mut imports = Vec::new();
        let mut visibility = "public"; // Ruby default visibility
        walk_ruby_decls(source, root, &mut chunks, &mut imports, &mut visibility);

        // Create __imports__ chunk if we collected any imports
        if !imports.is_empty() {
            chunks.push(SymbolChunk {
                symbol_name: Some("__imports__".to_string()),
                kind: "imports".to_string(),
                signature: None,
                docstring: None,
                start_line: 1,
                end_line: 1,
                metadata: Some(serde_json::json!(imports)),
            });
        }
    }

    chunks
}

fn walk_ruby_decls(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
    visibility: &mut &str,
) {
    match node.kind() {
        "class" => {
            extract_ruby_class(source, node, chunks, visibility);
        }
        "module" => {
            extract_ruby_module(source, node, chunks, visibility);
        }
        "method" => {
            extract_ruby_method(source, node, chunks, visibility);
        }
        "singleton_method" => {
            extract_ruby_singleton_method(source, node, chunks);
        }
        "assignment" => {
            extract_ruby_assignment(source, node, chunks);
        }
        "call" => {
            // Check if this is a visibility modifier call
            if let Some(method) = node.child_by_field_name("method") {
                if let Ok(method_text) = method.utf8_text(source.as_bytes()) {
                    let method_text = method_text.trim();
                    // Check if no arguments (affects subsequent methods)
                    // A call node with just the method name and no arguments is a visibility modifier
                    let has_arguments = node.child_by_field_name("arguments").is_some();
                    if !has_arguments {
                        match method_text {
                            "private" => *visibility = "private",
                            "protected" => *visibility = "protected",
                            "public" => *visibility = "public",
                            _ => {}
                        }
                    }
                }
            }
            // Also check for import calls
            collect_ruby_import(source, node, imports);
        }
        _ => {}
    }

    // Recursively walk child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_ruby_decls(source, child, chunks, imports, visibility);
    }
}

fn extract_ruby_assignment(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Get left side of assignment
    let Some(left_node) = node.child_by_field_name("left") else {
        return;
    };

    // Check if it's a constant (uppercase name)
    if left_node.kind() != "constant" {
        return; // Skip non-constant assignments
    }

    let Ok(name) = left_node.utf8_text(source.as_bytes()) else {
        return;
    };

    // Get right side value for signature
    let signature = node
        .child_by_field_name("right")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract doc comment
    let docstring = extract_ruby_doc_comment(source, node);

    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "constant".to_string(),
        signature,
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: None,
    });
}

fn collect_ruby_import(source: &str, node: Node, imports: &mut Vec<serde_json::Value>) {
    let Some(method_node) = node.child_by_field_name("method") else {
        return;
    };
    let Ok(method_text) = method_node.utf8_text(source.as_bytes()) else {
        return;
    };

    let import_type = match method_text {
        "require" => "require",
        "require_relative" => "require_relative",
        "include" => "include",
        "extend" => "extend",
        "prepend" => "prepend",
        _ => return, // Not an import
    };

    // Extract argument (target)
    let target = if let Some(arg_node) = node.child_by_field_name("arguments") {
        arg_node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    imports.push(serde_json::json!({
        "type": import_type,
        "target": target
    }));
}

fn extract_ruby_class(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    visibility: &mut &str,
) {
    // Extract class name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract superclass for signature
    let signature = node
        .child_by_field_name("superclass")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract doc comment
    let docstring = extract_ruby_doc_comment(source, node);

    // Extract superclass name for metadata (strip "< " prefix if present)
    let base_class = node
        .child_by_field_name("superclass")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim().strip_prefix('<').unwrap_or(s).trim().to_string());

    // Build metadata object
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.to_string()),
    );
    if let Some(base) = base_class {
        metadata_obj.insert("base_class".to_string(), serde_json::Value::String(base));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "class".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });

    // Save current visibility, reset to public for nested content, then restore
    let saved_visibility = *visibility;
    *visibility = "public";

    // Recurse into class body
    if let Some(body) = node.child_by_field_name("body") {
        let mut imports = Vec::new();
        walk_ruby_decls(source, body, chunks, &mut imports, visibility);
    }

    *visibility = saved_visibility;
}

fn extract_ruby_module(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    visibility: &mut &str,
) {
    // Extract module name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract doc comment
    let docstring = extract_ruby_doc_comment(source, node);

    // Build metadata object
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.to_string()),
    );

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "module".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });

    // Save current visibility, reset to public for nested content, then restore
    let saved_visibility = *visibility;
    *visibility = "public";

    // Recurse into module body
    if let Some(body) = node.child_by_field_name("body") {
        let mut imports = Vec::new();
        walk_ruby_decls(source, body, chunks, &mut imports, visibility);
    }

    *visibility = saved_visibility;
}

fn extract_ruby_doc_comment(source: &str, node: Node) -> Option<String> {
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();
    let mut doc_lines = Vec::new();

    // Walk backward from the line before the node
    for i in (0..start_line).rev() {
        let line = lines.get(i)?.trim();
        if line.starts_with('#') {
            // Strip "# " or "#" prefix
            let comment = if let Some(stripped) = line.strip_prefix("# ") {
                stripped
            } else {
                line.strip_prefix('#').unwrap_or_default()
            };
            doc_lines.insert(0, comment);
        } else if !line.is_empty() {
            // Non-comment, non-blank line - stop
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

fn is_inside_ruby_class(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "class" | "module" => return true,
            _ => current = parent.parent(),
        }
    }
    false
}

fn extract_ruby_method(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>, visibility: &str) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract parameters (may be "parameters" or "method_parameters" depending on tree-sitter version)
    let signature = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract doc comment
    let docstring = extract_ruby_doc_comment(source, node);

    // Determine if inside class/module
    let kind = if is_inside_ruby_class(node) {
        "method"
    } else {
        "func"
    };

    // Build metadata
    let metadata = serde_json::json!({
        "visibility": visibility,
        "is_class_method": false
    });

    let start = node.start_position();
    let end = node.end_position();

    // Push chunk
    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(metadata),
    });

    // Do NOT recurse into method body - methods in Ruby don't typically contain other methods
}

fn extract_ruby_singleton_method(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract parameters
    let signature = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract doc comment
    let docstring = extract_ruby_doc_comment(source, node);

    // Build metadata - class methods are always public and is_class_method is true
    let metadata = serde_json::json!({
        "visibility": "public",
        "is_class_method": true
    });

    let start = node.start_position();
    let end = node.end_position();

    // Push chunk - kind is always "method" (class methods are only defined inside classes)
    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "method".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(metadata),
    });

    // Do NOT recurse into method body
}
