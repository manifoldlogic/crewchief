# Ticket: LANG_PARSE-1003: Python Import/Dependency Extraction

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
Implement comprehensive Python import and dependency extraction from Python source files, including standard imports, from imports, relative imports, and dynamic import detection. This enables Maproom to understand Python module dependencies and relationships.

## Background
Python dependency tracking is essential for semantic code search and understanding module relationships. Python supports multiple import styles (standard, from, relative) and dynamic imports, all of which need to be captured to build a complete dependency graph. This is Phase 1, Week 1, Task 3 of the LANG_PARSE project plan.

Import relationships will be stored in the `chunk_edges` table, enabling queries like "what modules does this file depend on" and "what files import this module."

## Acceptance Criteria
- [ ] Standard imports extracted (e.g., `import foo`)
- [ ] From imports with specific names extracted (e.g., `from foo import bar, baz`)
- [ ] Relative imports handled correctly (e.g., `from .foo import bar`, `from ..parent import module`)
- [ ] Dynamic imports detected (e.g., `__import__()`, `importlib.import_module()`)
- [ ] Import relationships stored in `chunk_edges` table with appropriate edge types
- [ ] All test cases pass for various import patterns

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
- **New file**: `crates/maproom/src/parser/python/imports.rs` - Import extraction logic
- **Modified**: `crates/maproom/src/parser/python/extractor.rs` - Integration of import extraction
- **Modified**: `crates/maproom/src/parser/python/mod.rs` - Module exports
- **New test**: `crates/maproom/tests/parser/python_imports_test.rs` - Comprehensive import tests
- **Modified**: `crates/maproom/src/db/schema.rs` - If edge types need to be extended
