# Ticket: MD_ENHANCE-3001: Code Block Processing

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (55 tests: 18 code block + 27 markdown + 10 section boundary)
- [x] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Extract code blocks as separate searchable chunks with language tags, link them to their parent heading sections, create targeted search chunks for code examples, and preserve formatting. This enables users to search specifically for code examples in documentation.

## Background
Documentation code blocks are crucial for developers searching for examples. By extracting them as separate chunks with language metadata, we enable targeted searches like "authentication example" to find the actual code, not just the documentation text. Linking to parent sections maintains context.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 57-61

## Acceptance Criteria
- [x] Code blocks extracted as dedicated chunks separate from section chunks
- [x] Language tag captured from info_string (e.g., ```typescript -> "typescript")
- [x] Code blocks without language tags marked as "plain" or "unknown"
- [x] Parent heading path stored in code block metadata
- [x] Code content preserved with exact formatting (indentation, newlines)
- [x] Lines of code counted and stored
- [x] 100% of code blocks detected and extracted

## Technical Requirements
- Implement `process_code_block(node, source)` method
- Extract language from `info_string` child node if present
- Extract raw code content from `code_fence_content` node
- Create chunk with `SymbolKind::CodeBlock` type
- Store language, parent_heading, and lines_of_code in metadata
- Generate meaningful symbol_name like "Code: typescript" or "Code: bash"
- Preserve all whitespace and formatting in code content
- Handle edge cases: empty code blocks, no language tag, invalid language

Architecture Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md` lines 166-183

## Implementation Notes

### Code Block Extraction
```rust
fn process_code_block(&self, node: Node, source: &str) -> Chunk {
    let language = self.get_code_language(node, source);
    let code = self.get_code_content(node, source);

    Chunk {
        symbol_name: format!("Code: {}", language.unwrap_or("plain")),
        kind: SymbolKind::CodeBlock,
        preview: code.chars().take(200).collect(),
        start_line: node.start_position().row,
        end_line: node.end_position().row,
        metadata: json!({
            "language": language,
            "parent_heading": self.hierarchy.get_parent_path(),
            "lines_of_code": code.lines().count()
        }),
    }
}
```

### Language Detection
- Parse info_string node text
- Trim whitespace
- Normalize to lowercase
- Common languages: typescript, javascript, rust, python, bash, json, yaml

### Content Preservation
- Extract exact byte range from source
- Don't trim or modify whitespace
- Preserve empty lines
- Include all special characters

### Metadata Schema
```json
{
  "language": "typescript",
  "parent_heading": "API Reference > Authentication",
  "lines_of_code": 15,
  "has_imports": true,
  "is_inline": false
}
```

Reference Architecture: lines 166-182 for code block processing

## Dependencies
- MD_ENHANCE-2001 (Parent Tracking) - Needed for parent_heading metadata
- MD_ENHANCE-2002 (Section Boundaries) - Understanding of content organization

## Risk Assessment
- **Risk**: Language tag parsing misses non-standard formats (e.g., ```js {1-3})
  - **Mitigation**: Test with various info_string formats, extract first word only, handle annotations

- **Risk**: Code content extraction corrupts special characters or escape sequences
  - **Mitigation**: Use byte-level extraction, preserve UTF-8 encoding, test with various languages

- **Risk**: Large code blocks create oversized chunks
  - **Mitigation**: Monitor chunk sizes, implement splitting for very large blocks if needed

## Files/Packages Affected
- `crates/maproom/src/indexer/parser.rs` - Updated extract_code_block function and HierarchyTracker
- `crates/maproom/tests/code_blocks_test.rs` - New test file with 18 comprehensive tests
- `crates/maproom/tests/markdown_parser_test.rs` - Updated test to match new behavior

## Implementation Summary

### Changes Made

1. **Added `get_current_path()` method to HierarchyTracker** (lines 50-61)
   - Returns the full breadcrumb path from the current heading stack
   - Used by code blocks to link to their parent section
   - Non-mutating, preserves stack state

2. **Updated `extract_code_block()` function** (lines 268-319)
   - Added `hierarchy: &HierarchyTracker` parameter
   - Extracts language from info_string, handling annotations like "typescript {1-3}"
   - Uses first word from info_string to handle edge cases
   - Stores parent_path from hierarchy in metadata
   - Changed language field: now always a string ("plain" instead of null)
   - Metadata includes: language, parent_path, lines_of_code

3. **Updated call site** (line 135)
   - `extract_code_block(source, node, chunks, hierarchy)` now passes hierarchy

4. **Created comprehensive test suite** (`tests/code_blocks_test.rs`)
   - 18 tests covering all acceptance criteria
   - Tests language extraction (typescript, rust, python, bash, json, yaml)
   - Tests code blocks without language tags (marked as "plain")
   - Tests parent_path linking at various nesting levels
   - Tests lines_of_code counting
   - Tests edge cases: empty blocks, single line, end of file, annotations
   - Tests 100% detection rate

5. **Fixed existing test** (`tests/markdown_parser_test.rs`)
   - Updated test_markdown_code_blocks to expect "plain" instead of null
   - All 27 existing markdown tests still pass

### Acceptance Criteria Coverage

✓ Code blocks extracted as dedicated chunks separate from section chunks
  - Verified in test_code_blocks_separate_from_headings

✓ Language tag captured from info_string (e.g., ```typescript -> "typescript")
  - Verified in test_code_block_language_tags (6 languages tested)
  - Handles annotations: test_code_block_info_string_with_annotations

✓ Code blocks without language tags marked as "plain"
  - Verified in test_code_block_without_language_tag

✓ Parent heading path stored in code block metadata
  - Verified in test_code_block_parent_path_simple
  - Verified in test_code_block_parent_path_nested (full breadcrumb)
  - Verified in test_code_block_parent_path_multiple_sections

✓ Code content preserved with exact formatting
  - Content extraction uses tree-sitter byte-level extraction
  - Formatting preserved via code_fence_content node

✓ Lines of code counted and stored
  - Verified in test_code_block_lines_of_code
  - Verified in test_code_block_empty (0 lines)
  - Verified in test_code_block_single_line (1 line)

✓ 100% of code blocks detected and extracted
  - Verified in test_code_block_100_percent_detection (4 blocks)
  - Verified in test_multiple_code_blocks_detection (3 blocks)
  - Verified in test_code_block_real_world_readme (4 blocks)

### Test Results

All tests pass:
- 18/18 new code block tests pass
- 27/27 existing markdown parser tests pass
- 10/10 section boundaries tests pass
- No regressions introduced
