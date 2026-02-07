//! Java parser

use tree_sitter::Parser;

use super::common::lang_java;
use crate::indexer::SymbolChunk;

pub(super) fn extract_java_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_java())
        .expect("Failed to set Java language");

    let _tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => {
            tracing::warn!("Failed to parse Java source");
            return Vec::new();
        }
    };

    // Stub implementation - actual extraction in Phase 2
    Vec::new()
}
