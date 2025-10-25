use crewchief_maproom::indexer::parser::extract_chunks;
use std::fs;

#[test]
fn test_extract_links_from_readme() {
    let source = fs::read_to_string("/workspace/README.md")
        .expect("Failed to read README.md");

    let chunks = extract_chunks(&source, "md");

    let links: Vec<_> = chunks.iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    // README.md should have at least some links
    assert!(links.len() > 0, "README.md should contain links");

    println!("\nExtracted {} links from README.md:", links.len());
    for (i, link) in links.iter().enumerate() {
        let meta = link.metadata.as_ref().unwrap();
        println!("  {}. [{}] {} -> {} (line {})",
            i + 1,
            link.kind,
            link.symbol_name.as_ref().unwrap_or(&"<no text>".to_string()),
            link.signature.as_ref().unwrap_or(&"<no target>".to_string()),
            link.start_line
        );
        println!("      Type: {}, Is Image: {}",
            meta["link_type"],
            meta["is_image"]
        );
    }
}

#[test]
fn test_extract_links_from_claude_md() {
    let source = fs::read_to_string("/workspace/CLAUDE.md")
        .expect("Failed to read CLAUDE.md");

    let chunks = extract_chunks(&source, "md");

    let links: Vec<_> = chunks.iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    println!("\nExtracted {} links from CLAUDE.md:", links.len());

    // Count by type
    let external_links = links.iter()
        .filter(|l| l.metadata.as_ref().unwrap()["link_type"] == "external")
        .count();
    let relative_links = links.iter()
        .filter(|l| l.metadata.as_ref().unwrap()["link_type"] == "relative")
        .count();
    let anchor_links = links.iter()
        .filter(|l| l.metadata.as_ref().unwrap()["link_type"] == "anchor")
        .count();

    println!("  External: {}", external_links);
    println!("  Relative: {}", relative_links);
    println!("  Anchor: {}", anchor_links);

    // Show some sample links
    for link in links.iter().take(5) {
        let meta = link.metadata.as_ref().unwrap();
        println!("  - [{}] {} -> {}",
            meta["link_type"],
            link.symbol_name.as_ref().unwrap_or(&"<no text>".to_string()),
            link.signature.as_ref().unwrap_or(&"<no target>".to_string())
        );
    }
}

#[test]
fn test_link_classification_accuracy() {
    let source = r#"# Test Doc

[External](https://example.com)
[Anchor](#section)
[Relative](./other.md)
[Absolute](/docs/readme.md)
![Image](image.png)
"#;

    let chunks = extract_chunks(&source, "md");
    let links: Vec<_> = chunks.iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    assert_eq!(links.len(), 5);

    // Verify each link is classified correctly
    let external = &links[0];
    assert_eq!(external.metadata.as_ref().unwrap()["link_type"], "external");
    assert_eq!(external.kind, "link");

    let anchor = &links[1];
    assert_eq!(anchor.metadata.as_ref().unwrap()["link_type"], "anchor");
    assert_eq!(anchor.kind, "link");

    let relative = &links[2];
    assert_eq!(relative.metadata.as_ref().unwrap()["link_type"], "relative");
    assert_eq!(relative.kind, "link");

    let absolute = &links[3];
    assert_eq!(absolute.metadata.as_ref().unwrap()["link_type"], "absolute");
    assert_eq!(absolute.kind, "link");

    let image = &links[4];
    assert_eq!(image.metadata.as_ref().unwrap()["link_type"], "relative");
    assert_eq!(image.kind, "image_link");
    assert_eq!(image.metadata.as_ref().unwrap()["is_image"], true);
}
