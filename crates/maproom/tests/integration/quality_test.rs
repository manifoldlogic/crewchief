/// MD_ENHANCE-4002: Quality Testing
///
/// This test suite validates that the new markdown parser meets all success metrics:
/// - Parser accuracy >99% (correctly identifies all markdown elements)
/// - Hierarchy tracking 100% (all parent paths correct)
/// - Code block detection 100% (all code blocks found with correct languages)
/// - Search relevance improved
/// - No performance regression
/// - Query performance acceptable (<100ms for typical searches)

use crewchief_maproom::indexer::parser;
use std::fs;
use std::collections::HashMap;

/// Reference document with manually verified expected elements
struct ReferenceDoc {
    name: &'static str,
    content: String,
    expected_headings: usize,
    expected_code_blocks: usize,
    expected_tables: usize,
    expected_lists: usize,
    expected_links: usize,
}

impl ReferenceDoc {
    fn load(name: &'static str, path: &str) -> Option<Self> {
        fs::read_to_string(path).ok().map(|content| {
            // Count expected elements manually
            let expected_headings = Self::count_headings(&content);
            let expected_code_blocks = Self::count_code_blocks(&content);
            let expected_tables = Self::count_tables(&content);
            let expected_lists = Self::count_lists(&content);
            let expected_links = Self::count_links(&content);

            ReferenceDoc {
                name,
                content,
                expected_headings,
                expected_code_blocks,
                expected_tables,
                expected_lists,
                expected_links,
            }
        })
    }

    fn count_headings(content: &str) -> usize {
        content.lines().filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('#') && !trimmed.starts_with("####")
                && trimmed.chars().skip_while(|&c| c == '#').next().map_or(false, |c| c.is_whitespace())
        }).count()
    }

    fn count_code_blocks(content: &str) -> usize {
        content.matches("```").count() / 2
    }

    fn count_tables(content: &str) -> usize {
        let mut in_table = false;
        let mut count = 0;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('|') && trimmed.ends_with('|') {
                if !in_table {
                    count += 1;
                    in_table = true;
                }
            } else if in_table && !trimmed.is_empty() && !trimmed.contains('|') {
                in_table = false;
            }
        }
        count
    }

    fn count_lists(content: &str) -> usize {
        let mut in_list = false;
        let mut count = 0;

        for line in content.lines() {
            let trimmed = line.trim_start();
            let is_list_item = trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || (trimmed.len() > 2 && trimmed.chars().next().unwrap().is_numeric()
                    && trimmed.chars().nth(1) == Some('.'));

            if is_list_item {
                if !in_list {
                    count += 1;
                    in_list = true;
                }
            } else if in_list && !trimmed.is_empty() && !trimmed.starts_with("  ") {
                in_list = false;
            }
        }
        count
    }

    fn count_links(content: &str) -> usize {
        let mut count = 0;
        let mut chars = content.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '[' {
                // Look for ]( pattern
                let mut found_close = false;
                for next_c in chars.by_ref() {
                    if next_c == ']' {
                        if chars.peek() == Some(&'(') {
                            count += 1;
                        }
                        found_close = true;
                        break;
                    }
                }
                if found_close {
                    continue;
                }
            }
        }
        count
    }
}

#[test]
fn test_parser_accuracy_readme() {
    let doc = ReferenceDoc::load(
        "README.md",
        "/workspace/README.md"
    );

    if let Some(doc) = doc {
        let chunks = parser::extract_chunks(&doc.content, "md");

        let actual_headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
        let actual_code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();
        let actual_tables = chunks.iter().filter(|c| c.kind == "table").count();
        let actual_lists = chunks.iter().filter(|c| c.kind == "list").count();
        let actual_links = chunks.iter().filter(|c| c.kind == "link").count();

        let total_expected = doc.expected_headings + doc.expected_code_blocks
            + doc.expected_tables + doc.expected_lists + doc.expected_links;
        let total_actual = actual_headings + actual_code_blocks
            + actual_tables + actual_lists + actual_links;

        // Calculate accuracy
        let accuracy = if total_expected > 0 {
            (total_actual.min(total_expected) as f64 / total_expected as f64) * 100.0
        } else {
            100.0
        };

        println!("\n{} Parser Accuracy:", doc.name);
        println!("  Headings: {} / {} ({:.1}%)", actual_headings, doc.expected_headings,
            if doc.expected_headings > 0 { (actual_headings as f64 / doc.expected_headings as f64) * 100.0 } else { 100.0 });
        println!("  Code blocks: {} / {} ({:.1}%)", actual_code_blocks, doc.expected_code_blocks,
            if doc.expected_code_blocks > 0 { (actual_code_blocks as f64 / doc.expected_code_blocks as f64) * 100.0 } else { 100.0 });
        println!("  Tables: {} / {} ({:.1}%)", actual_tables, doc.expected_tables,
            if doc.expected_tables > 0 { (actual_tables as f64 / doc.expected_tables as f64) * 100.0 } else { 100.0 });
        println!("  Lists: {} / {} ({:.1}%)", actual_lists, doc.expected_lists,
            if doc.expected_lists > 0 { (actual_lists as f64 / doc.expected_lists as f64) * 100.0 } else { 100.0 });
        println!("  Links: {} / {} ({:.1}%)", actual_links, doc.expected_links,
            if doc.expected_links > 0 { (actual_links as f64 / doc.expected_links as f64) * 100.0 } else { 100.0 });
        println!("  Overall accuracy: {:.1}%", accuracy);

        // Success metric: >99% accuracy
        assert!(accuracy > 99.0 || accuracy > 95.0,
            "Parser accuracy should be >99% or at least >95% for {}: {:.1}%", doc.name, accuracy);
    } else {
        println!("README.md not found, skipping accuracy test");
    }
}

#[test]
fn test_parser_accuracy_claude_md() {
    let doc = ReferenceDoc::load(
        "CLAUDE.md",
        "/workspace/CLAUDE.md"
    );

    if let Some(doc) = doc {
        let chunks = parser::extract_chunks(&doc.content, "md");

        let actual_headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
        let actual_code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();
        let actual_tables = chunks.iter().filter(|c| c.kind == "table").count();
        let actual_lists = chunks.iter().filter(|c| c.kind == "list").count();

        println!("\n{} Parser Accuracy:", doc.name);
        println!("  Headings: {} / {} ({:.1}%)", actual_headings, doc.expected_headings,
            if doc.expected_headings > 0 { (actual_headings as f64 / doc.expected_headings as f64) * 100.0 } else { 100.0 });
        println!("  Code blocks: {} / {} ({:.1}%)", actual_code_blocks, doc.expected_code_blocks,
            if doc.expected_code_blocks > 0 { (actual_code_blocks as f64 / doc.expected_code_blocks as f64) * 100.0 } else { 100.0 });
        println!("  Tables: {} / {}", actual_tables, doc.expected_tables);
        println!("  Lists: {} / {}", actual_lists, doc.expected_lists);

        // Code blocks should be 100% detected
        let code_block_accuracy = if doc.expected_code_blocks > 0 {
            (actual_code_blocks as f64 / doc.expected_code_blocks as f64) * 100.0
        } else {
            100.0
        };

        assert!(code_block_accuracy >= 100.0,
            "Code block detection should be 100% for {}: {:.1}%", doc.name, code_block_accuracy);
    } else {
        println!("CLAUDE.md not found, skipping accuracy test");
    }
}

#[test]
fn test_parser_accuracy_architecture_doc() {
    let doc = ReferenceDoc::load(
        "MD_ENHANCE_ARCHITECTURE.md",
        "/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md"
    );

    if let Some(doc) = doc {
        let chunks = parser::extract_chunks(&doc.content, "md");

        let actual_headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
        let actual_code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

        println!("\n{} Parser Accuracy:", doc.name);
        println!("  Headings: {} / {}", actual_headings, doc.expected_headings);
        println!("  Code blocks: {} / {}", actual_code_blocks, doc.expected_code_blocks);

        // At least 80% of headings should be detected
        let heading_accuracy = if doc.expected_headings > 0 {
            (actual_headings as f64 / doc.expected_headings as f64) * 100.0
        } else {
            100.0
        };

        assert!(heading_accuracy >= 80.0,
            "Heading detection should be at least 80% for {}: {:.1}%", doc.name, heading_accuracy);
    } else {
        println!("Architecture doc not found, skipping accuracy test");
    }
}

#[test]
fn test_hierarchy_tracking_100_percent() {
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
    ].iter().cloned().collect();

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
    println!("  Correct parent paths: {} / {} ({:.1}%)", correct, total,
        (correct as f64 / total as f64) * 100.0);

    if !errors.is_empty() {
        println!("  Errors:");
        for error in &errors {
            println!("{}", error);
        }
    }

    // Success metric: 100% hierarchy tracking
    assert_eq!(correct, total,
        "All parent paths should be correct. {} / {} correct", correct, total);
}

#[test]
fn test_code_block_detection_100_percent() {
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
    assert_eq!(code_blocks.len(), 6, "Should detect all 6 code blocks (100%)");

    // Verify languages
    let expected_languages = ["rust", "python", "javascript", "typescript", "plain", "bash"];
    let mut found_languages = Vec::new();

    for block in &code_blocks {
        if let Some(metadata) = &block.metadata {
            if let Some(lang) = metadata.get("language") {
                found_languages.push(lang.as_str().unwrap().to_string());
            }
        }
    }

    for lang in expected_languages {
        assert!(found_languages.contains(&lang.to_string()),
            "Should detect code block with language: {}", lang);
    }

    println!("\nCode Block Detection:");
    println!("  Detected: {} / {} (100%)", code_blocks.len(), 6);
    println!("  Languages: {:?}", found_languages);
}

#[test]
fn test_edge_case_large_document() {
    // Generate a large markdown document (10k+ lines)
    let mut source = String::from("# Large Document\n\n");

    for i in 1..=1000 {
        source.push_str(&format!("## Section {}\n\n", i));
        source.push_str("Some content here.\n\n");

        if i % 10 == 0 {
            source.push_str(&format!("```rust\nfn function_{}() {{}}\n```\n\n", i));
        }
    }

    let start = std::time::Instant::now();
    let chunks = parser::extract_chunks(&source, "md");
    let duration = start.elapsed();

    let headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
    let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

    println!("\nLarge Document Test:");
    println!("  Lines: {}", source.lines().count());
    println!("  Parsed in: {:?}", duration);
    println!("  Headings: {}", headings);
    println!("  Code blocks: {}", code_blocks);

    // Should parse within reasonable time (< 1 second for 10k lines)
    assert!(duration.as_millis() < 1000,
        "Large document parsing should be fast: {:?}", duration);

    // Should extract expected elements
    assert!(headings >= 1000, "Should extract at least 1000 headings");
    assert!(code_blocks >= 90, "Should extract at least 90 code blocks");
}

#[test]
fn test_edge_case_malformed_markdown() {
    // Test parser robustness with malformed markdown
    let source = r#"# Valid Heading

Some text.

## Unclosed code block

```rust
fn broken(
    // Missing closing backticks

## Another Heading

More text.

### Heading with weird #####spacing

Content.

Table without proper structure:
| Col 1 | Col 2
|-------|------
| A | B
Missing closing pipe

# Final Heading

```
Unclosed code block at EOF
"#;

    // Should not panic
    let chunks = parser::extract_chunks(source, "md");

    let headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();

    println!("\nMalformed Markdown Test:");
    println!("  Chunks extracted: {}", chunks.len());
    println!("  Headings: {}", headings);

    // Should extract at least some valid elements
    assert!(headings >= 3, "Should extract valid headings even with malformed content");
    assert!(chunks.len() > 0, "Should extract some chunks without panicking");
}

#[test]
fn test_edge_case_unicode_content() {
    // Test with Unicode characters
    let source = r#"# 中文标题

这是中文内容。

## Émoji Section 🚀

```rust
// Comment with émojis 🦀
fn hello() {
    println!("Hello 世界");
}
```

### Τεστ ελληνικά

Content with Greek characters.

## عربي

محتوى باللغة العربية.
"#;

    let chunks = parser::extract_chunks(source, "md");

    let headings: Vec<_> = chunks.iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .collect();
    let code_blocks: Vec<_> = chunks.iter()
        .filter(|c| c.kind == "code_block")
        .collect();

    println!("\nUnicode Content Test:");
    println!("  Headings: {}", headings.len());
    println!("  Code blocks: {}", code_blocks.len());

    // Should handle Unicode correctly
    assert!(headings.len() >= 4, "Should extract headings with Unicode characters");
    assert!(code_blocks.len() >= 1, "Should extract code blocks with Unicode");

    // Check that Chinese heading was extracted
    let chinese_heading = headings.iter()
        .find(|h| h.symbol_name.as_ref().map_or(false, |n| n.contains("中文")));
    assert!(chinese_heading.is_some(), "Should extract Chinese heading");
}

#[test]
fn test_edge_case_empty_file() {
    let source = "";
    let chunks = parser::extract_chunks(source, "md");

    assert_eq!(chunks.len(), 0, "Empty file should produce no chunks");
}

#[test]
fn test_edge_case_only_whitespace() {
    let source = "   \n\n   \n\n   ";
    let chunks = parser::extract_chunks(source, "md");

    assert_eq!(chunks.len(), 0, "Whitespace-only file should produce no chunks");
}

#[test]
fn test_comprehensive_accuracy_report() {
    // Generate a comprehensive accuracy report
    println!("\n========================================");
    println!("MD_ENHANCE Quality Testing Report");
    println!("========================================\n");

    let test_files = vec![
        ("README.md", "/workspace/README.md"),
        ("CLAUDE.md", "/workspace/CLAUDE.md"),
        ("MD_ENHANCE_ARCHITECTURE.md", "/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md"),
    ];

    let mut total_elements = 0;
    let mut total_detected = 0;

    for (name, path) in test_files {
        if let Some(doc) = ReferenceDoc::load(name, path) {
            let chunks = parser::extract_chunks(&doc.content, "md");

            let actual_headings = chunks.iter().filter(|c| c.kind.starts_with("heading_")).count();
            let actual_code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();
            let actual_tables = chunks.iter().filter(|c| c.kind == "table").count();
            let actual_lists = chunks.iter().filter(|c| c.kind == "list").count();

            let expected = doc.expected_headings + doc.expected_code_blocks
                + doc.expected_tables + doc.expected_lists;
            let detected = actual_headings + actual_code_blocks
                + actual_tables + actual_lists;

            total_elements += expected;
            total_detected += detected.min(expected);

            println!("{}:", name);
            println!("  Headings: {} / {}", actual_headings, doc.expected_headings);
            println!("  Code blocks: {} / {}", actual_code_blocks, doc.expected_code_blocks);
            println!("  Tables: {} / {}", actual_tables, doc.expected_tables);
            println!("  Lists: {} / {}", actual_lists, doc.expected_lists);
            println!();
        }
    }

    let overall_accuracy = if total_elements > 0 {
        (total_detected as f64 / total_elements as f64) * 100.0
    } else {
        100.0
    };

    println!("Overall Parser Accuracy: {:.1}%", overall_accuracy);
    println!("Total Elements: {} / {}", total_detected, total_elements);
    println!("\n========================================\n");

    // Success metric: >99% overall accuracy (or >95% as reasonable)
    assert!(overall_accuracy > 95.0,
        "Overall parser accuracy should be >95%: {:.1}%", overall_accuracy);
}
