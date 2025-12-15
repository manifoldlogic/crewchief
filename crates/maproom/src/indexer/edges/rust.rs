//! Rust edge extraction.
//!
//! This module extracts call edges from Rust source code using tree-sitter
//! parsing to find function calls and method calls, resolving them to chunks
//! within the same file.

use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, trace, warn};
use tree_sitter::{Node, Parser};

use super::common::{build_symbol_table, find_enclosing_chunk};
use super::{ChunkWithId, Edge, EdgeType};

/// Extract call edges from Rust source
pub fn extract_calls(source: &str, chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    // Parse source with tree-sitter
    let mut parser = Parser::new();
    let language = tree_sitter_rust::language();
    parser
        .set_language(&language)
        .context("Failed to set Rust language")?;

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            warn!("Failed to parse Rust file for edge extraction");
            return Ok(Vec::new());
        }
    };

    // Build symbol table for same-file resolution
    let symbol_table = build_symbol_table(chunks);

    // Find all call expressions
    let mut edges = Vec::new();
    let root = tree.root_node();

    find_call_expressions(&root, source, chunks, &symbol_table, &mut edges);

    debug!("Extracted {} call edges from Rust file", edges.len());
    Ok(edges)
}

/// Recursively find call expressions in AST
fn find_call_expressions(
    node: &Node,
    source: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
) {
    match node.kind() {
        "call_expression" => {
            process_call_expression(node, source, chunks, symbol_table, edges);
        }
        "method_call_expression" => {
            process_method_call_expression(node, source, chunks, symbol_table, edges);
        }
        _ => {}
    }

    // Recursively traverse children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_call_expressions(&child, source, chunks, symbol_table, edges);
    }
}

/// Process a function call expression (foo(), module::function())
fn process_call_expression(
    node: &Node,
    source: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
) {
    // Extract function identifier
    let callee_name = match extract_function_identifier(node, source) {
        Some(name) => name,
        None => {
            trace!(
                "Could not extract function identifier from call at line {}",
                node.start_position().row + 1
            );
            return;
        }
    };

    resolve_and_create_edge(node, &callee_name, chunks, symbol_table, edges);
}

/// Process a method call expression (obj.method(), self.method())
fn process_method_call_expression(
    node: &Node,
    source: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
) {
    // Extract method name from the method field
    let method_node = match node.child_by_field_name("method") {
        Some(n) => n,
        None => {
            trace!(
                "Method call at line {} has no method field",
                node.start_position().row + 1
            );
            return;
        }
    };

    let method_name = match method_node.utf8_text(source.as_bytes()).ok() {
        Some(name) => name.to_string(),
        None => {
            trace!(
                "Could not extract method name at line {}",
                node.start_position().row + 1
            );
            return;
        }
    };

    resolve_and_create_edge(node, &method_name, chunks, symbol_table, edges);
}

/// Extract function identifier from call expression
/// Handles: foo(), module::function(), std::io::read()
fn extract_function_identifier(node: &Node, source: &str) -> Option<String> {
    // call_expression has a "function" field (the callee)
    let function_node = node.child_by_field_name("function")?;

    match function_node.kind() {
        "identifier" => {
            // Simple call: foo()
            Some(function_node.utf8_text(source.as_bytes()).ok()?.to_string())
        }
        "scoped_identifier" | "field_expression" => {
            // Qualified call: module::function() or obj.method()
            // Extract the rightmost identifier (the actual function name)
            extract_rightmost_identifier(&function_node, source)
        }
        _ => {
            // Complex call (macro, closure, etc.) - skip for Phase 1
            None
        }
    }
}

/// Extract rightmost identifier from scoped_identifier or field_expression
/// Example: std::io::read -> "read", obj.field -> "field"
fn extract_rightmost_identifier(node: &Node, source: &str) -> Option<String> {
    // For scoped_identifier, the rightmost part is the "name" field
    if let Some(name_node) = node.child_by_field_name("name") {
        return name_node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string());
    }

    // For field_expression, the rightmost part is the "field" field
    if let Some(field_node) = node.child_by_field_name("field") {
        return field_node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string());
    }

    // Fallback: try to get text of entire node
    node.utf8_text(source.as_bytes()).ok().map(|s| {
        // Extract last segment after ::
        s.split("::").last().unwrap_or(s).to_string()
    })
}

/// Resolve callee and create edge
fn resolve_and_create_edge(
    node: &Node,
    callee_name: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
) {
    // Resolve callee in symbol table
    let callee_id = match symbol_table.get(callee_name) {
        Some(&id) => id,
        None => {
            trace!(
                "Unresolved call: {} (may be cross-crate or built-in)",
                callee_name
            );
            return;
        }
    };

    // Find caller chunk (chunk containing this call)
    let call_line = node.start_position().row as i32 + 1; // tree-sitter is 0-indexed
    let caller_chunk = match find_enclosing_chunk(chunks, call_line) {
        Some(chunk) => chunk,
        None => {
            trace!("Call at line {} not in any chunk", call_line);
            return;
        }
    };

    // Create edge
    edges.push(Edge {
        src_chunk_id: caller_chunk.id,
        dst_chunk_id: callee_id,
        edge_type: EdgeType::Calls,
    });

    trace!(
        "Call edge: {} (chunk {}) → {} (chunk {})",
        caller_chunk.symbol_name.as_deref().unwrap_or("<anonymous>"),
        caller_chunk.id,
        callee_name,
        callee_id
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_call() {
        let source = r#"
            fn foo() -> i32 { 42 }
            fn bar() -> i32 {
                let x = foo();
                x
            }
        "#;

        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("foo".to_string()),
                kind: "function".to_string(),
                start_line: 2,
                end_line: 2,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("bar".to_string()),
                kind: "function".to_string(),
                start_line: 3,
                end_line: 6,
                file_id: 100,
            },
        ];

        let edges = extract_calls(source, &chunks).unwrap();

        assert_eq!(edges.len(), 1, "Should find one call edge");
        assert_eq!(edges[0].src_chunk_id, 2, "Caller should be bar");
        assert_eq!(edges[0].dst_chunk_id, 1, "Callee should be foo");
        assert_eq!(edges[0].edge_type, EdgeType::Calls);
    }

    #[test]
    fn test_extract_method_call() {
        let source = r#"
            struct Calculator;
            impl Calculator {
                fn add(&self, a: i32, b: i32) -> i32 { a + b }
                fn multiply(&self, a: i32, b: i32) -> i32 {
                    self.add(a, a) * b
                }
            }
        "#;

        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("add".to_string()),
                kind: "method".to_string(),
                start_line: 4,
                end_line: 4,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("multiply".to_string()),
                kind: "method".to_string(),
                start_line: 5,
                end_line: 7,
                file_id: 100,
            },
        ];

        let edges = extract_calls(source, &chunks).unwrap();

        // Should find multiply → add call
        assert!(edges.len() >= 1, "Should find at least one method call");
        let add_call = edges.iter().find(|e| e.dst_chunk_id == 1);
        assert!(add_call.is_some(), "Should find call to add method");
    }

    #[test]
    fn test_unresolved_call_skipped() {
        let source = r#"
            fn foo() {
                println!("test"); // println! is a macro, not in chunks
            }
        "#;

        let chunks = vec![ChunkWithId {
            id: 1,
            symbol_name: Some("foo".to_string()),
            kind: "function".to_string(),
            start_line: 2,
            end_line: 4,
            file_id: 100,
        }];

        let edges = extract_calls(source, &chunks).unwrap();

        // println! should be skipped (macro, not a function call)
        // Note: macros use macro_invocation node, not call_expression
        assert_eq!(edges.len(), 0, "Should skip macro invocations");
    }

    #[test]
    fn test_parse_error_returns_empty() {
        let invalid_source = "fn foo(";
        let chunks = vec![];

        let result = extract_calls(invalid_source, &chunks);

        assert!(result.is_ok(), "Should not fail on parse error");
        assert_eq!(result.unwrap().len(), 0, "Should return empty vec");
    }

    #[test]
    fn test_multiple_calls() {
        let source = r#"
            fn add(a: i32, b: i32) -> i32 { a + b }
            fn subtract(a: i32, b: i32) -> i32 { a - b }
            fn calculate() -> i32 {
                let x = add(1, 2);
                let y = subtract(5, 3);
                x + y
            }
        "#;

        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("add".to_string()),
                kind: "function".to_string(),
                start_line: 2,
                end_line: 2,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("subtract".to_string()),
                kind: "function".to_string(),
                start_line: 3,
                end_line: 3,
                file_id: 100,
            },
            ChunkWithId {
                id: 3,
                symbol_name: Some("calculate".to_string()),
                kind: "function".to_string(),
                start_line: 4,
                end_line: 8,
                file_id: 100,
            },
        ];

        let edges = extract_calls(source, &chunks).unwrap();

        assert_eq!(edges.len(), 2, "Should find two calls");
        assert!(edges.iter().any(|e| e.dst_chunk_id == 1), "Should call add");
        assert!(
            edges.iter().any(|e| e.dst_chunk_id == 2),
            "Should call subtract"
        );
    }

    #[test]
    fn test_qualified_call() {
        let source = r#"
            mod utils {
                pub fn helper() -> i32 { 42 }
            }
            fn main() {
                let x = utils::helper();
            }
        "#;

        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("helper".to_string()),
                kind: "function".to_string(),
                start_line: 3,
                end_line: 3,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("main".to_string()),
                kind: "function".to_string(),
                start_line: 5,
                end_line: 7,
                file_id: 100,
            },
        ];

        let edges = extract_calls(source, &chunks).unwrap();

        // Should resolve utils::helper() to "helper" and find the call
        assert_eq!(edges.len(), 1, "Should find qualified call");
        assert_eq!(edges[0].dst_chunk_id, 1, "Should resolve to helper");
    }
}
