---
name: add-treesitter-language-parser
description: Add a new tree-sitter language parser to Maproom indexer with dependency wiring, extension mapping, and dispatcher integration
origin: MLLANG-1005
created: 2026-02-08
tags: [parser, tree-sitter, indexer, language-support]
---

# Add Tree-Sitter Language Parser

## Overview

This skill documents the standard procedure for adding a new tree-sitter language parser to the Maproom code indexer. It covers the full integration path from adding the tree-sitter grammar dependency through wiring the parser into the modular dispatcher system. This pattern was established with the C language implementation and applies to any future language additions (Swift, Kotlin, Zig, etc.).

## When to Use

- When adding support for a new programming language to Maproom's indexer
- When the language has an available tree-sitter grammar published on crates.io
- When you need to enable `extract_chunks(source, "language")` to return semantic symbols

## Pattern/Procedure

### Step 1: Add Tree-Sitter Dependency

Add the tree-sitter grammar crate to `crates/maproom/Cargo.toml`:

```toml
[dependencies]
tree-sitter-LANG = "x.y.z"
```

Example from C implementation:
```toml
tree-sitter-c = "0.21.4"
```

Find available grammars at: https://github.com/tree-sitter

### Step 2: Add Language Provider Function

Add a language provider function to `crates/maproom/src/indexer/parser/common.rs`:

```rust
pub(crate) fn lang_LANGUAGE() -> Language {
    tree_sitter_LANG::language()
}
```

Example from C implementation:
```rust
pub(crate) fn lang_c() -> Language {
    tree_sitter_c::language()
}
```

This function provides the tree-sitter Language object used by the parser.

### Step 3: Create Parser Module

Create a new parser module at `crates/maproom/src/indexer/parser/LANG.rs`:

```rust
//! LANGUAGE language parser

use tree_sitter::{Node, Parser};
use super::common::lang_LANGUAGE;
use crate::indexer::SymbolChunk;

/// Main entry point for LANGUAGE chunk extraction.
pub(super) fn extract_LANGUAGE_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_LANGUAGE())
        .expect("Failed to set LANGUAGE language");

    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => {
            tracing::warn!("Failed to parse LANGUAGE source");
            return Vec::new();
        }
    };

    let mut chunks = Vec::new();
    let root = tree.root_node();

    // Implement AST traversal and symbol extraction here
    walk_LANGUAGE_decls(source, root, &mut chunks);

    chunks
}

fn walk_LANGUAGE_decls(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Dispatch on node kinds to extract symbols
    // Reference existing parsers (rust_lang.rs, go.rs, java.rs) for patterns
}
```

Start with a stub that returns empty results to verify compilation.

### Step 4: Register Module in Parser

Add the module declaration to `crates/maproom/src/indexer/parser/mod.rs`:

```rust
pub(crate) mod LANG;
```

Add the dispatch case to the `extract_chunks` function:

```rust
pub fn extract_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    match language {
        // ... existing cases ...
        "LANG" => LANG::extract_LANGUAGE_chunks(source),
        _ => typescript::extract_code_chunks(source, language),
    }
}
```

Example from C implementation:
```rust
pub(crate) mod c_lang;

// In extract_chunks:
"c" => c_lang::extract_c_chunks(source),
```

### Step 5: Add File Extension Mapping

Add the file extension mapping to `crates/maproom/src/indexer/mod.rs` in the `detect_language_from_path` function:

```rust
".EXTENSION" => "LANG",
```

Example from C implementation:
```rust
".c" => "c",
```

For languages with multiple extensions (e.g., C has `.c` and `.h`), add all relevant mappings:
```rust
".c" | ".h" => "c",
```

### Step 6: Verify Integration

Run the build to ensure compilation succeeds:

```bash
cargo build -p maproom
```

Verify the language is recognized:

```rust
#[test]
fn test_LANG_detection() {
    let lang = detect_language_from_path("test.EXTENSION");
    assert_eq!(lang, Some("LANG"));
}
```

Create a basic smoke test in `crates/maproom/tests/LANG_parser_test.rs`:

```rust
use crewchief_maproom::indexer::parser;

#[test]
fn test_LANG_basic_parsing() {
    let source = "// Minimal valid LANGUAGE code";
    let chunks = parser::extract_chunks(source, "LANG");
    // Stub should return empty, real implementation will return chunks
    assert!(chunks.is_empty() || !chunks.is_empty());
}
```

## Examples

### C Language Integration (MLLANG-1005)

Files modified:
- `Cargo.toml`: Added `tree-sitter-c = "0.21.4"`
- `parser/common.rs`: Added `lang_c()` function
- `parser/c_lang.rs`: Created with `extract_c_chunks()`
- `parser/mod.rs`: Added `pub(crate) mod c_lang;` and dispatch case
- `indexer/mod.rs`: Added `".c" | ".h" => "c",` mapping

Result: `extract_chunks(source, "c")` successfully parses C code and returns SymbolChunks.

### Existing Language Patterns

Reference these existing parser modules for implementation patterns:
- `rust_lang.rs`: Comprehensive Rust symbol extraction
- `go.rs`: Go packages, functions, structs
- `java.rs`: Java classes, methods, annotations
- `python.rs`: Python classes, functions, decorators
- `cpp.rs`: C++ templates, namespaces, classes

## References

- Ticket: MLLANG-1005
- Related files:
  - `crates/maproom/Cargo.toml`
  - `crates/maproom/src/indexer/mod.rs`
  - `crates/maproom/src/indexer/parser/mod.rs`
  - `crates/maproom/src/indexer/parser/common.rs`
  - `crates/maproom/src/indexer/parser/c_lang.rs` (reference implementation)
- Commits:
  - bdf58143: Initial C language detection and parser stub
  - a5dea708: Complete C parser implementation
