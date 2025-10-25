# Ticket: LANG_PARSE-2001: Rust Tree-Sitter Grammar Setup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 14/14 tests passed (13 unit tests, 1 integration test)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Add tree-sitter-rust dependency and create the foundational Rust parser infrastructure to enable parsing Rust source files. This includes implementing Rust parsing functions, basic macro handling, and registering the parser for .rs files.

## Background
As part of Phase 2 of the language parsing expansion (Week 3, Task 1), we need to add Rust language support to Maproom's code indexing capabilities. This builds on the successful Python parser implementation from Phase 1 and extends our multi-language support to compiled systems programming languages.

Rust presents unique parsing challenges including macro expansions, complex trait systems, and lifetime annotations. For this initial setup, we will treat macros as opaque blocks and focus on establishing the core parsing infrastructure. More sophisticated macro handling can be added in future iterations.

## Acceptance Criteria
- [x] tree-sitter-rust dependency (>= 0.20) added to Cargo.toml
- [x] Rust parsing functionality implemented (functional pattern like Python parser)
- [x] Basic macro handling working (treat as opaque blocks initially)
- [x] Parser registered for .rs file extensions
- [x] Can successfully parse Rust standard library files
- [x] Basic test suite passes with sample Rust code

## Technical Requirements
- Add tree-sitter-rust >= 0.20 to `crates/maproom/Cargo.toml` dependencies
- Implement Rust parsing functionality following the functional pattern from Python parser
- Handle Rust-specific syntax: modules, functions, structs, enums, traits, impl blocks
- Basic macro handling: treat macro invocations and definitions as opaque blocks for now
- Register the Rust parser for .rs file extensions in language detection and dispatch
- Follow the same architectural patterns established by the Python parser implementation

## Implementation Notes

### Parser Structure
The RustParser should follow the same architecture as PythonParser:
- Implement the `Parser` trait from `crates/maproom/src/parser/mod.rs`
- Create a dedicated module at `crates/maproom/src/parser/rust/`
- Use tree-sitter query patterns to extract symbols

### Macro Handling Strategy
For this initial implementation:
- Detect macro invocations (e.g., `println!()`, `vec![]`)
- Detect macro definitions (`macro_rules!`)
- Extract them as symbols but don't attempt to expand them
- Mark them with a special symbol type (e.g., `SymbolType::Macro`)

### Symbol Extraction Priority
Focus on extracting these Rust constructs:
1. Functions (`fn` keyword)
2. Structs (`struct` keyword)
3. Enums (`enum` keyword)
4. Traits (`trait` keyword)
5. Implementations (`impl` blocks)
6. Modules (`mod` keyword)
7. Constants and statics

### Testing Approach
- Test with simple Rust files first (basic functions and structs)
- Test with Rust standard library files (e.g., from `std::collections`)
- Verify macro handling doesn't crash the parser
- Ensure line numbers and ranges are accurate

## Dependencies
- None (runs parallel to Python parser work in Phase 1)
- The Parser trait and parser infrastructure already exist from the TypeScript/JavaScript implementation

## Risk Assessment
- **Risk**: tree-sitter-rust API differences from tree-sitter-javascript
  - **Mitigation**: Review tree-sitter-rust documentation and examples; follow patterns from existing parsers

- **Risk**: Macro expansion complexity may require more than "opaque blocks"
  - **Mitigation**: Start simple with opaque blocks; document limitations; plan for enhanced macro support in Phase 3 if needed

- **Risk**: Rust's lifetime and trait system complexity
  - **Mitigation**: Focus on symbol extraction, not full semantic analysis; lifetimes and traits can be captured as part of function/type signatures without special handling initially

- **Risk**: Performance issues with large Rust codebases
  - **Mitigation**: Use the same chunking strategy as other parsers; monitor memory usage during testing

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add tree-sitter-rust dependency
- `crates/maproom/src/indexer/parser.rs` - Rust parser implementation (added to existing file)
- `crates/maproom/tests/rust_parser_test.rs` - New test file for Rust parser (13 tests)
- `crates/maproom/tests/integration_rust_test.rs` - Integration test with real Rust file

## Implementation Notes (for verification)

### Changes Made:
1. **Dependency Added**: Added `tree-sitter-rust = "0.21"` to Cargo.toml (version 0.21 for compatibility)
2. **Language Function**: Added `lang_rust()` function to load tree-sitter-rust grammar
3. **Dispatch Logic**: Updated `extract_chunks()` to route .rs files to `extract_rust_chunks()`
4. **Parser Implementation**: Added comprehensive Rust parsing functions:
   - `extract_rust_chunks()` - Main entry point for Rust parsing
   - `walk_rust_decls()` - Recursive AST walker
   - `extract_rust_function()` - Extracts functions with visibility, async, const, unsafe modifiers
   - `extract_rust_struct()` - Extracts structs with generics
   - `extract_rust_enum()` - Extracts enums with generics
   - `extract_rust_trait()` - Extracts traits with generics
   - `extract_rust_impl()` - Extracts impl blocks (both inherent and trait impls)
   - `extract_rust_module()` - Extracts modules
   - `extract_rust_constant()` - Extracts const and static items
   - `extract_rust_macro()` - Extracts macro_rules! definitions (opaque blocks)
   - Helper functions for visibility, function modifiers, signatures, and doc comments

### Symbol Extraction:
- **Functions**: Extracts name, signature (with pub/async/const/unsafe), doc comments, metadata
- **Structs**: Extracts name, generics, doc comments, visibility
- **Enums**: Extracts name, generics, doc comments, visibility
- **Traits**: Extracts name, generics, doc comments, visibility
- **Impl Blocks**: Extracts both inherent impls and trait impls with proper names
- **Modules**: Extracts name, doc comments, visibility
- **Constants/Statics**: Extracts name, type, visibility
- **Macros**: Extracts macro_rules! definitions as opaque blocks

### Metadata Captured:
- Visibility modifiers (pub, pub(crate), private)
- Function modifiers (async, const, unsafe)
- Generic type parameters
- Doc comments (/// and //! style)

### Testing:
- 13 unit tests in `rust_parser_test.rs` covering all symbol types
- 1 integration test with real-world Rust file
- All tests passing
- Graceful handling of malformed code (no panics)
- Macro invocations handled correctly

### Verification Steps:
1. Run `cargo test --test rust_parser_test` - should pass all 13 tests
2. Run `cargo test --test integration_rust_test` - should extract 13 chunks from sample file
3. Check that .rs files are recognized by language detection in `indexer/mod.rs` (already present at line 84)
4. Verify no compilation warnings for the parser module
