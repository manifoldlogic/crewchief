---
name: add-language-support
description: Guide for adding a new programming language to Maproom's tree-sitter indexer. This skill should be used when a task requires implementing parser support for a language not yet indexed by Maproom (currently supported: TypeScript, JavaScript, Python, Rust, Go, Ruby). It covers all five touchpoint files, tree-sitter AST exploration, test creation, and language detector integration.
---

# Add Language Support to Maproom

Step-by-step procedure for adding a new programming language to the Maproom indexer.
Patterns validated across TypeScript, Python, Rust, Go, and Ruby implementations.

**Scope**: Validated for TypeScript, Python, Rust, Go, Ruby. Languages with
significantly different paradigms (Haskell, Prolog, assembly) may need adaptation.

## Executive Checklist

Complete these 11 steps in order. Each references a detailed section below.

1. **Explore the AST** -- Load sample code in tree-sitter playground; map constructs to node kinds and field names
2. **Assess language features** -- Walk through Language Feature Assessment to plan extractors
3. **Add grammar dependency** -- Add `tree-sitter-{LANG}` to `Cargo.toml` (Touchpoint 1)
4. **Create language function** -- Add `lang_{LANG}()` in `parser.rs` (Touchpoint 2)
5. **Wire dispatch** -- Add `"{EXT}" => extract_{LANG}_chunks(source)` to `extract_chunks()` (Touchpoint 2)
6. **Implement entry point** -- Write `extract_{LANG}_chunks()` with parser setup and import aggregation (Touchpoint 2)
7. **Implement AST walker** -- Write `walk_{LANG}_decls()` with kind-based dispatch (Touchpoint 2)
8. **Implement extractors** -- Write per-construct extraction functions (Touchpoint 2)
9. **Add extension mapping** -- Update `detect_language_from_path()` in `mod.rs` (Touchpoint 3)
10. **Update language detector** -- Add `Language::{Lang}` variant in `language_detector.rs` (Touchpoint 4)
11. **Write tests** -- Create `tests/{LANG}_parser_test.rs` with minimum 8 tests (Touchpoint 5)

---

## Touchpoint 1: Cargo.toml

**File**: `crates/maproom/Cargo.toml`
**Location**: The `# Parsing (Tree-sitter)` comment section. Add before `tree-sitter-md`.

```toml
tree-sitter-{LANG} = "{VERSION}"      # <-- ADD after tree-sitter-ruby
```

Check crates.io for compatibility with `tree-sitter = "0.22"`. Most grammars use `0.21.x`.

---

## Touchpoint 2: parser.rs

**File**: `crates/maproom/src/indexer/parser.rs`
Four additions: language function, dispatch arm, entry point, and AST walker with extractors.

### Language Function

Place alongside existing `lang_*()` functions (near `lang_ruby()`):
```rust
fn lang_{LANG}() -> Language {
    tree_sitter_{LANG}::language()
}
```

### Dispatch

Add match arm to `extract_chunks()`:
```rust
"{EXT}" => extract_{LANG}_chunks(source),
// For multiple extensions: "{EXT}" | "{ALT_EXT}" => extract_{LANG}_chunks(source),
```

### Entry Point

```rust
fn extract_{LANG}_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser.set_language(&lang_{LANG}()).expect("Failed to set {Lang} language");
    let tree = parser.parse(source, None);
    let mut chunks = Vec::new();

    if let Some(tree) = tree {
        let root = tree.root_node();
        let mut imports = Vec::new();
        // Add visibility state if needed: let mut visibility = "public";
        walk_{LANG}_decls(source, root, &mut chunks, &mut imports);

        if !imports.is_empty() {
            chunks.push(SymbolChunk {
                symbol_name: Some("__imports__".to_string()),
                kind: "imports".to_string(),
                signature: None,
                docstring: None,
                start_line: 1,
                end_line: 1,
                metadata: Some(serde_json::json!(imports)),
            });
        }
    }
    chunks
}
```

Remove imports vector if the language has no import mechanism. Add `&mut visibility`
parameter if the language has statement-based visibility modifiers (like Ruby).

### AST Walker

```rust
fn walk_{LANG}_decls(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    imports: &mut Vec<serde_json::Value>,
) {
    match node.kind() {
        "class_definition" => extract_{LANG}_class(source, node, chunks),
        "function_definition" => extract_{LANG}_function(source, node, chunks),
        // Add node kinds discovered in tree-sitter playground
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_{LANG}_decls(source, child, chunks, imports);
    }
}
```

Use `node.child_by_field_name()` over positional `node.child(i)`. Do NOT recurse
into method bodies. Only recurse into class/module bodies for nested declarations.

### Per-Construct Extractor Template

```rust
fn extract_{LANG}_{CONSTRUCT}(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());
    let signature = node.child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());
    let docstring = extract_{LANG}_doc_comment(source, node);
    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "{KIND}".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::json!({"visibility": "public"})),
    });
}
```

### Doc Comment Extraction (hash-style)

```rust
fn extract_{LANG}_doc_comment(source: &str, node: Node) -> Option<String> {
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();
    let mut doc_lines = Vec::new();
    for i in (0..start_line).rev() {
        let line = lines.get(i)?.trim();
        if line.starts_with('#') {
            let comment = line.strip_prefix("# ")
                .or_else(|| line.strip_prefix('#'))
                .unwrap_or("");
            doc_lines.insert(0, comment);
        } else if !line.is_empty() {
            break;
        }
    }
    if doc_lines.is_empty() { None } else { Some(doc_lines.join("\n")) }
}
```

For slash-style (`///`), see `extract_rust_doc_comment()`. For docstrings (`"""`),
see `extract_python_docstring()`.

### Import Collection

```rust
fn collect_{LANG}_import(source: &str, node: Node, imports: &mut Vec<serde_json::Value>) {
    imports.push(serde_json::json!({"type": "import_type", "target": "module_name"}));
}
```

All imports aggregate into a single `__imports__` chunk (see entry point).

---

## Touchpoint 3: mod.rs

**File**: `crates/maproom/src/indexer/mod.rs`
**Function**: `detect_language_from_path()`

Add extension to the match statement. For special filenames (like Ruby's `Gemfile`,
`Rakefile`), add a filename check before the extension match:

```rust
match path.file_name().and_then(|n| n.to_str()) {
    Some("SpecialFile") => return Some("{EXT}"),
    _ => {}
}
// In the extension match:
"{EXT}" => Some("{EXT}"),
```

---

## Touchpoint 4: language_detector.rs

**File**: `crates/maproom/src/context/language_detector.rs`
Five changes (Rust compiler will guide you via incomplete match warnings):

1. **Enum variant**: Add `{Lang}` to `Language` enum
2. **`as_str()`**: Add `Language::{Lang} => "{LANG}"`
3. **`from_str()`**: Add `"{LANG}" | "{EXT}" => Language::{Lang}`
4. **`detect_from_path()`**: Add `Some("{EXT}") => Language::{Lang}`
5. **`detect_from_content()`** (optional): Add heuristics for distinctive syntax

---

## Language Feature Assessment

Walk through these 8 decision points before writing code:

**DP1 -- Classes/Structs**: Does the language have class-like types? Use kind
`"class"` or `"struct"`. (Python/Ruby: `class`; Rust: `struct`; none: skip)

**DP2 -- Functions/Methods**: How are methods distinguished from functions?
Context-based (inside class = `"method"`, outside = `"func"`: Ruby, Python),
syntax-based (different node types: Ruby `singleton_method`), receiver-based
(Go), or modifier-based (`"async_func"`, `"async_method"`: Python).

**DP3 -- Modules/Namespaces**: Module declarations present? Use kind `"module"`.
(Ruby `module`, Rust `mod`; none: skip)

**DP4 -- Visibility/Access Control**: Statement-based (track mutable state, reset
in nested scopes: Ruby), per-declaration (`pub`: Rust), convention-based
(`_prefix`: Python), or none.

**DP5 -- Imports/Dependencies**: Aggregate into `__imports__` chunk (Python, Ruby),
extract individually as `"use"` (Rust) or `"import"` (Go), or skip.

**DP6 -- Constants/Variables**: Uppercase convention (Ruby, Python), keyword-based
(`const`/`static`: Rust, Go), or skip.

**DP7 -- Doc Comment Format**: Line comments before declaration (`#`, `///`, `//`),
block docstrings (`"""`), attribute-based, or skip.

**DP8 -- Language-Specific Constructs**: Unique indexable constructs? (Rust: `trait`,
`impl`, `enum`, `macro`; Go: `interface`; Python: decorators; Ruby: `singleton_method`)

---

## Tree-sitter AST Exploration Guide

**Playground URL**: https://tree-sitter.github.io/tree-sitter/playground

1. Select the language from the dropdown
2. Paste representative code (classes, methods, imports, constants, doc comments, nesting)
3. Click AST nodes to see `kind`, `field_name`, `start_position`/`end_position`
4. Document the mapping: construct -> node kind + field names
5. Test edge cases: empty bodies, missing comments, nested constructs, syntax errors
6. Record findings -- node kind strings become match arms in `walk_{LANG}_decls()`

**Tips**: Grammars vary in field naming (`parameters` vs `method_parameters`).
Some constructs produce unexpected node types (Ruby `def self.m` -> `singleton_method`).
Optional fields return `None` from `child_by_field_name()` -- always handle it.

---

## Known Pitfalls

**Pitfall 1 -- Visibility state bleed**: Languages with statement-based visibility
(Ruby) need save/reset/restore when entering nested scopes. See
`extract_ruby_class()` for the pattern.

**Pitfall 2 -- Visibility modifiers vs method calls**: In Ruby, `private` is a
`call` node. Distinguish by checking `child_by_field_name("arguments")` is `None`.
See `walk_ruby_decls()`.

**Pitfall 3 -- Inline comments captured as docstrings**: Only collect comments on
lines BEFORE declarations. Stop at first non-comment, non-blank line. Use
`doc_lines.insert(0, comment)` for backward walking. See `extract_ruby_doc_comment()`.

**Pitfall 4 -- 0-indexed line numbers**: Tree-sitter rows are 0-based; SymbolChunk
uses 1-based. Always: `(node.start_position().row + 1) as i32`.

**Pitfall 5 -- Recursing into method bodies**: Do NOT recurse into method bodies.
Only class/module bodies contain nested declarations. See `extract_ruby_method()`.

**Pitfall 6 -- Import chunk proliferation**: Collect imports into a Vec during
traversal, emit single `__imports__` chunk at the end. See `collect_ruby_import()`
and `extract_python_imports()`.

**Pitfall 7 -- Positional child access**: Use `child_by_field_name()` instead of
`node.child(i)`. Field names are stable across grammar versions.

---

## SymbolChunk Output Contract

```rust
#[derive(Debug, Clone)]
pub struct SymbolChunk {
    pub symbol_name: Option<String>,   // Searchable name ("MyClass", "__imports__")
    pub kind: String,                   // Category (see standard kinds below)
    pub signature: Option<String>,      // Parameters, type info, or value
    pub docstring: Option<String>,      // Documentation from comments
    pub start_line: i32,                // 1-based start line
    pub end_line: i32,                  // 1-based end line (inclusive)
    pub metadata: Option<serde_json::Value>, // Language-specific JSON
}
```

### Standard Kind Values

| Kind | Languages | Description |
|------|-----------|-------------|
| `func` | All | Free function (not inside a class) |
| `async_func` | Python | Async free function |
| `method` | Python, Ruby, Go | Method inside a class/struct |
| `async_method` | Python | Async method inside a class |
| `class` | Python, Ruby | Class definition |
| `struct` | Rust | Struct definition |
| `enum` | Rust | Enum definition |
| `trait` | Rust | Trait definition |
| `impl` | Rust | Implementation block |
| `module` | Ruby, Rust | Module/namespace definition |
| `macro` | Rust | Macro definition |
| `constant` | Python, Ruby, Rust, Go | Named constant |
| `variable` | Python, Go | Module-level variable |
| `package` | Go | Package declaration |
| `imports` | Python, Ruby | Aggregated import chunk |
| `import` | Go | Individual import statement |
| `use` | Rust | Use declaration |
| `require` | Go (go.mod) | Go module requirement |

**When to introduce new kinds**: Only when the construct is a primary searchable
symbol, no existing kind fits, and the construct appears in most files of that
language. Document new kinds in this table.

---

## Test Template and Requirements

**File**: `crates/maproom/tests/{LANG}_parser_test.rs`
**Import**: `use crewchief_maproom::indexer::parser;`
**Naming**: `#[test] fn test_{LANG}_{construct}()`

```rust
use crewchief_maproom::indexer::parser;

#[test]
fn test_{LANG}_class_with_methods() {
    let source = r#"
// Representative {Lang} code here
"#;
    let chunks = parser::extract_chunks(source, "{EXT}");
    let class_chunk = chunks.iter()
        .find(|c| c.symbol_name == Some("MyClass".to_string()))
        .expect("MyClass not found");
    assert_eq!(class_chunk.kind, "class");
    assert_eq!(class_chunk.signature, Some("expected".to_string()));
    assert!(class_chunk.docstring.as_ref().unwrap().contains("expected text"));
    let metadata = class_chunk.metadata.as_ref().unwrap();
    assert_eq!(metadata["visibility"], "public");
}
```

### Minimum 8 Required Tests

1. **Class/struct with methods** -- `test_{LANG}_class_with_methods()`
2. **Module/namespace** -- `test_{LANG}_module_definition()`
3. **Function parameters** -- `test_{LANG}_method_parameters()`
4. **Constants** -- `test_{LANG}_constants()`
5. **Imports** -- `test_{LANG}_imports()`
6. **Nested constructs** -- `test_{LANG}_nested_classes_modules()`
7. **Doc comments** -- `test_{LANG}_doc_comments()`
8. **Empty file** -- `test_{LANG}_empty_file()`

Additional recommended: syntax errors, visibility modifiers, language-specific constructs.

---

## Definition of Done

All commands must pass:

```bash
cargo build -p crewchief-maproom                        # Compiles cleanly
cargo test -p crewchief-maproom --test {LANG}_parser_test  # New tests pass (8+ tests)
cargo test -p crewchief-maproom                          # No regressions
cargo clippy -p crewchief-maproom                        # No warnings
cargo fmt --check -p crewchief-maproom                   # No formatting issues
cargo test -p crewchief-maproom detect_language           # Extension detection works
cargo test -p crewchief-maproom -- language_detector      # Enum methods work
```

Manual verification: `cargo run --bin crewchief-maproom -- scan --path /path/to/{LANG}-repo --repo test --worktree main` -- files detected and chunks produced.

---

## Maintaining This Document

**When**: After adding a language with novel patterns, when `SymbolChunk` struct
changes, when `extract_chunks()` dispatch changes, when new standard kinds are
introduced, or when new pitfalls are discovered.

**How**: Update the specific section. Add new kinds to the Standard Kind Values
table. Append new pitfalls as the next numbered entry. Verify code templates
still compile (modulo placeholders). Update scope validation note if needed.

**Owner**: Maproom indexer team. Changes reviewed by someone familiar with
tree-sitter integration patterns in `parser.rs`.
