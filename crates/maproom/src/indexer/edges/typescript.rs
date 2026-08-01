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
use super::{ChunkWithId, Edge, EdgeType, UnresolvedRef};

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
/// use maproom::indexer::edges::typescript::extract_calls;
/// use maproom::indexer::edges::ChunkWithId;
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
/// let (edges, _unresolved) = extract_calls(source, "ts", &chunks)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract_calls(
    source: &str,
    language: &str,
    chunks: &[ChunkWithId],
) -> Result<(Vec<Edge>, Vec<UnresolvedRef>)> {
    // Parse source with tree-sitter, selecting the grammar PER DIALECT
    // (spec A4) exactly like the chunk parser: edge extraction previously
    // used the plain TypeScript grammar for tsx/jsx too, mis-parsing JSX.
    let mut parser = Parser::new();
    let grammar = match language {
        "tsx" => tree_sitter_typescript::language_tsx(),
        "js" | "jsx" => tree_sitter_javascript::language(),
        _ => tree_sitter_typescript::language_typescript(),
    };
    parser
        .set_language(&grammar)
        .context("Failed to set TypeScript/JavaScript language")?;

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            warn!("Failed to parse TypeScript file for edge extraction");
            return Ok((Vec::new(), Vec::new()));
        }
    };

    // Build symbol table for same-file resolution, restricted to CALLABLE kinds
    // (spec A6, matching the Rust/Python extractors): a same-file `class`/type
    // chunk sharing a callee's name must not become a call target — and keeping it
    // out also lets a real cross-file function of that name resolve in the post-pass.
    let callable: Vec<ChunkWithId> = chunks
        .iter()
        .filter(|c| super::is_callable_kind(&c.kind))
        .cloned()
        .collect();
    let symbol_table = build_symbol_table(&callable);

    // Find all call expressions
    let mut edges = Vec::new();
    let mut unresolved = Vec::new();
    let root = tree.root_node();

    find_call_expressions(
        &root,
        source,
        chunks,
        &symbol_table,
        &mut edges,
        &mut unresolved,
    );

    debug!(
        "Extracted {} same-file call edges, {} unresolved refs from TypeScript file",
        edges.len(),
        unresolved.len()
    );
    Ok((edges, unresolved))
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
    unresolved: &mut Vec<UnresolvedRef>,
) {
    if node.kind() == "call_expression" {
        process_call_expression(node, source, chunks, symbol_table, edges, unresolved);
    }

    // Recursively traverse children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_call_expressions(&child, source, chunks, symbol_table, edges, unresolved);
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
    unresolved: &mut Vec<UnresolvedRef>,
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

    // Find caller chunk FIRST — an unresolved ref must be attributed to its caller.
    let call_line = node.start_position().row as i32 + 1; // tree-sitter is 0-indexed
    let caller_chunk = match find_enclosing_chunk(chunks, call_line) {
        Some(chunk) => chunk,
        None => {
            trace!("Call at line {} not in any chunk", call_line);
            return;
        }
    };

    // Resolve callee in the same-file symbol table.
    let callee_id = match symbol_table.get(&callee_name) {
        Some(&id) => id,
        None => {
            // Spec B1: not silently dropped — handed to the cross-file post-pass.
            trace!(
                "Unresolved local call: {} (cross-file candidate)",
                callee_name
            );
            unresolved.push(UnresolvedRef {
                src_chunk_id: caller_chunk.id,
                callee_name: callee_name.clone(),
            });
            return;
        }
    };

    // Spec A6: no self-edges (recursion is not a relationship worth a row) —
    // matching the Rust/Python extractors.
    if caller_chunk.id == callee_id {
        return;
    }

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

    /// Spec A4: tsx files parse with the TSX grammar — call sites inside
    /// JSX elements were mis-parsed by the plain TS grammar before.
    #[test]
    fn test_tsx_calls_inside_jsx_extracted() {
        let source = r#"
function formatName(n: string): string {
    return n.trim();
}

export function Badge(props: { name: string }) {
    return <span className="badge">{formatName(props.name)}</span>;
}
"#;
        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("formatName".to_string()),
                kind: "func".to_string(),
                start_line: 2,
                end_line: 4,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("Badge".to_string()),
                kind: "func".to_string(),
                start_line: 6,
                end_line: 8,
                file_id: 100,
            },
        ];
        let (edges, _unresolved) = extract_calls(source, "tsx", &chunks).unwrap();
        assert!(
            edges
                .iter()
                .any(|e| e.src_chunk_id == 2 && e.dst_chunk_id == 1),
            "Badge -> formatName call inside JSX must be extracted: {edges:?}"
        );
    }

    /// Spec A6 (review fix): a recursive call produces no self-edge, and a same-file
    /// `class` sharing a callee name is not a call target (kept out of the symbol
    /// table so the call instead becomes a cross-file candidate).
    #[test]
    fn test_no_self_edge_and_class_not_a_target() {
        let source = r#"
            class Widget {}
            function fib(n: number): number { return fib(n - 1) + fib(n - 2); }
            function build() { return Widget(); }
        "#;
        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("Widget".to_string()),
                kind: "class".to_string(),
                start_line: 2,
                end_line: 2,
                file_id: 100,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("fib".to_string()),
                kind: "func".to_string(),
                start_line: 3,
                end_line: 3,
                file_id: 100,
            },
            ChunkWithId {
                id: 3,
                symbol_name: Some("build".to_string()),
                kind: "func".to_string(),
                start_line: 4,
                end_line: 4,
                file_id: 100,
            },
        ];
        let (edges, unresolved) = extract_calls(source, "ts", &chunks).unwrap();
        // No self-edge for the recursive fib.
        assert!(
            !edges.iter().any(|e| e.src_chunk_id == e.dst_chunk_id),
            "self-edge leaked: {edges:?}"
        );
        // No edge targeting the class chunk.
        assert!(
            !edges.iter().any(|e| e.dst_chunk_id == 1),
            "class became a call target: {edges:?}"
        );
        // Widget() is instead handed to the cross-file post-pass.
        assert!(
            unresolved.iter().any(|u| u.callee_name == "Widget"),
            "Widget() must be an unresolved ref: {unresolved:?}"
        );
    }

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

        let (edges, _unresolved) = extract_calls(source, "ts", &chunks).unwrap();

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

        let (edges, _unresolved) = extract_calls(source, "ts", &chunks).unwrap();

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

        let (edges, _unresolved) = extract_calls(source, "ts", &chunks).unwrap();

        // console.log should be skipped (not in symbol table)
        assert_eq!(edges.len(), 0, "Should skip unresolved calls");
    }

    #[test]
    fn test_parse_error_returns_empty() {
        let invalid_source = "function foo(";
        let chunks = vec![];

        let result = extract_calls(invalid_source, "ts", &chunks);

        assert!(result.is_ok(), "Should not fail on parse error");
        let (edges, unresolved) = result.unwrap();
        assert_eq!(edges.len(), 0, "Should return empty edges");
        assert_eq!(unresolved.len(), 0, "Should return empty unresolved refs");
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

        let (edges, _unresolved) = extract_calls(source, "ts", &chunks).unwrap();

        assert_eq!(edges.len(), 2, "Should find two calls");
        assert!(edges.iter().any(|e| e.dst_chunk_id == 1), "Should call add");
        assert!(
            edges.iter().any(|e| e.dst_chunk_id == 2),
            "Should call subtract"
        );
    }
}
