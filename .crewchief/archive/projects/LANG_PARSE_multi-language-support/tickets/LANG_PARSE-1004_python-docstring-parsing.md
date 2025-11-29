# Ticket: LANG_PARSE-1004: Python Docstring Parsing

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 69/69 Python tests passed (18 docstring + 5 integration + 18 extraction + 12 parser + 16 import)
- [x] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement parsing and extraction of Python docstrings from function and class definitions. Support Google-style, NumPy-style, and basic reStructuredText docstring formats, storing parsed documentation in the chunks.summary field for semantic search.

## Background
Python uses docstrings (string literals immediately after function/class definitions) as the standard documentation format. Different projects use different conventions:
- **Google style**: Uses sections like `Args:`, `Returns:`, `Raises:`
- **NumPy style**: Uses underlined sections like `Parameters`, `Returns`
- **reStructuredText**: Uses field lists like `:param name:`, `:returns:`

Extracting and parsing these docstrings will significantly improve semantic search quality by capturing developer intent and API documentation. This is Phase 1, Week 1, Task 4 of the language parser expansion plan.

## Acceptance Criteria
- [x] Docstrings are extracted from documented Python symbols (functions, classes, methods)
- [x] Google-style docstrings are parsed correctly (Args:, Returns:, Raises: sections)
- [x] NumPy-style docstrings are parsed correctly (Parameters, Returns sections with underlines)
- [x] Basic reStructuredText field lists are supported (:param:, :returns:, :raises:)
- [x] Parsed docstrings are stored in the chunks.docstring field (Note: ticket mentioned "summary" but actual field is "docstring")
- [x] Tests verify all three docstring styles with realistic examples

## Technical Requirements
- Extract string_content nodes that immediately follow function_definition or class_definition nodes in the Python AST
- Implement docstring format detection (Google vs NumPy vs reST)
- Parse Google-style sections: Args:, Returns:, Raises:, Yields:, Examples:, Note:, Warning:
- Parse NumPy-style sections: Parameters, Returns, Raises, Yields (with underline markers)
- Parse basic reST field lists: :param name:, :type name:, :returns:, :rtype:, :raises:
- Handle multi-line parameter descriptions with proper indentation
- Store structured docstring data in chunks.summary field for semantic indexing
- Preserve formatting for code examples within docstrings

## Implementation Notes

### Tree-sitter Pattern
Use tree-sitter queries to find string_content nodes immediately after definitions:
```
(function_definition
  body: (block
    . (expression_statement
        (string (string_content) @docstring))))
```

### Format Detection
- **Google style**: Look for lines ending with `:` followed by indented content (Args:, Returns:)
- **NumPy style**: Look for section headers followed by lines of `---` or `===`
- **reST style**: Look for `:param` or `:returns:` field markers

### Parsing Strategy
1. Detect docstring format by examining first few lines
2. Split docstring into sections based on format-specific markers
3. Parse each section's content (parameter names, types, descriptions)
4. Generate a normalized summary for the chunks.summary field
5. Consider storing raw docstring as well for exact rendering

### Storage Format
Store in chunks.summary as structured text:
```
Brief description from first paragraph.

Parameters:
- param_name (type): Description
- another_param: Description

Returns:
- type: Description
```

## Dependencies
- **LANG_PARSE-1002** - Python symbol extraction must be complete to attach docstrings to
- Tree-sitter Python grammar (already available)

## Risk Assessment
- **Risk**: Docstring format variations and edge cases may be more complex than expected
  - **Mitigation**: Start with common patterns, add support for edge cases incrementally based on real-world examples

- **Risk**: Performance impact from parsing long docstrings with complex formatting
  - **Mitigation**: Set reasonable size limits (e.g., 10KB per docstring), test with large codebases

- **Risk**: Ambiguous format detection when docstrings mix styles
  - **Mitigation**: Use explicit heuristics and fallback to treating as plain text if format unclear

## Files/Packages Affected
- `crates/maproom/src/indexer/parser.rs` - Enhanced docstring parsing logic (added parsing functions)
- `crates/maproom/tests/python_docstrings_test.rs` - New test file with 18 comprehensive unit tests
- `crates/maproom/tests/python_docstrings_integration_test.rs` - New integration test file with 5 tests
- `crates/maproom/tests/fixtures/python/google_style_docstrings.py` - Google-style fixture
- `crates/maproom/tests/fixtures/python/numpy_style_docstrings.py` - NumPy-style fixture
- `crates/maproom/tests/fixtures/python/rst_style_docstrings.py` - reST-style fixture

## Implementation Summary

Successfully implemented Python docstring parsing with support for all three major formats:

### Changes to `/workspace/crates/maproom/src/indexer/parser.rs`:
1. Enhanced `extract_python_docstring()` to call new parsing logic
2. Added `detect_docstring_format()` - Detects Google, NumPy, reST, or Plain format
3. Added `parse_python_docstring()` - Main dispatcher for format-specific parsing
4. Added `parse_google_docstring()` - Parses Google-style (Args:, Returns:, Raises:, Yields:, Examples:, Note:, Warning:, Attributes:)
5. Added `parse_numpy_docstring()` - Parses NumPy-style (Parameters, Returns, Raises, Yields, Notes, Attributes with underlines)
6. Added `parse_rst_docstring()` - Parses reST-style (:param:, :type:, :returns:, :rtype:, :raises:)

All parsers normalize docstrings into a consistent format with:
- Brief description at the top
- Parameters: section with "- param_name (type): description" format
- Returns: section with return type and description
- Raises: section with "- ExceptionType: description" format

### Test Coverage:
- **18 unit tests** covering all three docstring styles with various scenarios
- **5 integration tests** using realistic fixture files
- All existing Python parser tests (12) still pass
- Total: **35 tests** validating docstring parsing functionality

### Key Features:
- Automatic format detection (no manual configuration needed)
- Multi-line parameter descriptions supported
- Handles decorated functions, async functions, classes, and methods
- Preserves plain docstrings without special formatting
- Graceful handling of edge cases (empty docstrings, no docstrings, mixed styles)

### Files Modified:
1. `/workspace/crates/maproom/src/indexer/parser.rs` - Added ~440 lines of docstring parsing logic
2. Created 3 comprehensive test fixture files (~200 lines each)
3. Created 2 test files with 23 total tests

All acceptance criteria met and verified with comprehensive test coverage.
