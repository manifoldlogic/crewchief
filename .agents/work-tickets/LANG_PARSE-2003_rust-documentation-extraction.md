# Ticket: LANG_PARSE-2003: Rust Documentation Extraction

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement documentation extraction for Rust code, capturing doc comments (///), module-level docs (//!), and code examples embedded in documentation. Documentation will be stored in the chunks.summary field and linked to their corresponding symbols.

## Background
Rust has a rich documentation system with several types of documentation comments:
- Doc comments (///) that document the following item
- Module-level docs (//!) that document the enclosing item
- Code examples embedded in doc comments (used for doctests)

This work is part of Phase 2, Week 3 of the language parsing expansion plan. Extracting and preserving this documentation is crucial for:
- Understanding code purpose and usage
- Enabling semantic search over documentation content
- Preserving code examples that demonstrate API usage
- Linking documentation context to symbols for richer search results

## Acceptance Criteria
- [ ] Doc comments (///) are extracted and associated with their symbols
- [ ] Module-level docs (//!) are captured and linked to modules
- [ ] Code examples in doc comments are preserved in their original formatting
- [ ] Documentation is stored in chunks.summary field
- [ ] Extracted documentation is linked to their corresponding symbols in the database
- [ ] All tests pass for Rust documentation extraction

## Technical Requirements
- Extract line_comment nodes that start with /// from tree-sitter AST
- Extract block_comment nodes that start with //! for module-level documentation
- Preserve markdown formatting and code blocks within documentation
- Identify and preserve code examples (typically in triple-backtick blocks)
- Store extracted documentation in the chunks.summary field
- Create proper associations between documentation chunks and symbol chunks
- Handle multi-line doc comments correctly
- Strip the /// or //! prefixes while preserving content formatting

## Implementation Notes

### File Structure
Create a new `docs.rs` module within the Rust parser to handle documentation extraction:
- Implement `extract_doc_comments()` function to find /// comments
- Implement `extract_module_docs()` function to find //! comments
- Implement `parse_code_examples()` to identify and preserve code blocks
- Update the main extractor to call documentation extraction during symbol processing

### Tree-sitter Node Handling
- Query for `line_comment` nodes in the AST
- Filter comments by their prefix (/// or //!)
- Associate doc comments with the next AST node (for ///)
- Associate module docs with the containing module (for //!)

### Code Example Preservation
- Detect markdown code blocks within doc comments (```rust, ```, etc.)
- Preserve indentation and formatting of code examples
- Maintain the relationship between examples and their parent documentation

### Integration Points
- Update `crates/maproom/src/parser/rust/extractor.rs` to call documentation extraction
- Ensure documentation chunks are created alongside symbol chunks
- Link documentation to symbols via chunk associations or metadata

### Testing Strategy
Create comprehensive tests in `crates/maproom/tests/parser/rust_docs_test.rs`:
- Test extraction of simple /// doc comments
- Test extraction of multi-line doc comments
- Test extraction of //! module docs
- Test code example preservation
- Test edge cases (empty docs, malformed comments, nested code blocks)

## Dependencies
- LANG_PARSE-2002 (Rust symbols) - Must be completed first as documentation extraction builds on symbol extraction infrastructure

## Risk Assessment
- **Risk**: Code examples in doc comments may have complex formatting that's difficult to preserve
  - **Mitigation**: Use tree-sitter's raw text extraction to preserve exact formatting, test with various real-world Rust projects

- **Risk**: Incorrect association between doc comments and symbols could lead to documentation being linked to wrong items
  - **Mitigation**: Carefully handle AST traversal order, test with complex module structures, verify associations in tests

- **Risk**: Performance impact from processing large amounts of documentation
  - **Mitigation**: Benchmark with documentation-heavy Rust projects, optimize text processing if needed

## Files/Packages Affected
- `crates/maproom/src/parser/rust/docs.rs` (new file)
- `crates/maproom/src/parser/rust/extractor.rs` (modified)
- `crates/maproom/src/parser/rust/mod.rs` (modified to export docs module)
- `crates/maproom/tests/parser/rust_docs_test.rs` (new test file)
- `crates/maproom/src/db/chunks.rs` (potentially modified if chunk schema needs updates)
