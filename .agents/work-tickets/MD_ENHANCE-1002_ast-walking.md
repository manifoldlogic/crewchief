# Ticket: MD_ENHANCE-1002: AST Walking

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (20 markdown parser tests passed)
- [x] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement tree traversal logic to walk the markdown AST, extracting headings with their levels, code blocks with language tags, and identifying tables and lists. This builds the core extraction pipeline on top of the parser setup.

## Background
With the tree-sitter parser initialized, we need to systematically walk the AST to extract all markdown elements. Unlike regex which operates on flat text, tree-sitter provides a hierarchical AST that we traverse recursively to find all document structures.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 17-21

## Acceptance Criteria
- [x] Tree cursor traversal implemented recursively
- [x] All heading levels (h1-h6) extracted with accurate level numbers
- [x] Code blocks extracted with language tags (when present)
- [x] Tables identified and boundaries determined
- [x] Lists (ordered and unordered) detected
- [x] Line numbers captured for all elements
- [x] Traversal completes without errors on real documentation

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
- `crates/maproom/src/indexer/parser.rs` - Added table and list extraction (extract_table, extract_list)
- `crates/maproom/tests/markdown_parser_test.rs` - Added comprehensive tests for tables and lists

## Implementation Summary

### What Was Implemented

1. **Table Extraction** (`extract_table` function):
   - Detects `pipe_table` nodes in the AST
   - Counts rows by identifying `pipe_table_header` and `pipe_table_row` children
   - Counts columns by examining cells in the header row
   - Captures table boundaries (start_line, end_line)
   - Stores metadata: row count, column count, has_header flag
   - Symbol name format: "Table {rows}x{columns}"

2. **List Extraction** (`extract_list` function):
   - Detects `list` nodes in the AST
   - Counts `list_item` children to determine item count
   - Distinguishes ordered vs unordered by checking for `list_marker_dot` (ordered) vs `list_marker_minus` (unordered)
   - Captures list boundaries (start_line, end_line)
   - Stores metadata: list_type ("ordered" or "unordered"), item_count
   - Symbol name format: "List ({count} items)"

3. **AST Walking**:
   - Extended `walk_markdown_nodes()` match statement with "pipe_table" and "list" cases
   - Recursive traversal continues to work for all node types
   - Line numbers captured from tree-sitter node positions (row + 1 for 1-indexed)

4. **Comprehensive Tests** (8 new tests):
   - `test_markdown_table_extraction` - Basic table with header and data rows
   - `test_markdown_empty_table` - Header-only table edge case
   - `test_markdown_unordered_list` - Bullet list with multiple items
   - `test_markdown_ordered_list` - Numbered list with multiple items
   - `test_markdown_nested_list` - Nested list structure (outer list extracted)
   - `test_markdown_mixed_table_and_list` - Document with both tables and lists
   - `test_markdown_single_item_list` - Single-item list edge case

### Notes on Implementation

- Tree-sitter-md provides well-structured nodes for tables (`pipe_table`, `pipe_table_header`, `pipe_table_row`, `pipe_table_cell`)
- Lists are parsed as `list` nodes containing `list_item` children
- Nested lists are extracted as separate list chunks (inner and outer lists are separate nodes)
- All extraction functions follow the same pattern as existing `extract_heading()` and `extract_code_block()` functions
- Metadata is stored as JSON for flexible querying in the indexing system

### Test Results

All 20 markdown parser tests pass, including:
- 6 existing heading tests
- 3 existing code block tests
- 8 new table and list tests
- 3 existing edge case tests
