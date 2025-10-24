# Ticket: MD_ENHANCE-1001: Parser Setup

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
Add tree-sitter-markdown dependency, configure parser initialization, create query patterns for markdown elements, and validate on sample files. This establishes the foundation for AST-based markdown parsing replacing the current regex approach.

## Background
Maproom currently uses basic regex-based markdown parsing which misses critical structural information like heading hierarchy, code block languages, and nested elements. Tree-sitter provides accurate AST parsing that enables rich document understanding and improved search quality.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ANALYSIS.md` lines 5-11, 39-44

## Acceptance Criteria
- [ ] tree-sitter-markdown crate added to dependencies
- [ ] Parser initializes successfully with markdown language
- [ ] Query patterns defined for headings (h1-h6)
- [ ] Query patterns defined for code blocks
- [ ] Query patterns defined for links
- [ ] Parser tested on at least 3 sample markdown files
- [ ] No parsing errors on real documentation files

## Technical Requirements
- Add `tree-sitter-md` dependency to `Cargo.toml`
- Create `MarkdownParser` struct with parser instance and compiled queries
- Implement heading query pattern matching `atx_heading` nodes with levels 1-6
- Implement code block query pattern matching `fenced_code_block` with `info_string` and `code_fence_content`
- Implement link query pattern for internal and external links
- Initialize parser with `tree_sitter_md::language()`
- Create test suite with diverse markdown samples (nested headings, code blocks, tables, lists)

Architecture Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md` lines 13-58

## Implementation Notes

### Parser Initialization
```rust
use tree_sitter_md::language;

pub struct MarkdownParser {
    parser: Parser,
    heading_query: Query,
    code_query: Query,
    link_query: Query,
}
```

### Query Pattern Structure
- Heading query must capture level markers (h1-h6) and heading content
- Code block query must capture language info_string and code content
- Link query must distinguish between internal (#anchor), relative (./path), and external (http) links

### Test Coverage
- Nested headings (h1 > h2 > h3)
- Code blocks with and without language tags
- Mixed content (text, lists, tables)
- Edge cases (empty headings, special characters)

Reference Architecture: lines 29-44 for query pattern structure

## Dependencies
- None (first ticket in MD_ENHANCE project)

## Risk Assessment
- **Risk**: Tree-sitter-markdown crate compatibility or API changes
  - **Mitigation**: Pin to specific version, review crate documentation, create wrapper layer for abstraction

- **Risk**: Query patterns don't match all markdown variants
  - **Mitigation**: Test against CommonMark spec, GitHub-flavored markdown samples, and actual project docs

- **Risk**: Parser performance issues on large documents
  - **Mitigation**: Benchmark with various file sizes, implement streaming if needed

## Files/Packages Affected
- `crates/maproom/Cargo.toml` - Add tree-sitter-md dependency
- `crates/maproom/src/parser/` - New directory for parser module
- `crates/maproom/src/parser/markdown.rs` - New file implementing MarkdownParser
- `crates/maproom/src/parser/mod.rs` - Module declarations
- `crates/maproom/tests/parser_test.rs` - New test file with sample markdown files
