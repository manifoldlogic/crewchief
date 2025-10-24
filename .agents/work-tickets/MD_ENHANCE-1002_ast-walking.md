# Ticket: MD_ENHANCE-1002: AST Walking

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
Implement tree traversal logic to walk the markdown AST, extracting headings with their levels, code blocks with language tags, and identifying tables and lists. This builds the core extraction pipeline on top of the parser setup.

## Background
With the tree-sitter parser initialized, we need to systematically walk the AST to extract all markdown elements. Unlike regex which operates on flat text, tree-sitter provides a hierarchical AST that we traverse recursively to find all document structures.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 17-21

## Acceptance Criteria
- [ ] Tree cursor traversal implemented recursively
- [ ] All heading levels (h1-h6) extracted with accurate level numbers
- [ ] Code blocks extracted with language tags (when present)
- [ ] Tables identified and boundaries determined
- [ ] Lists (ordered and unordered) detected
- [ ] Line numbers captured for all elements
- [ ] Traversal completes without errors on real documentation

## Technical Requirements
- Implement `walk_tree()` method that recursively traverses AST nodes
- Extract heading level from marker count (# = h1, ## = h2, etc.)
- Extract heading text content from `heading_content` node
- Identify `fenced_code_block` nodes and extract language from `info_string`
- Detect `table` nodes and parse row/column structure
- Detect `list` nodes (both `bullet_list` and `ordered_list`)
- Capture start and end line positions for each element
- Handle malformed or incomplete markdown gracefully

Architecture Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md` lines 104-143

## Implementation Notes

### Tree Cursor Pattern
```rust
fn walk_tree(&mut self,
             cursor: &mut TreeCursor,
             source: &str,
             chunks: &mut Vec<Chunk>) {

    match cursor.node().kind() {
        "atx_heading" => {
            let chunk = self.process_heading(cursor.node(), source);
            chunks.push(chunk);
        },
        "fenced_code_block" => {
            let chunk = self.process_code_block(cursor.node(), source);
            chunks.push(chunk);
        },
        "table" => {
            let chunk = self.process_table(cursor.node(), source);
            chunks.push(chunk);
        },
        _ => {}
    }

    // Recurse to children
    if cursor.goto_first_child() {
        loop {
            self.walk_tree(cursor, source, chunks);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}
```

### Element Extraction
- **Headings**: Count `#` characters to determine level, extract text from child nodes
- **Code blocks**: Parse `info_string` for language, extract raw code content
- **Tables**: Identify header row, body rows, column count
- **Lists**: Track nesting level, distinguish bullet vs numbered

### Edge Cases
- Headings without content
- Code blocks without language tags
- Nested lists
- Malformed tables

Reference Architecture: lines 117-131 for node type matching

## Dependencies
- MD_ENHANCE-1001 (Parser Setup) - MUST be completed first

## Risk Assessment
- **Risk**: AST structure doesn't match expected node types
  - **Mitigation**: Log unknown node types, create comprehensive test suite, reference tree-sitter-markdown grammar

- **Risk**: Recursive traversal causes stack overflow on deeply nested documents
  - **Mitigation**: Add depth limit, test with pathological cases, consider iterative approach if needed

- **Risk**: Line number calculations off by one
  - **Mitigation**: Extensive testing with known line positions, validate against source text

## Files/Packages Affected
- `crates/maproom/src/parser/markdown.rs` - Add walk_tree, process_heading, process_code_block methods
- `crates/maproom/src/parser/extractor.rs` - New file for element extraction logic
- `crates/maproom/tests/parser_test.rs` - Add traversal tests with expected element counts
- `crates/maproom/tests/fixtures/` - Sample markdown files with known structure
