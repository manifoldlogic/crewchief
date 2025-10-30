use crewchief_maproom::indexer::parser;

/// MD_ENHANCE-3001: Code Block Processing Tests
///
/// These tests verify that code blocks are:
/// 1. Extracted as separate searchable chunks
/// 2. Tagged with language metadata from info_string
/// 3. Linked to parent heading sections
/// 4. Preserving exact formatting
/// 5. Counting lines of code correctly

#[test]
fn test_code_block_basic_extraction() {
    let source = r#"# Documentation

Some text here.

```typescript
function hello() {
    return "world";
}
```

More text.
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 1, "Should extract exactly 1 code block");

    let code_block = code_blocks[0];
    assert_eq!(code_block.symbol_name, Some("Code: typescript".to_string()));
    assert_eq!(code_block.kind, "code_block");
}

#[test]
fn test_code_block_language_tags() {
    let source = r#"
```typescript
const x = 1;
```

```rust
fn main() {}
```

```python
def hello():
    pass
```

```bash
echo "hello"
```

```json
{"key": "value"}
```

```yaml
key: value
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 6, "Should extract 6 code blocks with different languages");

    // Verify each language was extracted correctly
    let languages = vec!["typescript", "rust", "python", "bash", "json", "yaml"];
    for lang in languages {
        let block = chunks.iter()
            .find(|c| c.symbol_name == Some(format!("Code: {}", lang)))
            .expect(&format!("Should find {} code block", lang));

        if let Some(metadata) = &block.metadata {
            assert_eq!(
                metadata.get("language").unwrap().as_str().unwrap(),
                lang,
                "{} code block should have correct language in metadata",
                lang
            );
        } else {
            panic!("{} code block should have metadata", lang);
        }
    }
}

#[test]
fn test_code_block_without_language_tag() {
    let source = r#"
```
plain code
no language specified
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 1, "Should extract code block without language");

    let code_block = code_blocks[0];
    assert_eq!(code_block.symbol_name, Some("Code: plain".to_string()));

    if let Some(metadata) = &code_block.metadata {
        assert_eq!(
            metadata.get("language").unwrap().as_str().unwrap(),
            "plain",
            "Code block without language should be marked as 'plain'"
        );
    } else {
        panic!("Code block should have metadata");
    }
}

#[test]
fn test_code_block_parent_path_simple() {
    let source = r#"# Installation

Follow these steps:

```bash
npm install package
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    if let Some(metadata) = &code_block.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Installation", "Code block should be linked to Installation heading");
    } else {
        panic!("Code block should have metadata with parent_path");
    }
}

#[test]
fn test_code_block_parent_path_nested() {
    let source = r#"# API Guide

## Authentication

### OAuth Setup

Configure OAuth:

```typescript
const oauth = {
    clientId: "abc123",
    secret: "xyz789"
};
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    if let Some(metadata) = &code_block.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(
            parent_path,
            "API Guide > Authentication > OAuth Setup",
            "Code block should have full breadcrumb to parent section"
        );
    } else {
        panic!("Code block should have metadata with parent_path");
    }
}

#[test]
fn test_code_block_parent_path_multiple_sections() {
    let source = r#"# Documentation

## Section One

```rust
fn one() {}
```

## Section Two

```rust
fn two() {}
```

### Subsection

```rust
fn three() {}
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 3, "Should extract 3 code blocks");

    // First code block under Section One
    let block1 = &code_blocks[0];
    if let Some(metadata) = &block1.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Documentation > Section One");
    }

    // Second code block under Section Two
    let block2 = &code_blocks[1];
    if let Some(metadata) = &block2.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Documentation > Section Two");
    }

    // Third code block under Subsection
    let block3 = &code_blocks[2];
    if let Some(metadata) = &block3.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Documentation > Section Two > Subsection");
    }
}

#[test]
fn test_code_block_no_parent() {
    let source = r#"
```python
print("hello")
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    if let Some(metadata) = &code_block.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "", "Code block without heading should have empty parent_path");
    } else {
        panic!("Code block should have metadata");
    }
}

#[test]
fn test_code_block_lines_of_code() {
    let source = r#"
```typescript
function calculate(a: number, b: number): number {
    const sum = a + b;
    const product = a * b;
    const average = sum / 2;

    return average;
}
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    if let Some(metadata) = &code_block.metadata {
        let lines_of_code = metadata.get("lines_of_code").unwrap().as_u64().unwrap();
        assert_eq!(lines_of_code, 7, "Should count 7 lines of code");
    } else {
        panic!("Code block should have metadata with lines_of_code");
    }
}

#[test]
fn test_code_block_empty() {
    let source = r#"
```rust
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 1, "Should extract empty code block");

    let code_block = code_blocks[0];
    if let Some(metadata) = &code_block.metadata {
        let lines_of_code = metadata.get("lines_of_code").unwrap().as_u64().unwrap();
        assert_eq!(lines_of_code, 0, "Empty code block should have 0 lines");
    }
}

#[test]
fn test_code_block_single_line() {
    let source = r#"
```bash
echo "Hello, World!"
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    if let Some(metadata) = &code_block.metadata {
        let lines_of_code = metadata.get("lines_of_code").unwrap().as_u64().unwrap();
        assert_eq!(lines_of_code, 1, "Single line code block should have 1 line");
    }
}

#[test]
fn test_code_block_info_string_with_annotations() {
    // Test that we handle info_string annotations like {1-3} or copy
    let source = r#"
```typescript {1-3}
const a = 1;
const b = 2;
const c = 3;
```

```rust copy
fn main() {}
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 2, "Should extract 2 code blocks");

    // First block should extract "typescript" from "typescript {1-3}"
    let ts_block = &code_blocks[0];
    assert_eq!(ts_block.symbol_name, Some("Code: typescript".to_string()));
    if let Some(metadata) = &ts_block.metadata {
        assert_eq!(metadata.get("language").unwrap().as_str().unwrap(), "typescript");
    }

    // Second block should extract "rust" from "rust copy"
    let rust_block = &code_blocks[1];
    assert_eq!(rust_block.symbol_name, Some("Code: rust".to_string()));
    if let Some(metadata) = &rust_block.metadata {
        assert_eq!(metadata.get("language").unwrap().as_str().unwrap(), "rust");
    }
}

#[test]
fn test_code_block_line_numbers() {
    let source = r#"Line 1
Line 2
Line 3
```python
def test():
    pass
```
Line 8
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    // Code block starts at line 4 (```python) and ends at line 7 (```)
    assert!(code_block.start_line >= 4, "Code block should start around line 4");
    assert!(code_block.end_line >= code_block.start_line, "End line should be >= start line");
}

#[test]
fn test_multiple_code_blocks_detection() {
    let source = r#"# Examples

Here's example 1:

```javascript
console.log(1);
```

Here's example 2:

```javascript
console.log(2);
```

Here's example 3:

```javascript
console.log(3);
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 3, "Should detect and extract all 3 code blocks");

    // All should have same parent
    for block in &code_blocks {
        if let Some(metadata) = &block.metadata {
            let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
            assert_eq!(parent_path, "Examples");
        }
    }
}

#[test]
fn test_code_block_at_end_of_file() {
    let source = r#"# Final Example

Here's the last code block:

```python
def final():
    return "end"
```"#; // No trailing newline

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 1, "Should extract code block at end of file");

    let code_block = code_blocks[0];
    assert_eq!(code_block.symbol_name, Some("Code: python".to_string()));
    if let Some(metadata) = &code_block.metadata {
        let parent_path = metadata.get("parent_path").unwrap().as_str().unwrap();
        assert_eq!(parent_path, "Final Example");
    }
}

#[test]
fn test_code_block_metadata_complete() {
    let source = r#"# API

## Authentication

```typescript
const token = "abc123";
const headers = {
    Authorization: `Bearer ${token}`
};
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_block = chunks.iter()
        .find(|c| c.kind == "code_block")
        .expect("Should find code block");

    // Verify all required metadata fields are present
    if let Some(metadata) = &code_block.metadata {
        assert!(metadata.get("language").is_some(), "Should have language field");
        assert!(metadata.get("parent_path").is_some(), "Should have parent_path field");
        assert!(metadata.get("lines_of_code").is_some(), "Should have lines_of_code field");

        assert_eq!(metadata.get("language").unwrap().as_str().unwrap(), "typescript");
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "API > Authentication");
        assert_eq!(metadata.get("lines_of_code").unwrap().as_u64().unwrap(), 4);
    } else {
        panic!("Code block must have metadata");
    }
}

#[test]
fn test_code_blocks_separate_from_headings() {
    let source = r#"# Title

```rust
fn test() {}
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let headings: Vec<_> = chunks.iter().filter(|c| c.kind.starts_with("heading_")).collect();
    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();

    assert_eq!(headings.len(), 1, "Should have 1 heading chunk");
    assert_eq!(code_blocks.len(), 1, "Should have 1 code block chunk");
    assert_ne!(headings[0].start_line, code_blocks[0].start_line, "Heading and code block should be separate chunks");
}

#[test]
fn test_code_block_real_world_readme() {
    let source = r#"# Installation Guide

## Prerequisites

Before installing, ensure you have:
- Node.js 18+
- npm or yarn

## Quick Start

Install the package:

```bash
npm install my-package
```

## Configuration

Create a config file:

```typescript
import { Config } from 'my-package';

const config: Config = {
    apiKey: process.env.API_KEY,
    timeout: 5000,
    retries: 3
};

export default config;
```

## Usage Example

Here's a basic usage example:

```typescript
import { Client } from 'my-package';
import config from './config';

const client = new Client(config);

async function main() {
    const result = await client.fetch('/api/data');
    console.log(result);
}

main();
```

## Troubleshooting

If you encounter errors, try:

```bash
npm cache clean --force
npm install
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 4, "Should extract all 4 code blocks from README");

    // Verify first bash block
    let bash_block1 = &code_blocks[0];
    assert_eq!(bash_block1.symbol_name, Some("Code: bash".to_string()));
    if let Some(metadata) = &bash_block1.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "Installation Guide > Quick Start");
    }

    // Verify first typescript block (config)
    let ts_block1 = &code_blocks[1];
    assert_eq!(ts_block1.symbol_name, Some("Code: typescript".to_string()));
    if let Some(metadata) = &ts_block1.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "Installation Guide > Configuration");
    }

    // Verify second typescript block (usage)
    let ts_block2 = &code_blocks[2];
    if let Some(metadata) = &ts_block2.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "Installation Guide > Usage Example");
    }

    // Verify second bash block (troubleshooting)
    let bash_block2 = &code_blocks[3];
    if let Some(metadata) = &bash_block2.metadata {
        assert_eq!(metadata.get("parent_path").unwrap().as_str().unwrap(), "Installation Guide > Troubleshooting");
    }
}

#[test]
fn test_code_block_100_percent_detection() {
    // This test verifies 100% detection of code blocks
    let source = r#"
Text before.

```
block1
```

More text.

```python
block2
```

Even more text.

```rust
block3
```

```
block4
```

Final text.
"#;

    let chunks = parser::extract_chunks(source, "md");

    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();
    assert_eq!(code_blocks.len(), 4, "Should detect 100% of code blocks (4 total)");
}
