---
name: treesitter-symbol-extraction
description: Extract symbol metadata from tree-sitter AST nodes and build SymbolChunk structs with name, signature, docstring, and metadata
origin: MLLANG-1005
created: 2026-02-08
tags: [parser, tree-sitter, symbol-extraction, ast]
---

# Tree-Sitter Symbol Extraction Pattern

## Overview

This skill documents the standard pattern for extracting symbol information from tree-sitter AST nodes and building Maproom's SymbolChunk structures. All language parsers in Maproom follow this pattern to produce consistent, searchable symbol metadata. The pattern handles extracting symbol names, signatures, documentation, and language-specific metadata from tree-sitter parse trees.

## When to Use

- When implementing a new language parser's symbol extraction logic
- When adding support for a new symbol type (e.g., interfaces, traits, protocols)
- When modifying existing extraction to capture additional metadata
- When debugging why symbols aren't being extracted correctly

## Pattern/Procedure

### Core SymbolChunk Structure

All extracted symbols are represented as SymbolChunk structs:

```rust
pub struct SymbolChunk {
    pub symbol_name: Option<String>,      // Symbol identifier
    pub kind: String,                     // "func", "struct", "enum", "typedef", "variable", etc.
    pub signature: Option<String>,        // Type signature or parameters
    pub docstring: Option<String>,        // Extracted documentation
    pub start_line: i32,                  // 1-indexed start line
    pub end_line: i32,                    // 1-indexed end line
    pub metadata: Option<serde_json::Value>, // Language-specific metadata
}
```

### Extraction Pattern

#### 1. Identify Symbol Node

Walk the tree-sitter AST and dispatch on node kinds:

```rust
fn walk_decls(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    match node.kind() {
        "function_definition" => extract_function(source, node, chunks),
        "struct_specifier" => extract_struct(source, node, chunks),
        "enum_specifier" => extract_enum(source, node, chunks),
        _ => {}
    }

    // Recursively walk children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_decls(source, child, chunks);
    }
}
```

#### 2. Extract Symbol Name

Use tree-sitter field names or navigate declarator chains:

```rust
// Direct field access (common in most languages)
let name = node
    .child_by_field_name("name")
    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
    .map(|s| s.to_string());

// Declarator navigation (C-family languages)
let declarator = node.child_by_field_name("declarator")?;
let name = extract_declarator_name(source, declarator);
```

#### 3. Extract Signature

Build type signatures from multiple AST components:

```rust
// Function signature with return type and parameters
let return_type = node
    .child_by_field_name("type")
    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
    .map(|s| s.to_string());

let params = node
    .child_by_field_name("parameters")
    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
    .map(|s| s.to_string());

let signature = match (&return_type, &params) {
    (Some(ret), Some(par)) => Some(format!("{} {}", ret, par)),
    (Some(ret), None) => Some(ret.clone()),
    (None, Some(par)) => Some(par.clone()),
    (None, None) => None,
};
```

#### 4. Extract Documentation

Collect documentation comments from preceding nodes:

```rust
// Walk backward from declaration to collect doc comments
fn extract_doc_comment(source: &str, node: Node) -> Option<String> {
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();
    let mut doc_lines = Vec::new();

    for i in (0..start_line).rev() {
        let line = lines.get(i)?.trim();

        if line.starts_with("//") {
            let comment = line.strip_prefix("//").unwrap_or("").trim();
            doc_lines.insert(0, comment);
        } else if line.ends_with("*/") {
            // Handle block comments
            break;
        } else if !line.is_empty() {
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}
```

#### 5. Build Metadata Object

Include language-specific details in metadata:

```rust
let mut metadata_obj = serde_json::Map::new();

// Storage class (C/C++)
if let Some(storage) = extract_storage_class(source, node) {
    metadata_obj.insert(
        "storage_class".to_string(),
        serde_json::Value::String(storage),
    );
}

// Return type
if let Some(ret) = return_type {
    metadata_obj.insert(
        "return_type".to_string(),
        serde_json::Value::String(ret),
    );
}

// Field count (structs)
metadata_obj.insert(
    "field_count".to_string(),
    serde_json::Value::Number(serde_json::Number::from(field_count)),
);

let metadata = if metadata_obj.is_empty() {
    None
} else {
    Some(serde_json::Value::Object(metadata_obj))
};
```

#### 6. Create SymbolChunk

Combine extracted data into a SymbolChunk:

```rust
let start = node.start_position();
let end = node.end_position();

chunks.push(SymbolChunk {
    symbol_name: Some(name),
    kind: "func".to_string(),
    signature,
    docstring: extract_doc_comment(source, node),
    start_line: (start.row + 1) as i32,
    end_line: (end.row + 1) as i32,
    metadata,
});
```

### Common Metadata Fields

Different symbol types use different metadata:

**Functions:**
- `return_type`: Return type string
- `storage_class`: static, extern, inline, etc.
- `visibility`: public, private, protected

**Structs:**
- `field_count`: Number of fields
- `visibility`: Access modifier

**Enums:**
- `enumerator_count`: Number of enum values
- `base_type`: Underlying integer type (if specified)

**Typedefs:**
- `underlying_type`: The aliased type

**Variables:**
- `type`: Variable type
- `storage_class`: static, extern, const, etc.

## Examples

### C Function Extraction

From `c_lang.rs`:

```rust
fn extract_c_function_common(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract return type
    let return_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract declarator (contains function name and parameters)
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => return,
    };

    // Navigate nested declarators to find name and parameters
    let (name, params) = extract_function_name_and_params(source, declarator);
    let name = match name {
        Some(n) => n,
        None => return,
    };

    // Build signature
    let signature = match (&return_type, &params) {
        (Some(ret), Some(par)) => Some(format!("{} {}", ret, par)),
        (Some(ret), None) => Some(ret.clone()),
        (None, Some(par)) => Some(par.clone()),
        (None, None) => None,
    };

    // Extract doc comment and storage class
    let docstring = extract_c_doc_comment(source, node);
    let storage_class = extract_storage_class(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    if let Some(ref storage) = storage_class {
        metadata_obj.insert("storage_class".to_string(), serde_json::Value::String(storage.clone()));
    }
    if let Some(ref ret) = return_type {
        metadata_obj.insert("return_type".to_string(), serde_json::Value::String(ret.clone()));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: Some(name),
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: if metadata_obj.is_empty() { None } else { Some(serde_json::Value::Object(metadata_obj)) },
    });
}
```

### C Struct Extraction

From `c_lang.rs`:

```rust
fn extract_c_struct(
    source: &str,
    type_node: Node,
    body: Node,
    declaration_node: Node,
    chunks: &mut Vec<SymbolChunk>,
) {
    // Extract struct name from type_node
    let name = type_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Count fields in body
    let field_count = count_struct_fields(body);

    // Extract doc comment from declaration_node
    let docstring = extract_c_doc_comment(source, declaration_node);

    // Build metadata with field count
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "field_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(field_count)),
    );

    let start = declaration_node.start_position();
    let end = declaration_node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "struct".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}
```

### Multi-Variable Declaration Handling

From `c_lang.rs` handling `int a, b, c;`:

```rust
fn extract_multi_variable_declarators(
    source: &str,
    node: Node,
    type_text: &Option<String>,
    chunks: &mut Vec<SymbolChunk>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "init_declarator" || child.kind() == "pointer_declarator" {
            if let Some(name) = extract_declarator_name(source, child) {
                add_variable_chunk(source, node, &name, type_text, chunks);
            }
        }
    }
}
```

## References

- Ticket: MLLANG-1005
- Related files:
  - `crates/maproom/src/indexer/mod.rs` (SymbolChunk definition)
  - `crates/maproom/src/indexer/parser/c_lang.rs` (comprehensive extraction examples)
  - `crates/maproom/src/indexer/parser/rust_lang.rs` (Rust extraction patterns)
  - `crates/maproom/src/indexer/parser/go.rs` (Go extraction patterns)
  - `crates/maproom/src/indexer/parser/java.rs` (Java extraction patterns)
- tree-sitter documentation: https://tree-sitter.github.io/tree-sitter/
