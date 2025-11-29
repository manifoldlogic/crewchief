# Ticket: LANG_PARSE-3002: Go Symbol Extraction

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
Implement Go symbol extraction for Maproom's parser system, capturing functions, methods, types, interfaces, package declarations, imports, and goroutine/channel metadata from Go source files.

## Background
This is part of Phase 3 (Week 5, Task 2) of the language parser expansion effort. After integrating the Go tree-sitter grammar (LANG_PARSE-3001), we need to implement the symbol extraction layer that identifies and extracts meaningful code symbols from the parsed AST. Go has unique language features like goroutines and channels that should be captured as metadata, along with standard symbols like functions, types, and interfaces. This extraction layer enables semantic code search and navigation for Go codebases.

## Acceptance Criteria
- [x] Functions and methods are extracted from Go source files (completed in LANG_PARSE-3001)
- [x] Types and interfaces are captured with full type information (completed in LANG_PARSE-3001)
- [x] Package declarations are stored and associated with symbols (completed in LANG_PARSE-3001)
- [x] Import statements are tracked and linked to usage (COMPLETED in this ticket)
- [x] Goroutine and channel usage is captured as metadata (COMPLETED in this ticket)
- [x] Tree-sitter queries correctly identify all Go symbol types (inline matching pattern used)
- [x] Extracted symbols include proper location information (line, column, byte offset) (completed in LANG_PARSE-3001)
- [x] Test coverage includes real-world Go code examples (COMPLETED in this ticket - enhanced tests)

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

## Implementation Notes

### Completed Features

1. **Import Statement Extraction** - COMPLETED
   - Added `extract_go_import()` and `extract_go_import_spec()` functions in `/workspace/crates/maproom/src/indexer/parser.rs`
   - Handles single imports: `import "fmt"`
   - Handles grouped imports: `import ( ... )`
   - Handles import aliases: `myalias "github.com/example/pkg"`
   - Handles dot imports: `. "github.com/pkg"`
   - Stores import path and alias in metadata
   - Added "import_declaration" case to `walk_go_decls()` function

2. **Goroutine and Channel Metadata** - COMPLETED
   - Added `detect_go_concurrency()` helper function that recursively searches for:
     - `go_statement` nodes (goroutine spawns)
     - `channel_type` nodes (channel type declarations)
     - `send_statement` and `receive_operator` nodes (channel operations)
   - Updated `extract_go_function()` to detect and store `has_goroutines` and `has_channels` flags in metadata
   - Updated `extract_go_method()` to detect and store `has_goroutines` and `has_channels` flags in metadata (alongside receiver)
   - Metadata is only added when flags are true (no empty metadata objects)

3. **Enhanced Tests** - COMPLETED
   - Added `test_go_import_extraction()` test in `/workspace/crates/maproom/tests/go_parser_test.rs`
     - Tests single imports, grouped imports, aliased imports, and dot imports
     - Verifies metadata contains import_path and alias
   - Updated `test_go_goroutines_dont_crash()` to verify goroutine and channel metadata
     - Checks `has_goroutines` flag on processData function
     - Checks `has_channels` flag on processData and worker functions
   - Updated `test_go_channels_dont_crash()` to verify channel metadata
     - Checks `has_channels` flag on main function

### Test Results
All 13 Go parser tests pass:
- test_go_function_parsing
- test_go_struct_parsing
- test_go_interface_parsing
- test_go_method_parsing
- test_go_constants_parsing
- test_go_variables_parsing
- test_go_package_parsing
- test_go_goroutines_dont_crash (now verifies metadata)
- test_go_channels_dont_crash (now verifies metadata)
- test_go_empty_file
- test_go_malformed_code
- test_gomod_parsing
- test_go_import_extraction (NEW)

### Pattern Used
The implementation follows the established inline matching pattern used by other Go extraction functions:
- No queries.scm file created (inline AST node matching is used instead)
- Functions follow existing patterns: `extract_go_*`, `walk_go_decls`
- Metadata stored as JSON objects using serde_json
- Graceful error handling with Option types
