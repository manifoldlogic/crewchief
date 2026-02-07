//! C# parser

use tree_sitter::Parser;

use super::common::lang_csharp;
use crate::indexer::SymbolChunk;

pub(super) fn extract_csharp_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_csharp())
        .expect("Failed to set C# language");

    let tree = parser.parse(source, None);
    let chunks = Vec::new();

    if let Some(tree) = tree {
        let _root = tree.root_node();
        // Stub implementation - actual extraction in Phase 2
    }

    chunks
}
