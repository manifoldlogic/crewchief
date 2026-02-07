//! Go parser

use tree_sitter::{Node, Parser};

use super::common::lang_go;
use crate::indexer::SymbolChunk;

pub(super) fn extract_go_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser.set_language(&lang_go()).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut chunks = Vec::new();
    let root = tree.root_node();

    // Extract top-level declarations
    walk_go_decls(source, root, &mut chunks);

    chunks
}

fn walk_go_decls(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    match node.kind() {
        "function_declaration" => {
            extract_go_function(source, node, chunks);
        }
        "method_declaration" => {
            extract_go_method(source, node, chunks);
        }
        "type_declaration" => {
            extract_go_type_declaration(source, node, chunks);
        }
        "const_declaration" => {
            extract_go_const_declaration(source, node, chunks);
        }
        "var_declaration" => {
            extract_go_var_declaration(source, node, chunks);
        }
        "package_clause" => {
            extract_go_package(source, node, chunks);
        }
        "import_declaration" => {
            extract_go_import(source, node, chunks);
        }
        _ => {}
    }

    // Recursively walk child nodes
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_go_decls(source, child, chunks);
        }
    }
}

fn extract_go_function(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract function name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract parameters
    let params = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract return type
    let result = node
        .child_by_field_name("result")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = match (&params, &result) {
        (Some(p), Some(r)) => Some(format!("{} {}", p, r)),
        (Some(p), None) => Some(p.clone()),
        (None, Some(r)) => Some(r.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    // Detect goroutines and channels in the function body
    let (has_goroutines, has_channels) = detect_go_concurrency(node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with goroutine/channel flags and visibility
    let metadata = {
        let mut meta = serde_json::Map::new();

        // Add visibility based on function name
        if let Some(ref func_name) = name {
            meta.insert(
                "visibility".to_string(),
                serde_json::json!(go_visibility(func_name)),
            );
        }

        if has_goroutines {
            meta.insert("has_goroutines".to_string(), serde_json::json!(true));
        }
        if has_channels {
            meta.insert("has_channels".to_string(), serde_json::json!(true));
        }

        if meta.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(meta))
        }
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_method(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract method name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract receiver (e.g., "(r *MyType)")
    let receiver = node
        .child_by_field_name("receiver")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract parameters
    let params = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract return type
    let result = node
        .child_by_field_name("result")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature with receiver
    let signature = match (&receiver, &params, &result) {
        (Some(r), Some(p), Some(ret)) => Some(format!("{} {} {}", r, p, ret)),
        (Some(r), Some(p), None) => Some(format!("{} {}", r, p)),
        (Some(r), None, Some(ret)) => Some(format!("{} {}", r, ret)),
        (Some(r), None, None) => Some(r.clone()),
        (None, Some(p), Some(ret)) => Some(format!("{} {}", p, ret)),
        (None, Some(p), None) => Some(p.clone()),
        (None, None, Some(ret)) => Some(ret.clone()),
        (None, None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    // Detect goroutines and channels in the method body
    let (has_goroutines, has_channels) = detect_go_concurrency(node);

    // Parse receiver to get type and pointer/value information
    let (receiver_type_name, receiver_type) = if let Some(ref r) = receiver {
        parse_go_receiver(r)
    } else {
        (None, None)
    };

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with receiver, receiver_type, visibility, and goroutine/channel flags
    let metadata = {
        let mut meta = serde_json::Map::new();

        // Add visibility based on method name
        if let Some(ref method_name) = name {
            meta.insert(
                "visibility".to_string(),
                serde_json::json!(go_visibility(method_name)),
            );
        }

        if let Some(r) = receiver {
            meta.insert("receiver".to_string(), serde_json::json!(r));
        }
        if let Some(rt) = receiver_type {
            meta.insert("receiver_type".to_string(), serde_json::json!(rt));
        }
        if let Some(rtn) = receiver_type_name {
            meta.insert("receiver_type_name".to_string(), serde_json::json!(rtn));
        }
        if has_goroutines {
            meta.insert("has_goroutines".to_string(), serde_json::json!(true));
        }
        if has_channels {
            meta.insert("has_channels".to_string(), serde_json::json!(true));
        }
        if meta.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(meta))
        }
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "method".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_type_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go type declarations can have multiple specs (e.g., type ( ... ))
    // Look for type_spec children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "type_spec" {
                extract_go_type_spec(source, child, chunks);
            }
        }
    }
}

fn extract_go_type_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract type name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract type definition
    let type_def = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Determine the kind based on the type and extract additional metadata
    let type_node_opt = node.child_by_field_name("type");
    let kind = if let Some(ref type_node) = type_node_opt {
        match type_node.kind() {
            "struct_type" => "struct",
            "interface_type" => "interface",
            _ => "type",
        }
    } else {
        "type"
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with visibility and type-specific details
    let metadata = {
        let mut meta = serde_json::Map::new();

        // Add visibility based on type name
        if let Some(ref type_name) = name {
            meta.insert(
                "visibility".to_string(),
                serde_json::json!(go_visibility(type_name)),
            );
        }

        // For structs, extract embedded types
        if let Some(ref type_node) = type_node_opt {
            if type_node.kind() == "struct_type" {
                let embedded_types = extract_go_embedded_types(source, *type_node);
                if !embedded_types.is_empty() {
                    meta.insert(
                        "embedded_types".to_string(),
                        serde_json::json!(embedded_types),
                    );
                }
            } else if type_node.kind() == "interface_type" {
                // For interfaces, extract method signatures
                let interface_methods = extract_go_interface_methods(source, *type_node);
                if !interface_methods.is_empty() {
                    meta.insert(
                        "interface_methods".to_string(),
                        serde_json::json!(interface_methods),
                    );
                }
            }
        }

        if meta.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(meta))
        }
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature: type_def,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_const_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go const declarations can have multiple specs (e.g., const ( ... ))
    // Look for const_spec children or const_spec_list
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "const_spec" {
                extract_go_const_spec(source, child, chunks);
            } else if child.kind() == "const_spec_list" {
                // Handle const_spec_list which contains multiple const_spec nodes
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "const_spec" {
                            extract_go_const_spec(source, spec, chunks);
                        }
                    }
                }
            }
        }
    }
}

fn extract_go_const_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract constant name - look for first identifier child
    let name = {
        let mut const_name = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" {
                    const_name = child
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                    break;
                }
            }
        }
        const_name
    };

    // Extract type (if specified)
    let type_annotation = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract value
    let value = node
        .child_by_field_name("value")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = match (&type_annotation, &value) {
        (Some(t), Some(v)) => Some(format!("{} = {}", t, v)),
        (Some(t), None) => Some(t.clone()),
        (None, Some(v)) => Some(format!("= {}", v)),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with visibility
    let metadata = if let Some(ref const_name) = name {
        let mut meta = serde_json::Map::new();
        meta.insert(
            "visibility".to_string(),
            serde_json::json!(go_visibility(const_name)),
        );
        Some(serde_json::Value::Object(meta))
    } else {
        None
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "constant".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_var_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go var declarations can have multiple specs (e.g., var ( ... ))
    // Look for var_spec children or var_spec_list
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "var_spec" {
                extract_go_var_spec(source, child, chunks);
            } else if child.kind() == "var_spec_list" {
                // Handle var_spec_list which contains multiple var_spec nodes
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "var_spec" {
                            extract_go_var_spec(source, spec, chunks);
                        }
                    }
                }
            }
        }
    }
}

fn extract_go_var_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract variable name - look for first identifier child
    let name = {
        let mut var_name = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" {
                    var_name = child
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                    break;
                }
            }
        }
        var_name
    };

    // Extract type (if specified)
    let type_annotation = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract value
    let value = node
        .child_by_field_name("value")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = match (&type_annotation, &value) {
        (Some(t), Some(v)) => Some(format!("{} = {}", t, v)),
        (Some(t), None) => Some(t.clone()),
        (None, Some(v)) => Some(format!("= {}", v)),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with visibility
    let metadata = if let Some(ref var_name) = name {
        let mut meta = serde_json::Map::new();
        meta.insert(
            "visibility".to_string(),
            serde_json::json!(go_visibility(var_name)),
        );
        Some(serde_json::Value::Object(meta))
    } else {
        None
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "variable".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_package(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract package name from package_identifier child
    let name = {
        let mut pkg_name = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "package_identifier" {
                    pkg_name = child
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                    break;
                }
            }
        }
        pkg_name
    };

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "package".to_string(),
        signature: None,
        docstring: None,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: None,
    });
}

fn extract_go_import(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go import declarations can have multiple specs (single or grouped imports)
    // Look for import_spec or import_spec_list children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "import_spec" {
                extract_go_import_spec(source, child, chunks);
            } else if child.kind() == "import_spec_list" {
                // Handle grouped imports: import ( ... )
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "import_spec" {
                            extract_go_import_spec(source, spec, chunks);
                        }
                    }
                }
            }
        }
    }
}

fn extract_go_import_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract import path (the string literal)
    let import_path = node
        .child_by_field_name("path")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim_matches('"').to_string());

    // Extract import alias (if any)
    let alias = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // The symbol name is the alias if present, otherwise the import path
    let symbol_name = alias.clone().or_else(|| import_path.clone());

    let start = node.start_position();
    let end = node.end_position();

    // Create metadata with import details
    let mut metadata = serde_json::Map::new();
    if let Some(path) = &import_path {
        metadata.insert("import_path".to_string(), serde_json::json!(path));
    }
    if let Some(alias_name) = &alias {
        metadata.insert("alias".to_string(), serde_json::json!(alias_name));
    }

    chunks.push(SymbolChunk {
        symbol_name,
        kind: "import".to_string(),
        signature: import_path,
        docstring: None,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: if metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(metadata))
        },
    });
}

fn detect_go_concurrency(node: Node) -> (bool, bool) {
    // Recursively search for goroutine and channel usage in the AST
    let mut has_goroutines = false;
    let mut has_channels = false;

    fn walk_node(node: Node, has_goroutines: &mut bool, has_channels: &mut bool) {
        match node.kind() {
            "go_statement" => {
                // Found a goroutine spawn (go keyword)
                *has_goroutines = true;
            }
            "channel_type" => {
                // Found a channel type declaration (chan int, etc.)
                *has_channels = true;
            }
            "send_statement" | "receive_operator" => {
                // Found channel send/receive operations (<-)
                *has_channels = true;
            }
            _ => {}
        }

        // Recursively check child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                walk_node(child, has_goroutines, has_channels);
            }
        }
    }

    walk_node(node, &mut has_goroutines, &mut has_channels);
    (has_goroutines, has_channels)
}

fn extract_go_doc_comment(source: &str, node: Node) -> Option<String> {
    // Look for comment nodes before the declaration
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();

    let mut doc_lines = Vec::new();
    let mut scan_line = start_line.saturating_sub(1);

    // Scan backwards from the node to find doc comments
    while scan_line > 0 {
        let line = lines.get(scan_line)?;
        let trimmed = line.trim();

        if trimmed.starts_with("//") {
            // Doc comment line
            let comment_text = trimmed.trim_start_matches("//").trim();
            doc_lines.insert(0, comment_text.to_string());
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.is_empty() {
            // Empty line - check if we already have comments
            if doc_lines.is_empty() {
                scan_line = scan_line.saturating_sub(1);
            } else {
                // Empty line after comments - stop
                break;
            }
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

// Helper function to determine if a Go identifier is exported (PascalCase) or unexported (camelCase)
fn go_is_exported(name: &str) -> bool {
    // In Go, an identifier is exported if it starts with an uppercase letter
    name.chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
}

// Helper function to determine Go visibility based on identifier name
fn go_visibility(name: &str) -> &'static str {
    if go_is_exported(name) {
        "exported"
    } else {
        "unexported"
    }
}

// Helper function to parse receiver type and determine if it's a pointer or value receiver
// Returns (receiver_type_name, is_pointer)
fn parse_go_receiver(receiver_text: &str) -> (Option<String>, Option<&'static str>) {
    // Receiver format: "(r *Type)" or "(r Type)"
    // Strip parentheses and split on whitespace
    let stripped = receiver_text
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')');
    let parts: Vec<&str> = stripped.split_whitespace().collect();

    if parts.len() >= 2 {
        let type_part = parts[1];
        if type_part.starts_with('*') {
            // Pointer receiver
            let type_name = type_part.trim_start_matches('*').to_string();
            (Some(type_name), Some("pointer"))
        } else {
            // Value receiver
            (Some(type_part.to_string()), Some("value"))
        }
    } else {
        (None, None)
    }
}

// Helper function to extract embedded types from a struct_type node
fn extract_go_embedded_types(source: &str, struct_node: Node) -> Vec<String> {
    let mut embedded_types = Vec::new();

    // Look for field_declaration_list child
    for i in 0..struct_node.child_count() {
        if let Some(child) = struct_node.child(i) {
            if child.kind() == "field_declaration_list" {
                // Iterate through field declarations
                for j in 0..child.child_count() {
                    if let Some(field) = child.child(j) {
                        if field.kind() == "field_declaration" {
                            // Check if this is an embedded field (no explicit field name)
                            // An embedded field has a type but no name list
                            let has_name = field.child_by_field_name("name").is_some();

                            if !has_name {
                                // This is an embedded field - extract the type
                                if let Some(type_node) = field.child_by_field_name("type") {
                                    if let Ok(type_text) = type_node.utf8_text(source.as_bytes()) {
                                        // Handle pointer types (strip the *)
                                        let type_name = type_text.trim_start_matches('*').trim();
                                        embedded_types.push(type_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    embedded_types
}

// Helper function to extract interface method signatures from an interface_type node
fn extract_go_interface_methods(source: &str, interface_node: Node) -> Vec<String> {
    let mut methods = Vec::new();

    // Look for method_elem children (used by tree-sitter-go for interface methods)
    for i in 0..interface_node.child_count() {
        if let Some(child) = interface_node.child(i) {
            if child.kind() == "method_elem" {
                // Extract the full method signature
                if let Ok(method_sig) = child.utf8_text(source.as_bytes()) {
                    methods.push(method_sig.trim().to_string());
                }
            }
        }
    }

    methods
}

// go.mod parsing (simple text-based parsing)
pub(super) fn extract_gomod_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Extract module name
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("module ") {
            let module_name = trimmed.strip_prefix("module ").unwrap_or("").trim();

            chunks.push(SymbolChunk {
                symbol_name: Some(module_name.to_string()),
                kind: "module".to_string(),
                signature: None,
                docstring: None,
                start_line: (i + 1) as i32,
                end_line: (i + 1) as i32,
                metadata: Some(serde_json::json!({"type": "go_module"})),
            });
        } else if trimmed.starts_with("go ") {
            // Go version requirement
            let version = trimmed.strip_prefix("go ").unwrap_or("").trim();

            chunks.push(SymbolChunk {
                symbol_name: Some(format!("go {}", version)),
                kind: "go_version".to_string(),
                signature: None,
                docstring: None,
                start_line: (i + 1) as i32,
                end_line: (i + 1) as i32,
                metadata: Some(serde_json::json!({"version": version})),
            });
        } else if trimmed.starts_with("require ") {
            // Single-line require
            let req = trimmed.strip_prefix("require ").unwrap_or("").trim();
            if !req.is_empty() && !req.starts_with('(') {
                chunks.push(SymbolChunk {
                    symbol_name: Some(req.to_string()),
                    kind: "require".to_string(),
                    signature: None,
                    docstring: None,
                    start_line: (i + 1) as i32,
                    end_line: (i + 1) as i32,
                    metadata: Some(serde_json::json!({"dependency": req})),
                });
            }
        }
    }

    // Handle multi-line require blocks
    let mut in_require = false;
    let mut require_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("require (") || trimmed == "require (" {
            in_require = true;
            require_start = i;
        } else if in_require && trimmed == ")" {
            in_require = false;
        } else if in_require && !trimmed.is_empty() && !trimmed.starts_with("//") {
            // Extract dependency from require block
            let dep = trimmed.trim();
            if !dep.is_empty() {
                chunks.push(SymbolChunk {
                    symbol_name: Some(dep.to_string()),
                    kind: "require".to_string(),
                    signature: None,
                    docstring: None,
                    start_line: (require_start + 1) as i32,
                    end_line: (i + 1) as i32,
                    metadata: Some(serde_json::json!({"dependency": dep})),
                });
            }
        }
    }

    chunks
}

// Ruby-specific parsing functions
