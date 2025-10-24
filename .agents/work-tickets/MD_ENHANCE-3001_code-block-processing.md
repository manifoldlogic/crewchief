# Ticket: MD_ENHANCE-3001: Code Block Processing

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
Extract code blocks as separate searchable chunks with language tags, link them to their parent heading sections, create targeted search chunks for code examples, and preserve formatting. This enables users to search specifically for code examples in documentation.

## Background
Documentation code blocks are crucial for developers searching for examples. By extracting them as separate chunks with language metadata, we enable targeted searches like "authentication example" to find the actual code, not just the documentation text. Linking to parent sections maintains context.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 57-61

## Acceptance Criteria
- [ ] Code blocks extracted as dedicated chunks separate from section chunks
- [ ] Language tag captured from info_string (e.g., ```typescript -> "typescript")
- [ ] Code blocks without language tags marked as "plain" or "unknown"
- [ ] Parent heading path stored in code block metadata
- [ ] Code content preserved with exact formatting (indentation, newlines)
- [ ] Lines of code counted and stored
- [ ] 100% of code blocks detected and extracted

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
- `crates/maproom/src/parser/markdown.rs` - Add process_code_block, get_code_language, get_code_content methods
- `crates/maproom/src/models/symbol.rs` - Add SymbolKind::CodeBlock variant
- `crates/maproom/src/models/chunk.rs` - Ensure code block chunks stored correctly
- `crates/maproom/tests/code_blocks_test.rs` - New test file for code block extraction
- `crates/maproom/tests/fixtures/code_samples.md` - Test file with various code blocks
