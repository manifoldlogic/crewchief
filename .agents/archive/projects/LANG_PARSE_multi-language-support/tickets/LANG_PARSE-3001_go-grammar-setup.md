# Ticket: LANG_PARSE-3001: Go Tree-Sitter Grammar Setup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Set up tree-sitter-go grammar integration to enable Go language parsing in Maproom. This includes adding the dependency, configuring the parser, implementing basic go.mod analysis, and registering the parser for .go files.

## Background
As part of Phase 3 (Week 5, Task 1) of the LANG_PARSE project, we need to add Go language support to Maproom's semantic code indexing capabilities. Go is a widely-used systems programming language, and adding support will allow Maproom to index and search Go codebases including popular projects like Kubernetes. This ticket focuses on the foundational grammar setup and basic parsing infrastructure.

## Acceptance Criteria
- [x] tree-sitter-go dependency (>= 0.20) added to Cargo.toml
- [x] Go parsing functionality implemented (functional pattern like Python/Rust)
- [x] go.mod file parsing implemented for module information extraction
- [x] Parser registered to handle .go file extensions
- [x] Can successfully parse Go source files
- [x] Basic unit tests pass for Go parsing (12/12 tests)

## Technical Requirements
- Add tree-sitter-go >= 0.20 to `crates/maproom/Cargo.toml`
- Create GoParser struct following existing parser patterns (TypeScript/JavaScript/Python)
- Implement go.mod parsing to extract module name and dependencies
- Register Go parser in the parser factory/registry for .go files
- Handle Go-specific syntax (goroutines, channels, interfaces, struct embedding)
- Support parsing of both Go source files (.go) and Go module files (go.mod)

## Implementation Notes

### Parser Structure
Follow the established parser pattern from TypeScript/JavaScript/Python parsers:
- Create `crates/maproom/src/parser/go/mod.rs` as the module entry point
- Create `crates/maproom/src/parser/go/parser.rs` with the GoParser implementation
- Implement the Parser trait for GoParser

### Go.mod Analysis
The go.mod parser should extract:
- Module name/path
- Go version requirement
- Direct dependencies
- Indirect dependencies (if relevant)

### Tree-Sitter Go Integration
- tree-sitter-go provides the grammar for Go language
- The parser should handle Go-specific constructs:
  - Package declarations
  - Import statements
  - Function declarations (including methods)
  - Type declarations (structs, interfaces, type aliases)
  - Constant and variable declarations
  - Goroutine and channel operations

### Testing Approach
- Create basic unit tests in `crates/maproom/tests/parser/go_basic_test.rs`
- Test parsing of simple Go files with functions, structs, and interfaces
- Test go.mod parsing with module information
- Validate against Kubernetes sample files as integration test

## Dependencies
None - this is a parallel track independent of other Phase 3 work

## Risk Assessment
- **Risk**: tree-sitter-go version compatibility issues
  - **Mitigation**: Use version >= 0.20 which has stable API; test with multiple Go versions

- **Risk**: Go's unique concurrency primitives (goroutines, channels) may have complex AST structures
  - **Mitigation**: Start with basic constructs (functions, types) and iterate; reference tree-sitter-go documentation for AST structure

- **Risk**: go.mod parsing may be complex with replace directives and complex dependency specifications
  - **Mitigation**: Start with basic module name extraction; iterate to add more sophisticated parsing as needed

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add tree-sitter-go dependency
- `crates/maproom/src/parser/go/mod.rs` - New Go parser module
- `crates/maproom/src/parser/go/parser.rs` - New GoParser implementation
- `crates/maproom/src/parser/mod.rs` - Register Go parser (if needed)
- `crates/maproom/src/parser/factory.rs` - Register .go extension mapping (if applicable)
- `crates/maproom/tests/parser/go_basic_test.rs` - New test file for Go parsing

## Implementation Summary

The following changes were made to implement Go language support:

1. **Dependencies** (`Cargo.toml`):
   - Added `tree-sitter-go = "0.21"` to dependencies

2. **Parser Implementation** (`src/indexer/parser.rs`):
   - Added `lang_go()` function to provide tree-sitter-go language
   - Implemented `extract_go_chunks()` as main entry point
   - Implemented `walk_go_decls()` for AST traversal
   - Added symbol extraction functions:
     - `extract_go_function()` - extracts function declarations
     - `extract_go_method()` - extracts method declarations with receiver info
     - `extract_go_type_declaration()` - dispatches to type_spec extraction
     - `extract_go_type_spec()` - extracts structs, interfaces, and type aliases
     - `extract_go_const_declaration()` - dispatches to const_spec extraction
     - `extract_go_const_spec()` - extracts constant declarations
     - `extract_go_var_declaration()` - dispatches to var_spec extraction (handles var_spec_list)
     - `extract_go_var_spec()` - extracts variable declarations
     - `extract_go_package()` - extracts package name
     - `extract_go_doc_comment()` - extracts Go doc comments (// style)
   - Implemented `extract_gomod_chunks()` for go.mod file parsing (text-based)
   - Added "go" dispatch in `extract_chunks()` function
   - Added "gomod" dispatch in `extract_chunks()` function

3. **Language Detection** (`src/indexer/mod.rs`):
   - Added "go" extension mapping in `detect_language_from_path()`
   - Added special handling for "go.mod" filename -> "gomod" language
   - Added Go emoji (🔷) to language statistics output

4. **Tests** (`tests/go_parser_test.rs`):
   - Created comprehensive test suite with 12 tests covering:
     - Function parsing with doc comments
     - Struct parsing
     - Interface parsing
     - Method parsing with receiver metadata
     - Constants parsing (single and grouped)
     - Variables parsing (single and grouped)
     - Package parsing
     - Goroutines and channels (crash resistance)
     - Empty files and malformed code (error handling)
     - go.mod parsing (module name, version, dependencies)

All tests pass successfully. The implementation follows the existing pattern used by Python and Rust parsers in the codebase.
