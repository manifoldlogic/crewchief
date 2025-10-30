# Ticket: LANG_PARSE-1003: Python Import/Dependency Extraction

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 46/46 Python tests passed (16 import + 18 extraction + 12 parser)
- [x] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement comprehensive Python import and dependency extraction from Python source files, including standard imports, from imports, relative imports, and dynamic import detection. This enables Maproom to understand Python module dependencies and relationships.

## Background
Python dependency tracking is essential for semantic code search and understanding module relationships. Python supports multiple import styles (standard, from, relative) and dynamic imports, all of which need to be captured to build a complete dependency graph. This is Phase 1, Week 1, Task 3 of the LANG_PARSE project plan.

Import relationships will be stored in the `chunk_edges` table, enabling queries like "what modules does this file depend on" and "what files import this module."

## Acceptance Criteria
- [x] Standard imports extracted (e.g., `import foo`)
- [x] From imports with specific names extracted (e.g., `from foo import bar, baz`)
- [x] Relative imports handled correctly (e.g., `from .foo import bar`, `from ..parent import module`)
- [x] Dynamic imports detected (e.g., `__import__()`, `importlib.import_module()`)
- [x] Import relationships stored in chunk metadata (edge creation will be handled by database layer)
- [x] All test cases pass for various import patterns (16 tests passing)

## Technical Requirements
- Extract `import_statement` nodes from tree-sitter AST
- Extract `import_from_statement` nodes from tree-sitter AST
- Parse module names and imported identifiers from AST nodes
- Handle relative imports with proper parent path resolution
- Detect dynamic import patterns using AST patterns or text analysis
- Create `chunk_edges` entries with edge type "imports" or "imports_dynamic"
- Handle edge cases: aliased imports (`import foo as bar`), wildcard imports (`from foo import *`)
- Support multi-line imports and grouped imports

## Implementation Notes

### Import Statement Types
1. **Standard imports**: `import_statement` nodes
   - Format: `import module_name [as alias]`
   - Extract: module name, optional alias

2. **From imports**: `import_from_statement` nodes
   - Format: `from module_name import identifier [as alias] [, ...]`
   - Extract: module name, imported identifiers, optional aliases

3. **Relative imports**: `import_from_statement` with dotted_name containing dots
   - Format: `from .module import identifier` or `from ..parent.module import identifier`
   - Extract: relative path depth (number of dots), module path, identifiers

4. **Dynamic imports**: Call expressions with specific functions
   - Patterns: `__import__('module')`, `importlib.import_module('module')`
   - Mark these with distinct edge type for tracking

### Tree-sitter Node Structure
```python
# Standard import
import_statement:
  name: dotted_name | identifier
  alias?: 'as' identifier

# From import
import_from_statement:
  module_name: relative_import | dotted_name
  name: dotted_name | import_list | aliased_import | wildcard_import
```

### Edge Storage
- Edge type: `"imports"` for static imports, `"imports_dynamic"` for dynamic
- Source chunk: The file/chunk containing the import
- Target: The imported module name (as string reference)
- Metadata: Include import style, aliases, line numbers

### Error Handling
- Invalid/malformed imports should be logged but not halt processing
- Missing module names should be skipped gracefully
- Circular imports are valid and should be stored

## Dependencies
- **LANG_PARSE-1001**: Python grammar and tree-sitter setup (prerequisite)
- **LANG_PARSE-1002**: Python function/class extraction (recommended to complete first for consistent patterns)

## Risk Assessment
- **Risk**: Tree-sitter Python grammar variations across Python versions (2.x vs 3.x)
  - **Mitigation**: Focus on Python 3.x syntax; document Python 2.x limitations

- **Risk**: Dynamic imports are difficult to analyze statically
  - **Mitigation**: Use pattern matching for common dynamic import forms; document that not all dynamic imports can be detected

- **Risk**: Relative imports require path resolution context
  - **Mitigation**: Store relative import paths as-is; resolution can be done at query time with package context

## Files/Packages Affected
- **Modified**: `crates/maproom/src/indexer/parser.rs` - Added Python import extraction functions (monolithic pattern)
- **New test**: `crates/maproom/tests/python_imports_test.rs` - Comprehensive import tests (16 test cases)
- **Modified**: `crates/maproom/tests/python_extraction_test.rs` - Updated test to account for imports chunk
- **Modified**: `crates/maproom/tests/python_parser_test.rs` - Updated test to account for imports chunk

## Implementation Notes

### Approach
The implementation follows the monolithic parser pattern in `crates/maproom/src/indexer/parser.rs`. Import extraction is integrated directly into the Python parser module rather than creating separate files.

### Key Features Implemented

1. **Standard Imports** (`import foo`, `import foo.bar`, `import foo as bar`)
   - Extracts module name and optional alias
   - Handles dotted module paths

2. **From Imports** (`from foo import bar, baz`)
   - Extracts module name and all imported identifiers
   - Handles comma-separated import lists
   - Supports aliased imports (`from foo import bar as b`)

3. **Relative Imports** (`from .foo import bar`, `from ..parent import module`)
   - Tracks relative depth (number of dots)
   - Preserves module path after dots
   - Stores relative_depth in metadata

4. **Dynamic Imports** (`__import__('module')`, `importlib.import_module('module')`)
   - Pattern matches common dynamic import functions
   - Extracts module name from string arguments
   - Marks as "dynamic" type for differentiation

5. **Edge Cases**
   - Wildcard imports (`from foo import *`) - sets is_wildcard flag
   - Multi-line imports with parentheses
   - Multiple imports on same line
   - Mixed import styles in same file

### Data Structure

Each import is stored as a `PythonImport` struct (serialized to JSON in metadata):
```rust
{
  "import_type": "standard" | "from" | "relative" | "dynamic",
  "module": "module.name",
  "names": ["imported", "names"],
  "aliases": [["original", "alias"]],
  "relative_depth": Optional<usize>,
  "line": i32,
  "is_wildcard": bool
}
```

### Chunk Storage

Imports are stored in a special `__imports__` chunk with kind "imports":
- Always inserted at index 0 (first chunk)
- Contains all imports in metadata under "imports" key
- Line range spans from first to last import statement
- This provides a consistent query interface for import relationships

### Testing

Created comprehensive test suite with 16 test cases covering:
- Standard imports (simple and dotted)
- Aliased imports
- From imports (single and multiple names)
- From imports with aliases
- Wildcard imports
- Relative imports (single and multiple dots)
- Dynamic imports (__import__ and importlib)
- Multi-line imports
- Mixed import styles
- Line number tracking
- Edge cases (no imports, complex real-world examples)

All tests pass, including existing Python parser tests.
