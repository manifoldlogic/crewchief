# Maproom Markdown Indexing Plan

## Overview
Enhance maproom to provide semantic-aware indexing of markdown documentation, making it as searchable and useful as code files for AI assistants and developers.

## Implementation Status Summary

**Current State (as of 2025-10-23):**
- ✅ **Week 1 (MVP Markdown Support):** COMPLETE - Markdown files are indexed using regex-based heading chunking
- ⚠️ **Week 2 (Tree-Sitter Integration):** SKIPPED - Team decided to use simpler regex approach instead
- ✅ **Week 3 (Search Enhancements):** COMPLETE - Weighted scoring, filters, and context fields all working
- ✅ **Week 4 (Additional File Types):** COMPLETE - JSON, YAML, and TOML files fully supported

**What Works:**
- 18 markdown files indexed (107 total files, 579 chunks)
- Heading-based chunking (heading_1 through heading_6)
- Filter parameter: `all`, `code`, `docs`, `config`
- Weighted relevance: heading_1/2 (2.0x), heading_3 (1.5x), heading_4-6 (1.2x)
- Config file support: JSON (9 files), YAML (8 files), TOML (2 files)

**What's Missing (from Week 2):**
- Tree-sitter-markdown integration (using regex instead)
- Parent heading hierarchy tracking
- Code block extraction with language detection
- More sophisticated markdown structure parsing

**Recommendation:** The regex-based approach is working well for current needs. Week 2 features could be implemented later if needed for more complex documentation parsing.

## Goals
1. Index markdown files with awareness of document structure
2. Preserve heading hierarchy and context in search results
3. Enable searching across both code and documentation seamlessly
4. Maintain high relevance scoring for documentation searches

## Current State Analysis

### Existing Architecture
- **Parser**: Uses tree-sitter for TypeScript/JavaScript parsing
- **Chunking**: Creates chunks based on functions, classes, and modules
- **Storage**: PostgreSQL with `chunks` table storing symbol info and tsvector
- **Search**: Full-text search using PostgreSQL's tsvector/tsquery

### Limitations for Markdown
- No markdown file recognition (only .ts, .js, .rs)
- Chunking logic assumes code structure (functions/classes)
- Symbol types don't accommodate document structures

## Proposed Implementation

### Phase 1: Basic Markdown Support (MVP)
**Goal**: Get markdown files indexed and searchable quickly

#### 1.1 File Recognition
```rust
// In crates/maproom/src/scanner.rs
fn should_index_file(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some("ts") | Some("tsx") | Some("js") | Some("jsx") => true,
        Some("md") | Some("mdx") => true,  // Add markdown
        Some("json") => true,               // Add JSON configs
        _ => false,
    }
}
```

#### 1.2 Simple Chunking Strategy
- Chunk on heading boundaries (# ## ###)
- Include content until next heading of same/higher level
- Preserve parent heading context

#### 1.3 Database Schema Updates
```sql
-- Add document-specific chunk types
ALTER TYPE maproom.chunk_kind ADD VALUE 'heading_1';
ALTER TYPE maproom.chunk_kind ADD VALUE 'heading_2';
ALTER TYPE maproom.chunk_kind ADD VALUE 'heading_3';
ALTER TYPE maproom.chunk_kind ADD VALUE 'markdown_section';
ALTER TYPE maproom.chunk_kind ADD VALUE 'code_block';

-- Add metadata column for additional context
ALTER TABLE maproom.chunks 
ADD COLUMN metadata JSONB DEFAULT '{}';
-- Can store: parent_heading, language (for code blocks), etc.
```

### Phase 2: Tree-Sitter Markdown Integration
**Goal**: Robust parsing with full markdown awareness

#### 2.1 Add tree-sitter-markdown
```toml
# In Cargo.toml
[dependencies]
tree-sitter-markdown = "0.20"
```

#### 2.2 Markdown Parser Implementation
```rust
// New file: crates/maproom/src/parsers/markdown.rs
pub struct MarkdownParser {
    parser: Parser,
}

impl MarkdownParser {
    pub fn extract_chunks(&self, content: &str) -> Vec<Chunk> {
        let tree = self.parser.parse(content, None).unwrap();
        let mut chunks = Vec::new();
        let mut cursor = tree.walk();
        
        // Walk tree and create chunks for:
        // - Headings with their content
        // - Code blocks (with language metadata)
        // - Lists under their parent heading
        // - Tables as single chunks
        
        self.visit_node(&mut cursor, content, &mut chunks);
        chunks
    }
    
    fn visit_node(&self, cursor: &mut TreeCursor, source: &str, chunks: &mut Vec<Chunk>) {
        match cursor.node().kind() {
            "atx_heading" => {
                // Extract heading level, text, and following content
                let level = self.get_heading_level(cursor.node());
                let heading_text = self.get_node_text(cursor.node(), source);
                let content = self.get_section_content(cursor, source, level);
                
                chunks.push(Chunk {
                    symbol_name: heading_text,
                    kind: format!("heading_{}", level),
                    content: format!("{}\n{}", heading_text, content),
                    start_line: cursor.node().start_position().row,
                    end_line: cursor.node().end_position().row,
                    metadata: json!({
                        "level": level,
                        "parent_path": self.get_parent_path(cursor)
                    }),
                });
            },
            "fenced_code_block" => {
                // Extract code with language hint
                let lang = self.get_code_language(cursor.node(), source);
                let code = self.get_node_text(cursor.node(), source);
                
                chunks.push(Chunk {
                    symbol_name: format!("Code block ({})", lang.unwrap_or("plain")),
                    kind: "code_block",
                    content: code,
                    metadata: json!({ "language": lang }),
                    ..
                });
            },
            _ => {}
        }
    }
}
```

#### 2.3 Smart Content Segmentation
```rust
fn get_section_content(&self, cursor: &TreeCursor, source: &str, level: usize) -> String {
    // Collect all content until:
    // - Next heading of same or higher level
    // - End of document
    // Include: paragraphs, lists, code blocks, tables
    // Preserve: formatting, links, emphasis
}
```

### Phase 3: Enhanced Search Features
**Goal**: Optimal search experience across code and docs

#### 3.1 Weighted Search Relevance
```sql
-- Enhance search query to weight headings higher
SELECT 
    c.id,
    f.relpath,
    c.symbol_name,
    c.kind::text,
    CASE 
        WHEN c.kind IN ('heading_1', 'heading_2') THEN 
            ts_rank_cd(c.ts_doc, query) * 2.0  -- Boost headings
        WHEN c.kind = 'code_block' THEN
            ts_rank_cd(c.ts_doc, query) * 0.8  -- Slightly lower code blocks
        ELSE 
            ts_rank_cd(c.ts_doc, query)
    END as score
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', $1)
ORDER BY score DESC;
```

#### 3.2 Context-Aware Results
```typescript
// In maproom-mcp response
{
  "hits": [{
    "relpath": "README.md",
    "symbol_name": "Installation",
    "kind": "heading_2",
    "parent_context": "Getting Started > Installation",  // New field
    "preview": "To install the package, run npm install...",  // First 200 chars
    "start_line": 45,
    "end_line": 67,
    "score": 0.95
  }]
}
```

#### 3.3 Filter Options
```typescript
// Add to search tool parameters
{
  name: 'filter',
  type: 'string',
  enum: ['all', 'code', 'docs', 'config'],
  default: 'all',
  description: 'Filter results by file type'
}
```

### Phase 4: Additional File Types
**Goal**: Complete project understanding

#### 4.1 Configuration Files
- **JSON**: package.json, tsconfig.json, .eslintrc
- **YAML**: .github/workflows/*, docker-compose.yml
- **TOML**: Cargo.toml, pyproject.toml

#### 4.2 Specialized Parsers
```rust
pub enum ParserType {
    TypeScript(TypeScriptParser),
    Markdown(MarkdownParser),
    Json(JsonParser),      // Chunk by top-level keys
    Yaml(YamlParser),      // Chunk by document structure
    Toml(TomlParser),      // Chunk by sections
}
```

## Implementation Roadmap

### Week 1: MVP Markdown Support ✅ COMPLETE
- [x] Add .md file recognition to scanner
- [x] Implement basic regex-based heading chunking
- [x] Update database schema for markdown chunks
- [x] Test with README files

### Week 2: Tree-Sitter Integration ⚠️ SKIPPED (Using regex-based approach instead)
- [ ] Integrate tree-sitter-markdown
- [ ] Implement proper markdown parser
- [ ] Add heading hierarchy tracking
- [ ] Handle code blocks with language detection

### Week 3: Search Enhancements ✅ COMPLETE
- [x] Implement weighted relevance scoring
- [x] Add context fields to search results
- [x] Add filter parameter to search tool
- [x] Update MCP tool descriptions

### Week 4: Additional File Types ✅ COMPLETE
- [x] Add JSON/YAML/TOML recognition
- [x] Implement basic chunking for config files
- [x] Test across various project types
- [x] Update documentation

## Success Metrics

1. **Coverage**: 90% of project documentation indexed
2. **Relevance**: Documentation results appear with appropriate scores
3. **Context**: Each result shows its hierarchical position
4. **Performance**: Indexing markdown is <2x slower than code
5. **Usability**: AI assistants naturally use search for docs

## Testing Strategy

### Test Cases
1. Search for concept mentioned in both code and docs
2. Search for heading text directly
3. Search for content within code blocks in markdown
4. Search for configuration values in JSON/YAML
5. Verify heading hierarchy preservation

### Test Files
```
test-project/
├── README.md           (multi-level headings)
├── docs/
│   ├── API.md         (code examples)
│   └── ARCHITECTURE.md (diagrams, lists)
├── package.json       (configuration)
└── src/
    └── index.ts       (code)
```

## Migration Plan

1. **Backward Compatible**: Existing code indexing unchanged
2. **Incremental Rollout**: Enable per-repository via config
3. **Re-indexing**: Provide command to re-index with new parser
4. **Monitoring**: Track search quality metrics

## Future Enhancements

1. **Cross-references**: Link code mentions in docs to actual code
2. **Semantic linking**: "This implements: [README.md#authentication]"
3. **Doc coverage**: Show which code lacks documentation
4. **Change tracking**: Alert when docs are outdated vs code
5. **Multi-language**: Support for Python docstrings, JSDoc, etc.

## Conclusion

This plan provides a practical path to making maproom the single source of truth for understanding codebases, encompassing both implementation and documentation. The phased approach ensures we can deliver value quickly while building toward a comprehensive solution.