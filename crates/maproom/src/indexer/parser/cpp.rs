//! C++ parser

use tree_sitter::{Node, Parser};

use super::common::lang_cpp;
use crate::indexer::SymbolChunk;

pub(super) fn extract_cpp_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_cpp())
        .expect("Failed to set C++ language");

    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => return vec![],
    };

    let mut chunks = Vec::new();
    let mut includes = Vec::new();

    walk_cpp_decls(source, tree.root_node(), &mut chunks, &mut includes);

    // TODO: Create __imports__ chunk from includes in Phase 2

    chunks
}

fn walk_cpp_decls(
    _source: &str,
    _node: Node,
    _chunks: &mut Vec<SymbolChunk>,
    _includes: &mut Vec<serde_json::Value>,
) {
    // Skeletal implementation - Phase 2 will add node dispatch
}
