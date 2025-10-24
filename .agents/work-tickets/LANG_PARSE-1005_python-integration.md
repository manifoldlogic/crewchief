# Ticket: LANG_PARSE-1005: Python Integration with Indexing Pipeline

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
Integrate Python parser with the Maproom indexing pipeline by updating language detection, parser factory registration, and file scanning to support Python (.py) files.

## Background
This ticket completes the Python language support implementation by connecting the Python parser (built in LANG_PARSE-1001 through 1004) to the indexing pipeline. Without this integration, the Python parser exists but is not invoked during scanning and indexing operations. This work ensures that .py files are detected, parsed, and indexed with correct language tagging.

This is Phase 1, Week 2, Task 1 from the planning document, focusing on the final integration steps to make Python a first-class supported language in Maproom.

## Acceptance Criteria
- [ ] .py files are correctly detected as Python language
- [ ] PythonParser is used automatically for .py files
- [ ] File scanner includes Python files during scan operations
- [ ] Indexed chunks are created with language="python" tag
- [ ] Integration tests verify end-to-end Python file indexing

## Technical Requirements
- Add `.py` extension to LanguageDetector mapping
- Register PythonParser in ParserFactory for Python language variant
- Update file scanner filter logic to include `.py` files
- Ensure Python chunks flow through the pipeline with correct metadata
- Add integration test covering full scan → parse → index flow for Python files

## Implementation Notes

### LanguageDetector Updates
- Location: `crates/maproom/src/parser/language_detector.rs`
- Add `.py` extension mapping to Language::Python
- Follow existing pattern used for TypeScript (.ts) and JavaScript (.js)

### ParserFactory Registration
- Location: `crates/maproom/src/parser/factory.rs`
- Register PythonParser in the factory's language match statement
- Ensure proper initialization with tree-sitter Python grammar

### File Scanner Updates
- Location: `crates/maproom/src/scanner/mod.rs`
- Add `.py` to accepted file extensions list
- Verify no conflicts with existing extension filters
- Ensure Python files are included in recursive directory scans

### Integration Testing
- Create comprehensive integration test at `crates/maproom/tests/integration/python_pipeline_test.rs`
- Test should verify:
  - Python file detection
  - Parser invocation
  - Chunk creation with correct metadata
  - Language tagging accuracy
- Use sample Python code with classes, functions, and docstrings

### Quality Checks
- Verify existing TypeScript/JavaScript indexing still works
- Confirm no performance regression in file scanning
- Validate that Python chunks have same structure as other language chunks

## Dependencies
- **LANG_PARSE-1001**: Python Grammar Setup (must be complete)
- **LANG_PARSE-1002**: Python Symbol Extraction (must be complete)
- **LANG_PARSE-1003**: Python Import Extraction (must be complete)
- **LANG_PARSE-1004**: Python Docstring Parsing (must be complete)

All prerequisite tickets must be completed and merged before this integration work begins.

## Risk Assessment
- **Risk**: Integration may reveal edge cases in Python parser not caught by unit tests
  - **Mitigation**: Comprehensive integration tests with diverse Python code samples; plan for quick iteration if issues found

- **Risk**: File scanner performance may degrade with additional file type
  - **Mitigation**: Benchmark scan performance before/after; Python files use same efficient filtering as existing languages

- **Risk**: Parser factory registration may conflict with existing parsers
  - **Mitigation**: Follow existing registration patterns exactly; verify all languages still work in integration tests

## Files/Packages Affected
- `crates/maproom/src/parser/language_detector.rs` - Add Python extension detection
- `crates/maproom/src/parser/factory.rs` - Register PythonParser
- `crates/maproom/src/scanner/mod.rs` - Include .py files in scanning
- `crates/maproom/tests/integration/python_pipeline_test.rs` - New integration test file
