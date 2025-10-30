# MD_ENHANCE Plan: Enhanced Markdown Support

## Project Overview
Upgrade markdown parsing from regex-based to tree-sitter AST parsing, enabling rich document structure understanding and improved search quality.

## Phase 1: Tree-Sitter Integration (Week 1)

**Agent: parser-engineer**

### Tasks
1. **Parser Setup**
   - Add tree-sitter-markdown dependency
   - Configure parser initialization
   - Create query patterns
   - Test on sample files

2. **AST Walking**
   - Implement tree traversal
   - Extract headings with levels
   - Find code blocks with language
   - Identify tables and lists

**Acceptance Criteria:**
- [ ] Parser initializes successfully
- [ ] AST traversal works
- [ ] All markdown elements detected
- [ ] No parsing errors on real docs

## Phase 2: Hierarchy Tracking (Week 2)

**Agent: parser-engineer**

### Tasks
1. **Parent Tracking**
   - Build heading stack
   - Track parent relationships
   - Generate breadcrumbs
   - Store in metadata

2. **Section Boundaries**
   - Determine section ends
   - Handle nested sections
   - Include related content
   - Manage orphan content

**Acceptance Criteria:**
- [ ] Parent paths accurate
- [ ] Section boundaries correct
- [ ] Nesting handled properly
- [ ] Metadata stored correctly

## Phase 3: Enhanced Extraction (Week 3)

**Agent: parser-engineer**

### Tasks
1. **Code Block Processing**
   - Extract with language tags
   - Link to parent sections
   - Create searchable chunks
   - Preserve formatting

2. **Link Resolution**
   - Find all links
   - Resolve relative paths
   - Detect cross-references
   - Create link database

**Acceptance Criteria:**
- [ ] Code blocks extracted
- [ ] Languages detected
- [ ] Links resolved
- [ ] Cross-refs tracked

## Phase 4: Migration & Testing (Week 4)

**Agent: parser-engineer + integration-tester**

### Tasks
1. **Migration Script**
   - Backup existing data
   - Re-parse all markdown
   - Update database
   - Verify integrity

2. **Quality Testing**
   - Compare old vs new
   - Test search quality
   - Verify hierarchies
   - Performance benchmarks

**Acceptance Criteria:**
- [ ] Migration completes safely
- [ ] No data loss
- [ ] Search quality improved
- [ ] Performance acceptable

## Success Metrics
- Parse accuracy >99%
- Hierarchy tracking 100%
- Code block detection 100%
- No performance regression
- Search relevance improved

## Risk Mitigation
- Rollback procedure ready
- Incremental migration
- Parallel parser option
- Extensive testing