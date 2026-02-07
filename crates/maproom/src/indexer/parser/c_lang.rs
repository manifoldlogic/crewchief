//! C language parser

use tree_sitter::Parser;

use super::common::lang_c;
use crate::indexer::SymbolChunk;

pub(super) fn extract_c_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_c())
        .expect("Failed to set C language");

    let _tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => return Vec::new(),
    };

    // Stub implementation - actual extraction in Phase 2
    Vec::new()
}
