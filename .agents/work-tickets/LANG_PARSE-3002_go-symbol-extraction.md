# Ticket: LANG_PARSE-3002: Go Symbol Extraction

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement Go symbol extraction for Maproom's parser system, capturing functions, methods, types, interfaces, package declarations, imports, and goroutine/channel metadata from Go source files.

## Background
This is part of Phase 3 (Week 5, Task 2) of the language parser expansion effort. After integrating the Go tree-sitter grammar (LANG_PARSE-3001), we need to implement the symbol extraction layer that identifies and extracts meaningful code symbols from the parsed AST. Go has unique language features like goroutines and channels that should be captured as metadata, along with standard symbols like functions, types, and interfaces. This extraction layer enables semantic code search and navigation for Go codebases.

## Acceptance Criteria
- [ ] Functions and methods are extracted from Go source files
- [ ] Types and interfaces are captured with full type information
- [ ] Package declarations are stored and associated with symbols
- [ ] Import statements are tracked and linked to usage
- [ ] Goroutine and channel usage is captured as metadata
- [ ] Tree-sitter queries correctly identify all Go symbol types
- [ ] Extracted symbols include proper location information (line, column, byte offset)
- [ ] Test coverage includes real-world Go code examples

## Technical Requirements
- Extract `function_declaration` and `method_declaration` nodes from Go AST
- Extract `type_declaration` and `interface_type` nodes with full type signatures
- Parse and store `package_clause` declarations
- Extract `import_declaration` and `import_spec` statements
- Identify and capture goroutine spawns (`go` statement) as metadata
- Identify and capture channel operations (`chan`, `<-`) as metadata
- Implement tree-sitter queries in `queries.scm` for all symbol types
- Handle Go-specific constructs: receivers, embedded types, type parameters (generics)
- Support both named and anonymous functions
- Track struct field declarations and methods
- Capture function signatures including parameters and return types

## Implementation Notes

### Tree-sitter Query Patterns
Create `queries.scm` with patterns for:
- `(function_declaration)` - standalone functions
- `(method_declaration)` - methods with receivers
- `(type_declaration)` - type definitions
- `(interface_type)` - interface declarations
- `(package_clause)` - package statements
- `(import_declaration)` - import blocks
- `(go_statement)` - goroutine spawns
- `(channel_type)` and send/receive operations

### Extractor Implementation
The `extractor.rs` should:
- Implement the `SymbolExtractor` trait for Go
- Use tree-sitter-go queries to locate symbols
- Extract full context including documentation comments
- Handle Go's unique receiver syntax for methods
- Capture visibility (exported vs unexported based on capitalization)
- Store goroutine/channel metadata in symbol attributes

### Testing Strategy
- Unit tests for each symbol type extraction
- Integration tests with real Go code samples
- Test edge cases: generics, embedded types, variadic functions
- Verify goroutine and channel metadata capture
- Test package and import resolution

### Code Structure
Follow the pattern established by TypeScript/JavaScript extractors:
- Separate query file (`queries.scm`) for tree-sitter patterns
- Extractor struct implementing standard extraction interface
- Symbol type mapping from Go AST nodes to internal representation
- Proper error handling for malformed Go code

## Dependencies
- **LANG_PARSE-3001**: Go grammar integration (prerequisite - must be completed first)
- tree-sitter-go crate (external dependency)
- Existing Maproom parser infrastructure

## Risk Assessment
- **Risk**: Go generics (type parameters) may have complex AST structure
  - **Mitigation**: Study tree-sitter-go grammar for generic syntax, start with simple cases and expand coverage iteratively

- **Risk**: Goroutine/channel metadata capture may be incomplete for complex patterns
  - **Mitigation**: Focus on direct `go` keyword and channel type declarations initially, document limitations for complex control flow

- **Risk**: Package resolution across multiple files may be incomplete
  - **Mitigation**: Focus on single-file symbol extraction first, package-level aggregation can be enhanced in future tickets

- **Risk**: Tree-sitter-go grammar version compatibility
  - **Mitigation**: Pin specific tree-sitter-go version, test with common Go version patterns (1.18+)

## Files/Packages Affected
- `crates/maproom/src/parser/go/extractor.rs` (new file)
- `crates/maproom/src/parser/go/queries.scm` (new file)
- `crates/maproom/src/parser/go/mod.rs` (modifications to export extractor)
- `crates/maproom/tests/parser/go_extraction_test.rs` (new test file)
- `crates/maproom/Cargo.toml` (if additional dependencies needed)
