# Ticket: EDGEEXT-2001 - Rust Call Extraction

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary

Implement Rust call expression extraction using tree-sitter-rust to find function calls and method calls, resolving them to chunks within the same file. This extends the edge extraction system to support Rust codebases following the same pattern as TypeScript.

## Background

The EDGEEXT project has established a working edge extraction architecture for TypeScript/JavaScript in Phase 1. The system is designed to be extensible via language-specific extractors in `crates/maproom/src/indexer/edges/`.

Rust is a critical language for this codebase (CrewChief itself is written in Rust), and adding Rust call extraction will:
- Enable relationship-aware search for Rust codebases
- Demonstrate the extensibility of the edge extraction architecture
- Provide call graph data for quality scoring in the SRCHREL project

Tree-sitter-rust is already a dependency in the codebase and provides AST nodes for function calls and method calls. Like TypeScript, Phase 1 scope focuses on same-file resolution only (no cross-crate imports), which achieves 70-80% accuracy with simpler implementation and better performance.

**Integration Note:** This ticket implements the Rust extractor only. Integration with `scan_worktree()` and `upsert_files()` already exists from EDGEEXT-1003. The ChunkWithId structs are passed in by the caller (not loaded internally by this extractor).

## Acceptance Criteria

- [ ] Implement `extract_calls()` in `edges/rust.rs` using tree-sitter-rust
- [ ] Find all `call_expression` nodes in Rust AST (function calls)
- [ ] Find all `method_call_expression` nodes in Rust AST (method calls)
- [ ] Extract function/method identifier from call expressions
- [ ] Resolve callee symbol name against same-file chunks
- [ ] Find enclosing chunk for each call (caller)
- [ ] Create Edge structs for resolved calls
- [ ] Skip unresolved calls gracefully (log at trace level)
- [ ] Handle parse errors gracefully (return Ok(Vec::new()), log warning)
- [ ] Unit tests with synthetic Rust code snippets achieve ≥85% accuracy
- [ ] Follows same pattern as `edges/typescript.rs` for consistency

## Technical Requirements

**Rust Extractor Implementation (edges/rust.rs):**

```rust
use anyhow::{Context, Result};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};
use tracing::{debug, trace, warn};

use super::{ChunkWithId, Edge, EdgeType};
use super::common::{build_symbol_table, find_enclosing_chunk};

/// Extract call edges from Rust source
pub fn extract_calls(source: &str, chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    // Parse source with tree-sitter
    let mut parser = Parser::new();
    let language = tree_sitter_rust::language();
    parser.set_language(language).context("Failed to set Rust language")?;

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
            trace!("Could not extract function identifier from call at line {}", node.start_position().row + 1);
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
            trace!("Method call at line {} has no method field", node.start_position().row + 1);
            return;
        }
    };

    let method_name = match method_node.utf8_text(source.as_bytes()).ok() {
        Some(name) => name.to_string(),
        None => {
            trace!("Could not extract method name at line {}", node.start_position().row + 1);
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
        return name_node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
    }

    // For field_expression, the rightmost part is the "field" field
    if let Some(field_node) = node.child_by_field_name("field") {
        return field_node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
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
            trace!("Unresolved call: {} (may be cross-crate or built-in)", callee_name);
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
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("bar".to_string()),
                kind: "function".to_string(),
                start_line: 3,
                end_line: 6,
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
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("multiply".to_string()),
                kind: "method".to_string(),
                start_line: 5,
                end_line: 7,
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

        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("foo".to_string()),
                kind: "function".to_string(),
                start_line: 2,
                end_line: 4,
            },
        ];

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
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("subtract".to_string()),
                kind: "function".to_string(),
                start_line: 3,
                end_line: 3,
            },
            ChunkWithId {
                id: 3,
                symbol_name: Some("calculate".to_string()),
                kind: "function".to_string(),
                start_line: 4,
                end_line: 8,
            },
        ];

        let edges = extract_calls(source, &chunks).unwrap();

        assert_eq!(edges.len(), 2, "Should find two calls");
        assert!(edges.iter().any(|e| e.dst_chunk_id == 1), "Should call add");
        assert!(edges.iter().any(|e| e.dst_chunk_id == 2), "Should call subtract");
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
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("main".to_string()),
                kind: "function".to_string(),
                start_line: 5,
                end_line: 7,
            },
        ];

        let edges = extract_calls(source, &chunks).unwrap();

        // Should resolve utils::helper() to "helper" and find the call
        assert_eq!(edges.len(), 1, "Should find qualified call");
        assert_eq!(edges[0].dst_chunk_id, 1, "Should resolve to helper");
    }
}
```

**Dependencies (Cargo.toml):**
Already present in the codebase:
- `tree-sitter = "0.22"`
- `tree-sitter-rust` (version to be confirmed)
- `tracing = "0.1"`

**Module Registration:**
Update `edges/mod.rs` to expose the Rust extractor and wire it into the language dispatch logic.

## Implementation Notes

**Rust Call Expression Patterns:**
- Simple function call: `foo()` → `call_expression` with `identifier` function
- Qualified call: `module::function()` → `call_expression` with `scoped_identifier` function
- Method call: `obj.method()` → `method_call_expression` with method field
- Self method: `self.method()` → `method_call_expression`
- Macro invocation: `println!()` → `macro_invocation` (NOT a call_expression, skip)

**Key Differences from TypeScript:**
- Rust has separate node types for function calls (`call_expression`) and method calls (`method_call_expression`)
- TypeScript uses a single `call_expression` with member_expression for methods
- Rust has `scoped_identifier` for qualified paths (module::function)
- Macros are NOT function calls and use different AST nodes

**Symbol Resolution Strategy:**
1. Build symbol table from chunks (HashMap<symbol_name, chunk_id>)
2. For function calls, extract rightmost identifier from path (std::io::read → "read")
3. For method calls, extract method name from method field
4. Look up in symbol table
5. If found, create edge; if not found, skip (may be cross-crate or built-in)

**Performance:**
- Tree-sitter parse: ~5ms for 200-line Rust file
- AST traversal: ~10ms (Rust AST is similar complexity to TypeScript)
- Symbol table lookup: O(1) per call
- Total: ~15-20ms per file (acceptable)

**Logging Levels:**
- Trace: Unresolved calls (verbose, for debugging)
- Debug: Edge count per file
- Warn: Parse failures
- Info: Not used for this module

**Error Handling Strategy:**
- Parse failures: Return Ok(Vec::new()) and log warning (don't propagate error)
- Symbol resolution failures: Skip edge (log at trace level to avoid noise)
- Database errors: Not applicable (this module doesn't touch database)
- Partial edges better than no edges: Continue processing even with issues

## Dependencies

**Prerequisites:**
- EDGEEXT-1001 (edge extractor module with common utilities)
- tree-sitter-rust dependency in Cargo.toml

**Blocks:**
- None (Phase 2 work is independent)

**Related:**
- EDGEEXT-1002 (TypeScript implementation provides the pattern to follow)

## Risk Assessment

**Risk:** tree-sitter-rust node types differ from expected
**Mitigation:** Validate with simple test first, consult tree-sitter-rust grammar documentation

**Risk:** Qualified paths (module::function) harder to extract than expected
**Mitigation:** Start with simple calls, add qualified path support incrementally, use helper function for rightmost identifier extraction

**Risk:** Accuracy lower than 85% for same-file calls
**Mitigation:** Acceptable for initial implementation, iterate on heuristics based on test results

**Risk:** Method call syntax creates spurious edges
**Mitigation:** Use trace logging for unresolved calls to identify patterns, method calls are clearly distinguished in AST

## Files/Packages Affected

**New Files:**
- `crates/maproom/src/indexer/edges/rust.rs` (new Rust extractor)

**Modified Files:**
- `crates/maproom/src/indexer/edges/mod.rs` (register Rust extractor, add to language dispatch)
- `crates/maproom/Cargo.toml` (if tree-sitter-rust not already present)

**New Test Files:**
- Unit tests in `edges/rust.rs` test module

## Planning References

- Architecture: `.crewchief/projects/EDGEEXT_edge-extraction/planning/architecture.md` (extensible language-specific extractors pattern)
- Plan: `.crewchief/projects/EDGEEXT_edge-extraction/planning/plan.md` (Phase 3, lines 57-77)
- TypeScript Reference: `.crewchief/projects/EDGEEXT_edge-extraction/tickets/EDGEEXT-1002_typescript-call-extraction.md` (pattern to follow)
