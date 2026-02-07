//! TypeScript/JavaScript parser (stub implementation - Phase 1)

use tree_sitter::Node;

use crate::indexer::SymbolChunk;

/// Extract chunks from TypeScript, TSX, JavaScript, or JSX code
pub(super) fn extract_code_chunks(_source: &str, _language: &str) -> Vec<SymbolChunk> {
    // Stub implementation - will be populated in Phase 2
    Vec::new()
}

/// Walk AST and extract TypeScript/JavaScript declarations
pub(super) fn walk_add_decls(_source: &str, _node: Node, _out: &mut Vec<SymbolChunk>) {
    // Stub implementation - will be populated in Phase 2
}
