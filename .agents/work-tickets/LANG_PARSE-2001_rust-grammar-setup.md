# Ticket: LANG_PARSE-2001: Rust Tree-Sitter Grammar Setup

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
Add tree-sitter-rust dependency and create the foundational Rust parser infrastructure to enable parsing Rust source files. This includes setting up the RustParser struct, implementing basic macro handling, and registering the parser for .rs files.

## Background
As part of Phase 2 of the language parsing expansion (Week 3, Task 1), we need to add Rust language support to Maproom's code indexing capabilities. This builds on the successful Python parser implementation from Phase 1 and extends our multi-language support to compiled systems programming languages.

Rust presents unique parsing challenges including macro expansions, complex trait systems, and lifetime annotations. For this initial setup, we will treat macros as opaque blocks and focus on establishing the core parsing infrastructure. More sophisticated macro handling can be added in future iterations.

## Acceptance Criteria
- [ ] tree-sitter-rust dependency (>= 0.20) added to Cargo.toml
- [ ] RustParser struct created implementing the Parser trait
- [ ] Basic macro handling working (treat as opaque blocks initially)
- [ ] Parser registered for .rs file extensions
- [ ] Can successfully parse Rust standard library files
- [ ] Basic test suite passes with sample Rust code

## Technical Requirements
- Add tree-sitter-rust >= 0.20 to `crates/maproom/Cargo.toml` dependencies
- Create RustParser struct that implements the existing Parser trait from the parser module
- Handle Rust-specific syntax: modules, functions, structs, enums, traits, impl blocks
- Basic macro handling: treat macro invocations and definitions as opaque blocks for now
- Register the Rust parser in the parser factory for .rs file extensions
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
- `crates/maproom/src/parser/rust/mod.rs` - New module entry point
- `crates/maproom/src/parser/rust/parser.rs` - RustParser implementation
- `crates/maproom/src/parser/mod.rs` - Register Rust parser in factory
- `crates/maproom/tests/parser/rust_basic_test.rs` - New test file for Rust parser
- `crates/maproom/tests/fixtures/rust/` - Test fixtures directory (to be created)
