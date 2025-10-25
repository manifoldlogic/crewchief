use crewchief_maproom::indexer::parser;

#[test]
fn test_markdown_simple_headings() {
    let source = r#"# Main Title

Some intro text here.

## Section One

Content for section one.

### Subsection 1.1

More detailed content.

## Section Two

Content for section two.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should extract 4 headings
    assert!(chunks.len() >= 4, "Expected at least 4 heading chunks");

    // Check h1
    let h1 = chunks.iter().find(|c| c.kind == "heading_1").unwrap();
    assert_eq!(h1.symbol_name, Some("Main Title".to_string()));
    assert_eq!(h1.start_line, 1);

    // Check h2 sections
    let h2_sections: Vec<_> = chunks.iter().filter(|c| c.kind == "heading_2").collect();
    assert_eq!(h2_sections.len(), 2);

    // Check h3
    let h3 = chunks.iter().find(|c| c.kind == "heading_3").unwrap();
    assert_eq!(h3.symbol_name, Some("Subsection 1.1".to_string()));
}

#[test]
fn test_markdown_nested_headings() {
    let source = r#"# Level 1
## Level 2
### Level 3
#### Level 4
##### Level 5
###### Level 6
"#;

    let chunks = parser::extract_chunks(source, "md");

    assert_eq!(chunks.len(), 6, "Expected 6 heading chunks");

    // Verify all levels are present
    for level in 1..=6 {
        let heading = chunks.iter()
            .find(|c| c.kind == format!("heading_{}", level))
            .expect(&format!("Should find heading level {}", level));
        assert_eq!(heading.symbol_name, Some(format!("Level {}", level)));

        // Check metadata contains level
        if let Some(metadata) = &heading.metadata {
            assert_eq!(metadata.get("level").unwrap().as_u64().unwrap(), level as u64);
        }
    }
}

#[test]
fn test_markdown_code_blocks() {
    let source = r#"# Code Examples

Here's some Rust code:

```rust
fn main() {
    println!("Hello, world!");
}
```

And some JavaScript:

```javascript
console.log("Hello, world!");
```

And a code block without language:

```
generic code here
no language specified
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should have 1 heading + 3 code blocks
    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 3, "Expected 3 code blocks");

    // Check Rust code block
    let rust_block = chunks.iter()
        .find(|c| c.symbol_name.as_ref().map_or(false, |s| s.contains("rust")))
        .expect("Should find Rust code block");
    assert_eq!(rust_block.symbol_name, Some("Code: rust".to_string()));
    if let Some(metadata) = &rust_block.metadata {
        assert_eq!(metadata.get("language").unwrap().as_str().unwrap(), "rust");
        assert!(metadata.get("lines_of_code").unwrap().as_u64().unwrap() > 0);
    }

    // Check JavaScript code block
    let js_block = chunks.iter()
        .find(|c| c.symbol_name.as_ref().map_or(false, |s| s.contains("javascript")))
        .expect("Should find JavaScript code block");
    assert_eq!(js_block.symbol_name, Some("Code: javascript".to_string()));

    // Check plain code block
    let plain_block = chunks.iter()
        .find(|c| c.symbol_name == Some("Code: plain".to_string()))
        .expect("Should find plain code block");
    if let Some(metadata) = &plain_block.metadata {
        assert!(metadata.get("language").is_none() || metadata.get("language").unwrap().is_null());
    }
}

#[test]
fn test_markdown_links() {
    // Note: Link extraction is not implemented in MD_ENHANCE-1001
    // tree-sitter-md does not provide structured link nodes
    // This will be addressed in MD_ENHANCE-3002 using regex or alternative approach
    let source = r#"# Links Example

Here's an [external link](https://example.com) to a website.

Here's an [internal link](./other-doc.md) to another document.

Here's an [anchor link](#section-heading) to a section.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // For now, we should extract the heading successfully
    let headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
    assert_eq!(headings, 1, "Should extract heading even though links aren't extracted yet");

    // Links will be extracted in future ticket
    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();
    assert_eq!(links.len(), 0, "Link extraction not yet implemented");
}

#[test]
fn test_markdown_mixed_content() {
    let source = r#"# Complete Document

This document has various elements.

## Features

Here's what we support:

- Lists (not chunked separately)
- Tables (not chunked yet)
- Code blocks

```typescript
interface User {
    name: string;
    age: number;
}
```

## Links

Check out [the docs](https://docs.example.com).

### Internal References

See [installation guide](./install.md) for setup.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Count different chunk types
    let headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
    let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

    assert!(headings >= 4, "Expected at least 4 headings (h1, h2, h2, h3)");
    assert_eq!(code_blocks, 1, "Expected 1 code block");

    // Links not extracted in this ticket - will be added in MD_ENHANCE-3002
}

#[test]
fn test_markdown_section_boundaries() {
    let source = r#"# Main Title

Main content here.

## Section One

Section one content.

### Subsection

Nested content.

## Section Two

Section two content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Main Title heading
    let main_title = chunks.iter()
        .find(|c| c.symbol_name == Some("Main Title".to_string()))
        .expect("Should find Main Title");

    // Main Title should extend to end of file since it's h1
    assert!(main_title.end_line > 10, "Main Title section should extend to end");

    // Find Section One
    let section_one = chunks.iter()
        .find(|c| c.symbol_name == Some("Section One".to_string()))
        .expect("Should find Section One");

    // Section One should include Subsection but end before Section Two
    let section_two = chunks.iter()
        .find(|c| c.symbol_name == Some("Section Two".to_string()))
        .expect("Should find Section Two");

    assert!(section_one.end_line < section_two.start_line,
        "Section One should end before Section Two starts");
}

#[test]
fn test_markdown_empty_headings() {
    let source = r#"#

##

### Valid Heading

Normal content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should only extract the valid heading with content
    let headings: Vec<_> = chunks.iter().filter(|c| c.kind.starts_with("heading_")).collect();

    // Empty headings should be filtered out
    assert!(headings.iter().any(|h| h.symbol_name == Some("Valid Heading".to_string())));
}

#[test]
fn test_markdown_special_characters() {
    let source = r#"# Title with `code` and **bold**

## Section with [link](url) inline

Content here.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Headings should be extracted (inline formatting may be included)
    let headings: Vec<_> = chunks.iter().filter(|c| c.kind.starts_with("heading_")).collect();
    assert!(headings.len() >= 2);
}

#[test]
fn test_markdown_real_readme() {
    // Test on a real-world README-style document
    let source = r#"# Project Name

A comprehensive tool for managing things.

## Installation

```bash
npm install project-name
```

## Usage

Import and use:

```javascript
import { Tool } from 'project-name';

const tool = new Tool();
tool.run();
```

## API Reference

### `Tool.run()`

Runs the tool.

### `Tool.stop()`

Stops the tool.

## License

MIT License. See [LICENSE](./LICENSE) for details.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Verify we extracted key elements
    assert!(chunks.iter().any(|c| c.symbol_name == Some("Project Name".to_string())));
    assert!(chunks.iter().any(|c| c.symbol_name == Some("Installation".to_string())));
    assert!(chunks.iter().any(|c| c.symbol_name == Some("Usage".to_string())));
    assert!(chunks.iter().any(|c| c.symbol_name == Some("API Reference".to_string())));

    // Should have code blocks
    let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();
    assert!(code_blocks >= 2, "Expected at least 2 code blocks");

    // Links not extracted in this ticket - will be added in MD_ENHANCE-3002
}

#[test]
fn test_markdown_malformed_syntax() {
    let source = r#"# Valid Heading

Some content with broken markdown.

## Unclosed code block

```rust
fn broken(
    // Missing closing brace

## Another Heading

Normal content.
"#;

    // Parser should not panic, even with malformed markdown
    let _chunks = parser::extract_chunks(source, "md");

    // The test passes if we don't panic
}

#[test]
fn test_markdown_empty_file() {
    let source = "";

    let chunks = parser::extract_chunks(source, "md");

    // Empty file should return empty chunks
    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_markdown_no_headings() {
    let source = r#"This is just plain text.
No headings here.
Just paragraphs.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // No headings means no chunks (or very few)
    let headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
    assert_eq!(headings, 0, "Should have no headings");
}

#[test]
fn test_markdown_code_block_in_section() {
    let source = r#"# Main Section

Regular content here.

```python
def hello():
    print("Hello")
```

More content.

## Next Section

Other content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Check that Main Section extends past the code block
    let main_section = chunks.iter()
        .find(|c| c.symbol_name == Some("Main Section".to_string()))
        .expect("Should find Main Section");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    // Code block should be within main section's range
    assert!(code_block.start_line > main_section.start_line);
    assert!(code_block.end_line < main_section.end_line,
        "Code block should be within Main Section range");
}

#[test]
fn test_markdown_table_extraction() {
    let source = r#"# Data Overview

Here's a table of data:

| Name    | Age | City      |
|---------|-----|-----------|
| Alice   | 30  | Seattle   |
| Bob     | 25  | Portland  |
| Charlie | 35  | Vancouver |

End of table.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should extract heading and table
    let tables: Vec<_> = chunks.iter().filter(|c| c.kind == "table").collect();
    assert_eq!(tables.len(), 1, "Expected 1 table");

    let table = tables[0];
    assert_eq!(table.symbol_name, Some("Table 4x3".to_string()));
    assert!(table.start_line > 0);
    assert!(table.end_line > table.start_line);

    // Check metadata
    if let Some(metadata) = &table.metadata {
        assert_eq!(metadata.get("rows").unwrap().as_u64().unwrap(), 4);
        assert_eq!(metadata.get("columns").unwrap().as_u64().unwrap(), 3);
        assert_eq!(metadata.get("has_header").unwrap().as_bool().unwrap(), true);
    } else {
        panic!("Table should have metadata");
    }
}

#[test]
fn test_markdown_empty_table() {
    let source = r#"
| Column 1 | Column 2 |
|----------|----------|
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should extract table even if it has only header
    let tables: Vec<_> = chunks.iter().filter(|c| c.kind == "table").collect();
    assert_eq!(tables.len(), 1, "Expected 1 table (header only)");

    let table = tables[0];
    if let Some(metadata) = &table.metadata {
        assert_eq!(metadata.get("rows").unwrap().as_u64().unwrap(), 1);
        assert_eq!(metadata.get("columns").unwrap().as_u64().unwrap(), 2);
    }
}

#[test]
fn test_markdown_unordered_list() {
    let source = r#"# Features

Here are the features:

- Fast performance
- Easy to use
- Open source
- Cross-platform

That's the list.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should extract heading and list
    let lists: Vec<_> = chunks.iter().filter(|c| c.kind == "list").collect();
    assert_eq!(lists.len(), 1, "Expected 1 list");

    let list = lists[0];
    assert_eq!(list.symbol_name, Some("List (4 items)".to_string()));
    assert!(list.start_line > 0);
    assert!(list.end_line > list.start_line);

    // Check metadata
    if let Some(metadata) = &list.metadata {
        assert_eq!(metadata.get("list_type").unwrap().as_str().unwrap(), "unordered");
        assert_eq!(metadata.get("item_count").unwrap().as_u64().unwrap(), 4);
    } else {
        panic!("List should have metadata");
    }
}

#[test]
fn test_markdown_ordered_list() {
    let source = r#"# Steps

Follow these steps:

1. Install dependencies
2. Configure settings
3. Run the application
4. Test functionality
5. Deploy to production

All done!
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should extract heading and list
    let lists: Vec<_> = chunks.iter().filter(|c| c.kind == "list").collect();
    assert_eq!(lists.len(), 1, "Expected 1 ordered list");

    let list = lists[0];
    assert_eq!(list.symbol_name, Some("List (5 items)".to_string()));

    // Check metadata
    if let Some(metadata) = &list.metadata {
        assert_eq!(metadata.get("list_type").unwrap().as_str().unwrap(), "ordered");
        assert_eq!(metadata.get("item_count").unwrap().as_u64().unwrap(), 5);
    } else {
        panic!("List should have metadata");
    }
}

#[test]
fn test_markdown_nested_list() {
    let source = r#"# Outline

- Section 1
  - Subsection 1.1
  - Subsection 1.2
- Section 2
- Section 3
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Note: Nested lists are extracted as separate list chunks
    // The outer list and inner list are separate nodes in tree-sitter-md
    let lists: Vec<_> = chunks.iter().filter(|c| c.kind == "list").collect();
    assert!(lists.len() >= 1, "Expected at least 1 list (outer list)");

    // Check that we extracted at least the outer list
    let outer_list = &lists[0];
    assert_eq!(outer_list.symbol_name, Some("List (3 items)".to_string()));
}

#[test]
fn test_markdown_mixed_table_and_list() {
    let source = r#"# Documentation

## Data Table

| ID | Name  | Status |
|----|-------|--------|
| 1  | Task1 | Done   |
| 2  | Task2 | Active |

## Features

- Feature A
- Feature B
- Feature C

End of document.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should extract 2 headings, 1 table, and 1 list
    let headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
    let tables = chunks.iter().filter(|c| c.kind == "table").count();
    let lists = chunks.iter().filter(|c| c.kind == "list").count();

    assert!(headings >= 2, "Expected at least 2 headings");
    assert_eq!(tables, 1, "Expected 1 table");
    assert_eq!(lists, 1, "Expected 1 list");
}

#[test]
fn test_markdown_single_item_list() {
    let source = r#"
- Only one item
"#;

    let chunks = parser::extract_chunks(source, "md");

    let lists: Vec<_> = chunks.iter().filter(|c| c.kind == "list").collect();
    assert_eq!(lists.len(), 1, "Expected 1 list");

    let list = lists[0];
    if let Some(metadata) = &list.metadata {
        assert_eq!(metadata.get("item_count").unwrap().as_u64().unwrap(), 1);
    }
}
