//! TypeScript/JavaScript edge extraction.
//!
//! This module extracts call edges from TypeScript and JavaScript source code
//! using tree-sitter parsing to find function calls and resolve them to chunks
//! within the same file.

use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, trace, warn};
use tree_sitter::{Node, Parser};

use super::common::{build_symbol_table, find_enclosing_chunk};
use super::{ChunkWithId, Edge, EdgeType};

/// Extract call edges from TypeScript/JavaScript source.
///
/// Parses the source code using tree-sitter, finds all call expressions,
/// and resolves them to chunks within the same file. Returns edges representing
/// function calls between chunks.
///
/// # Arguments
///
/// * `source` - TypeScript/JavaScript source code
/// * `chunks` - Chunks with database IDs from the same file
///
/// # Returns
///
/// * `Ok(Vec<Edge>)` - Extracted call edges (may be empty)
/// * `Err(_)` - Critical failure (parser setup error)
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::indexer::edges::typescript::extract_calls;
/// use crewchief_maproom::indexer::edges::ChunkWithId;
///
/// let source = "function foo() { return 42; }\nfunction bar() { foo(); }";
/// let chunks = vec![
///     ChunkWithId {
///         id: 1,
///         symbol_name: Some("foo".to_string()),
///         kind: "function".to_string(),
///         start_line: 1,
///         end_line: 1,
///         file_id: 100,
///     },
///     ChunkWithId {
///         id: 2,
///         symbol_name: Some("bar".to_string()),
///         kind: "function".to_string(),
///         start_line: 2,
///         end_line: 2,
///         file_id: 100,
///     }
/// ];
///
/// let edges = extract_calls(source, &chunks)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract_calls(source: &str, chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    // Parse source with tree-sitter
    let mut parser = Parser::new();
    let language = tree_sitter_typescript::language_typescript();
    parser
        .set_language(&language)
        .context("Failed to set TypeScript language")?;

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            warn!("Failed to parse TypeScript file for edge extraction");
            return Ok(Vec::new());
        }
    };

    // Build symbol table for same-file resolution
    let symbol_table = build_symbol_table(chunks);

    // Find all call expressions
    let mut edges = Vec::new();
    let root = tree.root_node();

    find_call_expressions(&root, source, chunks, &symbol_table, &mut edges);

    debug!("Extracted {} call edges from TypeScript file", edges.len());
    Ok(edges)
}

/// Recursively find call expressions in AST.
///
/// Traverses the syntax tree depth-first to find all `call_expression` nodes,
/// processes each one to extract edges, and recursively visits child nodes.
fn find_call_expressions(
    node: &Node,
    source: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
) {
    if node.kind() == "call_expression" {
        process_call_expression(node, source, chunks, symbol_table, edges);
    }

    // Recursively traverse children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_call_expressions(&child, source, chunks, symbol_table, edges);
    }
}

/// Process a single call expression node.
///
/// Extracts the function identifier, resolves it in the symbol table,
/// finds the enclosing chunk (caller), and creates an edge if both
/// caller and callee are resolved.
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

    // Resolve callee in symbol table
    let callee_id = match symbol_table.get(&callee_name) {
        Some(&id) => id,
        None => {
            trace!(
                "Unresolved call: {} (may be cross-file or built-in)",
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

/// Extract function identifier from call expression.
///
/// Handles different call patterns:
/// - Simple call: `foo()` → extracts "foo"
/// - Method call: `obj.method()` → extracts "method"
/// - Complex calls (computed properties, etc.) → returns None for Phase 1
///
/// # Arguments
///
/// * `node` - The call_expression node
/// * `source` - Source code text
///
/// # Returns
///
/// * `Some(String)` - Function/method name
/// * `None` - Could not extract (complex expression)
fn extract_function_identifier(node: &Node, source: &str) -> Option<String> {
    // call_expression has a "function" child (the callee)
    let function_node = node.child_by_field_name("function")?;

    match function_node.kind() {
        "identifier" => {
            // Simple call: foo()
            Some(function_node.utf8_text(source.as_bytes()).ok()?.to_string())
        }
        "member_expression" => {
            // Method call: obj.method()
            // Extract the property (method name)
            let property = function_node.child_by_field_name("property")?;
            Some(property.utf8_text(source.as_bytes()).ok()?.to_string())
        }
        _ => {
            // Complex call (computed property, etc.) - skip for Phase 1
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_call() {
        let source = r#"
            function foo() { return 42; }
            function bar() {
                const x = foo();
                return x;
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
            class Calculator {
                add(a, b) { return a + b; }
                multiply(a, b) {
                    return this.add(a, a) * b;
                }
            }
        "#;

        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("add".to_string()),
                kind: "method".to_string(),
                start_line: 3,
                end_line: 3,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("multiply".to_string()),
                kind: "method".to_string(),
                start_line: 4,
                end_line: 6,
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
            function foo() {
                console.log("test"); // console.log is not in chunks
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

        // console.log should be skipped (not in symbol table)
        assert_eq!(edges.len(), 0, "Should skip unresolved calls");
    }

    #[test]
    fn test_parse_error_returns_empty() {
        let invalid_source = "function foo(";
        let chunks = vec![];

        let result = extract_calls(invalid_source, &chunks);

        assert!(result.is_ok(), "Should not fail on parse error");
        assert_eq!(result.unwrap().len(), 0, "Should return empty vec");
    }

    #[test]
    fn test_multiple_calls() {
        let source = r#"
            function add(a, b) { return a + b; }
            function subtract(a, b) { return a - b; }
            function calculate() {
                const x = add(1, 2);
                const y = subtract(5, 3);
                return x + y;
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
}
