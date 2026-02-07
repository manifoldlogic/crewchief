//! Java parser

use tree_sitter::{Node, Parser};

use super::common::lang_java;
use crate::indexer::SymbolChunk;

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

    // Extract superclass
    let superclass = node
        .child_by_field_name("superclass")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

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
    node.child_by_field_name("throws")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
}

/// Extract interfaces from "implements" clause
fn extract_interfaces(source: &str, node: Node) -> Vec<String> {
    let mut interfaces = Vec::new();
    if let Some(interfaces_node) = node.child_by_field_name("interfaces") {
        let mut cursor = interfaces_node.walk();
        for child in interfaces_node.children(&mut cursor) {
            if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                if let Ok(interface_text) = child.utf8_text(source.as_bytes()) {
                    interfaces.push(interface_text.to_string());
                }
            }
        }
    }
    interfaces
}

/// Extract extended interfaces from "extends" clause (for interfaces)
fn extract_extends_interfaces(source: &str, node: Node) -> Vec<String> {
    let mut extends = Vec::new();
    if let Some(extends_node) = node.child_by_field_name("interfaces") {
        let mut cursor = extends_node.walk();
        for child in extends_node.children(&mut cursor) {
            if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                if let Ok(interface_text) = child.utf8_text(source.as_bytes()) {
                    extends.push(interface_text.to_string());
                }
            }
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

/// Modifiers extracted from a Java declaration
struct JavaModifiers {
    visibility: String,       // "public", "private", "protected", "package-private"
    modifiers: Vec<String>,   // All modifier keywords
    annotations: Vec<String>, // Annotation strings
}

/// Extract modifiers and annotations from a declaration node
fn extract_java_modifiers(source: &str, node: Node) -> JavaModifiers {
    let mut visibility = "package-private".to_string();
    let mut modifiers = Vec::new();
    let mut annotations = Vec::new();

    // Find modifiers child
    if let Some(mods_node) = node.child_by_field_name("modifiers") {
        let mut cursor = mods_node.walk();
        for child in mods_node.children(&mut cursor) {
            match child.kind() {
                "public" | "private" | "protected" | "static" | "final" | "abstract"
                | "synchronized" | "native" | "strictfp" | "transient" | "volatile" => {
                    if let Ok(modifier) = child.utf8_text(source.as_bytes()) {
                        if modifier == "public" || modifier == "private" || modifier == "protected"
                        {
                            visibility = modifier.to_string();
                        }
                        modifiers.push(modifier.to_string());
                    }
                }
                "marker_annotation" | "annotation" => {
                    if let Ok(annotation_text) = child.utf8_text(source.as_bytes()) {
                        annotations.push(annotation_text.to_string());
                    }
                }
                _ => {}
            }
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
}
