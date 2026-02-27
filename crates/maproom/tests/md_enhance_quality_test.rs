/// MD_ENHANCE-4002: Quality Testing
///
/// Standalone quality tests that validate parser accuracy, hierarchy tracking,
/// and code block detection without dependencies on other integration tests.
use maproom::indexer::parser;
use std::collections::HashMap;
use std::fs;

#[test]
fn test_hierarchy_tracking_validation() {
    // Test all parent paths are correct
    let source = r#"# Root
## Child 1
### Grandchild 1.1
### Grandchild 1.2
## Child 2
### Grandchild 2.1
#### Great-grandchild 2.1.1
## Child 3
"#;

    let chunks = parser::extract_chunks(source, "md");

    let mut errors = Vec::new();
    let mut correct = 0;
    let mut total = 0;

    // Expected parent paths
    let expected: HashMap<&str, &str> = [
        ("Root", ""),
        ("Child 1", "Root"),
        ("Grandchild 1.1", "Root > Child 1"),
        ("Grandchild 1.2", "Root > Child 1"),
        ("Child 2", "Root"),
        ("Grandchild 2.1", "Root > Child 2"),
        ("Great-grandchild 2.1.1", "Root > Child 2 > Grandchild 2.1"),
        ("Child 3", "Root"),
    ]
    .iter()
    .cloned()
    .collect();

    for chunk in &chunks {
        if let Some(ref name) = chunk.symbol_name {
            if let Some(&expected_path) = expected.get(name.as_str()) {
                total += 1;

                if let Some(metadata) = &chunk.metadata {
                    if let Some(parent_path) = metadata.get("parent_path") {
                        let actual_path = parent_path.as_str().unwrap();
                        if actual_path == expected_path {
                            correct += 1;
                        } else {
                            errors.push(format!(
                                "  {} - expected: '{}', got: '{}'",
                                name, expected_path, actual_path
                            ));
                        }
                    } else {
                        errors.push(format!("  {} - missing parent_path in metadata", name));
                    }
                } else {
                    errors.push(format!("  {} - missing metadata", name));
                }
            }
        }
    }

    println!("\nHierarchy Tracking:");
    println!(
        "  Correct parent paths: {} / {} ({:.1}%)",
        correct,
        total,
        (correct as f64 / total as f64) * 100.0
    );

    if !errors.is_empty() {
        println!("  Errors:");
        for error in &errors {
            println!("{}", error);
        }
    }

    // Success metric: 100% hierarchy tracking
    assert_eq!(
        correct, total,
        "All parent paths should be correct. {} / {} correct",
        correct, total
    );
}

#[test]
fn test_code_block_detection_validation() {
    // Test that 100% of code blocks are detected with correct languages
    let source = r#"# Examples

## Rust Example

```rust
fn main() {
    println!("Hello");
}
```

## Python Example

```python
def main():
    print("Hello")
```

## JavaScript Example

```javascript
console.log("Hello");
```

## TypeScript Example

```typescript
const msg: string = "Hello";
console.log(msg);
```

## Plain Code Block

```
plain text
```

## Bash Example

```bash
echo "Hello"
```
"#;

    let chunks = parser::extract_chunks(source, "md");
    let code_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "code_block").collect();

    // Expected: 6 code blocks
    assert_eq!(
        code_blocks.len(),
        6,
        "Should detect all 6 code blocks (100%)"
    );

    // Verify languages
    let expected_languages = [
        "rust",
        "python",
        "javascript",
        "typescript",
        "plain",
        "bash",
    ];
    let mut found_languages = Vec::new();

    for block in &code_blocks {
        if let Some(metadata) = &block.metadata {
            if let Some(lang) = metadata.get("language") {
                found_languages.push(lang.as_str().unwrap().to_string());
            }
        }
    }

    for lang in expected_languages {
        assert!(
            found_languages.contains(&lang.to_string()),
            "Should detect code block with language: {}",
            lang
        );
    }

    println!("\nCode Block Detection:");
    println!("  Detected: {} / {} (100%)", code_blocks.len(), 6);
    println!("  Languages: {:?}", found_languages);
}

#[test]
#[ignore = "Requires /workspace/README.md (devcontainer-specific path)"]
fn test_parser_accuracy_readme() {
    if let Ok(content) = fs::read_to_string("/workspace/README.md") {
        let chunks = parser::extract_chunks(&content, "md");

        let headings = chunks
            .iter()
            .filter(|c| c.kind.starts_with("heading_"))
            .count();
        let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();
        // Tables now use markdown_section kind (MAPROOM-1001 fix)
        let tables = chunks
            .iter()
            .filter(|c| {
                c.kind == "markdown_section"
                    && c.symbol_name
                        .as_ref()
                        .map_or(false, |s| s.starts_with("Table "))
            })
            .count();
        // Lists now use markdown_section kind (MAPROOM-1001 fix)
        let lists = chunks
            .iter()
            .filter(|c| {
                c.kind == "markdown_section"
                    && c.symbol_name
                        .as_ref()
                        .map_or(false, |s| s.starts_with("List ("))
            })
            .count();

        println!("\nREADME.md Parser Accuracy:");
        println!("  Headings: {}", headings);
        println!("  Code blocks: {}", code_blocks);
        println!("  Tables: {}", tables);
        println!("  Lists: {}", lists);
        println!("  Total chunks: {}", chunks.len());

        // Should extract meaningful content
        assert!(headings > 0, "Should extract headings from README");
        assert!(chunks.len() > 0, "Should extract chunks from README");
    } else {
        println!("README.md not found, skipping test");
    }
}

#[test]
#[ignore = "Requires /workspace/CLAUDE.md (devcontainer-specific path)"]
fn test_parser_accuracy_claude_md() {
    if let Ok(content) = fs::read_to_string("/workspace/CLAUDE.md") {
        let chunks = parser::extract_chunks(&content, "md");

        let headings = chunks
            .iter()
            .filter(|c| c.kind.starts_with("heading_"))
            .count();
        let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

        println!("\nCLAUDE.md Parser Accuracy:");
        println!("  Headings: {}", headings);
        println!("  Code blocks: {}", code_blocks);
        println!("  Total chunks: {}", chunks.len());

        // Should extract meaningful content
        assert!(headings > 10, "Should extract many headings from CLAUDE.md");
        assert!(
            code_blocks >= 2,
            "Should extract code blocks from CLAUDE.md"
        );
    } else {
        println!("CLAUDE.md not found, skipping test");
    }
}

#[test]
fn test_edge_case_large_document() {
    // Generate a large markdown document
    let mut source = String::from("# Large Document\n\n");

    for i in 1..=500 {
        source.push_str(&format!("## Section {}\n\nSome content here.\n\n", i));

        if i % 10 == 0 {
            source.push_str(&format!("```rust\nfn function_{}() {{}}\n```\n\n", i));
        }
    }

    let start = std::time::Instant::now();
    let chunks = parser::extract_chunks(&source, "md");
    let duration = start.elapsed();

    let headings = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .count();
    let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

    println!("\nLarge Document Test:");
    println!("  Lines: {}", source.lines().count());
    println!("  Parsed in: {:?}", duration);
    println!("  Headings: {}", headings);
    println!("  Code blocks: {}", code_blocks);

    // Should parse within reasonable time
    assert!(
        duration.as_millis() < 1000,
        "Large document parsing should be fast: {:?}",
        duration
    );

    // Should extract expected elements
    assert!(headings >= 500, "Should extract at least 500 headings");
    assert!(code_blocks >= 45, "Should extract at least 45 code blocks");
}

#[test]
fn test_edge_case_malformed_markdown() {
    let source = r#"# Valid Heading

Some text.

## Unclosed code block

```rust
fn broken(
    // Missing closing backticks

## Another Heading

More text.
"#;

    // Should not panic
    let chunks = parser::extract_chunks(source, "md");

    let headings = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .count();

    println!("\nMalformed Markdown Test:");
    println!("  Chunks extracted: {}", chunks.len());
    println!("  Headings: {}", headings);

    // Should extract at least some valid elements
    assert!(
        headings >= 2,
        "Should extract valid headings even with malformed content"
    );
}

#[test]
fn test_edge_case_unicode_content() {
    let source = r#"# 中文标题

这是中文内容。

## Émoji Section 🚀

```rust
// Comment with émojis 🦀
fn hello() {
    println!("Hello 世界");
}
```
"#;

    let chunks = parser::extract_chunks(source, "md");

    let headings: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .collect();

    println!("\nUnicode Content Test:");
    println!("  Headings: {}", headings.len());

    // Should handle Unicode correctly
    assert!(
        headings.len() >= 2,
        "Should extract headings with Unicode characters"
    );

    // Check that Chinese heading was extracted
    let chinese_heading = headings
        .iter()
        .find(|h| h.symbol_name.as_ref().map_or(false, |n| n.contains("中文")));
    assert!(chinese_heading.is_some(), "Should extract Chinese heading");
}
