//! TypeScript/JavaScript parser

use tree_sitter::{Language, Node, Parser};

use super::common::push_chunk;
use crate::indexer::SymbolChunk;
use crate::profile_scope;

// Language providers
#[allow(dead_code)]
fn lang_typescript() -> Language {
    tree_sitter_typescript::language_typescript()
}

#[allow(dead_code)]
fn lang_tsx() -> Language {
    tree_sitter_typescript::language_tsx()
}

#[allow(dead_code)]
fn lang_javascript() -> Language {
    tree_sitter_javascript::language()
}

/// Extract chunks from TypeScript, TSX, JavaScript, or JSX code
pub(super) fn extract_code_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    profile_scope!("extract_code_chunks");
    let mut parser = Parser::new();
    let lang = match language {
        "ts" => lang_typescript(),
        "tsx" => lang_tsx(),
        "js" | "jsx" => lang_javascript(),
        _ => return Vec::new(),
    };
    parser.set_language(&lang).ok();
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let root = tree.root_node();
    let mut chunks = Vec::new();

    walk_add_decls(source, root, &mut chunks);
    chunks
}

/// Walk AST and extract TypeScript/JavaScript declarations
fn walk_add_decls(source: &str, node: Node, out: &mut Vec<SymbolChunk>) {
    let kind = node.kind();
    match kind {
        // Functions
        "function_declaration" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| Some(n.utf8_text(source.as_bytes()).ok()?.to_string()));
            push_chunk(source, node, name, "func", out);
        }
        // Classes
        "class_declaration" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| Some(n.utf8_text(source.as_bytes()).ok()?.to_string()));
            push_chunk(source, node, name, "class", out);
        }
        // Variable declarations may contain arrow functions assigned to const
        "lexical_declaration" | "variable_declaration" => {
            // Look for variable declarators with arrow_function
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "variable_declarator" {
                        let name = child
                            .child_by_field_name("name")
                            .and_then(|n| Some(n.utf8_text(source.as_bytes()).ok()?.to_string()));
                        let value = child.child_by_field_name("value");
                        if let Some(v) = value {
                            if v.kind() == "arrow_function" || v.kind() == "function" {
                                push_chunk(source, v, name, "func", out);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_add_decls(source, child, out);
        }
    }
}
