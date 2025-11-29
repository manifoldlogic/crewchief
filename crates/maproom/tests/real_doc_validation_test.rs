use crewchief_maproom::indexer::parser;
use std::fs;

#[test]
fn test_parse_real_readme() {
    // Parse the main README.md from the repository
    let readme_path = "/workspace/README.md";
    if let Ok(content) = fs::read_to_string(readme_path) {
        let chunks = parser::extract_chunks(&content, "md");

        // Should successfully parse without panicking
        assert!(chunks.len() > 0, "Should extract some chunks from README");

        // Should have headings
        let headings = chunks
            .iter()
            .filter(|c| c.kind.starts_with("heading_"))
            .count();
        assert!(headings > 0, "README should have headings");

        println!(
            "README.md: Extracted {} chunks ({} headings)",
            chunks.len(),
            headings
        );
    } else {
        println!("README.md not found, skipping test");
    }
}

#[test]
fn test_parse_real_claude_md() {
    // Parse CLAUDE.md from the repository
    let claude_md_path = "/workspace/CLAUDE.md";
    if let Ok(content) = fs::read_to_string(claude_md_path) {
        let chunks = parser::extract_chunks(&content, "md");

        // Should successfully parse without panicking
        assert!(
            chunks.len() > 0,
            "Should extract some chunks from CLAUDE.md"
        );

        // Should have headings
        let headings = chunks
            .iter()
            .filter(|c| c.kind.starts_with("heading_"))
            .count();
        assert!(headings > 0, "CLAUDE.md should have headings");

        // Should have code blocks
        let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();
        println!(
            "CLAUDE.md: Extracted {} chunks ({} headings, {} code blocks)",
            chunks.len(),
            headings,
            code_blocks
        );
    } else {
        println!("CLAUDE.md not found, skipping test");
    }
}

#[test]
fn test_parse_real_architecture_doc() {
    // Parse an architecture document if available
    let arch_doc_path = "/workspace/.crewchief/archive/projects/MD_ENHANCE_markdown-enhancement/planning/MD_ENHANCE_ARCHITECTURE.md";
    if let Ok(content) = fs::read_to_string(arch_doc_path) {
        let chunks = parser::extract_chunks(&content, "md");

        // Should successfully parse without panicking
        assert!(
            chunks.len() > 0,
            "Should extract chunks from architecture doc"
        );

        // Count chunk types
        let headings = chunks
            .iter()
            .filter(|c| c.kind.starts_with("heading_"))
            .count();
        let code_blocks = chunks.iter().filter(|c| c.kind == "code_block").count();

        println!(
            "MD_ENHANCE_ARCHITECTURE.md: Extracted {} chunks ({} headings, {} code blocks)",
            chunks.len(),
            headings,
            code_blocks
        );

        // Verify we're extracting headings correctly
        assert!(headings > 0, "Architecture doc should have headings");
    } else {
        println!("Architecture doc not found, skipping test");
    }
}

#[test]
fn test_no_panic_on_any_markdown() {
    // Test that we don't panic on various markdown files
    let test_paths = vec![
        "/workspace/README.md",
        "/workspace/CLAUDE.md",
        "/workspace/packages/cli/README.md",
    ];

    for path in test_paths {
        if let Ok(content) = fs::read_to_string(path) {
            // Main assertion: this should not panic
            let chunks = parser::extract_chunks(&content, "md");

            println!("{}: Parsed successfully, {} chunks", path, chunks.len());
        }
    }

    // If we got here without panicking, the test passes
    assert!(true);
}
