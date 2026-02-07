//! Parser module for extracting code chunks from source files
//!
//! This module provides language-specific parsing using tree-sitter grammars
//! to extract semantic chunks (functions, classes, methods, etc.) from source code.

use crate::indexer::SymbolChunk;
use crate::profile_scope;

// Submodules for language-specific parsing
pub(crate) mod c_lang;
pub(crate) mod common;
pub(crate) mod data_formats;
pub(crate) mod go;
pub(crate) mod markdown;
pub(crate) mod python;
pub(crate) mod python_docstrings;
pub(crate) mod ruby;
pub(crate) mod rust_lang;
pub(crate) mod typescript;

/// Main entry point for chunk extraction
///
/// Dispatches to the appropriate language-specific parser based on the language string.
pub fn extract_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    profile_scope!("extract_chunks");
    match language {
        "md" | "mdx" => markdown::extract_markdown_chunks(source),
        "json" => data_formats::extract_json_chunks(source),
        "yaml" | "yml" => data_formats::extract_yaml_chunks(source),
        "toml" => data_formats::extract_toml_chunks(source),
        "py" => python::extract_python_chunks(source),
        "rs" => rust_lang::extract_rust_chunks(source),
        "go" => go::extract_go_chunks(source),
        "gomod" => go::extract_gomod_chunks(source),
        "rb" => ruby::extract_ruby_chunks(source),
        "c" => c_lang::extract_c_chunks(source),
        _ => typescript::extract_code_chunks(source, language),
    }
}
