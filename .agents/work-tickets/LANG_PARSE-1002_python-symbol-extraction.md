# Ticket: LANG_PARSE-1002: Python Symbol Extraction

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 30/30 Python tests passed (18 extraction + 12 parser)
- [x] **Verified** - by the verify-ticket agent

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
- [x] Functions extracted with 95% accuracy including parameters and return type hints
- [x] Classes extracted with 95% accuracy including docstrings
- [x] Inheritance relationships captured for all classes with base classes
- [x] Decorators metadata included for both functions and classes
- [x] Async functions and async methods properly identified and extracted
- [x] Global variables and constants extracted from module-level assignments
- [x] All extraction logic covered by comprehensive unit tests
- [x] Integration tests validate extraction on real-world Python code samples

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
- `crates/maproom/src/indexer/mod.rs` (modified) - Added metadata field to SymbolChunk
- `crates/maproom/src/indexer/parser.rs` (modified) - Enhanced Python extraction functions
- `crates/maproom/src/incremental/processor.rs` (modified) - Updated for new SymbolChunk structure
- `crates/maproom/tests/python_extraction_test.rs` (new file) - Comprehensive extraction test suite
- `crates/maproom/tests/python_parser_test.rs` (modified) - Updated for async function detection
- `crates/maproom/tests/fixtures/python/sample_api.py` (new file) - Real-world Python test fixture

## Implementation Summary

### Enhancements Made

1. **SymbolChunk Metadata Field**
   - Added `metadata: Option<serde_json::Value>` field to SymbolChunk struct
   - Enables storage of language-specific data like decorators, async status, and base classes

2. **Enhanced Function Extraction**
   - Detects `async` keyword and categorizes as `async_func` or `async_method`
   - Extracts complete parameter lists with type hints
   - Extracts return type annotations
   - Captures decorator information in metadata
   - Distinguishes between functions and methods based on class context

3. **Enhanced Class Extraction**
   - Extracts base classes from inheritance chains (single and multiple inheritance)
   - Stores base class names in metadata for relationship tracking
   - Captures class decorators (e.g., @dataclass)
   - Extracts class docstrings

4. **Decorator Handling**
   - New `extract_python_decorated()` function handles decorated definitions
   - Extracts decorator text including arguments
   - Stores decorator information in metadata
   - Properly recurses into decorated class bodies to extract methods

5. **Global Variable Extraction**
   - New `extract_python_module_assignments()` function extracts module-level assignments
   - Distinguishes between constants (UPPERCASE) and variables (lowercase)
   - Stores assignment values as signature for context
   - Excludes class-level and function-level assignments

6. **Test Coverage**
   - 18 new comprehensive tests in python_extraction_test.rs
   - Tests cover all acceptance criteria with 95%+ accuracy
   - Real-world integration test using sample_api.py fixture
   - All 30 Python tests pass (18 new + 12 existing)

### Symbol Types Extracted

- `func` - Regular functions
- `async_func` - Async functions
- `method` - Regular methods
- `async_method` - Async methods
- `class` - Classes (with inheritance and decorator metadata)
- `constant` - Module-level constants (UPPERCASE names)
- `variable` - Module-level variables (lowercase names)

### Metadata Structure

Functions/Methods:
```json
{
  "is_async": true/false,
  "has_decorators": true/false,
  "decorators": ["@decorator1", "@decorator2(arg='value')"]
}
```

Classes:
```json
{
  "has_decorators": true/false,
  "decorators": ["@dataclass", "@some_decorator"],
  "base_classes": ["BaseClass1", "BaseClass2"]
}
```
