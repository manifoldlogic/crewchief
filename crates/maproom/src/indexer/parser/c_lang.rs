//! C language parser

use tree_sitter::{Node, Parser};

use super::common::lang_c;
use crate::indexer::SymbolChunk;

pub(super) fn extract_c_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_c())
        .expect("Failed to set C language");

    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => return Vec::new(),
    };

    let mut chunks = Vec::new();
    let mut includes = Vec::new();
    let root = tree.root_node();

    walk_c_decls(source, root, &mut chunks, &mut includes);

    // Create __imports__ chunk if we collected any includes
    if !includes.is_empty() {
        chunks.push(SymbolChunk {
            symbol_name: Some("__imports__".to_string()),
            kind: "imports".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 1,
            metadata: Some(serde_json::json!(includes)),
        });
    }

    chunks
}

fn walk_c_decls(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    includes: &mut Vec<serde_json::Value>,
) {
    match node.kind() {
        "function_definition" => {
            extract_c_function(source, node, chunks);
        }
        "declaration" => {
            extract_c_declaration(source, node, chunks);
        }
        "type_definition" => {
            extract_c_typedef(source, node, chunks);
        }
        "preproc_include" => {
            collect_c_include(source, node, includes);
        }
        "struct_specifier" => {
            // Standalone struct definition at top level (e.g., `struct User { ... };`)
            // tree-sitter-c parses these as struct_specifier, not declaration
            if let Some(body) = node.child_by_field_name("body") {
                extract_c_struct(source, node, body, node, chunks);
            }
        }
        "enum_specifier" => {
            // Standalone enum definition at top level (e.g., `enum Color { ... };`)
            if let Some(body) = node.child_by_field_name("body") {
                extract_c_enum(source, node, body, node, chunks);
            }
        }
        _ => {}
    }

    // Recursively walk child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_c_decls(source, child, chunks, includes);
    }
}

fn extract_c_function(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract return type
    let return_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract declarator (contains function name and parameters)
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => return,
    };

    // Navigate the declarator tree to find the function name and parameters
    let (name, params) = extract_function_name_and_params(source, declarator);

    let name = match name {
        Some(n) => n,
        None => return,
    };

    // Build signature with return type and parameters
    let signature = match (&return_type, &params) {
        (Some(ret), Some(par)) => Some(format!("{} {}", ret, par)),
        (Some(ret), None) => Some(ret.clone()),
        (None, Some(par)) => Some(par.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_c_doc_comment(source, node);

    // Check for storage class specifier (static, extern, etc.)
    let storage_class = extract_storage_class(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    if let Some(ref storage) = storage_class {
        metadata_obj.insert(
            "storage_class".to_string(),
            serde_json::Value::String(storage.clone()),
        );
    }
    if let Some(ref ret) = return_type {
        metadata_obj.insert(
            "return_type".to_string(),
            serde_json::Value::String(ret.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: Some(name),
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: if metadata_obj.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(metadata_obj))
        },
    });
}

fn extract_c_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Get the type specifier
    let type_node = match node.child_by_field_name("type") {
        Some(t) => t,
        None => return,
    };

    // Check if this is a struct or enum definition with a body
    match type_node.kind() {
        "struct_specifier" => {
            // Only extract if it has a body (definition, not just declaration)
            if let Some(body) = type_node.child_by_field_name("body") {
                extract_c_struct(source, type_node, body, node, chunks);
                return; // Don't also extract as a variable
            }
        }
        "enum_specifier" => {
            // Only extract if it has a body
            if let Some(body) = type_node.child_by_field_name("body") {
                extract_c_enum(source, type_node, body, node, chunks);
                return; // Don't also extract as a variable
            }
        }
        _ => {}
    }

    // Check if this is a function declaration (has function_declarator but no body)
    if let Some(declarator) = node.child_by_field_name("declarator") {
        if is_function_declarator(&declarator) {
            extract_c_function_declaration(source, node, chunks);
            return;
        }
    }

    // If we get here, it's a variable declaration (or forward declaration)
    extract_c_global_variable(source, node, chunks);
}

fn is_function_declarator(node: &Node) -> bool {
    match node.kind() {
        "function_declarator" => true,
        "pointer_declarator" => {
            // Check if it's a pointer to a function
            if let Some(declarator) = node.child_by_field_name("declarator") {
                is_function_declarator(&declarator)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn extract_c_function_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract return type
    let return_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract declarator (contains function name and parameters)
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => return,
    };

    // Navigate the declarator tree to find the function name and parameters
    let (name, params) = extract_function_name_and_params(source, declarator);

    let name = match name {
        Some(n) => n,
        None => return,
    };

    // Build signature with return type and parameters
    let signature = match (&return_type, &params) {
        (Some(ret), Some(par)) => Some(format!("{} {}", ret, par)),
        (Some(ret), None) => Some(ret.clone()),
        (None, Some(par)) => Some(par.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_c_doc_comment(source, node);

    // Check for storage class specifier (static, extern, etc.)
    let storage_class = extract_storage_class(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    if let Some(ref storage) = storage_class {
        metadata_obj.insert(
            "storage_class".to_string(),
            serde_json::Value::String(storage.clone()),
        );
    }
    if let Some(ref ret) = return_type {
        metadata_obj.insert(
            "return_type".to_string(),
            serde_json::Value::String(ret.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: Some(name),
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: if metadata_obj.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(metadata_obj))
        },
    });
}

fn extract_c_struct(
    source: &str,
    type_node: Node,
    body: Node,
    declaration_node: Node,
    chunks: &mut Vec<SymbolChunk>,
) {
    // Extract struct name
    let name = type_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Count fields in the body
    let field_count = count_struct_fields(body);

    // Extract doc comment from the declaration node
    let docstring = extract_c_doc_comment(source, declaration_node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "field_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(field_count)),
    );

    let start = declaration_node.start_position();
    let end = declaration_node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "struct".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_c_enum(
    source: &str,
    type_node: Node,
    body: Node,
    declaration_node: Node,
    chunks: &mut Vec<SymbolChunk>,
) {
    // Extract enum name
    let name = type_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Count enumerators in the body
    let enumerator_count = count_enumerators(body);

    // Extract doc comment from the declaration node
    let docstring = extract_c_doc_comment(source, declaration_node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "enumerator_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(enumerator_count)),
    );

    let start = declaration_node.start_position();
    let end = declaration_node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "enum".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_c_global_variable(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract type
    let type_text = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract declarator
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => return,
    };

    // Handle multiple declarators (e.g., int a, b, c;)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "init_declarator" || child.kind() == "pointer_declarator" {
            if let Some(name) = extract_declarator_name(source, child) {
                let docstring = extract_c_doc_comment(source, node);

                let mut metadata_obj = serde_json::Map::new();
                if let Some(ref typ) = type_text {
                    metadata_obj.insert("type".to_string(), serde_json::Value::String(typ.clone()));
                }

                let start = node.start_position();
                let end = node.end_position();

                chunks.push(SymbolChunk {
                    symbol_name: Some(name),
                    kind: "variable".to_string(),
                    signature: type_text.clone(),
                    docstring: docstring.clone(),
                    start_line: (start.row + 1) as i32,
                    end_line: (end.row + 1) as i32,
                    metadata: if metadata_obj.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(metadata_obj))
                    },
                });
            }
        }
    }

    // Fallback: try to extract name from the declarator directly
    if let Some(name) = extract_declarator_name(source, declarator) {
        // Check if we already added this (avoid duplicates)
        if !chunks
            .iter()
            .any(|c| c.symbol_name.as_deref() == Some(&name))
        {
            let docstring = extract_c_doc_comment(source, node);

            let mut metadata_obj = serde_json::Map::new();
            if let Some(ref typ) = type_text {
                metadata_obj.insert("type".to_string(), serde_json::Value::String(typ.clone()));
            }

            let start = node.start_position();
            let end = node.end_position();

            chunks.push(SymbolChunk {
                symbol_name: Some(name),
                kind: "variable".to_string(),
                signature: type_text,
                docstring,
                start_line: (start.row + 1) as i32,
                end_line: (end.row + 1) as i32,
                metadata: if metadata_obj.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(metadata_obj))
                },
            });
        }
    }
}

fn extract_c_typedef(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract the type being aliased
    let type_node = node.child_by_field_name("type");
    let type_text = type_node
        .as_ref()
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract the typedef name from declarator
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => {
            // Handle anonymous struct typedef: typedef struct { ... } Name;
            if let Some(type_node) = type_node {
                if type_node.kind() == "struct_specifier" {
                    // Try to find the identifier after the struct body
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "type_identifier" {
                            if let Ok(name) = child.utf8_text(source.as_bytes()) {
                                let docstring = extract_c_doc_comment(source, node);

                                let mut metadata_obj = serde_json::Map::new();
                                metadata_obj.insert(
                                    "underlying_type".to_string(),
                                    serde_json::Value::String("struct".to_string()),
                                );

                                let start = node.start_position();
                                let end = node.end_position();

                                chunks.push(SymbolChunk {
                                    symbol_name: Some(name.to_string()),
                                    kind: "typedef".to_string(),
                                    signature: Some("struct".to_string()),
                                    docstring,
                                    start_line: (start.row + 1) as i32,
                                    end_line: (end.row + 1) as i32,
                                    metadata: Some(serde_json::Value::Object(metadata_obj)),
                                });
                                return;
                            }
                        }
                    }
                }
            }
            return;
        }
    };

    let name = extract_declarator_name(source, declarator);

    if let Some(name) = name {
        let docstring = extract_c_doc_comment(source, node);

        let mut metadata_obj = serde_json::Map::new();
        if let Some(ref typ) = type_text {
            metadata_obj.insert(
                "underlying_type".to_string(),
                serde_json::Value::String(typ.clone()),
            );
        }

        let start = node.start_position();
        let end = node.end_position();

        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "typedef".to_string(),
            signature: type_text,
            docstring,
            start_line: (start.row + 1) as i32,
            end_line: (end.row + 1) as i32,
            metadata: if metadata_obj.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(metadata_obj))
            },
        });
    }
}

fn collect_c_include(source: &str, node: Node, includes: &mut Vec<serde_json::Value>) {
    // Extract the path
    let path = node
        .child_by_field_name("path")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    if let Some(path) = path {
        // Determine if it's a system include or local include
        let is_system = path.starts_with('<');

        includes.push(serde_json::json!({
            "type": if is_system { "system" } else { "local" },
            "path": path
        }));
    }
}

fn extract_c_doc_comment(source: &str, node: Node) -> Option<String> {
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();
    let mut doc_lines = Vec::new();

    // Walk backward from the line before the node
    for i in (0..start_line).rev() {
        let line = lines.get(i)?.trim();

        // Handle single-line comments
        if line.starts_with("//") {
            let comment = line.strip_prefix("//").unwrap_or("").trim();
            doc_lines.insert(0, comment);
        }
        // Handle block comments
        else if line.ends_with("*/") {
            // Multi-line block comment - collect all lines
            let mut block_lines = Vec::new();
            let mut j = i;
            let mut found_start = false;

            while j < lines.len() {
                let block_line = lines[j].trim();
                block_lines.push(block_line);

                if block_line.starts_with("/*") || block_line.starts_with("/**") {
                    found_start = true;
                    break;
                }

                if j == 0 {
                    break;
                }
                j -= 1;
            }

            if found_start {
                // Clean up block comment lines
                for block_line in block_lines.iter().rev() {
                    let cleaned = block_line
                        .trim_start_matches("/*")
                        .trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .trim_start_matches('*')
                        .trim();
                    if !cleaned.is_empty() {
                        doc_lines.insert(0, cleaned);
                    }
                }
            }
            break;
        } else if line.starts_with("/*") {
            // Single-line block comment
            let comment = line
                .strip_prefix("/*")
                .unwrap_or("")
                .trim_end_matches("*/")
                .trim();
            doc_lines.insert(0, comment);
            break;
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

// Helper functions

fn extract_function_name_and_params(source: &str, node: Node) -> (Option<String>, Option<String>) {
    match node.kind() {
        "function_declarator" => {
            // Get the nested declarator (contains the actual name)
            let name_node = node.child_by_field_name("declarator");
            let name = name_node.and_then(|n| extract_identifier(source, n));

            // Get parameters
            let params = node
                .child_by_field_name("parameters")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string());

            (name, params)
        }
        "pointer_declarator" => {
            // Pointer to function - recurse into the declarator
            if let Some(declarator) = node.child_by_field_name("declarator") {
                extract_function_name_and_params(source, declarator)
            } else {
                (None, None)
            }
        }
        "identifier" => {
            let name = node
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
            (name, None)
        }
        _ => (None, None),
    }
}

fn extract_identifier(source: &str, node: Node) -> Option<String> {
    match node.kind() {
        "identifier" => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string()),
        "pointer_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_identifier(source, declarator)
        }
        "function_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_identifier(source, declarator)
        }
        _ => None,
    }
}

fn extract_declarator_name(source: &str, node: Node) -> Option<String> {
    match node.kind() {
        "identifier" => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string()),
        "init_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_declarator_name(source, declarator)
        }
        "pointer_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_declarator_name(source, declarator)
        }
        "array_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_declarator_name(source, declarator)
        }
        "type_identifier" => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn extract_storage_class(source: &str, node: Node) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier" {
            return child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
    }
    None
}

fn count_struct_fields(body: Node) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            count += 1;
        }
    }
    count
}

fn count_enumerators(body: Node) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "enumerator" {
            count += 1;
        }
    }
    count
}
