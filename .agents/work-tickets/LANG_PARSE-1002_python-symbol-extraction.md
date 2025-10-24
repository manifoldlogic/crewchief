# Ticket: LANG_PARSE-1002: Python Symbol Extraction

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
Implement comprehensive symbol extraction for Python source code using tree-sitter. This includes extracting functions, methods, classes, inheritance relationships, global variables, and decorator metadata to enable semantic code search and analysis.

## Background
As part of Phase 1 of the multi-language parser expansion (LANG_PARSE project), Python symbol extraction is a critical foundation for enabling semantic search capabilities across Python codebases. Python's rich metadata including decorators, inheritance, and async constructs requires careful extraction to provide accurate code intelligence. This work builds on the Python grammar setup (LANG_PARSE-1001) and targets 95% accuracy for core symbol types.

## Acceptance Criteria
- [ ] Functions extracted with 95% accuracy including parameters and return type hints
- [ ] Classes extracted with 95% accuracy including docstrings
- [ ] Inheritance relationships captured for all classes with base classes
- [ ] Decorators metadata included for both functions and classes
- [ ] Async functions and async methods properly identified and extracted
- [ ] Global variables and constants extracted from module-level assignments
- [ ] All extraction logic covered by comprehensive unit tests
- [ ] Integration tests validate extraction on real-world Python code samples

## Technical Requirements
- Extract `function_definition` nodes with names, parameters, and type annotations
- Extract `class_definition` nodes with base classes and inheritance chains
- Extract `assignment` nodes for global variables at module scope
- Capture decorator information (`decorator` nodes) for functions and classes
- Handle async variants: `async_function_definition` and async methods
- Distinguish between regular functions, methods, class methods, and static methods
- Parse parameter lists including default values and type hints
- Extract docstrings for functions and classes
- Generate unique symbol IDs based on qualified names (module.class.method)

## Implementation Notes

### Core Components to Create

1. **Symbol Extractor** (`crates/maproom/src/parser/python/extractor.rs`):
   - Implement `PythonSymbolExtractor` struct
   - Walk tree-sitter AST and identify symbol nodes
   - Extract metadata for each symbol type
   - Handle nested scopes (module > class > method)
   - Generate qualified names for symbols

2. **Tree-sitter Queries** (`crates/maproom/src/parser/python/queries.scm`):
   - Define queries for function definitions
   - Define queries for class definitions
   - Define queries for assignments (global variables)
   - Define queries for decorators
   - Queries should capture node metadata (name, parameters, base classes, etc.)

3. **Test Suite** (`crates/maproom/tests/parser/python_extraction_test.rs`):
   - Unit tests for each symbol type extraction
   - Test edge cases: nested classes, decorated functions, async functions
   - Test inheritance extraction: single and multiple inheritance
   - Integration tests with real Python code samples
   - Accuracy validation tests to ensure 95% threshold

### Technical Approach

- Use tree-sitter's query system for efficient node matching
- Leverage tree-sitter-python grammar node types
- Build symbol context by tracking scope stack during traversal
- For qualified names, maintain path from module root to current symbol
- Handle Python-specific constructs:
  - Property decorators (@property, @setter, @deleter)
  - Class decorators and metaclasses
  - Async/await syntax
  - Type hints (PEP 484) in function signatures
  - Dataclasses and their fields

### Edge Cases to Handle

- Multiple decorators on single function/class
- Nested function definitions (closures)
- Lambda expressions (may exclude from initial scope)
- Class methods vs static methods vs instance methods
- Abstract base classes and abstract methods
- Generator functions and async generators

## Dependencies
- **LANG_PARSE-1001**: Python grammar setup and tree-sitter integration must be completed first
- External: tree-sitter-python grammar library

## Risk Assessment
- **Risk**: Python's dynamic nature may make some symbols difficult to extract statically (e.g., dynamically created classes)
  - **Mitigation**: Focus on statically defined symbols first; document limitations for dynamic constructs

- **Risk**: Complex decorator chains may be difficult to parse and represent
  - **Mitigation**: Extract decorator names as strings initially; enhanced decorator analysis can be phase 2

- **Risk**: Type hint syntax variations across Python versions (3.7+ vs 3.10+)
  - **Mitigation**: Test against multiple Python version syntax samples; tree-sitter should handle version differences

- **Risk**: Achieving 95% accuracy target may require extensive testing
  - **Mitigation**: Create comprehensive test corpus from popular Python projects; iterate on extraction logic based on test results

## Files/Packages Affected
- `crates/maproom/src/parser/python/extractor.rs` (new file) - Core symbol extraction logic
- `crates/maproom/src/parser/python/queries.scm` (new file) - Tree-sitter query definitions
- `crates/maproom/src/parser/python/mod.rs` (modify) - Module exports and integration
- `crates/maproom/tests/parser/python_extraction_test.rs` (new file) - Extraction test suite
- `crates/maproom/tests/fixtures/python/` (new directory) - Python test fixtures
