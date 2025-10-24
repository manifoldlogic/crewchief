# Ticket: LANG_PARSE-2002: Rust Symbol Extraction

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement comprehensive Rust symbol extraction capabilities for the Maproom parser, including functions, methods, structs, enums, traits, impl blocks, modules, use statements, generic parameters, and lifetime annotations.

## Background
This is Phase 2, Week 3, Task 2 of the LANG_PARSE project. Following the establishment of Rust grammar support (LANG_PARSE-2001), we need to extract meaningful symbols from Rust code to enable semantic search and code understanding. Rust's rich type system, trait implementations, and lifetime annotations require careful parsing to capture the full semantic structure of Rust codebases.

Rust symbol extraction is critical for indexing Rust projects in Maproom, enabling developers to search for functions, types, trait implementations, and understand code relationships through generic bounds and lifetime constraints.

## Acceptance Criteria
- [ ] Functions and methods are extracted with full signatures
- [ ] Structs, enums, and traits are captured with their fields/variants/methods
- [ ] Impl blocks are associated correctly with their target types
- [ ] Generic parameters and bounds are stored and indexed
- [ ] Lifetime annotations are parsed and preserved
- [ ] Module declarations and use statements are extracted
- [ ] Symbol visibility modifiers (pub, pub(crate), etc.) are captured
- [ ] All extracted symbols include proper source location information
- [ ] Test suite validates extraction across diverse Rust code patterns

## Technical Requirements
- Extract `function_item` nodes for standalone functions
- Extract `function_signature_item` and method nodes from impl blocks
- Extract `struct_item`, `enum_item`, `trait_item` nodes with their contents
- Parse `impl_item` blocks and associate methods with their implementing types
- Extract `mod` declarations (both inline and file-based modules)
- Parse `use_declaration` and `use_as_clause` nodes
- Capture `type_parameters` and `where_clause` for generic bounds
- Parse lifetime annotations (`'a`, `'static`, etc.) from function signatures and type definitions
- Store symbol relationships (which methods belong to which types)
- Handle associated types and constants in traits
- Extract macro invocations that define symbols (e.g., `derive` macros)
- Support Rust 2021 edition features

## Implementation Notes

### Tree-sitter Query Structure
Create comprehensive tree-sitter queries in `queries.scm` to capture:
1. **Functions**: `(function_item)` with parameters, return types, generics
2. **Methods**: Extract from `(impl_item (function_item))` with self parameter variants
3. **Types**: `(struct_item)`, `(enum_item)`, `(trait_item)` with all fields/variants
4. **Impl Blocks**: `(impl_item)` linking implementations to types, including trait impls
5. **Modules**: `(mod_item)` for both inline and file modules
6. **Imports**: `(use_declaration)` with full paths and aliases
7. **Generics**: `(type_parameters)`, `(lifetime)`, `(where_clause)` nodes
8. **Visibility**: Capture `pub`, `pub(crate)`, `pub(super)`, etc.

### Extractor Architecture
The `extractor.rs` file should:
- Implement visitor pattern for traversing Rust AST nodes
- Build symbol table with proper scoping (module hierarchy)
- Resolve impl blocks to their target types
- Handle generic context when extracting nested symbols
- Preserve lifetime bounds and constraints
- Extract documentation comments (`///`, `//!`)
- Handle edge cases: tuple structs, unit structs, function pointers, closures

### Symbol Representation
Store symbols with:
- Full qualified name (including module path)
- Symbol kind (function, method, struct, enum, trait, impl, etc.)
- Generic parameters with bounds
- Lifetime parameters
- Visibility level
- Source location (file, line, column range)
- Documentation text
- Relationships (impl -> type, method -> impl)

### Testing Strategy
- Unit tests for each symbol type extraction
- Integration tests with real-world Rust code samples
- Edge case tests: complex generics, nested modules, re-exports
- Performance tests with large Rust files
- Validation against known Rust projects (e.g., small crates from crates.io)

## Dependencies
- LANG_PARSE-2001 (Rust grammar setup) - REQUIRED
- Tree-sitter Rust parser integration must be complete
- Database schema should support storing Rust-specific symbol metadata

## Risk Assessment
- **Risk**: Rust's complex type system and generic syntax may lead to incomplete extraction
  - **Mitigation**: Start with common patterns, add edge cases iteratively. Use existing Rust projects as test corpus.

- **Risk**: Impl block association may be ambiguous for complex generic types
  - **Mitigation**: Store both the raw impl target and resolved type when possible. Document limitations for complex cases.

- **Risk**: Lifetime annotations have complex inference rules that may be hard to represent
  - **Mitigation**: Extract explicit lifetimes as written in source. Don't attempt lifetime inference (that's the compiler's job).

- **Risk**: Macro-generated code is not present in the AST
  - **Mitigation**: Document this limitation. Extract macro invocations themselves and their locations. Consider future expansion analysis if needed.

- **Risk**: Performance degradation with deeply nested generics or large impl blocks
  - **Mitigation**: Implement depth limits, timeouts, and incremental parsing strategies. Profile with real-world Rust codebases.

## Files/Packages Affected
- `crates/maproom/src/parser/rust/extractor.rs` - New file, main extraction logic
- `crates/maproom/src/parser/rust/queries.scm` - New file, tree-sitter queries for Rust
- `crates/maproom/src/parser/rust/mod.rs` - Update to export extractor
- `crates/maproom/tests/parser/rust_extraction_test.rs` - New test file
- `crates/maproom/tests/fixtures/rust/` - New directory with sample Rust files for testing
- `crates/maproom/src/parser/types.rs` - May need updates to symbol types for Rust-specific metadata
- `crates/maproom/src/db/schema.sql` - May need schema adjustments for Rust symbol relationships
