# MD_ENHANCE Analysis: Enhanced Markdown Support

## Problem Space

### Current Limitations
Maproom uses basic regex-based markdown parsing, missing:
- Parent heading hierarchy tracking
- Code block language detection
- List and table awareness
- Nested structure handling
- Cross-reference detection

The tree-sitter-markdown integration was deferred in favor of the simpler regex approach, but this limits the quality of documentation indexing.

### Industry Context
Documentation is critical for code understanding:
- GitHub uses tree-sitter for markdown rendering
- Docusaurus provides structured doc parsing
- MkDocs Material offers advanced search
- Obsidian implements bidirectional linking

### Current State
From the markdown-indexing-plan.md:
- ✅ Basic heading chunking works
- ✅ Files are indexed (18 markdown files)
- ⚠️ Tree-sitter integration skipped
- ⚠️ No parent hierarchy
- ⚠️ No code block extraction

## Key Insights

### 1. Documentation Structure Matters
Markdown documents have rich structure that aids understanding:
- Heading hierarchy provides context
- Code blocks contain examples
- Links show relationships
- Tables summarize data

### 2. Tree-Sitter Enables Precision
Unlike regex, tree-sitter provides:
- Accurate parsing of edge cases
- Nested structure understanding
- Consistent AST representation
- Language-specific code blocks

### 3. Documentation-Code Linking
Documentation references code that should be linked:
- API documentation → implementation
- Examples → actual usage
- Tutorials → complete code

### 4. Search Intent Differs
Users search documentation differently than code:
- Conceptual queries more common
- Natural language predominates
- Context more important

## Success Criteria

### Functional Requirements
- [ ] Parse markdown with tree-sitter
- [ ] Extract heading hierarchy
- [ ] Identify code blocks with language
- [ ] Parse tables and lists
- [ ] Detect cross-references

### Quality Requirements
- [ ] Heading context preserved
- [ ] Code blocks searchable
- [ ] Parent relationships tracked
- [ ] Accurate chunking

## Risk Assessment

### Technical Risks
1. **Tree-sitter compatibility**
   - Mitigation: Test extensively
2. **Performance impact**
   - Mitigation: Benchmark and optimize
3. **Breaking existing index**
   - Mitigation: Migration strategy

## Recommendations

### MVP Scope
- Basic tree-sitter integration
- Heading hierarchy extraction
- Code block detection
- Simple parent tracking

### Full Scope
- Complete AST utilization
- Link resolution
- Table parsing
- Metadata extraction