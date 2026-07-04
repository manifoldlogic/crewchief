//! Python edge extraction (spec F-D, SHOULD).
//!
//! Extracts call edges from Python source using tree-sitter, resolving callees to
//! callable chunks in the same file and returning unresolved references for the
//! worktree cross-file post-pass (shared with the Rust/TS extractors).

use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, trace, warn};
use tree_sitter::{Node, Parser};

use super::common::{build_symbol_table, find_enclosing_chunk};
use super::{ChunkWithId, Edge, EdgeType, UnresolvedRef};

/// Extract call edges from Python source.
///
/// Returns `(same_file_edges, unresolved_refs)` (spec B1), mirroring the Rust
/// extractor: direct calls `foo()` and attribute calls `obj.method()` are resolved
/// by the rightmost name against callable chunks; built-ins and unknown names become
/// unresolved refs.
pub fn extract_calls(
    source: &str,
    chunks: &[ChunkWithId],
) -> Result<(Vec<Edge>, Vec<UnresolvedRef>)> {
    let mut parser = Parser::new();
    let language = tree_sitter_python::language();
    parser
        .set_language(&language)
        .context("Failed to set Python language")?;

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            warn!("Failed to parse Python file for edge extraction");
            return Ok((Vec::new(), Vec::new()));
        }
    };

    // Same-file symbol table restricted to CALLABLE kinds (spec A6/B3): `class`,
    // import, and module chunks share names with real functions/methods and must
    // not become call targets.
    let callable: Vec<ChunkWithId> = chunks
        .iter()
        .filter(|c| super::is_callable_kind(&c.kind))
        .cloned()
        .collect();
    let symbol_table = build_symbol_table(&callable);

    let mut edges = Vec::new();
    let mut unresolved = Vec::new();
    let root = tree.root_node();
    find_call_expressions(&root, source, chunks, &symbol_table, &mut edges, &mut unresolved);

    debug!(
        "Extracted {} same-file call edges, {} unresolved refs from Python file",
        edges.len(),
        unresolved.len()
    );
    Ok((edges, unresolved))
}

/// Recursively find `call` nodes in the AST.
fn find_call_expressions(
    node: &Node,
    source: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
    unresolved: &mut Vec<UnresolvedRef>,
) {
    if node.kind() == "call" {
        process_call_expression(node, source, chunks, symbol_table, edges, unresolved);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_call_expressions(&child, source, chunks, symbol_table, edges, unresolved);
    }
}

/// Process a single `call` node.
fn process_call_expression(
    node: &Node,
    source: &str,
    chunks: &[ChunkWithId],
    symbol_table: &HashMap<String, i64>,
    edges: &mut Vec<Edge>,
    unresolved: &mut Vec<UnresolvedRef>,
) {
    let callee_name = match extract_function_identifier(node, source) {
        Some(name) => name,
        None => {
            trace!(
                "Could not extract callee from call at line {}",
                node.start_position().row + 1
            );
            return;
        }
    };

    // Find caller chunk FIRST — an unresolved ref must be attributed to its caller.
    let call_line = node.start_position().row as i32 + 1;
    let caller_chunk = match find_enclosing_chunk(chunks, call_line) {
        Some(chunk) => chunk,
        None => {
            trace!("Call at line {} not in any chunk", call_line);
            return;
        }
    };

    let callee_id = match symbol_table.get(&callee_name) {
        Some(&id) => id,
        None => {
            // Spec B1: not silently dropped — handed to the cross-file post-pass.
            trace!("Unresolved local call: {} (cross-file candidate)", callee_name);
            unresolved.push(UnresolvedRef {
                src_chunk_id: caller_chunk.id,
                callee_name,
            });
            return;
        }
    };

    // No self-edges (recursion is not a relationship worth a row).
    if caller_chunk.id == callee_id {
        return;
    }

    edges.push(Edge {
        src_chunk_id: caller_chunk.id,
        dst_chunk_id: callee_id,
        edge_type: EdgeType::Calls,
    });
}

/// Extract the callee name from a `call` node's `function` field.
///
/// - `foo()` -> `identifier` -> "foo".
/// - `obj.method()` / `pkg.mod.func()` -> `attribute` -> the rightmost `attribute`
///   field ("method" / "func").
/// - subscripts, lambdas, and other complex callees are skipped.
fn extract_function_identifier(node: &Node, source: &str) -> Option<String> {
    let function_node = node.child_by_field_name("function")?;
    match function_node.kind() {
        "identifier" => Some(function_node.utf8_text(source.as_bytes()).ok()?.to_string()),
        "attribute" => {
            let attr = function_node.child_by_field_name("attribute")?;
            Some(attr.utf8_text(source.as_bytes()).ok()?.to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_chunks(source: &str) -> Vec<ChunkWithId> {
        crate::indexer::parser::extract_chunks(source, "py")
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

    #[test]
    fn test_direct_function_call() {
        let source = "\
def helper():
    return 1


def caller():
    return helper()
";
        let chunks = real_chunks(source);
        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();
        let helper = chunk_by_symbol(&chunks, "helper");
        let caller = chunk_by_symbol(&chunks, "caller");
        assert!(
            edges
                .iter()
                .any(|e| e.src_chunk_id == caller.id && e.dst_chunk_id == helper.id),
            "caller -> helper edge required, got {edges:?}"
        );
    }

    #[test]
    fn test_self_method_call_resolves() {
        let source = "\
class Service:
    def run(self):
        return self.step()

    def step(self):
        return 2
";
        let chunks = real_chunks(source);
        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();
        let run = chunk_by_symbol(&chunks, "run");
        let step = chunk_by_symbol(&chunks, "step");
        assert!(
            edges
                .iter()
                .any(|e| e.src_chunk_id == run.id && e.dst_chunk_id == step.id),
            "run -> step (self.step()) edge required, got {edges:?}"
        );
    }

    #[test]
    fn test_builtin_and_unknown_calls_unresolved_not_edges() {
        let source = "\
def caller():
    xs = []
    xs.append(1)
    print(xs)
    return undefined_helper()
";
        let chunks = real_chunks(source);
        let (edges, unresolved) = extract_calls(source, &chunks).unwrap();
        // No same-file callee exists for append/print/undefined_helper.
        assert!(edges.is_empty(), "no same-file edges expected, got {edges:?}");
        // They are captured as unresolved refs (not silently dropped).
        assert!(
            unresolved.iter().any(|u| u.callee_name == "undefined_helper"),
            "unknown call must be an unresolved ref, got {unresolved:?}"
        );
    }

    #[test]
    fn test_class_name_not_a_call_target() {
        // Instantiating a class `Widget()` must NOT create an edge (class is not
        // a callable kind).
        let source = "\
class Widget:
    pass


def build():
    return Widget()
";
        let chunks = real_chunks(source);
        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();
        assert!(
            edges.is_empty(),
            "class instantiation must not become a calls edge, got {edges:?}"
        );
    }

    /// Review fix: async defs (chunker kind async_func/async_method) are ordinary
    /// call targets and must resolve, not be dropped as non-callable.
    #[test]
    fn test_async_def_is_a_callable_target() {
        let source = "\
async def fetch():
    return 1


async def run():
    return await fetch()
";
        let chunks = real_chunks(source);
        // Sanity: the chunker really tags these async.
        assert!(
            chunks.iter().any(|c| c.kind == "async_func"),
            "fixture must exercise async_func kind: {chunks:?}"
        );
        let (edges, _unresolved) = extract_calls(source, &chunks).unwrap();
        let fetch = chunk_by_symbol(&chunks, "fetch");
        let run = chunk_by_symbol(&chunks, "run");
        assert!(
            edges
                .iter()
                .any(|e| e.src_chunk_id == run.id && e.dst_chunk_id == fetch.id),
            "run -> fetch edge required (async target), got {edges:?}"
        );
    }

    #[test]
    fn test_parse_error_returns_empty() {
        let (edges, unresolved) = extract_calls("def foo(", &[]).unwrap();
        assert!(edges.is_empty() && unresolved.is_empty());
    }
}
