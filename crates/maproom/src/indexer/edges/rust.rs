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
use super::{ChunkWithId, Edge, EdgeType, UnresolvedRef};

/// Extract call edges from Rust source.
///
/// Returns `(same_file_edges, unresolved_refs)` (spec B1): callees found among
/// this file's callable chunks become edges immediately; callees not found locally
/// are returned as unresolved references for the worktree cross-file post-pass.
pub fn extract_calls(
    source: &str,
    chunks: &[ChunkWithId],
) -> Result<(Vec<Edge>, Vec<UnresolvedRef>)> {
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
            return Ok((Vec::new(), Vec::new()));
        }
    };

    // Build symbol table for same-file resolution, restricted to CALLABLE
    // kinds (spec A6): `use`/struct/module chunks share names with real
    // functions and would otherwise become bogus call targets.
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
        "Extracted {} same-file call edges, {} unresolved refs from Rust file",
        edges.len(),
        unresolved.len()
    );
    Ok((edges, unresolved))
}

/// Recursively find call expressions in AST
fn find_call_expressions(
    node: &Node,
    source: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
    unresolved: &mut Vec<UnresolvedRef>,
) {
    // NOTE: tree-sitter-rust has no separate method-call node kind —
    // method calls arrive as call_expression with a field_expression callee
    // and are handled by extract_function_identifier below.
    if node.kind() == "call_expression" {
        process_call_expression(node, source, chunks, symbol_table, edges, unresolved);
    }

    // Recursively traverse children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_call_expressions(&child, source, chunks, symbol_table, edges, unresolved);
    }
}

/// Process a function call expression (foo(), module::function())
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

    resolve_and_create_edge(node, &callee_name, chunks, symbol_table, edges, unresolved);
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

/// Resolve callee and create an edge, or record an unresolved cross-file reference.
fn resolve_and_create_edge(
    node: &Node,
    callee_name: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
    unresolved: &mut Vec<UnresolvedRef>,
) {
    // Find caller chunk (chunk containing this call) FIRST — an unresolved ref must
    // be attributed to its caller, so we need the enclosing chunk even on a miss.
    let call_line = node.start_position().row as i32 + 1; // tree-sitter is 0-indexed
    let caller_chunk = match find_enclosing_chunk(chunks, call_line) {
        Some(chunk) => chunk,
        None => {
            trace!("Call at line {} not in any chunk", call_line);
            return;
        }
    };

    // Resolve callee in the same-file symbol table.
    let callee_id = match symbol_table.get(callee_name) {
        Some(&id) => id,
        None => {
            // Spec B1: not silently dropped — handed to the cross-file post-pass.
            trace!(
                "Unresolved local call: {} (cross-file candidate)",
                callee_name
            );
            unresolved.push(UnresolvedRef {
                src_chunk_id: caller_chunk.id,
                callee_name: callee_name.to_string(),
            });
            return;
        }
    };

    // Spec A6: no self-edges (recursion is not a relationship worth a row).
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Map REAL chunker output to ChunkWithId (sequential fake ids) — the
    /// hand-built chunk lists in the older tests hide container overlap.
    fn real_chunks(source: &str) -> Vec<ChunkWithId> {
        crate::indexer::parser::extract_chunks(source, "rs")
            .into_iter()
            .enumerate()
            .map(|(i, c)| ChunkWithId {
                id: (i + 1) as i64,
                symbol_name: c.symbol_name,
                kind: c.kind,
                start_line: c.start_line,
                end_line: c.end_line,
                file_id: 100,
            })
            .collect()
    }

    fn chunk_by_symbol<'a>(chunks: &'a [ChunkWithId], name: &str) -> &'a ChunkWithId {
        chunks
            .iter()
            .find(|c| c.symbol_name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("chunk {name} not found in {chunks:?}"))
    }

    /// Spec A3 acceptance: with REAL chunker output (impl container chunk
    /// overlapping methods), a method-body call attributes to the METHOD.
    #[test]
    fn test_method_call_attributes_to_method_not_impl() {
        let source = r#"
pub struct Calculator;

impl Calculator {
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    pub fn multiply(&self, a: i32, b: i32) -> i32 {
        self.add(a, a) * b
    }
}
"#;
        let chunks = real_chunks(source);
        // Sanity: the chunker must actually emit an overlapping container.
        assert!(
            chunks
                .iter()
                .any(|c| c.kind == "impl" || c.kind == "struct"),
            "fixture must exercise container overlap: {chunks:?}"
        );
        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();
        let add = chunk_by_symbol(&chunks, "add");
        let multiply = chunk_by_symbol(&chunks, "multiply");
        let call = edges
            .iter()
            .find(|e| e.dst_chunk_id == add.id)
            .expect("multiply -> add edge required");
        assert_eq!(
            call.src_chunk_id, multiply.id,
            "src must be the METHOD chunk (innermost), not the impl container"
        );
    }

    /// Spec A6: `use` statements sharing a callee's name must not become
    /// call targets, and recursion produces no self-edge.
    #[test]
    fn test_callable_kind_filter_and_no_self_edge() {
        let source = r#"
use other::helper;

pub fn helper() -> i32 {
    helper_inner()
}

pub fn helper_inner() -> i32 {
    helper_inner_recurse()
}

pub fn helper_inner_recurse() -> i32 {
    helper_inner_recurse()
}
"#;
        let chunks = real_chunks(source);
        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();
        for e in &edges {
            let dst = chunks.iter().find(|c| c.id == e.dst_chunk_id).unwrap();
            assert!(
                super::super::is_callable_kind(&dst.kind),
                "non-callable dst leaked: {dst:?}"
            );
            assert_ne!(e.src_chunk_id, e.dst_chunk_id, "self-edge leaked");
        }
    }

    /// Spec A: cfg(test) fn calling the target is a plain calls edge with
    /// the test fn as src (feeds F-B test_of derivation).
    #[test]
    fn test_cfg_test_module_call_extracted() {
        let source = r#"
pub fn alpha() -> i32 {
    43
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha() {
        // call OUTSIDE a macro: assert_eq!(alpha(), ..) hides the call in a
        // macro token-tree (documented out-of-scope for extraction)
        let result = alpha();
        assert_eq!(result, 43);
    }
}
"#;
        let chunks = real_chunks(source);
        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();
        let alpha = chunk_by_symbol(&chunks, "alpha");
        let test_alpha = chunk_by_symbol(&chunks, "test_alpha");
        assert!(
            edges
                .iter()
                .any(|e| e.src_chunk_id == test_alpha.id && e.dst_chunk_id == alpha.id),
            "test_alpha -> alpha calls edge required; got {edges:?} chunks {chunks:?}"
        );
    }

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

        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();

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

        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();

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

        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();

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
        let (edges, unresolved) = result.unwrap();
        assert_eq!(edges.len(), 0, "Should return empty edges");
        assert_eq!(unresolved.len(), 0, "Should return empty unresolved refs");
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

        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();

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

        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();

        // Should resolve utils::helper() to "helper" and find the call
        assert_eq!(edges.len(), 1, "Should find qualified call");
        assert_eq!(edges[0].dst_chunk_id, 1, "Should resolve to helper");
    }
}
