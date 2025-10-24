# Ticket: LANG_PARSE-1004: Python Docstring Parsing

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
Implement parsing and extraction of Python docstrings from function and class definitions. Support Google-style, NumPy-style, and basic reStructuredText docstring formats, storing parsed documentation in the chunks.summary field for semantic search.

## Background
Python uses docstrings (string literals immediately after function/class definitions) as the standard documentation format. Different projects use different conventions:
- **Google style**: Uses sections like `Args:`, `Returns:`, `Raises:`
- **NumPy style**: Uses underlined sections like `Parameters`, `Returns`
- **reStructuredText**: Uses field lists like `:param name:`, `:returns:`

Extracting and parsing these docstrings will significantly improve semantic search quality by capturing developer intent and API documentation. This is Phase 1, Week 1, Task 4 of the language parser expansion plan.

## Acceptance Criteria
- [ ] Docstrings are extracted from documented Python symbols (functions, classes, methods)
- [ ] Google-style docstrings are parsed correctly (Args:, Returns:, Raises: sections)
- [ ] NumPy-style docstrings are parsed correctly (Parameters, Returns sections with underlines)
- [ ] Basic reStructuredText field lists are supported (:param:, :returns:, :raises:)
- [ ] Parsed docstrings are stored in the chunks.summary field
- [ ] Tests verify all three docstring styles with realistic examples

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
- `crates/maproom/src/parser/python/docstrings.rs` - New file for docstring parsing logic
- `crates/maproom/src/parser/python/extractor.rs` - Update to extract and attach docstrings to symbols
- `crates/maproom/src/parser/python/mod.rs` - Add docstring module to Python parser
- `crates/maproom/tests/parser/python_docstrings_test.rs` - New test file with comprehensive docstring test cases
- `crates/maproom/tests/fixtures/python/` - Add sample Python files with various docstring styles for testing
