//! Rust parser

use tree_sitter::{Node, Parser};

use super::common::lang_rust;
use crate::indexer::SymbolChunk;

pub(super) fn extract_rust_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser.set_language(&lang_rust()).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut chunks = Vec::new();
    let root = tree.root_node();

    // Extract top-level declarations
    walk_rust_decls(source, root, &mut chunks);

    chunks
}

fn walk_rust_decls(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    match node.kind() {
        "function_item" => {
            extract_rust_function(source, node, chunks);
        }
        "struct_item" => {
            extract_rust_struct(source, node, chunks);
        }
        "enum_item" => {
            extract_rust_enum(source, node, chunks);
        }
        "trait_item" => {
            extract_rust_trait(source, node, chunks);
        }
        "impl_item" => {
            extract_rust_impl(source, node, chunks);
        }
        "mod_item" => {
            extract_rust_module(source, node, chunks);
        }
        "const_item" | "static_item" => {
            extract_rust_constant(source, node, chunks);
        }
        "macro_definition" => {
            extract_rust_macro(source, node, chunks);
        }
        "use_declaration" => {
            extract_rust_use_statement(source, node, chunks);
        }
        _ => {}
    }

    // Recursively walk child nodes
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_rust_decls(source, child, chunks);
        }
    }
}

fn extract_rust_function(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract function name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility modifier (pub, pub(crate), etc.)
    let visibility = extract_rust_visibility(source, node);

    // Extract function modifiers (async, const, unsafe)
    let modifiers = extract_rust_function_modifiers(source, node);
    let is_async = modifiers.contains(&"async");
    let is_const = modifiers.contains(&"const");
    let is_unsafe = modifiers.contains(&"unsafe");

    // Extract generic type parameters
    let type_params = node
        .child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Extract parameters for signature
    let params = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract return type
    let return_type = node
        .child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string());

    // Build signature
    let signature = build_rust_function_signature(
        visibility.as_deref(),
        is_async,
        is_const,
        is_unsafe,
        type_params.as_deref(),
        params.as_deref(),
        return_type.as_deref(),
        where_clause.as_deref(),
    );

    // Extract doc comment (lines starting with /// or //! before the function)
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())),
    );
    metadata_obj.insert("is_async".to_string(), serde_json::Value::Bool(is_async));
    metadata_obj.insert("is_const".to_string(), serde_json::Value::Bool(is_const));
    metadata_obj.insert("is_unsafe".to_string(), serde_json::Value::Bool(is_unsafe));

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert(
            "generics".to_string(),
            serde_json::Value::String(generics.clone()),
        );
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert(
            "where_clause".to_string(),
            serde_json::Value::String(wc.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_struct(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract struct name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type parameters (generics)
    let type_params = node
        .child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Build signature
    let signature = match (&type_params, &where_clause) {
        (Some(params), Some(wc)) => Some(format!("{} {}", params, wc)),
        (Some(params), None) => Some(params.clone()),
        (None, Some(wc)) => Some(wc.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())),
    );

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert(
            "generics".to_string(),
            serde_json::Value::String(generics.clone()),
        );
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert(
            "where_clause".to_string(),
            serde_json::Value::String(wc.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "struct".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_enum(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract enum name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type parameters (generics)
    let type_params = node
        .child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Build signature
    let signature = match (&type_params, &where_clause) {
        (Some(params), Some(wc)) => Some(format!("{} {}", params, wc)),
        (Some(params), None) => Some(params.clone()),
        (None, Some(wc)) => Some(wc.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())),
    );

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert(
            "generics".to_string(),
            serde_json::Value::String(generics.clone()),
        );
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert(
            "where_clause".to_string(),
            serde_json::Value::String(wc.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "enum".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_trait(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract trait name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type parameters (generics)
    let type_params = node
        .child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Build signature
    let signature = match (&type_params, &where_clause) {
        (Some(params), Some(wc)) => Some(format!("{} {}", params, wc)),
        (Some(params), None) => Some(params.clone()),
        (None, Some(wc)) => Some(wc.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())),
    );

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert(
            "generics".to_string(),
            serde_json::Value::String(generics.clone()),
        );
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert(
            "where_clause".to_string(),
            serde_json::Value::String(wc.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "trait".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_impl(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract the type being implemented for
    let type_node = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract the trait being implemented (if any)
    let trait_node = node
        .child_by_field_name("trait")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build name and signature
    let (name, signature) = if let Some(trait_name) = trait_node {
        let type_name = type_node.unwrap_or_else(|| "Unknown".to_string());
        (
            Some(format!("impl {} for {}", trait_name, type_name)),
            Some(format!("{} for {}", trait_name, type_name)),
        )
    } else if let Some(type_name) = type_node {
        (Some(format!("impl {}", type_name)), Some(type_name))
    } else {
        (None, None)
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "impl".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: None,
    });
}

fn extract_rust_module(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract module name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())),
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
}

fn extract_rust_constant(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract constant name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type annotation
    let type_annotation = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract value for signature
    let value = node
        .child_by_field_name("value")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = type_annotation.map(|ty| format!(": {}", ty));

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Determine kind (const vs static)
    let kind = if node.kind() == "static_item" {
        "static"
    } else {
        "constant"
    };

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "visibility".to_string(),
        serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())),
    );
    if let Some(v) = value {
        metadata_obj.insert("value".to_string(), serde_json::Value::String(v));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_macro(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract macro name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Mark macros as opaque blocks for now (as per ticket requirement)
    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "macro".to_string(),
        signature: Some("macro_rules!".to_string()),
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: None,
    });
}

// Helper functions for Rust parsing

fn extract_rust_visibility(source: &str, node: Node) -> Option<String> {
    // Look for visibility_modifier child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "visibility_modifier" {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.to_string());
            }
        }
    }
    None
}

fn extract_rust_function_modifiers(_source: &str, node: Node) -> Vec<&'static str> {
    let mut modifiers = Vec::new();

    // Look for function_modifiers child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "function_modifiers" {
                // Extract each modifier from the function_modifiers node
                for j in 0..child.child_count() {
                    if let Some(modifier) = child.child(j) {
                        match modifier.kind() {
                            "async" => modifiers.push("async"),
                            "const" => modifiers.push("const"),
                            "unsafe" => modifiers.push("unsafe"),
                            "default" => modifiers.push("default"),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    modifiers
}

#[allow(clippy::too_many_arguments)] // Each parameter maps to a distinct Rust syntax element
fn build_rust_function_signature(
    visibility: Option<&str>,
    is_async: bool,
    is_const: bool,
    is_unsafe: bool,
    type_params: Option<&str>,
    params: Option<&str>,
    return_type: Option<&str>,
    where_clause: Option<&str>,
) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(vis) = visibility {
        parts.push(vis.to_string());
    }

    if is_const {
        parts.push("const".to_string());
    }

    if is_async {
        parts.push("async".to_string());
    }

    if is_unsafe {
        parts.push("unsafe".to_string());
    }

    parts.push("fn".to_string());

    if let Some(tp) = type_params {
        parts.push(tp.to_string());
    }

    if let Some(p) = params {
        parts.push(p.to_string());
    }

    if let Some(ret) = return_type {
        parts.push(format!("-> {}", ret));
    }

    if let Some(wc) = where_clause {
        parts.push(wc.to_string());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn extract_rust_where_clause(source: &str, node: Node) -> Option<String> {
    // Look for where_clause child node
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "where_clause" {
                return child
                    .utf8_text(source.as_bytes())
                    .ok()
                    .map(|s| s.to_string());
            }
        }
    }
    None
}

fn extract_rust_use_statement(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract the full use statement text
    let use_text = node
        .utf8_text(source.as_bytes())
        .ok()
        .map(|s| s.to_string());

    // Try to extract a meaningful name for the use statement
    // For simple cases like "use std::collections::HashMap;", extract "HashMap"
    // For complex cases like "use std::io::{Read, Write};", extract the whole path
    let name = if let Some(ref text) = use_text {
        // Remove "use " prefix and ";" suffix
        let trimmed = text
            .trim_start_matches("use")
            .trim()
            .trim_end_matches(";")
            .trim();

        // Handle different use statement patterns:
        // - use foo::bar; -> "foo::bar"
        // - use foo::bar as baz; -> "foo::bar as baz"
        // - use foo::{bar, baz}; -> "foo::{bar, baz}"
        // - use super::*; -> "super::*"
        Some(trimmed.to_string())
    } else {
        None
    };

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    if let Some(vis) = visibility {
        metadata_obj.insert("visibility".to_string(), serde_json::Value::String(vis));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "use".to_string(),
        signature: use_text,
        docstring: None,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_doc_comment(source: &str, node: Node) -> Option<String> {
    // Look for line_comment or block_comment nodes before the declaration
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();

    let mut doc_lines = Vec::new();
    let mut scan_line = start_line.saturating_sub(1);

    // Scan backwards from the node to find doc comments
    while scan_line > 0 {
        let line = lines.get(scan_line)?;
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            // Doc comment line
            let comment_text = trimmed.trim_start_matches("///").trim();
            doc_lines.insert(0, comment_text.to_string());
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.starts_with("//!") {
            // Inner doc comment
            let comment_text = trimmed.trim_start_matches("//!").trim();
            doc_lines.insert(0, comment_text.to_string());
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.is_empty() {
            // Empty line, continue scanning
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.starts_with("#[") || trimmed.starts_with("#![") {
            // Attribute, skip
            scan_line = scan_line.saturating_sub(1);
        } else {
            // Non-comment, non-empty line - stop scanning
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

// Go-specific parsing functions
