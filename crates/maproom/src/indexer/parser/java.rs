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
            "class_declaration" => extract_java_class(source, child, chunks, imports),
            "interface_declaration" => extract_java_interface(source, child, chunks, imports),
            "enum_declaration" => extract_java_enum(source, child, chunks, imports),
            "record_declaration" => extract_java_record(source, child, chunks, imports),
            "annotation_type_declaration" => extract_java_annotation_type(source, child, chunks),
            "method_declaration" => extract_java_method(source, child, chunks),
            "constructor_declaration" => extract_java_constructor(source, child, chunks),
            "field_declaration" => extract_java_field(source, child, chunks),
            "import_declaration" => collect_java_import(source, child, imports),
            _ => {
                // Recurse into other nodes
                walk_java_decls(source, child, chunks, imports);
            }
        }
    }
}

/// Collect import declaration metadata
fn collect_java_import(source: &str, node: Node, imports: &mut Vec<serde_json::Value>) {
    // Determine if static import
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
            // The modifiers node's text contains space-separated modifiers
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
                        _ if word.starts_with('@') => {
                            annotations.push(word.to_string());
                        }
                        _ => {}
                    }
                }
            }

            // Also check for annotation children
            let mut mod_cursor = child.walk();
            for mod_child in child.children(&mut mod_cursor) {
                if mod_child.kind() == "marker_annotation" || mod_child.kind() == "annotation" {
                    if let Ok(annotation_text) = mod_child.utf8_text(source.as_bytes()) {
                        if !annotations.contains(&annotation_text.to_string()) {
                            annotations.push(annotation_text.to_string());
                        }
                    }
                }
            }
            break;
        }
    }

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
}
