# MD_ENHANCE Architecture: Enhanced Markdown Support

## Architecture Overview

```
Markdown File → Tree-Sitter Parser → AST Walker → Chunk Builder → Database
                                           ↓
                                    Hierarchy Tracker
```

## Core Components

### 1. Tree-Sitter Markdown Parser
```rust
use tree_sitter_md::language;

pub struct MarkdownParser {
    parser: Parser,
    heading_query: Query,
    code_query: Query,
    link_query: Query,
}

impl MarkdownParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();

        let heading_query = Query::new(
            language(),
            r#"
            (atx_heading
              (atx_h1_marker) @level
              (heading_content) @content) @heading.h1

            (atx_heading
              (atx_h2_marker) @level
              (heading_content) @content) @heading.h2

            (atx_heading
              (atx_h3_marker) @level
              (heading_content) @content) @heading.h3
            "#
        ).unwrap();

        let code_query = Query::new(
            language(),
            r#"
            (fenced_code_block
              (info_string) @language
              (code_fence_content) @code) @codeblock
            "#
        ).unwrap();

        Self { parser, heading_query, code_query, link_query }
    }
}
```

### 2. Hierarchy Tracker
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

impl HierarchyTracker {
    pub fn enter_heading(&mut self, level: u8, text: String, line: usize) {
        // Pop stack to parent level
        while self.stack.last().map_or(false, |h| h.level >= level) {
            self.stack.pop();
        }

        self.stack.push(HeadingNode {
            level,
            text,
            start_line: line,
            children: Vec::new(),
        });
    }

    pub fn get_parent_path(&self) -> String {
        self.stack.iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" > ")
    }
}
```

### 3. Enhanced Chunk Builder
```rust
pub struct MarkdownChunkBuilder {
    hierarchy: HierarchyTracker,
}

impl MarkdownChunkBuilder {
    pub fn build_chunks(&mut self, tree: Tree, source: &str) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut cursor = tree.walk();

        self.walk_tree(&mut cursor, source, &mut chunks);
        chunks
    }

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

        // Recurse
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

    fn process_heading(&mut self, node: Node, source: &str) -> Chunk {
        let level = self.get_heading_level(node);
        let text = self.get_heading_text(node, source);
        let content = self.get_section_content(node, source, level);

        self.hierarchy.enter_heading(level, text.clone(), node.start_position().row);

        Chunk {
            symbol_name: text,
            kind: SymbolKind::from(format!("heading_{}", level)),
            preview: content.chars().take(200).collect(),
            start_line: node.start_position().row,
            end_line: self.find_section_end(node, source, level),
            metadata: json!({
                "level": level,
                "parent_path": self.hierarchy.get_parent_path(),
                "has_code_blocks": self.section_has_code(node)
            }),
        }
    }

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
}
```

### 4. Link Resolver
```rust
pub struct LinkResolver {
    base_path: PathBuf,
}

impl LinkResolver {
    pub fn resolve_links(&self, tree: &Tree, source: &str) -> Vec<Link> {
        let mut links = Vec::new();
        let mut cursor = tree.walk();

        self.find_links(&mut cursor, source, &mut links);
        links
    }

    fn find_links(&self, cursor: &mut TreeCursor, source: &str, links: &mut Vec<Link>) {
        match cursor.node().kind() {
            "link" => {
                let url = self.get_link_url(cursor.node(), source);
                let text = self.get_link_text(cursor.node(), source);

                if let Some(target) = self.resolve_target(&url) {
                    links.push(Link {
                        source_line: cursor.node().start_position().row,
                        target,
                        text,
                        link_type: self.classify_link(&url),
                    });
                }
            },
            _ => {}
        }

        // Recurse...
    }

    fn resolve_target(&self, url: &str) -> Option<Target> {
        if url.starts_with("http") {
            Some(Target::External(url.to_string()))
        } else if url.starts_with("#") {
            Some(Target::Anchor(url[1..].to_string()))
        } else {
            // Resolve relative path
            let path = self.base_path.join(url);
            if path.exists() {
                Some(Target::File(path))
            } else {
                None
            }
        }
    }
}
```

### 5. Migration Strategy
```rust
pub struct MarkdownMigrator {
    old_parser: RegexParser,
    new_parser: MarkdownParser,
}

impl MarkdownMigrator {
    pub async fn migrate(&self, repo_id: i64) -> Result<()> {
        // Get all markdown files
        let files = self.get_markdown_files(repo_id).await?;

        for file in files {
            // Parse with new parser
            let new_chunks = self.new_parser.parse(&file.content)?;

            // Delete old chunks
            self.delete_old_chunks(file.id).await?;

            // Insert new chunks
            self.insert_new_chunks(file.id, new_chunks).await?;
        }

        Ok(())
    }
}
```

## Database Extensions

```sql
-- Additional metadata for markdown
ALTER TABLE maproom.chunks
  ALTER COLUMN metadata SET DEFAULT '{}';

-- Index for parent paths
CREATE INDEX idx_chunks_parent_path ON maproom.chunks
  ((metadata->>'parent_path')) WHERE metadata->>'parent_path' IS NOT NULL;

-- Table for document links
CREATE TABLE maproom.doc_links (
  source_chunk_id BIGINT REFERENCES maproom.chunks(id),
  target_chunk_id BIGINT REFERENCES maproom.chunks(id),
  link_type TEXT,
  anchor TEXT,
  PRIMARY KEY (source_chunk_id, target_chunk_id)
);
```

## Configuration

```yaml
markdown:
  parser: tree-sitter  # tree-sitter | regex
  features:
    heading_hierarchy: true
    code_blocks: true
    tables: true
    links: true
    task_lists: true
  chunking:
    min_heading_content: 50  # chars
    max_chunk_size: 5000     # chars
    include_code_in_section: true
```