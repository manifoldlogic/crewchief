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
        let heading = chunks
            .iter()
            .find(|c| c.kind == format!("heading_{}", level))
            .expect(&format!("Should find heading level {}", level));
        assert_eq!(heading.symbol_name, Some(format!("Level {}", level)));

        // Check metadata contains level
        if let Some(metadata) = &heading.metadata {
            assert_eq!(
                metadata.get("level").unwrap().as_u64().unwrap(),
                level as u64
            );
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
    let rust_block = chunks
        .iter()
        .find(|c| c.symbol_name.as_ref().map_or(false, |s| s.contains("rust")))
        .expect("Should find Rust code block");
    assert_eq!(rust_block.symbol_name, Some("Code: rust".to_string()));
    if let Some(metadata) = &rust_block.metadata {
        assert_eq!(metadata.get("language").unwrap().as_str().unwrap(), "rust");
        assert!(metadata.get("lines_of_code").unwrap().as_u64().unwrap() > 0);
    }

    // Check JavaScript code block
    let js_block = chunks
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .map_or(false, |s| s.contains("javascript"))
        })
        .expect("Should find JavaScript code block");
    assert_eq!(js_block.symbol_name, Some("Code: javascript".to_string()));

    // Check plain code block
    let plain_block = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Code: plain".to_string()))
        .expect("Should find plain code block");
    if let Some(metadata) = &plain_block.metadata {
        // Code blocks without language tags should be marked as "plain" (MD_ENHANCE-3001)
        assert_eq!(metadata.get("language").unwrap().as_str().unwrap(), "plain");
    }
}

#[test]
fn test_markdown_links() {
    // Link extraction implemented in MD_ENHANCE-3002 using regex
    // tree-sitter-md does not provide structured link nodes, so we use regex patterns
    let source = r#"# Links Example

Here's an [external link](https://example.com) to a website.

Here's an [internal link](./other-doc.md) to another document.

Here's an [anchor link](#section-heading) to a section.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Should extract the heading successfully
    let headings = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .count();
    assert_eq!(headings, 1, "Should extract heading");

    // Links are now extracted via regex
    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();
    assert_eq!(links.len(), 3, "Should extract all 3 links");

    // Verify link types
    let external = links
        .iter()
        .find(|l| l.metadata.as_ref().unwrap()["link_type"] == "external")
        .expect("Should have external link");
    assert_eq!(external.signature.as_ref().unwrap(), "https://example.com");

    let relative = links
        .iter()
        .find(|l| l.metadata.as_ref().unwrap()["link_type"] == "relative")
        .expect("Should have relative link");
    assert_eq!(relative.signature.as_ref().unwrap(), "./other-doc.md");

    let anchor = links
        .iter()
        .find(|l| l.metadata.as_ref().unwrap()["link_type"] == "anchor")
        .expect("Should have anchor link");
    assert_eq!(anchor.signature.as_ref().unwrap(), "#section-heading");
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
    let headings = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .count();
    let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

    assert!(
        headings >= 4,
        "Expected at least 4 headings (h1, h2, h2, h3)"
    );
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
    let main_title = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Main Title".to_string()))
        .expect("Should find Main Title");

    // Main Title should extend to end of file since it's h1
    assert!(
        main_title.end_line > 10,
        "Main Title section should extend to end"
    );

    // Find Section One
    let section_one = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section One".to_string()))
        .expect("Should find Section One");

    // Section One should include Subsection but end before Section Two
    let section_two = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section Two".to_string()))
        .expect("Should find Section Two");

    assert!(
        section_one.end_line < section_two.start_line,
        "Section One should end before Section Two starts"
    );
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
    let headings: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .collect();

    // Empty headings should be filtered out
    assert!(headings
        .iter()
        .any(|h| h.symbol_name == Some("Valid Heading".to_string())));
}

#[test]
fn test_markdown_special_characters() {
    let source = r#"# Title with `code` and **bold**

## Section with [link](url) inline

Content here.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Headings should be extracted (inline formatting may be included)
    let headings: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .collect();
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
    assert!(chunks
        .iter()
        .any(|c| c.symbol_name == Some("Project Name".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.symbol_name == Some("Installation".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.symbol_name == Some("Usage".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.symbol_name == Some("API Reference".to_string())));

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
    let headings = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .count();
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
    let main_section = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Main Section".to_string()))
        .expect("Should find Main Section");

    let code_block = chunks
        .iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    // Code block should be within main section's range
    assert!(code_block.start_line > main_section.start_line);
    assert!(
        code_block.end_line < main_section.end_line,
        "Code block should be within Main Section range"
    );
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

    // Should extract heading and table (as markdown_section after MAPROOM-1001 fix)
    let tables: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("Table "))
        })
        .collect();
    assert_eq!(tables.len(), 1, "Expected 1 table");

    let table = tables[0];
    assert_eq!(table.symbol_name, Some("Table 4x3".to_string()));
    assert!(table.start_line > 0);
    assert!(table.end_line > table.start_line);

    // Check metadata
    if let Some(metadata) = &table.metadata {
        assert_eq!(
            metadata.get("section_type").unwrap().as_str().unwrap(),
            "table"
        );
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

    // Should extract table even if it has only header (as markdown_section after MAPROOM-1001 fix)
    let tables: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("Table "))
        })
        .collect();
    assert_eq!(tables.len(), 1, "Expected 1 table (header only)");

    let table = tables[0];
    if let Some(metadata) = &table.metadata {
        assert_eq!(
            metadata.get("section_type").unwrap().as_str().unwrap(),
            "table"
        );
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

    // Should extract heading and list (as markdown_section after MAPROOM-1001 fix)
    let lists: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("List ("))
        })
        .collect();
    assert_eq!(lists.len(), 1, "Expected 1 list");

    let list = lists[0];
    assert_eq!(list.symbol_name, Some("List (4 items)".to_string()));
    assert!(list.start_line > 0);
    assert!(list.end_line > list.start_line);

    // Check metadata
    if let Some(metadata) = &list.metadata {
        assert_eq!(
            metadata.get("list_type").unwrap().as_str().unwrap(),
            "unordered"
        );
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

    // Should extract heading and list (as markdown_section after MAPROOM-1001 fix)
    let lists: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("List ("))
        })
        .collect();
    assert_eq!(lists.len(), 1, "Expected 1 ordered list");

    let list = lists[0];
    assert_eq!(list.symbol_name, Some("List (5 items)".to_string()));

    // Check metadata
    if let Some(metadata) = &list.metadata {
        assert_eq!(
            metadata.get("list_type").unwrap().as_str().unwrap(),
            "ordered"
        );
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
    // Lists are now mapped to markdown_section kind (MAPROOM-1001 fix)
    let lists: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("List ("))
        })
        .collect();
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

    // Should extract 2 headings, 1 table, and 1 list (as markdown_section after MAPROOM-1001)
    let headings = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .count();
    let tables = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("Table "))
        })
        .count();
    let lists = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("List ("))
        })
        .count();

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

    // Lists are now mapped to markdown_section kind (MAPROOM-1001 fix)
    let lists: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "markdown_section"
                && c.symbol_name
                    .as_ref()
                    .map_or(false, |s| s.starts_with("List ("))
        })
        .collect();
    assert_eq!(lists.len(), 1, "Expected 1 list");

    let list = lists[0];
    if let Some(metadata) = &list.metadata {
        assert_eq!(metadata.get("item_count").unwrap().as_u64().unwrap(), 1);
    }
}

// MD_ENHANCE-2001: Parent Tracking Tests

#[test]
fn test_heading_parent_path_simple_nesting() {
    let source = r#"# Guide

## Setup

### Database

Some content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Find all headings
    let guide = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Guide".to_string()))
        .expect("Should find Guide heading");

    let setup = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Setup".to_string()))
        .expect("Should find Setup heading");

    let database = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Database".to_string()))
        .expect("Should find Database heading");

    // Check parent paths
    if let Some(metadata) = &guide.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "", "Root h1 should have empty parent path");
    } else {
        panic!("Guide should have metadata");
    }

    if let Some(metadata) = &setup.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Guide", "h2 should have parent 'Guide'");
    } else {
        panic!("Setup should have metadata");
    }

    if let Some(metadata) = &database.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(
            parent_path, "Guide > Setup",
            "h3 should have parent 'Guide > Setup'"
        );
    } else {
        panic!("Database should have metadata");
    }
}

#[test]
fn test_heading_parent_path_sibling_headings() {
    let source = r#"# Main

## Section One

Content here.

## Section Two

More content.

## Section Three

Final content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    let main = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Main".to_string()))
        .expect("Should find Main heading");

    let section_one = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section One".to_string()))
        .expect("Should find Section One");

    let section_two = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section Two".to_string()))
        .expect("Should find Section Two");

    let section_three = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section Three".to_string()))
        .expect("Should find Section Three");

    // Check all siblings have same parent
    if let Some(metadata) = &main.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "", "Root should have empty parent path");
    }

    for section in [section_one, section_two, section_three] {
        if let Some(metadata) = &section.metadata {
            let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
            assert_eq!(
                parent_path, "Main",
                "All h2 siblings should have parent 'Main'"
            );
        } else {
            panic!("Section should have metadata");
        }
    }
}

#[test]
fn test_heading_parent_path_level_jumping() {
    let source = r#"# Top

#### Deep

Content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    let top = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Top".to_string()))
        .expect("Should find Top heading");

    let deep = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Deep".to_string()))
        .expect("Should find Deep heading");

    // Check that jumping from h1 to h4 works correctly
    if let Some(metadata) = &top.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "", "h1 should have empty parent path");
    }

    if let Some(metadata) = &deep.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Top", "h4 after h1 should have parent 'Top'");
    }
}

#[test]
fn test_heading_parent_path_complex_transitions() {
    let source = r#"# Root

## Level 2 A

### Level 3 A

Content.

## Level 2 B

### Level 3 B

More content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    let level_3a = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Level 3 A".to_string()))
        .expect("Should find Level 3 A");

    let level_2b = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Level 2 B".to_string()))
        .expect("Should find Level 2 B");

    let level_3b = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Level 3 B".to_string()))
        .expect("Should find Level 3 B");

    // Check Level 3 A has correct path
    if let Some(metadata) = &level_3a.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Root > Level 2 A");
    } else {
        panic!("Level 3 A should have metadata");
    }

    // Check Level 2 B (transition from h3 to h2)
    if let Some(metadata) = &level_2b.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(
            parent_path, "Root",
            "h2 after h3 should pop back to h1 parent"
        );
    } else {
        panic!("Level 2 B should have metadata");
    }

    // Check Level 3 B has correct path
    if let Some(metadata) = &level_3b.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Root > Level 2 B");
    } else {
        panic!("Level 3 B should have metadata");
    }
}

#[test]
fn test_heading_parent_path_multiple_roots() {
    let source = r#"# First Root

## Child of First

Content.

# Second Root

## Child of Second

More content.
"#;

    let chunks = parser::extract_chunks(source, "md");

    let first_root = chunks
        .iter()
        .find(|c| c.symbol_name == Some("First Root".to_string()))
        .expect("Should find First Root");

    let child_of_first = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Child of First".to_string()))
        .expect("Should find Child of First");

    let second_root = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Second Root".to_string()))
        .expect("Should find Second Root");

    let child_of_second = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Child of Second".to_string()))
        .expect("Should find Child of Second");

    // Check first section
    if let Some(metadata) = &first_root.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "");
    }

    if let Some(metadata) = &child_of_first.metadata {
        assert_eq!(
            metadata.get("parent_path").unwrap().as_str().unwrap(),
            "First Root"
        );
    }

    // Check second section (stack should reset for new h1)
    if let Some(metadata) = &second_root.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "");
    }

    if let Some(metadata) = &child_of_second.metadata {
        assert_eq!(
            metadata.get("parent_path").unwrap().as_str().unwrap(),
            "Second Root"
        );
    }
}

#[test]
fn test_heading_parent_path_all_levels() {
    let source = r#"# L1

## L2

### L3

#### L4

##### L5

###### L6

Content at max depth.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Check L1
    let l1 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L1".to_string()))
        .unwrap();
    if let Some(metadata) = &l1.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "");
    }

    // Check L2
    let l2 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L2".to_string()))
        .unwrap();
    if let Some(metadata) = &l2.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "L1");
    }

    // Check L3
    let l3 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L3".to_string()))
        .unwrap();
    if let Some(metadata) = &l3.metadata {
        assert_eq!(
            metadata.get("parent_path").unwrap().as_str().unwrap(),
            "L1 > L2"
        );
    }

    // Check L4
    let l4 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L4".to_string()))
        .unwrap();
    if let Some(metadata) = &l4.metadata {
        assert_eq!(
            metadata.get("parent_path").unwrap().as_str().unwrap(),
            "L1 > L2 > L3"
        );
    }

    // Check L5
    let l5 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L5".to_string()))
        .unwrap();
    if let Some(metadata) = &l5.metadata {
        assert_eq!(
            metadata.get("parent_path").unwrap().as_str().unwrap(),
            "L1 > L2 > L3 > L4"
        );
    }

    // Check L6 (max depth)
    let l6 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L6".to_string()))
        .unwrap();
    if let Some(metadata) = &l6.metadata {
        assert_eq!(
            metadata.get("parent_path").unwrap().as_str().unwrap(),
            "L1 > L2 > L3 > L4 > L5"
        );
    }
}

#[test]
fn test_heading_parent_path_readme_structure() {
    // Real-world README structure test
    let source = r#"# Project Name

## Installation

### Prerequisites

System requirements.

### Quick Start

Installation steps.

## API Reference

### Authentication

#### OAuth Flow

Details here.

#### API Keys

More details.

### Database

#### Connections

Connection info.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Check OAuth Flow has correct path
    let oauth = chunks
        .iter()
        .find(|c| c.symbol_name == Some("OAuth Flow".to_string()))
        .expect("Should find OAuth Flow");

    if let Some(metadata) = &oauth.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(
            parent_path, "Project Name > API Reference > Authentication",
            "OAuth Flow should have full breadcrumb path"
        );
    }

    // Check Connections has correct path
    let connections = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Connections".to_string()))
        .expect("Should find Connections");

    if let Some(metadata) = &connections.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(
            parent_path, "Project Name > API Reference > Database",
            "Connections should have correct breadcrumb after transitioning sections"
        );
    }
}

// MAPROOM-1001: Test that lists use valid enum value (markdown_section, not "list")
#[test]
fn test_maproom_1001_list_uses_valid_enum() {
    // This test verifies the fix for MAPROOM-1001
    // Lists must use "markdown_section" kind to avoid PostgreSQL enum errors
    let source = r#"# Test Document

Some intro text.

## Lists Section

Unordered list:
- Item 1
- Item 2
- Item 3

Ordered list:
1. First
2. Second
3. Third

Nested list:
- Parent item
  - Nested item 1
  - Nested item 2
- Another parent

Task list:
- [ ] Todo item
- [x] Done item

End of document.
"#;

    let chunks = parser::extract_chunks(source, "md");

    // Find all list chunks
    let lists: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.symbol_name
                .as_ref()
                .map_or(false, |s| s.starts_with("List ("))
        })
        .collect();

    // Should extract multiple lists
    assert!(
        lists.len() >= 3,
        "Expected at least 3 lists (unordered, ordered, nested)"
    );

    // CRITICAL: All lists must use "markdown_section" kind, not "list"
    // This prevents PostgreSQL enum errors during database insertion
    for list in &lists {
        assert_eq!(
            list.kind,
            "markdown_section",
            "List '{}' must use 'markdown_section' kind to avoid enum error. \
             The PostgreSQL symbol_kind enum does not include 'list' as a valid value.",
            list.symbol_name.as_ref().unwrap_or(&"unknown".to_string())
        );

        // Verify metadata still contains list-specific information
        if let Some(metadata) = &list.metadata {
            assert!(
                metadata.get("list_type").is_some(),
                "List should have list_type in metadata"
            );
            assert!(
                metadata.get("item_count").is_some(),
                "List should have item_count in metadata"
            );
        } else {
            panic!("List should have metadata with list_type and item_count");
        }
    }

    // Verify different list types are detected correctly
    let unordered = lists
        .iter()
        .find(|l| {
            l.metadata
                .as_ref()
                .and_then(|m| m.get("list_type"))
                .and_then(|v| v.as_str())
                == Some("unordered")
        })
        .expect("Should find unordered list");
    assert!(unordered.symbol_name.as_ref().unwrap().contains("items"));

    let ordered = lists
        .iter()
        .find(|l| {
            l.metadata
                .as_ref()
                .and_then(|m| m.get("list_type"))
                .and_then(|v| v.as_str())
                == Some("ordered")
        })
        .expect("Should find ordered list");
    assert!(ordered.symbol_name.as_ref().unwrap().contains("items"));
}
