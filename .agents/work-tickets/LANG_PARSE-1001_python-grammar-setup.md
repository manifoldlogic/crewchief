# Ticket: LANG_PARSE-1001: Python Tree-Sitter Grammar Setup

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
Set up tree-sitter-python grammar integration in the Maproom parser infrastructure. This establishes the foundation for parsing and indexing Python code files, enabling semantic search capabilities for Python codebases.

## Background
As part of Phase 1 (Week 1, Task 1) of the multi-language parser expansion, we need to add Python language support to Maproom's semantic search capabilities. The existing parser infrastructure (currently supporting TypeScript/JavaScript) needs to be extended with Python grammar support using tree-sitter-python. This is the first step in a series of language additions that will expand Maproom's code indexing capabilities beyond JavaScript/TypeScript to include Python, Go, Rust, and other languages.

## Acceptance Criteria
- [ ] tree-sitter-python dependency (>= 0.20) added to Cargo.toml
- [ ] Parser initializes successfully without errors
- [ ] PythonParser struct created implementing the Parser trait
- [ ] Basic parsing works without crashes for simple Python files
- [ ] Python parser is registered in the parser factory
- [ ] Basic unit tests demonstrate successful parsing

## Technical Requirements
- Add tree-sitter-python >= 0.20 to `crates/maproom/Cargo.toml` dependencies
- Initialize Python grammar in parser factory
- Create PythonParser struct that implements the existing Parser trait
- Ensure PythonParser can parse simple Python files without panicking
- Follow existing parser patterns from TypeScript/JavaScript implementation
- Include basic error handling for malformed Python syntax

## Implementation Notes
**Parser Structure:**
- The PythonParser should follow the same architectural pattern as the existing TypeScript/JavaScript parser
- Implement the Parser trait with methods for initialization, parsing, and AST traversal
- Use tree-sitter-python's language bindings to instantiate the parser

**Testing Strategy:**
- Create basic test cases with simple Python files (functions, classes, imports)
- Verify that the parser can successfully create an AST without panicking
- Test error handling with intentionally malformed Python code

**Reference Implementation:**
- Look at existing TypeScript/JavaScript parser implementation for architectural patterns
- The parser factory pattern should be extended to recognize `.py` file extensions
- Consider Python's significant whitespace when handling AST nodes

**Key Considerations:**
- Python uses indentation for block structure (unlike braces in C-style languages)
- Tree-sitter-python handles Python 2 and Python 3 syntax - default to Python 3 parsing
- AST node types will differ from JavaScript; document Python-specific node types for future chunk extraction work

## Dependencies
- Existing parser infrastructure in `crates/maproom/src/parser/`
- Parser trait definition
- Parser factory pattern
- No prerequisite tickets - this is the foundational ticket for Python support

## Risk Assessment
- **Risk**: tree-sitter-python version incompatibility with other tree-sitter dependencies
  - **Mitigation**: Test build immediately after adding dependency; consult tree-sitter version compatibility matrix if issues arise

- **Risk**: Parser trait may need modifications to accommodate Python-specific features
  - **Mitigation**: Start with minimal implementation; flag any trait limitations for discussion rather than modifying the core trait in this ticket

- **Risk**: Performance regression with additional language parser
  - **Mitigation**: This ticket focuses on correctness; performance optimization will be addressed in later tickets

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add tree-sitter-python dependency
- `crates/maproom/src/parser/python/mod.rs` - New module definition
- `crates/maproom/src/parser/python/parser.rs` - New PythonParser struct implementation
- `crates/maproom/src/parser/factory.rs` - Register Python parser for .py files
- `crates/maproom/tests/parser/python_basic_test.rs` - New basic parser tests
- `crates/maproom/src/parser/mod.rs` - Export Python parser module (if needed)
