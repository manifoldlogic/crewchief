# Ticket: EDGEEXT-1002 - TypeScript Call Extraction

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary

Implement TypeScript/JavaScript call expression extraction using tree-sitter to find function calls and resolve them to chunks within the same file. This is the core functionality for Phase 1 MVP.

## Background

TypeScript and JavaScript files contain function calls in various forms (simple calls, method calls, constructor calls). We need to extract these call expressions using tree-sitter, identify the caller and callee, and create edges between chunks.

Phase 1 focuses on same-file resolution only (no cross-file imports), which achieves 70-80% accuracy in typical codebases with simpler implementation and better performance (no database queries).

Tree-sitter provides a `call_expression` node type that captures function calls. We traverse the AST to find these nodes, extract the function identifier, and resolve it against chunks in the same file.

**Integration Note:** This ticket implements the TypeScript extractor only. Integration with scan_worktree() and upsert_files() is handled in EDGEEXT-1003. The ChunkWithId structs are passed in by the caller (not loaded internally by this extractor).

## Acceptance Criteria

- [ ] Implement `extract_calls()` in `edges/typescript.rs` using tree-sitter
- [ ] Find all `call_expression` nodes in TypeScript/JavaScript AST
- [ ] Extract function identifier from call expressions (handle simple calls, method calls)
- [ ] Resolve callee symbol name against same-file chunks
- [ ] Find enclosing chunk for each call (caller)
- [ ] Create Edge structs for resolved calls
- [ ] Skip unresolved calls gracefully (log at trace level)
- [ ] Works with both TypeScript and JavaScript parsers (ts, tsx, js, jsx)
- [ ] Unit tests with synthetic TypeScript snippets achieve ≥85% accuracy
- [ ] Handles parse errors gracefully (return Ok(Vec::new()), log warning)

## Technical Requirements

**TypeScript Extractor Implementation (edges/typescript.rs):**

```rust
use anyhow::{Context, Result};
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Query, QueryCursor};
use tracing::{debug, trace, warn};

use super::{ChunkWithId, Edge, EdgeType};
use super::common::{build_symbol_table, find_enclosing_chunk};

/// Extract call edges from TypeScript/JavaScript source
pub fn extract_calls(source: &str, chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    // Parse source with tree-sitter
    let mut parser = Parser::new();
    let language = tree_sitter_typescript::language_typescript();
    parser.set_language(language).context("Failed to set TypeScript language")?;

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

/// Recursively find call expressions in AST
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

/// Process a single call expression node
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

    // Resolve callee in symbol table
    let callee_id = match symbol_table.get(&callee_name) {
        Some(&id) => id,
        None => {
            trace!("Unresolved call: {} (may be cross-file or built-in)", callee_name);
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

/// Extract function identifier from call expression
/// Handles: foo(), obj.method(), new Constructor()
fn extract_function_identifier(node: &Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();

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
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("multiply".to_string()),
                kind: "method".to_string(),
                start_line: 4,
                end_line: 6,
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
}
```

**Dependencies (Cargo.toml):**
Already present in the codebase:
- `tree-sitter = "0.22"`
- `tree-sitter-typescript = "0.21"`
- `tracing = "0.1"`

## Implementation Notes

**Call Expression Patterns:**
- Simple call: `foo()` → identifier node
- Method call: `obj.method()` → member_expression with property
- Constructor: `new Foo()` → new_expression (treat as call for Phase 1)
- Arrow functions: `array.map(x => x * 2)` → skip for Phase 1 (inline function)

**Symbol Resolution Strategy:**
1. Build symbol table from chunks (HashMap<symbol_name, chunk_id>)
2. For each call, extract function name
3. Look up in symbol table
4. If found, create edge; if not found, skip (may be cross-file or built-in)

**Performance:**
- Tree-sitter parse: ~5ms for 200-line file
- AST traversal: ~10ms (3-5× LOC nodes)
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

**Blocks:**
- EDGEEXT-1003 (integration with scan/upsert - needs working extractor)

## Risk Assessment

**Risk:** Tree-sitter node type is not "call_expression"
**Mitigation:** Validate with simple test first, tree-sitter-typescript docs confirm node type

**Risk:** Method call extraction is more complex than expected
**Mitigation:** Start with simple calls, add method call support incrementally

**Risk:** Accuracy lower than 85% for same-file calls
**Mitigation:** Acceptable for MVP, iterate on heuristics based on test results

## Files/Packages Affected

**Modified Files:**
- `crates/maproom/src/indexer/edges/typescript.rs` (replace stub with full implementation)

**New Test Files:**
- Unit tests in `edges/typescript.rs` test module

## Planning References

- Architecture: `.crewchief/projects/EDGEEXT_edge-extraction/planning/architecture.md` (lines 125-140)
- Plan: `.crewchief/projects/EDGEEXT_edge-extraction/planning/plan.md` (Phase 1, lines 14-16)
- Quality Strategy: `.crewchief/projects/EDGEEXT_edge-extraction/planning/quality-strategy.md` (lines 30-65)
