//! Java language parser for semantic code indexing.
//!
//! Extracts classes, interfaces, enums, records, annotation types, methods,
//! constructors, fields, and import declarations from Java source code using
//! the tree-sitter-java grammar. Each extracted element becomes a [`SymbolChunk`]
//! with metadata capturing Java-specific details such as visibility modifiers,
//! annotations, inheritance, and throws clauses.

use tree_sitter::{Node, Parser};

use super::common::lang_java;
use crate::indexer::SymbolChunk;

/// Extracts semantic chunks from Java source code.
///
/// Parses the given Java source using tree-sitter-java and walks the AST to
/// extract classes, interfaces, enums, records, annotation types, methods,
/// constructors, and fields as individual [`SymbolChunk`] values. Import
/// declarations are aggregated into a single `__imports__` chunk with metadata
/// containing the import paths, static/wildcard flags.
///
/// # Arguments
///
/// * `source` - Java source code to parse.
///
/// # Returns
///
/// A vector of [`SymbolChunk`] representing the extracted Java symbols. Returns
/// an empty vector if the source cannot be parsed (e.g., completely invalid input).
///
/// # Examples
///
/// ```no_run
/// # use crewchief_maproom::indexer::parser::java::extract_java_chunks;
/// let source = "public class Foo { void bar() {} }";
/// let chunks = extract_java_chunks(source);
/// assert_eq!(chunks.len(), 2); // class + method
/// ```
pub(super) fn extract_java_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_java())
        .expect("Failed to set Java language");

    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => {
            tracing::warn!("Failed to parse Java source");
            return Vec::new();
        }
    };

    let mut chunks = Vec::new();
    let mut imports = Vec::new();
    let root = tree.root_node();
    walk_java_decls(source, root, &mut chunks, &mut imports);

    // Aggregate imports into __imports__ chunk
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

    chunks
}

/// Recursive AST walker that dispatches on node kind
fn walk_java_decls(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => {
                tracing::debug!(
                    node_kind = "class_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java class"
                );
                extract_java_class(source, child, chunks, imports);
            }
            "interface_declaration" => {
                tracing::debug!(
                    node_kind = "interface_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java interface"
                );
                extract_java_interface(source, child, chunks, imports);
            }
            "enum_declaration" => {
                tracing::debug!(
                    node_kind = "enum_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java enum"
                );
                extract_java_enum(source, child, chunks, imports);
            }
            "record_declaration" => {
                tracing::debug!(
                    node_kind = "record_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java record"
                );
                extract_java_record(source, child, chunks, imports);
            }
            "annotation_type_declaration" => {
                tracing::debug!(
                    node_kind = "annotation_type_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java annotation type"
                );
                extract_java_annotation_type(source, child, chunks);
            }
            "method_declaration" => {
                tracing::debug!(
                    node_kind = "method_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java method"
                );
                extract_java_method(source, child, chunks);
            }
            "constructor_declaration" => {
                tracing::debug!(
                    node_kind = "constructor_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java constructor"
                );
                extract_java_constructor(source, child, chunks);
            }
            "field_declaration" => {
                tracing::debug!(
                    node_kind = "field_declaration",
                    start_line = child.start_position().row + 1,
                    "Extracting Java field"
                );
                extract_java_field(source, child, chunks);
            }
            "import_declaration" => {
                tracing::debug!(
                    node_kind = "import_declaration",
                    start_line = child.start_position().row + 1,
                    "Collecting Java import"
                );
                collect_java_import(source, child, imports);
            }
            _ => {
                // Recurse into other nodes
                walk_java_decls(source, child, chunks, imports);
            }
        }
    }
}

/// Collect import declaration metadata
fn collect_java_import(source: &str, node: Node, imports: &mut Vec<serde_json::Value>) {
    // Determine if static import (e.g. "import static java.lang.Math.PI;")
    let is_static = node
        .children(&mut node.walk())
        .any(|child| child.kind() == "static");

    // Extract import path and check for wildcard
    let mut path = String::new();
    let mut is_wildcard = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "scoped_identifier" | "identifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    path = text.to_string();
                }
            }
            // Asterisk indicates wildcard import (e.g. "import java.util.*;")
            "asterisk" => {
                is_wildcard = true;
                path.push_str(".*");
            }
            _ => {}
        }
    }

    if !path.is_empty() {
        imports.push(serde_json::json!({
            "path": path,
            "is_static": is_static,
            "is_wildcard": is_wildcard,
        }));
    }
}

/// Extract class declaration
fn extract_java_class(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("UnknownClass");

    // Extract superclass (strip "extends " prefix)
    let superclass = node
        .child_by_field_name("superclass")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .and_then(|s| s.strip_prefix("extends "))
        .map(|s| s.trim().to_string());

    // Extract interfaces
    let interfaces = extract_interfaces(source, node);

    // Extract Javadoc
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers
    let modifiers = extract_java_modifiers(source, node);

    // Build signature: "extends SuperClass implements Interface1, Interface2"
    let mut sig_parts = Vec::new();
    if let Some(ref sc) = superclass {
        sig_parts.push(format!("extends {}", sc));
    }
    if !interfaces.is_empty() {
        sig_parts.push(format!("implements {}", interfaces.join(", ")));
    }
    let signature = if sig_parts.is_empty() {
        None
    } else {
        Some(sig_parts.join(" "))
    };

    // Build metadata
    let mut metadata = serde_json::json!({
        "visibility": modifiers.visibility,
        "modifiers": modifiers.modifiers,
        "annotations": modifiers.annotations,
    });
    if let Some(sc) = superclass {
        metadata["superclass"] = serde_json::Value::String(sc);
    }
    if !interfaces.is_empty() {
        metadata["interfaces"] = serde_json::Value::Array(
            interfaces
                .iter()
                .map(|i| serde_json::Value::String(i.clone()))
                .collect(),
        );
    }

    // Push chunk
    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "class".to_string(),
        signature,
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: Some(metadata),
    });

    // Recurse into class body
    if let Some(body) = node.child_by_field_name("body") {
        walk_java_decls(source, body, chunks, imports);
    }
}

/// Extract interface declaration
fn extract_java_interface(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("UnknownInterface");

    // Extract extended interfaces
    let extends = extract_extends_interfaces(source, node);

    // Extract Javadoc
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers
    let modifiers = extract_java_modifiers(source, node);

    // Build signature: "extends Interface1, Interface2"
    let signature = if extends.is_empty() {
        None
    } else {
        Some(format!("extends {}", extends.join(", ")))
    };

    // Build metadata
    let mut metadata = serde_json::json!({
        "visibility": modifiers.visibility,
        "modifiers": modifiers.modifiers,
        "annotations": modifiers.annotations,
    });
    if !extends.is_empty() {
        metadata["extends"] = serde_json::Value::Array(
            extends
                .iter()
                .map(|i| serde_json::Value::String(i.clone()))
                .collect(),
        );
    }

    // Push chunk
    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "interface".to_string(),
        signature,
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: Some(metadata),
    });

    // Recurse into interface body
    if let Some(body) = node.child_by_field_name("body") {
        walk_java_decls(source, body, chunks, imports);
    }
}

/// Extract enum declaration
fn extract_java_enum(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("UnknownEnum");

    // Extract interfaces
    let interfaces = extract_interfaces(source, node);

    // Extract Javadoc
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers
    let modifiers = extract_java_modifiers(source, node);

    // Build metadata
    let mut metadata = serde_json::json!({
        "visibility": modifiers.visibility,
        "modifiers": modifiers.modifiers,
        "annotations": modifiers.annotations,
    });
    if !interfaces.is_empty() {
        metadata["interfaces"] = serde_json::Value::Array(
            interfaces
                .iter()
                .map(|i| serde_json::Value::String(i.clone()))
                .collect(),
        );
    }

    // Push chunk
    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "enum".to_string(),
        signature: None,
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: Some(metadata),
    });

    // Recurse into enum body
    if let Some(body) = node.child_by_field_name("body") {
        walk_java_decls(source, body, chunks, imports);
    }
}

/// Extract record declaration
fn extract_java_record(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("UnknownRecord");

    // Extract parameters (record components)
    let parameters = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract interfaces
    let interfaces = extract_interfaces(source, node);

    // Extract Javadoc
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers
    let modifiers = extract_java_modifiers(source, node);

    // Build metadata
    let mut metadata = serde_json::json!({
        "visibility": modifiers.visibility,
        "modifiers": modifiers.modifiers,
        "annotations": modifiers.annotations,
        "is_record": true,
    });
    if !interfaces.is_empty() {
        metadata["interfaces"] = serde_json::Value::Array(
            interfaces
                .iter()
                .map(|i| serde_json::Value::String(i.clone()))
                .collect(),
        );
    }

    // Push chunk (kind is "class" with is_record=true metadata)
    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "class".to_string(),
        signature: parameters,
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: Some(metadata),
    });

    // Recurse into record body
    if let Some(body) = node.child_by_field_name("body") {
        walk_java_decls(source, body, chunks, imports);
    }
}

/// Extract annotation type declaration
fn extract_java_annotation_type(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("UnknownAnnotation");

    // Extract Javadoc
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers
    let modifiers = extract_java_modifiers(source, node);

    // Build metadata
    let metadata = serde_json::json!({
        "visibility": modifiers.visibility,
        "modifiers": modifiers.modifiers,
    });

    // Push chunk
    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "annotation".to_string(),
        signature: None,
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: Some(metadata),
    });
}

/// Extract method declaration
fn extract_java_method(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract method name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("unknownMethod");

    // Extract return type
    let return_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("void");

    // Extract parameters
    let params = extract_java_parameters(source, node);

    // Extract throws clause (optional)
    let throws = extract_java_throws(source, node);

    // Extract Javadoc
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers
    let modifiers = extract_java_modifiers(source, node);

    // Build signature: "(Type1 param1, Type2 param2) -> ReturnType"
    let signature = if params.is_empty() {
        format!("() -> {}", return_type)
    } else {
        format!("({}) -> {}", params.join(", "), return_type)
    };

    // Build metadata
    let mut metadata = serde_json::json!({
        "visibility": modifiers.visibility,
        "modifiers": modifiers.modifiers,
        "annotations": modifiers.annotations,
        "is_static": modifiers.modifiers.contains(&"static".to_string()),
    });
    if let Some(throws_clause) = throws {
        metadata["throws"] = serde_json::Value::String(throws_clause);
    }

    // Determine kind: "method" (inside class) or "func" (top-level, rare)
    // For Java, always use "method" since top-level methods don't exist
    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "method".to_string(),
        signature: Some(signature),
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: Some(metadata),
    });
}

/// Extract constructor declaration
fn extract_java_constructor(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract constructor name (same as class name)
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("UnknownConstructor");

    // Extract parameters
    let params = extract_java_parameters(source, node);

    // Extract throws clause
    let throws = extract_java_throws(source, node);

    // Extract Javadoc
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers
    let modifiers = extract_java_modifiers(source, node);

    // Build signature: parameter list
    let signature = if params.is_empty() {
        "()".to_string()
    } else {
        format!("({})", params.join(", "))
    };

    // Build metadata
    let mut metadata = serde_json::json!({
        "visibility": modifiers.visibility,
        "modifiers": modifiers.modifiers,
        "annotations": modifiers.annotations,
    });
    if let Some(throws_clause) = throws {
        metadata["throws"] = serde_json::Value::String(throws_clause);
    }

    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "constructor".to_string(),
        signature: Some(signature),
        docstring,
        start_line: (node.start_position().row + 1) as i32,
        end_line: (node.end_position().row + 1) as i32,
        metadata: Some(metadata),
    });
}

/// Extract field declaration (handles multi-declarator fields like `int x, y;`)
fn extract_java_field(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract field type (applies to all declarators)
    let field_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("unknown");

    // Extract Javadoc (once for the entire field declaration)
    let docstring = extract_java_doc_comment(source, node);

    // Extract modifiers (once for the entire field declaration)
    let modifiers = extract_java_modifiers(source, node);

    // Iterate over declarators (e.g., "int x = 1, y = 2;" has two declarators)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            // Extract variable name
            let var_name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or("unknownField");

            // Extract initial value (optional)
            let initial_value = child
                .child_by_field_name("value")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok());

            // Build metadata
            let mut metadata = serde_json::json!({
                "visibility": modifiers.visibility.clone(),
                "modifiers": modifiers.modifiers.clone(),
                "annotations": modifiers.annotations.clone(),
                "type": field_type,
            });
            if let Some(init_val) = initial_value {
                metadata["initial_value"] = serde_json::Value::String(init_val.to_string());
            }

            // Push chunk for this variable
            chunks.push(SymbolChunk {
                symbol_name: Some(var_name.to_string()),
                kind: "field".to_string(),
                signature: Some(field_type.to_string()),
                docstring: docstring.clone(),
                start_line: (child.start_position().row + 1) as i32,
                end_line: (child.end_position().row + 1) as i32,
                metadata: Some(metadata),
            });
        }
    }
}

/// Extract parameters from method or constructor
fn extract_java_parameters(source: &str, node: Node) -> Vec<String> {
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
                // Extract "Type name" from parameter
                if let Ok(param_text) = child.utf8_text(source.as_bytes()) {
                    params.push(param_text.to_string());
                }
            }
        }
    }
    params
}

/// Extract throws clause from method or constructor
fn extract_java_throws(source: &str, node: Node) -> Option<String> {
    // Try finding by field name first
    if let Some(throws_node) = node.child_by_field_name("throws") {
        if let Ok(text) = throws_node.utf8_text(source.as_bytes()) {
            // Strip "throws " prefix if present
            let cleaned = text.strip_prefix("throws ").unwrap_or(text).trim();
            return Some(cleaned.to_string());
        }
    }

    // Try finding by child node kind
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "throws" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                // Strip "throws " keyword if present
                let cleaned = text.strip_prefix("throws ").unwrap_or(text).trim();
                return Some(cleaned.to_string());
            }
        }
    }

    None
}

/// Extract interfaces from "implements" clause
fn extract_interfaces(source: &str, node: Node) -> Vec<String> {
    let mut interfaces = Vec::new();

    // The field name is "interfaces" for class implements clause
    if let Some(interfaces_node) = node.child_by_field_name("interfaces") {
        // Look for type_list child
        let mut cursor = interfaces_node.walk();
        for child in interfaces_node.children(&mut cursor) {
            if child.kind() == "type_list" {
                // Extract all type identifiers from type_list
                let mut type_cursor = child.walk();
                for type_child in child.children(&mut type_cursor) {
                    if type_child.kind() == "type_identifier" || type_child.kind() == "generic_type"
                    {
                        if let Ok(interface_text) = type_child.utf8_text(source.as_bytes()) {
                            interfaces.push(interface_text.to_string());
                        }
                    }
                }
            }
        }
    }
    interfaces
}

/// Extract extended interfaces from "extends" clause (for interfaces)
fn extract_extends_interfaces(source: &str, node: Node) -> Vec<String> {
    let mut extends = Vec::new();

    // Find extends_interfaces child by kind (not by field name)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "extends_interfaces" {
            // The extends_interfaces text contains "extends InterfaceName"
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                // Strip "extends " prefix and parse interfaces
                let cleaned = text.strip_prefix("extends ").unwrap_or(text).trim();
                // Split by comma for multiple interfaces
                for interface in cleaned.split(',') {
                    extends.push(interface.trim().to_string());
                }
            }
            break;
        }
    }

    extends
}

/// Extract Javadoc comment (/** ... */) preceding a declaration
fn extract_java_doc_comment(source: &str, node: Node) -> Option<String> {
    // Walk backward through siblings to find preceding comment
    let mut current = node.prev_sibling();
    while let Some(sibling) = current {
        if sibling.kind() == "block_comment" {
            let text = sibling.utf8_text(source.as_bytes()).ok()?;
            // Check if it's Javadoc (starts with /** but not /**)
            if text.starts_with("/**") && !text.starts_with("/***") {
                // Strip /** and */ delimiters
                let mut lines: Vec<&str> = text.lines().collect();
                if lines.is_empty() {
                    return None;
                }
                // Remove first line (/**)
                lines.remove(0);
                // Remove last line if it's just */
                if lines.last().map(|l| l.trim()) == Some("*/") {
                    lines.pop();
                }
                // Strip leading * from each line
                let cleaned: Vec<String> = lines
                    .iter()
                    .map(|line| {
                        let trimmed = line.trim_start();
                        if trimmed.starts_with('*') {
                            trimmed[1..].trim_start().to_string()
                        } else {
                            trimmed.to_string()
                        }
                    })
                    .collect();
                let result = cleaned.join("\n").trim().to_string();
                return if result.is_empty() {
                    None
                } else {
                    Some(result)
                };
            }
            break; // Found non-Javadoc comment, stop
        } else if sibling.kind() != "line_comment" && !sibling.kind().contains("comment") {
            break; // Found non-comment node, stop
        }
        current = sibling.prev_sibling();
    }
    None
}

/// Modifiers and annotations extracted from a Java declaration node.
///
/// Captures the access level, modifier keywords (e.g., `static`, `final`,
/// `abstract`, `synchronized`), and any annotations applied to a class,
/// method, field, or other declaration. When no explicit access modifier is
/// present, `visibility` defaults to `"package-private"`.
struct JavaModifiers {
    /// Access modifier: `"public"`, `"protected"`, `"private"`, or
    /// `"package-private"` (the default when no modifier is specified).
    visibility: String,
    /// All modifier keywords found on the declaration, including access
    /// modifiers as well as `static`, `final`, `abstract`, `synchronized`,
    /// `native`, `strictfp`, `transient`, `volatile`, and `default`.
    modifiers: Vec<String>,
    /// Annotation strings (e.g., `"@Override"`, `"@Deprecated"`) applied to
    /// the declaration, preserving the source text of each annotation.
    annotations: Vec<String>,
}

/// Extract modifiers and annotations from a declaration node
fn extract_java_modifiers(source: &str, node: Node) -> JavaModifiers {
    let mut visibility = "package-private".to_string();
    let mut modifiers = Vec::new();
    let mut annotations = Vec::new();

    // Find modifiers child by kind (not by field name)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            // Parse modifier keywords from space-separated text
            if let Ok(mods_text) = child.utf8_text(source.as_bytes()) {
                for word in mods_text.split_whitespace() {
                    match word {
                        "public" => {
                            visibility = "public".to_string();
                            modifiers.push("public".to_string());
                        }
                        "private" => {
                            visibility = "private".to_string();
                            modifiers.push("private".to_string());
                        }
                        "protected" => {
                            visibility = "protected".to_string();
                            modifiers.push("protected".to_string());
                        }
                        "static" | "final" | "abstract" | "synchronized" | "native"
                        | "strictfp" | "transient" | "volatile" | "default" => {
                            modifiers.push(word.to_string());
                        }
                        // Inline annotations like @Override appear as words in modifiers text
                        _ if word.starts_with('@') => {
                            annotations.push(word.to_string());
                        }
                        _ => {}
                    }
                }
            }

            // Also check for annotation children (handles complex annotations with arguments)
            let mut mod_cursor = child.walk();
            for mod_child in child.children(&mut mod_cursor) {
                if mod_child.kind() == "marker_annotation" || mod_child.kind() == "annotation" {
                    if let Ok(annotation_text) = mod_child.utf8_text(source.as_bytes()) {
                        // Avoid duplicates from both text and node traversal
                        if !annotations.contains(&annotation_text.to_string()) {
                            annotations.push(annotation_text.to_string());
                        }
                    }
                }
            }
            break;
        }
    }

    // Java defaults to package-private visibility when no modifier is specified
    JavaModifiers {
        visibility,
        modifiers,
        annotations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_modifiers_ast_debug() {
        let source = "public class Test {}";
        let mut parser = Parser::new();
        parser.set_language(&lang_java()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        eprintln!("Root kind: {}", root.kind());

        // Find class_declaration
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            eprintln!("Top-level child: kind={}", child.kind());
            if child.kind() == "class_declaration" {
                eprintln!("Found class_declaration");
                eprintln!("Class children:");
                let mut class_cursor = child.walk();
                for class_child in child.children(&mut class_cursor) {
                    eprintln!(
                        "  Child: kind={}, text={:?}",
                        class_child.kind(),
                        class_child.utf8_text(source.as_bytes()).ok()
                    );
                }

                if let Some(modifiers) = child.child_by_field_name("modifiers") {
                    eprintln!("\n=== modifiers node ===");
                    eprintln!("  Kind: {}", modifiers.kind());
                    eprintln!("  Child count: {}", modifiers.child_count());
                    eprintln!("  Text: {:?}", modifiers.utf8_text(source.as_bytes()).ok());

                    eprintln!("  All children:");
                    let mut mod_cursor = modifiers.walk();
                    for mod_child in modifiers.children(&mut mod_cursor) {
                        eprintln!(
                            "    Child: kind='{}', is_named={}, text='{:?}'",
                            mod_child.kind(),
                            mod_child.is_named(),
                            mod_child.utf8_text(source.as_bytes()).ok()
                        );
                    }
                } else {
                    eprintln!("\nNo modifiers field found!");
                }
            }
        }
    }

    #[test]
    fn test_java_ast_debug() {
        let source = r#"
public class UserService extends BaseService implements Serializable {
}
"#;
        let mut parser = Parser::new();
        parser.set_language(&lang_java()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find class_declaration
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "class_declaration" {
                eprintln!("\n=== class_declaration node ===");
                eprintln!("Node kind: {}", child.kind());
                eprintln!("Node text: {:?}", child.utf8_text(source.as_bytes()).ok());

                // Print all named children
                let mut class_cursor = child.walk();
                for class_child in child.named_children(&mut class_cursor) {
                    eprintln!(
                        "  Child: kind='{}', text='{:?}'",
                        class_child.kind(),
                        class_child.utf8_text(source.as_bytes()).ok().and_then(|s| {
                            if s.len() < 50 {
                                Some(s)
                            } else {
                                None
                            }
                        })
                    );
                }

                // Try specific fields
                if let Some(modifiers) = child.child_by_field_name("modifiers") {
                    eprintln!("\n  === modifiers node ===");
                    eprintln!(
                        "    Modifiers text: {:?}",
                        modifiers.utf8_text(source.as_bytes()).ok()
                    );
                    eprintln!("    Modifiers kind: {}", modifiers.kind());
                    eprintln!("    Modifiers child count: {}", modifiers.child_count());
                    let mut mod_cursor = modifiers.walk();
                    for mod_child in modifiers.children(&mut mod_cursor) {
                        eprintln!(
                            "    Modifier child: kind='{}', text='{:?}'",
                            mod_child.kind(),
                            mod_child.utf8_text(source.as_bytes()).ok()
                        );
                    }
                }

                if let Some(superclass) = child.child_by_field_name("superclass") {
                    eprintln!("\n  === superclass node ===");
                    eprintln!("    Kind: {}", superclass.kind());
                    eprintln!(
                        "    Text: {:?}",
                        superclass.utf8_text(source.as_bytes()).ok()
                    );
                }

                if let Some(interfaces) = child.child_by_field_name("interfaces") {
                    eprintln!("\n  === interfaces node ===");
                    eprintln!("    Kind: {}", interfaces.kind());
                    eprintln!(
                        "    Text: {:?}",
                        interfaces.utf8_text(source.as_bytes()).ok()
                    );
                    let mut int_cursor = interfaces.walk();
                    for int_child in interfaces.children(&mut int_cursor) {
                        eprintln!(
                            "    Interface child: kind='{}', text='{:?}'",
                            int_child.kind(),
                            int_child.utf8_text(source.as_bytes()).ok()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_java_import_collection() {
        let source = r#"
import java.util.List;
import java.util.*;
import static java.lang.Math.PI;
import static java.lang.System.*;

public class TestImports {
    public void testMethod() {
        System.out.println("Hello");
    }
}
"#;

        let chunks = extract_java_chunks(source);

        // Find the __imports__ chunk
        let imports_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_deref() == Some("__imports__"));

        assert!(imports_chunk.is_some(), "__imports__ chunk should exist");

        let imports_chunk = imports_chunk.unwrap();
        assert_eq!(imports_chunk.kind, "imports");
        assert_eq!(imports_chunk.start_line, 1);
        assert_eq!(imports_chunk.end_line, 1);

        // Verify metadata contains array of imports
        let metadata = imports_chunk.metadata.as_ref().unwrap();
        let imports_array = metadata.as_array().unwrap();
        assert_eq!(imports_array.len(), 4, "Should have 4 imports");

        // Check regular import
        let import1 = &imports_array[0];
        assert_eq!(import1["path"], "java.util.List");
        assert_eq!(import1["is_static"], false);
        assert_eq!(import1["is_wildcard"], false);

        // Check wildcard import
        let import2 = &imports_array[1];
        assert_eq!(import2["path"], "java.util.*");
        assert_eq!(import2["is_static"], false);
        assert_eq!(import2["is_wildcard"], true);

        // Check static import
        let import3 = &imports_array[2];
        assert_eq!(import3["path"], "java.lang.Math.PI");
        assert_eq!(import3["is_static"], true);
        assert_eq!(import3["is_wildcard"], false);

        // Check static wildcard import
        let import4 = &imports_array[3];
        assert_eq!(import4["path"], "java.lang.System.*");
        assert_eq!(import4["is_static"], true);
        assert_eq!(import4["is_wildcard"], true);
    }

    #[test]
    fn test_java_no_imports() {
        let source = r#"
public class NoImports {
    public void method() {
        System.out.println("Hello");
    }
}
"#;

        let chunks = extract_java_chunks(source);

        // Should not have __imports__ chunk
        let imports_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_deref() == Some("__imports__"));

        assert!(
            imports_chunk.is_none(),
            "__imports__ chunk should not exist when no imports"
        );
    }

    #[test]
    fn test_java_class_with_methods() {
        let source = r#"
/**
 * A service for managing users.
 */
public class UserService extends BaseService implements Serializable {
    /**
     * Find a user by name.
     * @param name the user's name
     * @return the user object
     */
    public User findUser(String name) {
        return null;
    }

    private void deleteUser(int id) {
        // implementation
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Assert chunk count
        assert_eq!(chunks.len(), 3, "Expected 3 chunks: 1 class + 2 methods");

        // Find chunks by name
        let class_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "UserService")
            .expect("UserService class not found");
        let find_user_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "findUser")
            .expect("findUser method not found");
        let delete_user_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "deleteUser")
            .expect("deleteUser method not found");

        // Assert class chunk
        assert_eq!(class_chunk.kind, "class");
        assert!(class_chunk
            .signature
            .as_ref()
            .unwrap()
            .contains("extends BaseService"));
        assert!(class_chunk
            .signature
            .as_ref()
            .unwrap()
            .contains("implements Serializable"));
        assert!(class_chunk
            .docstring
            .as_ref()
            .unwrap()
            .contains("service for managing users"));

        // Assert class metadata
        let class_metadata = class_chunk.metadata.as_ref().unwrap();
        assert_eq!(class_metadata["visibility"], "public");
        assert_eq!(class_metadata["superclass"], "BaseService");
        assert_eq!(class_metadata["interfaces"][0], "Serializable");

        // Assert findUser method
        assert_eq!(find_user_method.kind, "method");
        assert!(find_user_method
            .signature
            .as_ref()
            .unwrap()
            .contains("String name"));
        assert!(find_user_method
            .signature
            .as_ref()
            .unwrap()
            .contains("User"));
        assert!(find_user_method
            .docstring
            .as_ref()
            .unwrap()
            .contains("@param name"));
        assert_eq!(
            find_user_method.metadata.as_ref().unwrap()["visibility"],
            "public"
        );

        // Assert deleteUser method
        assert_eq!(delete_user_method.kind, "method");
        assert_eq!(
            delete_user_method.metadata.as_ref().unwrap()["visibility"],
            "private"
        );
    }

    #[test]
    fn test_java_interface_definition() {
        let source = r#"
/**
 * Repository interface for data access.
 */
public interface Repository extends BaseRepository {
    User findById(int id);

    default void log(String message) {
        System.out.println(message);
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find interface chunk
        let interface_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Repository")
            .expect("Repository interface not found");

        assert_eq!(interface_chunk.kind, "interface");
        assert!(interface_chunk
            .signature
            .as_ref()
            .unwrap()
            .contains("extends BaseRepository"));
        assert!(interface_chunk
            .docstring
            .as_ref()
            .unwrap()
            .contains("Repository interface"));

        let metadata = interface_chunk.metadata.as_ref().unwrap();
        assert_eq!(metadata["visibility"], "public");
        assert_eq!(metadata["extends"][0], "BaseRepository");

        // Check for abstract and default methods
        let abstract_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "findById")
            .expect("findById method not found");
        let default_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "log")
            .expect("log method not found");

        assert_eq!(abstract_method.kind, "method");
        assert_eq!(default_method.kind, "method");
        assert!(default_method.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == "default"));
    }

    #[test]
    fn test_java_constructor() {
        let source = r#"
public class Connection {
    /**
     * Create a connection with default settings.
     */
    public Connection() {
        this("localhost", 8080);
    }

    /**
     * Create a connection with specific host and port.
     * @param host the hostname
     * @param port the port number
     * @throws ConnectionException if connection fails
     */
    public Connection(String host, int port) throws ConnectionException {
        // implementation
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find constructor chunks
        let constructors: Vec<_> = chunks.iter().filter(|c| c.kind == "constructor").collect();

        assert_eq!(constructors.len(), 2, "Expected 2 constructor overloads");

        // Find parameterless constructor
        let default_constructor = constructors
            .iter()
            .find(|c| c.signature.as_ref().unwrap() == "()")
            .expect("Default constructor not found");
        assert!(default_constructor
            .docstring
            .as_ref()
            .unwrap()
            .contains("default settings"));

        // Find parameterized constructor
        let param_constructor = constructors
            .iter()
            .find(|c| c.signature.as_ref().unwrap().contains("String host"))
            .expect("Parameterized constructor not found");
        assert!(param_constructor
            .signature
            .as_ref()
            .unwrap()
            .contains("int port"));
        assert!(param_constructor
            .docstring
            .as_ref()
            .unwrap()
            .contains("@param host"));

        // Check throws clause
        let metadata = param_constructor.metadata.as_ref().unwrap();
        assert!(metadata["throws"]
            .as_str()
            .unwrap()
            .contains("ConnectionException"));
    }

    #[test]
    fn test_java_enum_declaration() {
        let source = r#"
/**
 * HTTP status codes.
 */
public enum Status implements Comparable<Status> {
    OK, NOT_FOUND, ERROR;

    public String getMessage() {
        return name().toLowerCase();
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find enum chunk
        let enum_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Status")
            .expect("Status enum not found");

        assert_eq!(enum_chunk.kind, "enum");
        assert!(enum_chunk
            .docstring
            .as_ref()
            .unwrap()
            .contains("HTTP status"));

        let metadata = enum_chunk.metadata.as_ref().unwrap();
        assert_eq!(metadata["visibility"], "public");
        assert_eq!(metadata["interfaces"][0], "Comparable<Status>");

        // Check enum method
        let method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "getMessage")
            .expect("getMessage method not found");
        assert_eq!(method.kind, "method");
    }

    #[test]
    fn test_java_annotations() {
        let source = r#"
/**
 * Custom annotation for validation.
 */
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Validate {
    String value() default "";
}

public class Service {
    @Override
    @Validate("user-input")
    public void processData(String data) {
        // implementation
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find annotation type
        let annotation_type = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Validate")
            .expect("Validate annotation not found");

        assert_eq!(annotation_type.kind, "annotation");
        assert!(annotation_type
            .docstring
            .as_ref()
            .unwrap()
            .contains("Custom annotation"));
        assert_eq!(
            annotation_type.metadata.as_ref().unwrap()["visibility"],
            "public"
        );

        // Check applied annotations on method
        let method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "processData")
            .expect("processData method not found");

        let annotations = method.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(annotations
            .iter()
            .any(|a| a.as_str().unwrap() == "@Override"));
        assert!(annotations
            .iter()
            .any(|a| a.as_str().unwrap().contains("@Validate")));
    }

    #[test]
    fn test_java_field_declarations() {
        let source = r#"
public class Config {
    /**
     * Server hostname.
     */
    private String host = "localhost";

    private final int timeout = 5000;

    public static int x, y, z;
}
"#;
        let chunks = extract_java_chunks(source);

        // Find field chunks
        let fields: Vec<_> = chunks.iter().filter(|c| c.kind == "field").collect();
        assert!(
            fields.len() >= 5,
            "Expected at least 5 field chunks (host, timeout, x, y, z)"
        );

        // Check host field
        let host_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "host")
            .expect("host field not found");
        assert_eq!(host_field.signature.as_ref().unwrap(), "String");
        assert!(host_field
            .docstring
            .as_ref()
            .unwrap()
            .contains("Server hostname"));
        let host_metadata = host_field.metadata.as_ref().unwrap();
        assert_eq!(host_metadata["visibility"], "private");
        assert_eq!(host_metadata["initial_value"], "\"localhost\"");

        // Check timeout field
        let timeout_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "timeout")
            .expect("timeout field not found");
        let timeout_metadata = timeout_field.metadata.as_ref().unwrap();
        assert!(timeout_metadata["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == "final"));

        // Check multi-declarator fields (x, y, z)
        let x_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "x")
            .expect("x field not found");
        let y_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "y")
            .expect("y field not found");
        let z_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "z")
            .expect("z field not found");

        for field in [x_field, y_field, z_field] {
            assert_eq!(field.signature.as_ref().unwrap(), "int");
            let metadata = field.metadata.as_ref().unwrap();
            assert_eq!(metadata["visibility"], "public");
            assert!(metadata["modifiers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m == "static"));
        }
    }

    #[test]
    fn test_java_javadoc_comments() {
        let source = r#"
/**
 * Main service class.
 * This is a multi-line Javadoc.
 * @author John Doe
 * @version 1.0
 */
public class Service {
    /* This is a regular block comment, should NOT be captured */
    public void method1() {
        // regular line comment
    }

    /**
     * Method with Javadoc.
     * @param data the input data
     * @return the result
     * @throws Exception if processing fails
     */
    public String method2(String data) throws Exception {
        return data;
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find class with Javadoc
        let class_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Service")
            .expect("Service class not found");
        let docstring = class_chunk.docstring.as_ref().unwrap();
        assert!(docstring.contains("Main service class"));
        assert!(docstring.contains("@author John Doe"));
        assert!(docstring.contains("@version 1.0"));

        // Find method1 (should have NO docstring due to block comment)
        let method1 = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "method1")
            .expect("method1 not found");
        assert!(
            method1.docstring.is_none(),
            "Regular block comment should not be captured as Javadoc"
        );

        // Find method2 with Javadoc
        let method2 = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "method2")
            .expect("method2 not found");
        let method2_doc = method2.docstring.as_ref().unwrap();
        assert!(method2_doc.contains("Method with Javadoc"));
        assert!(method2_doc.contains("@param data"));
        assert!(method2_doc.contains("@return the result"));
        assert!(method2_doc.contains("@throws Exception"));
    }

    #[test]
    fn test_java_nested_classes() {
        let source = r#"
public class Outer {
    /**
     * Inner class.
     */
    public class Inner {
        public void innerMethod() {}
    }

    /**
     * Static nested class.
     */
    public static class StaticNested {
        public void staticMethod() {}
    }

    /**
     * Nested interface.
     */
    public interface NestedInterface {
        void interfaceMethod();
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find all class/interface chunks
        let outer_class = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Outer")
            .expect("Outer class not found");
        let inner_class = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Inner")
            .expect("Inner class not found");
        let static_nested = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "StaticNested")
            .expect("StaticNested class not found");
        let nested_interface = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "NestedInterface")
            .expect("NestedInterface not found");

        assert_eq!(outer_class.kind, "class");
        assert_eq!(inner_class.kind, "class");
        assert_eq!(static_nested.kind, "class");
        assert_eq!(nested_interface.kind, "interface");

        // Check static modifier on StaticNested
        let static_metadata = static_nested.metadata.as_ref().unwrap();
        assert!(static_metadata["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == "static"));

        // Check that nested classes have their own methods
        assert!(chunks
            .iter()
            .any(|c| c.symbol_name.as_ref().unwrap() == "innerMethod"));
        assert!(chunks
            .iter()
            .any(|c| c.symbol_name.as_ref().unwrap() == "staticMethod"));
        assert!(chunks
            .iter()
            .any(|c| c.symbol_name.as_ref().unwrap() == "interfaceMethod"));
    }

    #[test]
    fn test_java_modifiers() {
        let source = r#"
public abstract class AbstractService {
    public static final int MAX_CONNECTIONS = 100;

    protected volatile boolean flag;

    private transient String tempData;

    abstract void abstractMethod();

    public synchronized void syncMethod() {}

    // Package-private (no modifier)
    void packagePrivateMethod() {}
}
"#;
        let chunks = extract_java_chunks(source);

        // Check abstract class
        let class_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "AbstractService")
            .expect("AbstractService class not found");
        let class_modifiers = class_chunk.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(class_modifiers.iter().any(|m| m == "abstract"));

        // Check static final field
        let max_conn_field = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "MAX_CONNECTIONS")
            .expect("MAX_CONNECTIONS field not found");
        let max_conn_modifiers = max_conn_field.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(max_conn_modifiers.iter().any(|m| m == "static"));
        assert!(max_conn_modifiers.iter().any(|m| m == "final"));

        // Check volatile field
        let flag_field = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "flag")
            .expect("flag field not found");
        let flag_modifiers = flag_field.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(flag_modifiers.iter().any(|m| m == "volatile"));

        // Check transient field
        let temp_field = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "tempData")
            .expect("tempData field not found");
        let temp_modifiers = temp_field.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(temp_modifiers.iter().any(|m| m == "transient"));

        // Check abstract method
        let abstract_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "abstractMethod")
            .expect("abstractMethod not found");
        let abstract_modifiers = abstract_method.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(abstract_modifiers.iter().any(|m| m == "abstract"));

        // Check synchronized method
        let sync_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "syncMethod")
            .expect("syncMethod not found");
        let sync_modifiers = sync_method.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(sync_modifiers.iter().any(|m| m == "synchronized"));

        // Check package-private method
        let pkg_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "packagePrivateMethod")
            .expect("packagePrivateMethod not found");
        assert_eq!(
            pkg_method.metadata.as_ref().unwrap()["visibility"],
            "package-private"
        );
    }

    #[test]
    fn test_java_empty_file() {
        let source = "";
        let chunks = extract_java_chunks(source);
        assert_eq!(chunks.len(), 0, "Empty source should produce no chunks");
    }

    #[test]
    fn test_java_syntax_error() {
        let source = r#"
public class Broken { {{{{ }
"#;
        // Should not panic - tree-sitter is error-tolerant
        let chunks = extract_java_chunks(source);
        // May produce partial chunks or empty vector, both acceptable
        // The key requirement is no panic
        assert!(
            true,
            "Test passed - no panic on malformed source (chunks: {})",
            chunks.len()
        );
    }

    #[test]
    fn test_java_record_declaration() {
        let source = r#"
/**
 * A point in 2D space.
 */
public record Point(double x, double y) implements Serializable {
    public double distance() {
        return Math.sqrt(x * x + y * y);
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find record chunk
        let record_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Point")
            .expect("Point record not found");

        // Records are stored as class with is_record=true
        assert_eq!(record_chunk.kind, "class");
        assert!(record_chunk
            .signature
            .as_ref()
            .unwrap()
            .contains("double x"));
        assert!(record_chunk
            .signature
            .as_ref()
            .unwrap()
            .contains("double y"));
        assert!(record_chunk
            .docstring
            .as_ref()
            .unwrap()
            .contains("point in 2D"));

        let metadata = record_chunk.metadata.as_ref().unwrap();
        assert_eq!(metadata["is_record"], true);
        assert_eq!(metadata["interfaces"][0], "Serializable");

        // Check record method
        let method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "distance")
            .expect("distance method not found");
        assert_eq!(method.kind, "method");
    }

    #[test]
    fn test_java_generics() {
        let source = r#"
public class Container<T extends Comparable<T>> {
    private List<T> items;

    public void add(T item) {
        items.add(item);
    }

    public <U> U transform(Function<T, U> mapper) {
        return null;
    }
}
"#;
        let chunks = extract_java_chunks(source);

        // Find class with generic parameter
        let class_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Container")
            .expect("Container class not found");
        assert_eq!(class_chunk.kind, "class");

        // Check field with generic type
        let items_field = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "items")
            .expect("items field not found");
        assert_eq!(items_field.signature.as_ref().unwrap(), "List<T>");

        // Check method with generic parameter
        let add_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "add")
            .expect("add method not found");
        assert!(add_method.signature.as_ref().unwrap().contains("T item"));

        // Check method with generic return type
        let transform_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "transform")
            .expect("transform method not found");
        assert!(transform_method
            .signature
            .as_ref()
            .unwrap()
            .contains("Function<T, U>"));
    }

    #[test]
    fn test_java_real_world_spring_application() {
        // Comprehensive integration test covering realistic Java patterns:
        // - Spring annotations (@RestController, @RequestMapping, @Autowired, @GetMapping, @PostMapping, @PathVariable, @RequestBody)
        // - JPA/Hibernate annotations (@Entity, @Table, @Column, @Id, @GeneratedValue)
        // - Generic types (List<T>, Map<K,V>, Optional<T>)
        // - Lambda-friendly interfaces (Predicate)
        // - Builder pattern (nested static builder class)
        // - Exception handling (throws clauses, custom exceptions)
        // - Multiple nested classes (inner classes, builder)
        let source = r#"
package com.example.shop;

import org.springframework.web.bind.annotation.*;
import org.springframework.beans.factory.annotation.Autowired;
import javax.persistence.*;
import java.util.*;
import java.util.function.Predicate;

/**
 * REST controller for managing products in the shop.
 * Demonstrates Spring Boot best practices and common patterns.
 */
@RestController
@RequestMapping("/api/products")
public class ProductController {

    @Autowired
    private ProductService productService;

    @Autowired
    private ProductRepository productRepository;

    /**
     * Retrieve a product by ID.
     * @param id the product identifier
     * @return optional product if found
     */
    @GetMapping("/{id}")
    public Optional<Product> getProduct(@PathVariable Long id) {
        return productService.findById(id);
    }

    /**
     * Create a new product.
     * @param product the product data
     * @return created product with ID
     * @throws ValidationException if product data is invalid
     */
    @PostMapping
    public Product createProduct(@RequestBody Product product) throws ValidationException {
        if (product.getName() == null) {
            throw new ValidationException("Product name is required");
        }
        return productService.save(product);
    }

    /**
     * Search products by criteria.
     * @param criteria search criteria map
     * @return list of matching products
     */
    @PostMapping("/search")
    public List<Product> searchProducts(@RequestBody Map<String, Object> criteria) {
        Predicate<Product> filter = p -> {
            if (criteria.containsKey("minPrice")) {
                return p.getPrice() >= (Double) criteria.get("minPrice");
            }
            return true;
        };
        return productRepository.findAll().stream()
            .filter(filter)
            .toList();
    }

    /**
     * Custom exception for validation errors.
     */
    public static class ValidationException extends Exception {
        public ValidationException(String message) {
            super(message);
        }
    }
}

/**
 * JPA entity representing a product.
 */
@Entity
@Table(name = "products")
class Product {

    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    @Column(nullable = false, length = 255)
    private String name;

    @Column(nullable = false)
    private Double price;

    @Column(length = 500)
    private String description;

    /**
     * Default constructor for JPA.
     */
    public Product() {
    }

    /**
     * Private constructor for builder.
     */
    private Product(Builder builder) {
        this.name = builder.name;
        this.price = builder.price;
        this.description = builder.description;
    }

    // Getters and setters

    public Long getId() {
        return id;
    }

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }

    public Double getPrice() {
        return price;
    }

    public void setPrice(Double price) {
        this.price = price;
    }

    public String getDescription() {
        return description;
    }

    public void setDescription(String description) {
        this.description = description;
    }

    /**
     * Builder for fluent product creation.
     */
    public static class Builder {
        private String name;
        private Double price;
        private String description;

        public Builder withName(String name) {
            this.name = name;
            return this;
        }

        public Builder withPrice(Double price) {
            this.price = price;
            return this;
        }

        public Builder withDescription(String description) {
            this.description = description;
            return this;
        }

        /**
         * Build the product instance.
         * @return new product
         * @throws IllegalStateException if required fields are missing
         */
        public Product build() throws IllegalStateException {
            if (name == null || price == null) {
                throw new IllegalStateException("Name and price are required");
            }
            return new Product(this);
        }
    }
}

/**
 * Service interface for product operations.
 */
interface ProductService {
    Optional<Product> findById(Long id);
    Product save(Product product);
    List<Product> findAll();
}

/**
 * Repository interface for product data access.
 */
interface ProductRepository {
    List<Product> findAll();
    Optional<Product> findById(Long id);
}
"#;

        let chunks = extract_java_chunks(source);

        // Verify total chunk count (rough estimate)
        // Expected: ProductController class, Product class, ValidationException nested class, Builder nested class,
        //           ProductService interface, ProductRepository interface,
        //           Multiple methods across all types, multiple fields
        assert!(
            chunks.len() >= 30,
            "Expected at least 30 chunks from realistic Spring application, got {}",
            chunks.len()
        );

        // Verify class chunks
        let controller_class = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "ProductController")
            .expect("ProductController class not found");
        assert_eq!(controller_class.kind, "class");
        assert!(controller_class
            .docstring
            .as_ref()
            .unwrap()
            .contains("REST controller"));

        let product_class = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Product")
            .expect("Product class not found");
        assert_eq!(product_class.kind, "class");

        // Verify Spring annotations on controller class
        let controller_annotations = controller_class.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(
            controller_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@RestController")),
            "Should have @RestController annotation"
        );
        assert!(
            controller_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@RequestMapping")),
            "Should have @RequestMapping annotation"
        );

        // Verify JPA annotations on entity class
        let product_annotations = product_class.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(
            product_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@Entity")),
            "Should have @Entity annotation"
        );
        assert!(
            product_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@Table")),
            "Should have @Table annotation"
        );

        // Verify method chunks with Spring annotations
        let get_product_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "getProduct")
            .expect("getProduct method not found");
        assert_eq!(get_product_method.kind, "method");
        assert!(get_product_method
            .signature
            .as_ref()
            .unwrap()
            .contains("Optional<Product>"));
        assert!(get_product_method
            .signature
            .as_ref()
            .unwrap()
            .contains("Long id"));

        let get_annotations = get_product_method.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(
            get_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@GetMapping")),
            "Should have @GetMapping annotation"
        );

        let create_product_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "createProduct")
            .expect("createProduct method not found");
        assert_eq!(create_product_method.kind, "method");
        let create_annotations = create_product_method.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(
            create_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@PostMapping")),
            "Should have @PostMapping annotation"
        );

        // Verify throws clause
        let create_throws = create_product_method.metadata.as_ref().unwrap()["throws"]
            .as_str()
            .unwrap();
        assert!(
            create_throws.contains("ValidationException"),
            "Should throw ValidationException"
        );

        // Verify generic types in method signature
        let search_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "searchProducts")
            .expect("searchProducts method not found");
        assert!(search_method
            .signature
            .as_ref()
            .unwrap()
            .contains("Map<String, Object>"));
        assert!(search_method
            .signature
            .as_ref()
            .unwrap()
            .contains("List<Product>"));

        // Verify field chunks with annotations
        let fields: Vec<_> = chunks.iter().filter(|c| c.kind == "field").collect();
        assert!(fields.len() >= 6, "Expected at least 6 field chunks");

        let service_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "productService")
            .expect("productService field not found");
        let service_annotations = service_field.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(
            service_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@Autowired")),
            "Should have @Autowired annotation"
        );

        let id_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "id")
            .expect("id field not found");
        let id_annotations = id_field.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(
            id_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@Id")),
            "Should have @Id annotation"
        );
        assert!(
            id_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@GeneratedValue")),
            "Should have @GeneratedValue annotation"
        );

        let name_field = fields
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "name")
            .expect("name field not found");
        let name_annotations = name_field.metadata.as_ref().unwrap()["annotations"]
            .as_array()
            .unwrap();
        assert!(
            name_annotations
                .iter()
                .any(|a| a.as_str().unwrap().contains("@Column")),
            "Should have @Column annotation"
        );

        // Verify nested classes
        let validation_exception = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "ValidationException")
            .expect("ValidationException nested class not found");
        assert_eq!(validation_exception.kind, "class");
        let validation_metadata = validation_exception.metadata.as_ref().unwrap();
        assert!(validation_metadata["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == "static"));

        let builder_class = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Builder")
            .expect("Builder nested class not found");
        assert_eq!(builder_class.kind, "class");
        let builder_metadata = builder_class.metadata.as_ref().unwrap();
        assert!(builder_metadata["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == "static"));

        // Verify builder methods (fluent API)
        let with_name_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "withName")
            .expect("withName builder method not found");
        assert_eq!(with_name_method.kind, "method");
        assert!(with_name_method
            .signature
            .as_ref()
            .unwrap()
            .contains("Builder"));

        let build_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "build")
            .expect("build method not found");
        assert_eq!(build_method.kind, "method");
        let build_throws = build_method.metadata.as_ref().unwrap()["throws"]
            .as_str()
            .unwrap();
        assert!(
            build_throws.contains("IllegalStateException"),
            "Should throw IllegalStateException"
        );

        // Verify interfaces
        let interfaces: Vec<_> = chunks.iter().filter(|c| c.kind == "interface").collect();
        assert_eq!(interfaces.len(), 2, "Expected 2 interface chunks");

        let service_interface = interfaces
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "ProductService")
            .expect("ProductService interface not found");
        assert_eq!(service_interface.kind, "interface");

        let repo_interface = interfaces
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "ProductRepository")
            .expect("ProductRepository interface not found");
        assert_eq!(repo_interface.kind, "interface");

        // Verify constructors
        let constructors: Vec<_> = chunks.iter().filter(|c| c.kind == "constructor").collect();
        assert!(
            constructors.len() >= 2,
            "Expected at least 2 constructor chunks"
        );

        let default_constructor = constructors
            .iter()
            .find(|c| {
                c.symbol_name.as_ref().unwrap() == "Product"
                    && c.signature.as_ref().unwrap() == "()"
            })
            .expect("Default constructor not found");
        assert!(default_constructor
            .docstring
            .as_ref()
            .unwrap()
            .contains("Default constructor"));

        let validation_constructor = constructors
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "ValidationException")
            .expect("ValidationException constructor not found");
        assert!(validation_constructor
            .signature
            .as_ref()
            .unwrap()
            .contains("String message"));

        // Verify imports chunk
        let imports_chunk = chunks
            .iter()
            .find(|c| c.symbol_name.as_deref() == Some("__imports__"))
            .expect("__imports__ chunk not found");
        assert_eq!(imports_chunk.kind, "imports");

        let imports_metadata = imports_chunk.metadata.as_ref().unwrap();
        let imports_array = imports_metadata.as_array().unwrap();
        assert!(
            imports_array.len() >= 3,
            "Expected at least 3 import declarations"
        );

        // Verify some key imports
        let has_spring_import = imports_array
            .iter()
            .any(|i| i["path"].as_str().unwrap().contains("springframework"));
        assert!(has_spring_import, "Should have Spring import");

        let has_jpa_import = imports_array
            .iter()
            .any(|i| i["path"].as_str().unwrap().contains("javax.persistence"));
        assert!(has_jpa_import, "Should have JPA import");
    }

    #[test]
    fn test_java_unicode_identifiers() {
        let source = r#"
package com.example;

public class 用户管理器 {
    private String 用户名;
    private int số_lượng;

    public void 获取用户(String prénom) {
        System.out.println(prénom);
    }

    public String get数据() {
        return "データ";
    }
}
"#;

        let chunks = extract_java_chunks(source);

        // Verify class chunk with Chinese characters
        let class_chunk = chunks
            .iter()
            .find(|c| c.kind == "class")
            .expect("Should extract class chunk");
        assert_eq!(class_chunk.symbol_name.as_ref().unwrap(), "用户管理器");

        // Verify field with Chinese characters
        let username_field = chunks
            .iter()
            .find(|c| c.kind == "field" && c.symbol_name.as_ref().unwrap() == "用户名")
            .expect("Should extract 用户名 field");
        assert_eq!(username_field.symbol_name.as_ref().unwrap(), "用户名");

        // Verify field with Vietnamese characters
        let count_field = chunks
            .iter()
            .find(|c| c.kind == "field" && c.symbol_name.as_ref().unwrap() == "số_lượng")
            .expect("Should extract số_lượng field");
        assert_eq!(count_field.symbol_name.as_ref().unwrap(), "số_lượng");

        // Verify method with Chinese characters
        let get_user_method = chunks
            .iter()
            .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "获取用户")
            .expect("Should extract 获取用户 method");
        assert_eq!(get_user_method.symbol_name.as_ref().unwrap(), "获取用户");
        assert!(get_user_method
            .signature
            .as_ref()
            .unwrap()
            .contains("prénom")); // French accented parameter

        // Verify method with mixed Japanese/Chinese
        let get_data_method = chunks
            .iter()
            .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "get数据")
            .expect("Should extract get数据 method");
        assert_eq!(get_data_method.symbol_name.as_ref().unwrap(), "get数据");
    }

    #[test]
    fn test_java_whitespace_only() {
        // Test empty string
        let chunks = extract_java_chunks("");
        assert_eq!(chunks.len(), 0, "Empty string should return no chunks");

        // Test spaces only
        let chunks = extract_java_chunks("    ");
        assert_eq!(chunks.len(), 0, "Spaces-only should return no chunks");

        // Test tabs only
        let chunks = extract_java_chunks("\t\t\t");
        assert_eq!(chunks.len(), 0, "Tabs-only should return no chunks");

        // Test newlines only
        let chunks = extract_java_chunks("\n\n\n");
        assert_eq!(chunks.len(), 0, "Newlines-only should return no chunks");

        // Test mixed whitespace
        let chunks = extract_java_chunks("  \n\t  \n  ");
        assert_eq!(chunks.len(), 0, "Mixed whitespace should return no chunks");

        // Test file with only comments (edge case)
        let chunks = extract_java_chunks("// Just a comment\n/* Block comment */");
        assert_eq!(chunks.len(), 0, "Comments-only should return no chunks");
    }

    #[test]
    fn test_java_multiple_top_level_classes() {
        // Test file with multiple top-level classes
        // (Discouraged in Java but legal)
        let source = r#"
package com.example;

// Main public class
public class MainClass {
    void mainMethod() {}
}

// Helper class (package-private)
class HelperClass {
    void helperMethod() {}
}

// Another helper class
class AnotherHelper {
    private int field;

    void anotherMethod() {}
}
"#;

        let chunks = extract_java_chunks(source);

        // Find all class chunks
        let class_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "class").collect();

        // Should extract all 3 top-level classes
        assert_eq!(
            class_chunks.len(),
            3,
            "Should extract all 3 top-level classes"
        );

        // Verify each class is present
        let class_names: Vec<&str> = class_chunks
            .iter()
            .map(|c| c.symbol_name.as_ref().unwrap().as_str())
            .collect();

        assert!(
            class_names.contains(&"MainClass"),
            "Should extract MainClass"
        );
        assert!(
            class_names.contains(&"HelperClass"),
            "Should extract HelperClass"
        );
        assert!(
            class_names.contains(&"AnotherHelper"),
            "Should extract AnotherHelper"
        );

        // Verify methods are also extracted
        let method_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "method").collect();

        assert!(
            method_chunks.len() >= 3,
            "Should extract methods from all classes"
        );
    }

    #[test]
    fn test_java_large_corpus_validation() {
        // Comprehensive test with diverse Java patterns
        let source = r#"
package com.example.test;

import java.util.*;
import java.util.function.*;
import java.io.*;

/**
 * Abstract base service with complex patterns.
 */
public abstract class AbstractDataService<T extends Comparable<T>> {
    // Static initializer
    static {
        System.setProperty("service.initialized", "true");
    }

    // Final class constants
    public static final int MAX_RETRIES = 3;
    private static final String DEFAULT_ENCODING = "UTF-8";

    // Generic field with wildcards
    private List<? extends Serializable> items;
    private Map<String, ? super Number> cache;

    /**
     * Abstract method for data processing.
     */
    public abstract T process(T data) throws IOException;

    /**
     * Synchronized method with varargs.
     */
    public synchronized void updateAll(T... elements) {
        for (T elem : elements) {
            cache.put(elem.toString(), elem);
        }
    }

    /**
     * Native method declaration.
     */
    public native int nativeCompute(long value);

    /**
     * Final class with static nested enum.
     */
    public static final class Configuration {
        private final String name;
        private final int timeout;

        public Configuration(String name, int timeout) {
            this.name = name;
            this.timeout = timeout;
        }
    }

    /**
     * Enum with abstract methods.
     */
    public enum Operation {
        ADD {
            @Override
            public double apply(double a, double b) {
                return a + b;
            }
        },
        SUBTRACT {
            @Override
            public double apply(double a, double b) {
                return a - b;
            }
        };

        public abstract double apply(double a, double b);
    }
}

/**
 * Interface with default methods and generic bounds.
 */
interface DataRepository<T extends Comparable<? super T>> {
    /**
     * Default method implementation.
     */
    default Optional<T> findFirst() {
        return findAll().stream().findFirst();
    }

    List<T> findAll();

    /**
     * Method with complex generic signature.
     */
    <U extends T> void save(U entity);
}

/**
 * Sealed interface (Java 17+).
 * Note: tree-sitter-java may not fully support sealed yet.
 */
interface Shape permits Circle, Rectangle {
    double area();
}

/**
 * Record with compact constructor (Java 17+).
 */
record Circle(double radius) implements Shape {
    // Compact constructor
    public Circle {
        if (radius <= 0) {
            throw new IllegalArgumentException("Radius must be positive");
        }
    }

    @Override
    public double area() {
        return Math.PI * radius * radius;
    }
}

/**
 * Record implementing sealed interface.
 */
record Rectangle(double width, double height) implements Shape {
    @Override
    public double area() {
        return width * height;
    }
}
"#;

        let chunks = extract_java_chunks(source);

        // Should extract a substantial number of chunks
        assert!(
            chunks.len() >= 20,
            "Large corpus should extract 20+ chunks, got {}",
            chunks.len()
        );

        // Verify abstract class
        let abstract_class = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "AbstractDataService")
            .expect("Should extract AbstractDataService");
        assert_eq!(abstract_class.kind, "class");
        let class_modifiers = abstract_class.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(class_modifiers.iter().any(|m| m == "abstract"));

        // Verify final nested class
        let config_class = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Configuration")
            .expect("Should extract Configuration class");
        let config_modifiers = config_class.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(config_modifiers.iter().any(|m| m == "final"));
        assert!(config_modifiers.iter().any(|m| m == "static"));

        // Verify enum
        let operation_enum = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Operation")
            .expect("Should extract Operation enum");
        assert_eq!(operation_enum.kind, "enum");

        // Verify synchronized method
        let update_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "updateAll")
            .expect("Should extract updateAll method");
        let update_modifiers = update_method.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(update_modifiers.iter().any(|m| m == "synchronized"));

        // Verify native method
        let native_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "nativeCompute")
            .expect("Should extract nativeCompute method");
        let native_modifiers = native_method.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(native_modifiers.iter().any(|m| m == "native"));

        // Verify interface with default method
        let repo_interface = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "DataRepository")
            .expect("Should extract DataRepository interface");
        assert_eq!(repo_interface.kind, "interface");

        let default_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "findFirst")
            .expect("Should extract findFirst default method");
        let default_modifiers = default_method.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(default_modifiers.iter().any(|m| m == "default"));

        // Verify records
        let circle_record = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Circle")
            .expect("Should extract Circle record");
        assert_eq!(circle_record.kind, "class");
        assert_eq!(circle_record.metadata.as_ref().unwrap()["is_record"], true);

        let rectangle_record = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "Rectangle")
            .expect("Should extract Rectangle record");
        assert_eq!(rectangle_record.kind, "class");
        assert_eq!(
            rectangle_record.metadata.as_ref().unwrap()["is_record"],
            true
        );

        // Verify static constants
        let max_retries = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "MAX_RETRIES")
            .expect("Should extract MAX_RETRIES field");
        let retries_modifiers = max_retries.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(retries_modifiers.iter().any(|m| m == "static"));
        assert!(retries_modifiers.iter().any(|m| m == "final"));

        // Verify abstract method
        let process_method = chunks
            .iter()
            .find(|c| c.symbol_name.as_ref().unwrap() == "process")
            .expect("Should extract process method");
        let process_modifiers = process_method.metadata.as_ref().unwrap()["modifiers"]
            .as_array()
            .unwrap();
        assert!(process_modifiers.iter().any(|m| m == "abstract"));

        // Verify throws clause
        assert!(process_method.metadata.as_ref().unwrap()["throws"]
            .as_str()
            .unwrap()
            .contains("IOException"));
    }

    #[test]
    fn test_java_parser_performance_benchmark() {
        // Simple benchmark test (not using criterion, just timing)
        use std::time::Instant;

        // Moderate-sized Java source (realistic Spring controller)
        let source = r#"
package com.example.controller;

import org.springframework.web.bind.annotation.*;
import org.springframework.beans.factory.annotation.Autowired;
import java.util.*;

@RestController
@RequestMapping("/api/users")
public class UserController {
    @Autowired
    private UserService userService;

    @GetMapping
    public List<User> getAllUsers() {
        return userService.findAll();
    }

    @GetMapping("/{id}")
    public Optional<User> getUser(@PathVariable Long id) {
        return userService.findById(id);
    }

    @PostMapping
    public User createUser(@RequestBody User user) {
        return userService.save(user);
    }

    @PutMapping("/{id}")
    public User updateUser(@PathVariable Long id, @RequestBody User user) {
        return userService.update(id, user);
    }

    @DeleteMapping("/{id}")
    public void deleteUser(@PathVariable Long id) {
        userService.delete(id);
    }
}

class User {
    private Long id;
    private String name;
    private String email;

    public User() {}

    public User(String name, String email) {
        this.name = name;
        this.email = email;
    }

    public Long getId() { return id; }
    public void setId(Long id) { this.id = id; }
    public String getName() { return name; }
    public void setName(String name) { this.name = name; }
    public String getEmail() { return email; }
    public void setEmail(String email) { this.email = email; }
}

interface UserService {
    List<User> findAll();
    Optional<User> findById(Long id);
    User save(User user);
    User update(Long id, User user);
    void delete(Long id);
}
"#;

        // Warm up
        for _ in 0..5 {
            let _ = extract_java_chunks(source);
        }

        // Benchmark
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            let chunks = extract_java_chunks(source);
            assert!(!chunks.is_empty());
        }
        let elapsed = start.elapsed();

        let avg_micros = elapsed.as_micros() / iterations;
        println!(
            "Java parser benchmark: {} iterations in {:?} (avg: {} µs per parse)",
            iterations, elapsed, avg_micros
        );

        // Sanity check - should complete in reasonable time
        assert!(
            avg_micros < 10000,
            "Parser too slow: {} µs per iteration",
            avg_micros
        );

        // Verify chunks extracted correctly
        let chunks = extract_java_chunks(source);
        assert!(
            chunks.len() >= 15,
            "Should extract at least 15 chunks from benchmark source"
        );
    }
}
