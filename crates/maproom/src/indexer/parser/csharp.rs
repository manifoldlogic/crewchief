//! C# parser

use tree_sitter::{Node, Parser};

use super::common::lang_csharp;
use crate::indexer::SymbolChunk;

pub(super) fn extract_csharp_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_csharp())
        .expect("Failed to set C# language");

    let tree = parser.parse(source, None);
    let mut chunks = Vec::new();

    if let Some(tree) = tree {
        let root = tree.root_node();
        let mut imports = Vec::new();
        walk_csharp_decls(source, root, &mut chunks, &mut imports);

        // Import aggregation will be added in task 2003
    }

    chunks
}

fn walk_csharp_decls(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    match node.kind() {
        "class_declaration" => extract_csharp_class(source, node, chunks, imports),
        "interface_declaration" => extract_csharp_interface(source, node, chunks, imports),
        "struct_declaration" => extract_csharp_struct(source, node, chunks, imports),
        "enum_declaration" => extract_csharp_enum(source, node, chunks),
        "delegate_declaration" => extract_csharp_delegate(source, node, chunks),
        "namespace_declaration" | "file_scoped_namespace_declaration" => {
            extract_csharp_namespace(source, node, chunks, imports)
        }
        "method_declaration" => extract_csharp_method(source, node, chunks),
        "constructor_declaration" => extract_csharp_constructor(source, node, chunks),
        "property_declaration" => extract_csharp_property(source, node, chunks),
        "event_declaration" => extract_csharp_event(source, node, chunks),
        "event_field_declaration" => extract_csharp_event_field(source, node, chunks),
        "using_directive" => collect_csharp_using(source, node, imports),
        _ => {
            // Recurse into children for unhandled node types
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk_csharp_decls(source, child, chunks, imports);
            }
        }
    }
}

/// Find first child node with the given kind (for nodes not registered as fields)
#[allow(clippy::manual_find)]
fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

fn extract_csharp_class(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Extract name from 'name' field
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility and modifiers
    let visibility = extract_csharp_visibility(node, source);
    let modifiers = extract_csharp_modifiers(node, source);

    // Extract generics from 'type_parameter_list'
    let generics = find_child_by_kind(node, "type_parameter_list")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract base types from 'base_list'
    let base_list =
        find_child_by_kind(node, "base_list").and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract doc comment
    let docstring = extract_csharp_doc_comment(source, node);

    // Build signature
    let mut signature = String::new();
    if let Some(generics_str) = generics {
        signature.push_str(generics_str);
    }
    if let Some(base_str) = base_list {
        if !signature.is_empty() {
            signature.push(' ');
        }
        signature.push_str(base_str);
    }

    // Build metadata
    let metadata = serde_json::json!({
        "visibility": visibility,
        "base_types": base_list,
        "is_abstract": modifiers.contains(&"abstract".to_string()),
        "is_static": modifiers.contains(&"static".to_string()),
        "is_partial": modifiers.contains(&"partial".to_string()),
    });

    // Push chunk
    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "class".to_string(),
            signature: if signature.is_empty() {
                None
            } else {
                Some(signature)
            },
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }

    // Recurse into body for nested members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            walk_csharp_decls(source, child, chunks, imports);
        }
    }
}

fn extract_csharp_interface(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Extract name from 'name' field
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility and modifiers
    let visibility = extract_csharp_visibility(node, source);
    let modifiers = extract_csharp_modifiers(node, source);

    // Extract generics from 'type_parameter_list'
    let generics = find_child_by_kind(node, "type_parameter_list")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract base interfaces from 'base_list'
    let base_list =
        find_child_by_kind(node, "base_list").and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract doc comment
    let docstring = extract_csharp_doc_comment(source, node);

    // Build signature
    let mut signature = String::new();
    if let Some(generics_str) = generics {
        signature.push_str(generics_str);
    }
    if let Some(base_str) = base_list {
        if !signature.is_empty() {
            signature.push(' ');
        }
        signature.push_str(base_str);
    }

    // Build metadata
    let metadata = serde_json::json!({
        "visibility": visibility,
        "base_types": base_list,
        "is_partial": modifiers.contains(&"partial".to_string()),
    });

    // Push chunk
    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "interface".to_string(),
            signature: if signature.is_empty() {
                None
            } else {
                Some(signature)
            },
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }

    // Recurse into body for nested members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            walk_csharp_decls(source, child, chunks, imports);
        }
    }
}

fn extract_csharp_struct(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Extract name from 'name' field
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility and modifiers
    let visibility = extract_csharp_visibility(node, source);
    let modifiers = extract_csharp_modifiers(node, source);

    // Extract generics from 'type_parameter_list'
    let generics = find_child_by_kind(node, "type_parameter_list")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract interfaces from 'base_list'
    let base_list =
        find_child_by_kind(node, "base_list").and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract doc comment
    let docstring = extract_csharp_doc_comment(source, node);

    // Build signature
    let mut signature = String::new();
    if let Some(generics_str) = generics {
        signature.push_str(generics_str);
    }
    if let Some(base_str) = base_list {
        if !signature.is_empty() {
            signature.push(' ');
        }
        signature.push_str(base_str);
    }

    // Build metadata
    let metadata = serde_json::json!({
        "visibility": visibility,
        "base_types": base_list,
        "is_static": modifiers.contains(&"static".to_string()),
        "is_partial": modifiers.contains(&"partial".to_string()),
    });

    // Push chunk
    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "struct".to_string(),
            signature: if signature.is_empty() {
                None
            } else {
                Some(signature)
            },
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }

    // Recurse into body for nested members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            walk_csharp_decls(source, child, chunks, imports);
        }
    }
}

fn extract_csharp_enum(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let visibility = extract_csharp_visibility(node, source);

    // Extract underlying type from base_list (e.g., ": byte")
    let base_list =
        find_child_by_kind(node, "base_list").and_then(|n| n.utf8_text(source.as_bytes()).ok());

    let docstring = extract_csharp_doc_comment(source, node);

    let metadata = serde_json::json!({
        "visibility": visibility,
    });

    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "enum".to_string(),
            signature: base_list.map(|s| s.to_string()),
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }

    // Do NOT recurse into enum body (members are not standalone symbols)
}

fn extract_csharp_delegate(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let visibility = extract_csharp_visibility(node, source);

    // Extract return type
    let return_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract parameters
    let parameters = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract generics
    let generics = find_child_by_kind(node, "type_parameter_list")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    let docstring = extract_csharp_doc_comment(source, node);

    // Build signature: <T>(int x, string y) : ReturnType
    let mut signature = String::new();
    if let Some(g) = generics {
        signature.push_str(g);
    }
    if let Some(p) = parameters {
        signature.push_str(p);
    }
    if let Some(rt) = return_type {
        if !signature.is_empty() {
            signature.push_str(" : ");
        }
        signature.push_str(rt);
    }

    let metadata = serde_json::json!({
        "visibility": visibility,
        "return_type": return_type,
    });

    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "delegate".to_string(),
            signature: if signature.is_empty() {
                None
            } else {
                Some(signature)
            },
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }
}

// Stub functions - to be implemented in later tasks

fn extract_csharp_method(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let visibility = extract_csharp_visibility(node, source);
    let modifiers = extract_csharp_modifiers(node, source);

    // Extract return type - find type node (predefined_type, identifier, generic_name, etc.)
    // The return type is the first type-like child before the method name
    let return_type = {
        let mut cursor = node.walk();
        let mut found_type = None;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "predefined_type" | "identifier" | "generic_name" | "nullable_type"
                | "array_type" | "qualified_name" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        found_type = Some(text);
                        break;
                    }
                }
                "modifier" => continue, // Skip modifiers
                _ if child.kind() == name.as_deref().unwrap_or("") => break, // Stop at method name
                _ => {}
            }
        }
        found_type
    };

    // Extract parameters
    let parameters = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract generics - use child_by_field_name for type_parameters field
    let type_params = node
        .child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract type parameter constraints (where clauses)
    let mut constraints = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter_constraints_clause" {
            if let Ok(constraint_text) = child.utf8_text(source.as_bytes()) {
                constraints.push(constraint_text.to_string());
            }
        }
    }

    let docstring = extract_csharp_doc_comment(source, node);

    // Build signature: <T>(int x, string y) : ReturnType where T : IComparable
    let mut signature = String::new();
    if let Some(tp) = type_params {
        signature.push_str(tp);
    }
    if let Some(p) = parameters {
        signature.push_str(p);
    }
    if let Some(rt) = return_type {
        // Always add return type with : separator
        signature.push_str(" : ");
        signature.push_str(rt);
    }
    if !constraints.is_empty() {
        signature.push(' ');
        signature.push_str(&constraints.join(" "));
    }

    let metadata = serde_json::json!({
        "visibility": visibility,
        "return_type": return_type,
        "is_static": modifiers.contains(&"static".to_string()),
        "is_async": modifiers.contains(&"async".to_string()),
        "is_virtual": modifiers.contains(&"virtual".to_string()),
        "is_override": modifiers.contains(&"override".to_string()),
        "is_abstract": modifiers.contains(&"abstract".to_string()),
    });

    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "method".to_string(),
            signature: if signature.is_empty() {
                None
            } else {
                Some(signature)
            },
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }

    // Do NOT recurse into method body
}

fn extract_csharp_constructor(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let visibility = extract_csharp_visibility(node, source);

    // Extract parameters
    let parameters = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    // Extract constructor initializer (: base(...) or : this(...))
    // Note: constructor_initializer is a child node kind, not a field
    let initializer = find_child_by_kind(node, "constructor_initializer")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    let docstring = extract_csharp_doc_comment(source, node);

    // Signature is just the parameters
    let signature = parameters.map(|s| s.to_string());

    let metadata = serde_json::json!({
        "visibility": visibility,
        "initializer": initializer,
    });

    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "constructor".to_string(),
            signature,
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }

    // Do NOT recurse into constructor body
}

fn extract_csharp_property(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let visibility = extract_csharp_visibility(node, source);

    // Extract type
    let prop_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    let docstring = extract_csharp_doc_comment(source, node);

    // Determine accessor pattern
    let mut signature = String::new();
    if let Some(pt) = prop_type {
        signature.push_str(pt);
        signature.push(' ');
    }

    // Check for accessor list (get/set/init)
    // Note: accessor_list is a child node kind, not a field
    if let Some(accessor_list) = find_child_by_kind(node, "accessor_list") {
        if let Ok(accessor_text) = accessor_list.utf8_text(source.as_bytes()) {
            signature.push_str(accessor_text);
        }
    } else {
        // Expression-bodied property (=> expr)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "arrow_expression_clause" {
                signature.push_str("=> ...");
                break;
            }
        }
    }

    let metadata = serde_json::json!({
        "visibility": visibility,
        "type": prop_type,
    });

    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "property".to_string(),
            signature: if signature.is_empty() {
                None
            } else {
                Some(signature)
            },
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }
}

fn extract_csharp_event(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let visibility = extract_csharp_visibility(node, source);

    // Extract type (event handler type)
    let event_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok());

    let docstring = extract_csharp_doc_comment(source, node);

    let signature = event_type.map(|s| s.to_string());

    let metadata = serde_json::json!({
        "visibility": visibility,
        "type": event_type,
    });

    if let Some(name) = name {
        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "event".to_string(),
            signature,
            docstring,
            start_line: (node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            metadata: Some(metadata),
        });
    }
}

fn extract_csharp_event_field(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Event field declarations can have multiple declarators
    let visibility = extract_csharp_visibility(node, source);
    let docstring = extract_csharp_doc_comment(source, node);

    // Extract type - it's in the variable_declaration child
    let event_type =
        if let Some(var_decl_node) = node.children(&mut node.walk()).find(|n| n.kind() == "variable_declaration") {
            var_decl_node
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        } else {
            node.child_by_field_name("type")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        };

    // Event fields can declare multiple events in one statement
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declaration" {
            // variable_declaration contains variable_declarator children
            let mut var_cursor = child.walk();
            for var_child in child.children(&mut var_cursor) {
                if var_child.kind() == "variable_declarator" {
                    if let Some(name_node) = var_child.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            let signature = event_type.map(|s| s.to_string());

                            let metadata = serde_json::json!({
                                "visibility": visibility,
                                "type": event_type,
                            });

                            chunks.push(SymbolChunk {
                                symbol_name: Some(name.to_string()),
                                kind: "event".to_string(),
                                signature,
                                docstring: docstring.clone(),
                                start_line: (var_child.start_position().row + 1) as i32,
                                end_line: (var_child.end_position().row + 1) as i32,
                                metadata: Some(metadata),
                            });
                        }
                    }
                }
            }
        }
    }
}

#[allow(unused_variables, clippy::ptr_arg)]
fn extract_csharp_namespace(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Stub - will be implemented in task 2003
}

#[allow(unused_variables, clippy::ptr_arg)]
fn collect_csharp_using(source: &str, node: Node, imports: &mut Vec<serde_json::Value>) {
    // Stub - will be implemented in task 2003
}

#[allow(unused_variables)]
fn extract_csharp_doc_comment(source: &str, node: Node) -> Option<String> {
    // Stub - will be implemented in task 2004
    None
}

#[allow(unused_variables)]
fn extract_csharp_visibility(node: Node, source: &str) -> String {
    // Stub - will be implemented in task 2004
    "private".to_string()
}

#[allow(unused_variables)]
fn extract_csharp_modifiers(node: Node, source: &str) -> Vec<String> {
    // Stub - will be implemented in task 2004
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_class() {
        let source = r#"
public class MyClass<T> : BaseClass, IInterface {
    public void Method() {}
}
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].kind, "class");
        assert_eq!(chunks[0].symbol_name.as_deref(), Some("MyClass"));
    }

    #[test]
    fn test_extract_interface() {
        let source = r#"
interface IMyInterface<T> : IBase {
    void Method();
}
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].kind, "interface");
        assert_eq!(chunks[0].symbol_name.as_deref(), Some("IMyInterface"));
    }

    #[test]
    fn test_extract_struct() {
        let source = r#"
public struct MyStruct : IInterface {
    public int Value;
}
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].kind, "struct");
        assert_eq!(chunks[0].symbol_name.as_deref(), Some("MyStruct"));
    }

    #[test]
    fn test_extract_enum() {
        let source = r#"
public enum Color : byte {
    Red,
    Green,
    Blue
}
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].kind, "enum");
        assert_eq!(chunks[0].symbol_name.as_deref(), Some("Color"));
        assert!(chunks[0].signature.as_deref().unwrap().contains("byte"));
    }

    #[test]
    fn test_extract_delegate() {
        let source = r#"
public delegate void MyDelegate<T>(int x, string y);
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].kind, "delegate");
        assert_eq!(chunks[0].symbol_name.as_deref(), Some("MyDelegate"));
    }

    #[test]
    fn test_nested_types() {
        let source = r#"
public class OuterClass {
    public class InnerClass {
        public void Method() {}
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].kind, "class");
        assert_eq!(chunks[0].symbol_name.as_deref(), Some("OuterClass"));
        assert_eq!(chunks[1].kind, "class");
        assert_eq!(chunks[1].symbol_name.as_deref(), Some("InnerClass"));
    }

    #[test]
    fn test_enum_no_recursion() {
        let source = r#"
public enum Status {
    Active,
    Inactive
}
"#;
        let chunks = extract_csharp_chunks(source);
        // Should only have the enum itself, not the members
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, "enum");
    }

    // Member extractor tests (task 2002)

    #[test]
    fn test_extract_method_basic() {
        let source = r#"
class MyClass {
    public void DoSomething(int x, string y) {
        // body
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(chunks.len() >= 2);
        let method = chunks.iter().find(|c| c.kind == "method").unwrap();
        assert_eq!(method.symbol_name.as_deref(), Some("DoSomething"));
        assert!(method
            .signature
            .as_ref()
            .unwrap()
            .contains("(int x, string y)"));
        assert!(method.signature.as_ref().unwrap().contains("void"));
    }

    #[test]
    fn test_extract_method_with_generics() {
        let source = r#"
class MyClass {
    public T Get<T>(string key) {
        return default(T);
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let method = chunks.iter().find(|c| c.kind == "method").unwrap();
        assert_eq!(method.symbol_name.as_deref(), Some("Get"));
        assert!(method.signature.as_ref().unwrap().contains("<T>"));
        assert!(method.signature.as_ref().unwrap().contains("(string key)"));
        assert!(method.signature.as_ref().unwrap().contains(": T"));
    }

    #[test]
    fn test_extract_method_with_constraints() {
        let source = r#"
class MyClass {
    public T Process<T>(T item) where T : IComparable {
        return item;
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let method = chunks.iter().find(|c| c.kind == "method").unwrap();
        assert_eq!(method.symbol_name.as_deref(), Some("Process"));
        let sig = method.signature.as_ref().unwrap();
        assert!(sig.contains("<T>"));
        assert!(sig.contains("where T : IComparable"));
    }

    #[test]
    fn test_extract_method_async() {
        let source = r#"
class MyClass {
    public async Task<int> FetchAsync() {
        return 42;
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let method = chunks.iter().find(|c| c.kind == "method").unwrap();
        assert_eq!(method.symbol_name.as_deref(), Some("FetchAsync"));
        // Note: is_async will be false until task 2004 (modifiers extraction)
        let metadata = method.metadata.as_ref().unwrap();
        assert_eq!(metadata["is_async"], false); // Stubbed - will be true after task 2004
    }

    #[test]
    fn test_extract_method_static_virtual_override() {
        let source = r#"
class MyClass {
    public static void StaticMethod() {}
    public virtual void VirtualMethod() {}
    public override void OverrideMethod() {}
    public abstract void AbstractMethod();
}
"#;
        let chunks = extract_csharp_chunks(source);
        let methods: Vec<_> = chunks.iter().filter(|c| c.kind == "method").collect();
        assert!(methods.len() >= 4);

        // Note: All modifiers will be false until task 2004 (modifiers extraction)
        let static_method = methods
            .iter()
            .find(|m| m.symbol_name.as_deref() == Some("StaticMethod"))
            .unwrap();
        assert_eq!(static_method.metadata.as_ref().unwrap()["is_static"], false); // Stubbed

        let virtual_method = methods
            .iter()
            .find(|m| m.symbol_name.as_deref() == Some("VirtualMethod"))
            .unwrap();
        assert_eq!(
            virtual_method.metadata.as_ref().unwrap()["is_virtual"],
            false
        ); // Stubbed

        let override_method = methods
            .iter()
            .find(|m| m.symbol_name.as_deref() == Some("OverrideMethod"))
            .unwrap();
        assert_eq!(
            override_method.metadata.as_ref().unwrap()["is_override"],
            false
        ); // Stubbed

        let abstract_method = methods
            .iter()
            .find(|m| m.symbol_name.as_deref() == Some("AbstractMethod"))
            .unwrap();
        assert_eq!(
            abstract_method.metadata.as_ref().unwrap()["is_abstract"],
            false
        ); // Stubbed
    }

    #[test]
    fn test_extract_constructor_basic() {
        let source = r#"
class MyClass {
    public MyClass(int x) {
        // body
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let constructor = chunks.iter().find(|c| c.kind == "constructor").unwrap();
        assert_eq!(constructor.symbol_name.as_deref(), Some("MyClass"));
        assert!(constructor
            .signature
            .as_ref()
            .unwrap()
            .contains("(int x)"));
    }

    #[test]
    fn test_extract_constructor_with_initializer() {
        let source = r#"
class MyClass {
    public MyClass(int x) : base(x) {
        // body
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let constructor = chunks.iter().find(|c| c.kind == "constructor").unwrap();
        assert_eq!(constructor.symbol_name.as_deref(), Some("MyClass"));
        let metadata = constructor.metadata.as_ref().unwrap();
        assert!(metadata["initializer"]
            .as_str()
            .unwrap()
            .contains(": base(x)"));
    }

    #[test]
    fn test_extract_property_auto() {
        let source = r#"
class MyClass {
    public string Name { get; set; }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let property = chunks.iter().find(|c| c.kind == "property").unwrap();
        assert_eq!(property.symbol_name.as_deref(), Some("Name"));
        let sig = property.signature.as_ref().unwrap();
        assert!(sig.contains("string"));
        assert!(sig.contains("get") && sig.contains("set"));
    }

    #[test]
    fn test_extract_property_get_only() {
        let source = r#"
class MyClass {
    public int Count { get; }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let property = chunks.iter().find(|c| c.kind == "property").unwrap();
        assert_eq!(property.symbol_name.as_deref(), Some("Count"));
        let sig = property.signature.as_ref().unwrap();
        assert!(sig.contains("int"));
        assert!(sig.contains("get"));
        assert!(!sig.contains("set"));
    }

    #[test]
    fn test_extract_property_expression_bodied() {
        let source = r#"
class MyClass {
    public string FullName => $"{First} {Last}";
}
"#;
        let chunks = extract_csharp_chunks(source);
        let property = chunks.iter().find(|c| c.kind == "property").unwrap();
        assert_eq!(property.symbol_name.as_deref(), Some("FullName"));
        let sig = property.signature.as_ref().unwrap();
        assert!(sig.contains("string"));
        assert!(sig.contains("=>"));
    }

    #[test]
    fn test_extract_event_declaration() {
        let source = r#"
class MyClass {
    public event EventHandler MyEvent {
        add { }
        remove { }
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let event = chunks.iter().find(|c| c.kind == "event").unwrap();
        assert_eq!(event.symbol_name.as_deref(), Some("MyEvent"));
        assert_eq!(
            event.signature.as_deref(),
            Some("EventHandler"),
            "Event signature should be the type"
        );
    }

    #[test]
    fn test_extract_event_field() {
        let source = r#"
class MyClass {
    public event EventHandler OnClick, OnHover;
}
"#;
        let chunks = extract_csharp_chunks(source);
        let events: Vec<_> = chunks.iter().filter(|c| c.kind == "event").collect();
        assert_eq!(events.len(), 2);

        let onclick = events
            .iter()
            .find(|e| e.symbol_name.as_deref() == Some("OnClick"))
            .unwrap();
        assert_eq!(onclick.signature.as_deref(), Some("EventHandler"));

        let onhover = events
            .iter()
            .find(|e| e.symbol_name.as_deref() == Some("OnHover"))
            .unwrap();
        assert_eq!(onhover.signature.as_deref(), Some("EventHandler"));
    }

    #[test]
    fn test_method_no_recursion() {
        let source = r#"
class MyClass {
    public void Outer() {
        void LocalFunction() {
            // local function
        }
    }
}
"#;
        let chunks = extract_csharp_chunks(source);
        let methods: Vec<_> = chunks.iter().filter(|c| c.kind == "method").collect();
        // Should only extract Outer, not LocalFunction (no recursion into body)
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].symbol_name.as_deref(), Some("Outer"));
    }

    #[test]
    fn test_class_with_members() {
        let source = r#"
class MyClass {
    public MyClass() {}
    public void Method() {}
    public string Name { get; set; }
    public event EventHandler OnEvent;
}
"#;
        let chunks = extract_csharp_chunks(source);
        assert!(chunks.len() >= 5); // class + constructor + method + property + event

        assert!(chunks.iter().any(|c| c.kind == "class"));
        assert!(chunks.iter().any(|c| c.kind == "constructor"));
        assert!(chunks.iter().any(|c| c.kind == "method"));
        assert!(chunks.iter().any(|c| c.kind == "property"));
        assert!(chunks.iter().any(|c| c.kind == "event"));
    }
}
