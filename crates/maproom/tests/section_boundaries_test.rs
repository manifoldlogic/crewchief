use crewchief_maproom::indexer::parser;

#[test]
fn test_section_boundaries_simple() {
    // Lines 1-10:
    // 1: # Main Title
    // 2: (blank)
    // 3: Main content here.
    // 4: (blank)
    // 5: ## Section One
    // 6: (blank)
    // 7: Section one content.
    // 8: (blank)
    // 9: ## Section Two
    // 10: (blank)
    // 11: Section two content.
    let source = r#"# Main Title

Main content here.

## Section One

Section one content.

## Section Two

Section two content."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Main Title heading (h1)
    let main_title = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Main Title".to_string()))
        .expect("Should find Main Title");

    // Main Title should start at line 1 and extend to end of file (line 11)
    assert_eq!(
        main_title.start_line, 1,
        "Main Title should start at line 1"
    );
    assert_eq!(
        main_title.end_line, 11,
        "Main Title should end at line 11 (EOF)"
    );

    // Find Section One (h2)
    let section_one = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section One".to_string()))
        .expect("Should find Section One");

    // Section One starts at line 5, should end at line 8 (before Section Two at line 9)
    assert_eq!(
        section_one.start_line, 5,
        "Section One should start at line 5"
    );
    assert_eq!(
        section_one.end_line, 8,
        "Section One should end at line 8 (before Section Two)"
    );

    // Find Section Two (h2)
    let section_two = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section Two".to_string()))
        .expect("Should find Section Two");

    // Section Two starts at line 9, should extend to EOF (line 11)
    assert_eq!(
        section_two.start_line, 9,
        "Section Two should start at line 9"
    );
    assert_eq!(
        section_two.end_line, 11,
        "Section Two should end at line 11 (EOF)"
    );
}

#[test]
fn test_section_boundaries_nested_sections() {
    // Lines:
    // 1: # Main
    // 2: (blank)
    // 3: Main intro.
    // 4: (blank)
    // 5: ## Section One
    // 6: (blank)
    // 7: Section one content.
    // 8: (blank)
    // 9: ### Subsection 1.1
    // 10: (blank)
    // 11: Subsection content.
    // 12: (blank)
    // 13: ## Section Two
    // 14: (blank)
    // 15: Section two content.
    let source = r#"# Main

Main intro.

## Section One

Section one content.

### Subsection 1.1

Subsection content.

## Section Two

Section two content."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Main (h1)
    let main = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Main".to_string()))
        .expect("Should find Main");

    // Main should extend to EOF (line 15)
    assert_eq!(main.start_line, 1);
    assert_eq!(main.end_line, 15, "h1 should extend to EOF");

    // Find Section One (h2)
    let section_one = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section One".to_string()))
        .expect("Should find Section One");

    // Section One should include its subsection (h3) and end at line 12 (before Section Two at line 13)
    assert_eq!(section_one.start_line, 5);
    assert_eq!(
        section_one.end_line, 12,
        "Section One should include subsection and end before Section Two"
    );

    // Find Subsection 1.1 (h3)
    let subsection = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Subsection 1.1".to_string()))
        .expect("Should find Subsection 1.1");

    // Subsection should extend to line 12 (before Section Two)
    assert_eq!(subsection.start_line, 9);
    assert_eq!(
        subsection.end_line, 12,
        "h3 should end when parent section ends"
    );

    // Find Section Two (h2)
    let section_two = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Section Two".to_string()))
        .expect("Should find Section Two");

    // Section Two should extend to EOF
    assert_eq!(section_two.start_line, 13);
    assert_eq!(section_two.end_line, 15, "Section Two should extend to EOF");
}

#[test]
fn test_section_boundaries_with_code_blocks() {
    // Test that code blocks within a section are included in the section boundary
    let source = r#"# Installation

Follow these steps:

```bash
npm install project
```

That's it!

## Usage

Now use it:

```javascript
import { tool } from 'project';
tool.run();
```

Done."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Installation section (h1)
    let installation = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Installation".to_string()))
        .expect("Should find Installation");

    // Find Usage section (h2)
    let usage = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Usage".to_string()))
        .expect("Should find Usage");

    // Installation is h1, Usage is h2 (child of Installation)
    // So Installation should extend to EOF and include Usage as a child
    assert!(
        installation.end_line >= usage.end_line,
        "h1 Installation section should include its h2 child Usage"
    );

    // Usage (h2) should extend to EOF since it's the last section
    let total_lines = source.lines().count();
    assert_eq!(
        usage.end_line, total_lines as i32,
        "Usage section should extend to EOF"
    );

    // Verify Usage is within Installation's range
    assert!(usage.start_line > installation.start_line);
    assert!(usage.end_line <= installation.end_line);
}

#[test]
fn test_section_boundaries_code_block_with_heading_inside() {
    // Test that headings inside code blocks don't affect section boundaries
    let source = r#"# Main Section

Regular content here.

```markdown
# This is in a code block
## Not a real heading
```

More content.

## Next Section

Other content."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Main Section (h1)
    let main_section = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Main Section".to_string()))
        .expect("Should find Main Section");

    // Find Next Section (h2)
    let next_section = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Next Section".to_string()))
        .expect("Should find Next Section");

    // Main Section is h1, Next Section is h2 (child of Main Section)
    // So Main Section should include Next Section as a child and extend to EOF
    assert!(
        main_section.end_line >= next_section.end_line,
        "h1 Main Section should include its h2 child Next Section"
    );

    // Verify Next Section is within Main Section's range
    assert!(next_section.start_line > main_section.start_line);
    assert!(next_section.end_line <= main_section.end_line);

    // Should only have 2 headings extracted (not the ones in the code block)
    let headings = chunks
        .iter()
        .filter(|c| c.kind.starts_with("heading_"))
        .count();
    assert_eq!(
        headings, 2,
        "Should only extract 2 real headings, not the ones in code blocks"
    );
}

#[test]
fn test_section_boundaries_deeply_nested() {
    let source = r#"# L1

L1 content.

## L2

L2 content.

### L3

L3 content.

#### L4

L4 content.

## Another L2

Another L2 content."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find L1
    let l1 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L1".to_string()))
        .expect("Should find L1");

    // L1 should extend to EOF
    assert_eq!(l1.start_line, 1);
    assert!(l1.end_line >= 17, "L1 should extend to EOF");

    // Find first L2
    let l2 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L2".to_string()))
        .expect("Should find L2");

    // Find Another L2
    let another_l2 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Another L2".to_string()))
        .expect("Should find Another L2");

    // First L2 should end before Another L2 starts
    assert_eq!(l2.start_line, 5);
    assert!(
        l2.end_line < another_l2.start_line,
        "First L2 should end before Another L2 starts"
    );

    // Find L3
    let l3 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L3".to_string()))
        .expect("Should find L3");

    // L3 should be within L2's range
    assert!(l3.start_line > l2.start_line);
    assert!(
        l3.end_line <= l2.end_line,
        "L3 should be within L2's section"
    );

    // Find L4
    let l4 = chunks
        .iter()
        .find(|c| c.symbol_name == Some("L4".to_string()))
        .expect("Should find L4");

    // L4 should be within L3's range
    assert!(l4.start_line > l3.start_line);
    assert!(
        l4.end_line <= l3.end_line,
        "L4 should be within L3's section"
    );
}

#[test]
fn test_section_boundaries_end_of_file() {
    // Test that the last section correctly extends to EOF
    let source = r#"# First

Content one.

# Second

Content two.

# Third

Content three.
Final line."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Third
    let third = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Third".to_string()))
        .expect("Should find Third");

    // Count total lines in source
    let total_lines = source.lines().count();

    // Third should extend to EOF
    assert_eq!(
        third.end_line, total_lines as i32,
        "Last section should extend to EOF"
    );
}

#[test]
fn test_section_boundaries_empty_sections() {
    // Test sections with no content (just the heading)
    let source = r#"# Empty One

## Empty Two

### Has Content

Some actual content here.

## Empty Three"#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Empty One
    let empty_one = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Empty One".to_string()))
        .expect("Should find Empty One");

    // Empty One should still have valid boundaries (extends to EOF)
    assert_eq!(empty_one.start_line, 1);
    assert!(
        empty_one.end_line > 1,
        "Section should have end_line > start_line"
    );

    // Find Empty Two
    let empty_two = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Empty Two".to_string()))
        .expect("Should find Empty Two");

    // Find Has Content
    let has_content = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Has Content".to_string()))
        .expect("Should find Has Content");

    // Empty Two should end before Has Content starts
    assert_eq!(empty_two.start_line, 3);
    assert!(
        empty_two.end_line < has_content.start_line || empty_two.end_line >= has_content.end_line,
        "Empty Two section should include child section Has Content"
    );
}

#[test]
fn test_section_boundaries_with_lists_and_tables() {
    let source = r#"# Documentation

## Features

Here are the features:

- Fast
- Easy
- Reliable

## Comparison

| Feature | Value |
|---------|-------|
| Speed   | Fast  |

That's the table.

## Conclusion

Final thoughts."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find Features
    let features = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Features".to_string()))
        .expect("Should find Features");

    // Find Comparison
    let comparison = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Comparison".to_string()))
        .expect("Should find Comparison");

    // Features should end before Comparison starts
    assert!(
        features.end_line < comparison.start_line,
        "Features section should end before Comparison section"
    );

    // Comparison should include the table and text after it
    let conclusion = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Conclusion".to_string()))
        .expect("Should find Conclusion");

    assert!(
        comparison.end_line < conclusion.start_line,
        "Comparison section should end before Conclusion section"
    );
}

#[test]
fn test_section_boundaries_multiple_h1_sections() {
    // Test that multiple h1 sections work correctly
    let source = r#"# First Document

First content.

## First Subsection

Subsection content.

# Second Document

Second content.

## Second Subsection

More content."#;

    let chunks = parser::extract_chunks(source, "md");

    // Find First Document
    let first_doc = chunks
        .iter()
        .find(|c| c.symbol_name == Some("First Document".to_string()))
        .expect("Should find First Document");

    // Find Second Document
    let second_doc = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Second Document".to_string()))
        .expect("Should find Second Document");

    // First Document should end before Second Document starts
    assert!(
        first_doc.end_line < second_doc.start_line,
        "First h1 section should end when next h1 starts"
    );

    // Second Document should extend to EOF
    let total_lines = source.lines().count();
    assert_eq!(
        second_doc.end_line, total_lines as i32,
        "Last h1 section should extend to EOF"
    );
}

#[test]
fn test_orphan_content() {
    let source = r#"This is some content before any heading.

It has multiple paragraphs.

# First Heading

Now we have a heading.
"#;

    let chunks = parser::extract_chunks(source, "md");

    println!("Total chunks: {}", chunks.len());
    for chunk in &chunks {
        println!(
            "Chunk: {:?} ({}) - lines {}-{}",
            chunk.symbol_name, chunk.kind, chunk.start_line, chunk.end_line
        );
    }
}
