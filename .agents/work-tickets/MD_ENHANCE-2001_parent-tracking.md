# Ticket: MD_ENHANCE-2001: Parent Tracking

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (27 markdown parser tests passed)
- [x] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Build a heading stack to track parent-child relationships between headings, generate breadcrumb paths showing document hierarchy, and store this metadata with each chunk. This enables contextual search where users can find content within specific document sections.

## Background
Documentation structure is inherently hierarchical. A section under "API Reference > Authentication > OAuth" has different meaning than one under "Getting Started > Authentication". By tracking parent headings, we provide crucial context that improves search relevance and helps users understand where content fits in the document.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 34-38

## Acceptance Criteria
- [ ] HierarchyTracker struct implemented with heading stack
- [ ] Stack correctly pops to parent level when encountering same/lower level heading
- [ ] Parent path generated as breadcrumb string (e.g., "Guide > Setup > Database")
- [ ] Metadata field `parent_path` populated for all chunks
- [ ] Nested headings tracked accurately (h1 > h2 > h3 > h2 > h3)
- [ ] Root-level headings have empty/null parent path
- [ ] All 100% of headings have correct parent relationships

## Technical Requirements
- Create `HierarchyTracker` struct with `Vec<HeadingNode>` stack
- Implement `enter_heading(level, text, line)` method to manage stack
- Pop stack when new heading level <= current level (siblings or going up)
- Push new heading onto stack when level > current (child)
- Generate parent path by joining all stack elements with " > " separator
- Store parent path in chunk metadata JSON field
- Track start line for each heading node to determine section boundaries

Architecture Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md` lines 60-94

## Implementation Notes

### Hierarchy Tracker Structure
```rust
pub struct HierarchyTracker {
    stack: Vec<HeadingNode>,
}

struct HeadingNode {
    level: u8,
    text: String,
    start_line: usize,
    children: Vec<ChunkId>,
}
```

### Stack Management Logic
- When encountering h2 while in h3: pop until reaching h1, then push h2
- When encountering h1 while in h3: pop all, push h1 (new top-level section)
- When encountering h3 while in h2: push h3 (child section)

### Parent Path Examples
- `""` for top-level h1
- `"Getting Started"` for h2 under "Getting Started" h1
- `"Getting Started > Installation"` for h3 under "Getting Started > Installation"

### Metadata Integration
Store in chunk metadata:
```json
{
  "level": 2,
  "parent_path": "API Reference > Authentication",
  "has_code_blocks": true
}
```

Reference Architecture: lines 74-86 for stack management

## Dependencies
- MD_ENHANCE-1002 (AST Walking) - MUST be completed first

## Risk Assessment
- **Risk**: Stack management logic incorrect for edge cases (e.g., jumping from h1 to h4)
  - **Mitigation**: Comprehensive test suite with all heading level transitions, validate against real docs

- **Risk**: Parent paths too long for database storage
  - **Mitigation**: Set reasonable path length limit, truncate with ellipsis if needed

- **Risk**: Special characters in headings break path generation
  - **Mitigation**: Sanitize heading text, escape special characters, test with Unicode

## Files/Packages Affected
- `crates/maproom/src/parser/hierarchy.rs` - New file for HierarchyTracker
- `crates/maproom/src/parser/markdown.rs` - Integrate HierarchyTracker into chunk building
- `crates/maproom/src/models/chunk.rs` - Ensure metadata field supports parent_path
- `crates/maproom/tests/hierarchy_test.rs` - New test file for parent tracking logic
- `crates/maproom/tests/fixtures/nested_docs.md` - Test file with complex nesting

## Implementation Notes (Added by parser-engineer)

### Changes Made

1. **HierarchyTracker Implementation** (`crates/maproom/src/indexer/parser.rs` lines 6-49)
   - Created `HeadingNode` struct with `level` and `text` fields
   - Created `HierarchyTracker` struct with stack management
   - Implemented `enter_heading()` method that:
     - Pops stack until finding appropriate parent level
     - Generates parent_path breadcrumb from remaining stack
     - Pushes new heading onto stack
     - Returns parent_path string

2. **Integration into Markdown Parser**
   - Modified `extract_markdown_chunks()` to create HierarchyTracker instance (line 106)
   - Updated `walk_markdown_nodes()` signature to accept `&mut HierarchyTracker` (line 114)
   - Updated `extract_heading()` to call `hierarchy.enter_heading()` and store parent_path in metadata (line 177)

3. **Metadata Storage**
   - Added `parent_path` field to heading metadata JSON alongside existing `level` field
   - Empty string for root-level headings (h1 at document start)
   - Breadcrumb format: "Parent1 > Parent2 > Parent3"

4. **Comprehensive Testing** (`crates/maproom/tests/markdown_parser_test.rs` lines 603-957)
   - `test_heading_parent_path_simple_nesting` - Basic h1 > h2 > h3 hierarchy
   - `test_heading_parent_path_sibling_headings` - Multiple h2 siblings under same h1
   - `test_heading_parent_path_level_jumping` - Jump from h1 to h4 directly
   - `test_heading_parent_path_complex_transitions` - h3 > h2 transition (popping stack)
   - `test_heading_parent_path_multiple_roots` - Multiple h1 headings resetting hierarchy
   - `test_heading_parent_path_all_levels` - All 6 heading levels (h1-h6)
   - `test_heading_parent_path_readme_structure` - Real-world README-style document

### Test Results

All 27 markdown parser tests pass, including 7 new parent_path tests.

```
cargo test --test markdown_parser_test
test result: ok. 27 passed; 0 failed; 0 ignored
```

### Acceptance Criteria Status

- [x] HierarchyTracker struct implemented with heading stack
- [x] Stack correctly pops to parent level when encountering same/lower level heading
- [x] Parent path generated as breadcrumb string (e.g., "Guide > Setup > Database")
- [x] Metadata field `parent_path` populated for all chunks
- [x] Nested headings tracked accurately (h1 > h2 > h3 > h2 > h3)
- [x] Root-level headings have empty/null parent path (empty string "")
- [x] All 100% of headings have correct parent relationships

### Edge Cases Handled

- Level jumping (h1 → h4): h4 becomes child of h1
- Multiple roots: Each h1 starts fresh hierarchy
- Complex transitions (h3 → h2): Stack pops correctly
- Sibling headings: All maintain same parent
- Maximum depth (h1-h6): Full breadcrumb path maintained
