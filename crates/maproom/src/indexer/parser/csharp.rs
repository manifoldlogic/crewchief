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

#[allow(unused_variables)]
fn extract_csharp_method(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Stub - will be implemented in task 2002
}

#[allow(unused_variables)]
fn extract_csharp_constructor(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Stub - will be implemented in task 2002
}

#[allow(unused_variables)]
fn extract_csharp_property(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Stub - will be implemented in task 2002
}

#[allow(unused_variables)]
fn extract_csharp_event(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Stub - will be implemented in task 2002
}

#[allow(unused_variables)]
fn extract_csharp_event_field(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Stub - will be implemented in task 2002
}

#[allow(unused_variables)]
fn extract_csharp_namespace(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    // Stub - will be implemented in task 2003
}

#[allow(unused_variables)]
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
}
