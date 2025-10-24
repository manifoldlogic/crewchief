# Ticket: MD_ENHANCE-2002: Section Boundaries

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
Determine accurate section boundaries by calculating where each heading's content ends (when the next heading begins), handle nested sections correctly, include all related content (paragraphs, code blocks, lists) within sections, and manage orphan content that appears before any heading.

## Background
A heading chunk should include all content that belongs to that section, not just the heading text. For "## Installation", we want to include all paragraphs, code blocks, and lists until the next heading of equal or higher level. This creates meaningful, searchable chunks that provide complete context.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 40-44

## Acceptance Criteria
- [ ] Section end line calculated as (next_heading.start_line - 1)
- [ ] Nested sections correctly contained within parent sections
- [ ] All content between headings associated with correct section
- [ ] Orphan content (before first heading) handled gracefully
- [ ] Code blocks within sections included in section chunk
- [ ] Lists and tables within sections included
- [ ] Section boundaries tested with 100% accuracy

## Technical Requirements
- Implement `find_section_end(node, source, level)` method
- Scan forward from heading to find next heading of same or lower level (sibling or parent)
- For h2, section ends when encountering h1 or h2 (not h3/h4 which are children)
- For h1, section ends when encountering next h1
- Handle end-of-file case (section extends to document end)
- Extract all text content between start and end lines
- Include child headings' content within parent section
- Create separate chunk for orphan content with metadata flag

Architecture Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md` lines 145-164

## Implementation Notes

### Section End Detection
```rust
fn find_section_end(&self, heading_node: Node, source: &str, level: u8) -> usize {
    let start_line = heading_node.end_position().row;

    // Scan siblings for next heading at same or lower level
    let mut current = heading_node;
    while let Some(sibling) = current.next_sibling() {
        if sibling.kind() == "atx_heading" {
            let sibling_level = self.get_heading_level(sibling);
            if sibling_level <= level {
                // Found section boundary
                return sibling.start_position().row - 1;
            }
        }
        current = sibling;
    }

    // No boundary found, extends to end of file
    source.lines().count()
}
```

### Content Inclusion
- Section content = all lines from heading to section_end
- Includes nested headings and their content
- Includes code blocks, lists, tables within range
- Preserves formatting and whitespace

### Orphan Content
Content before first heading:
```json
{
  "symbol_name": "Document Preamble",
  "kind": "orphan_content",
  "metadata": {
    "is_orphan": true,
    "parent_path": ""
  }
}
```

### Edge Cases
- Empty sections (heading with no content)
- Sections with only whitespace
- Last section in document
- Documents with no headings

Reference Architecture: lines 148-157 for section content extraction

## Dependencies
- MD_ENHANCE-2001 (Parent Tracking) - MUST be completed first

## Risk Assessment
- **Risk**: Section boundary calculation includes too much or too little content
  - **Mitigation**: Extensive testing with known section boundaries, visual verification of chunks

- **Risk**: Nested sections double-count content
  - **Mitigation**: Clear inclusion rules, test with deeply nested docs, verify chunk overlap

- **Risk**: Large sections create oversized chunks
  - **Mitigation**: Implement max chunk size limit, split large sections if needed

## Files/Packages Affected
- `crates/maproom/src/parser/markdown.rs` - Add find_section_end, get_section_content methods
- `crates/maproom/src/parser/boundaries.rs` - New file for boundary detection logic
- `crates/maproom/src/parser/mod.rs` - Export boundary types
- `crates/maproom/tests/boundaries_test.rs` - Test section end detection
- `crates/maproom/tests/fixtures/sections.md` - Test file with various section structures
