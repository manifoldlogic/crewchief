# Ticket: LANG_PARSE-3001: Go Tree-Sitter Grammar Setup

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

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
- [ ] tree-sitter-go dependency (>= 0.20) added to Cargo.toml
- [ ] GoParser struct created with basic parsing capabilities
- [ ] go.mod file parsing implemented for module information extraction
- [ ] Parser registered to handle .go file extensions
- [ ] Can successfully parse Kubernetes sample files
- [ ] Basic unit tests pass for Go parsing

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
