# Tree-Sitter Markdown Integration Plan

## Status
⚠️ **NOT IMPLEMENTED** - Currently using regex-based approach instead.

This document captures the deferred tree-sitter integration work from the original markdown indexing plan.

## Goal
Robust parsing with full markdown awareness using tree-sitter AST parsing instead of regex.

## Implementation Tasks

### Add tree-sitter-markdown Dependency
```toml
# In crates/maproom/Cargo.toml
[dependencies]
tree-sitter-markdown = "0.20"
```

### Markdown Parser Implementation
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

### Smart Content Segmentation
```rust
fn get_section_content(&self, cursor: &TreeCursor, source: &str, level: usize) -> String {
    // Collect all content until:
    // - Next heading of same or higher level
    // - End of document
    // Include: paragraphs, lists, code blocks, tables
    // Preserve: formatting, links, emphasis
}
```

## Features Enabled by Tree-Sitter

1. **Heading Hierarchy Tracking** - Parent heading context stored in metadata
2. **Code Block Extraction** - Language detection and proper chunking
3. **List Parsing** - Lists associated with their parent heading
4. **Table Handling** - Tables as single searchable chunks
5. **Robust Parsing** - Handles edge cases better than regex

## Missing from Current Regex Implementation

- Parent heading hierarchy tracking (`metadata.parent_path`)
- Code block extraction with language detection
- List and table awareness
- Proper handling of nested structures
- More accurate section boundary detection

## Roadmap Items

From the original plan:

### Week 2: Tree-Sitter Integration
- [ ] Integrate tree-sitter-markdown
- [ ] Implement proper markdown parser
- [ ] Add heading hierarchy tracking
- [ ] Handle code blocks with language detection
