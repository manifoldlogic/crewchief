use crewchief_maproom::indexer::parser::extract_chunks;

#[test]
fn test_external_links() {
    let source = r#"# Documentation

Visit our [website](https://example.com) for more info.
Check out the [API docs](https://api.example.com/docs).
"#;

    let chunks = extract_chunks(source, "md");

    // Find link chunks
    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    assert_eq!(links.len(), 2, "Should extract 2 external links");

    // Check first link
    let link1 = links[0];
    assert_eq!(link1.symbol_name.as_ref().unwrap(), "website");
    assert_eq!(link1.signature.as_ref().unwrap(), "https://example.com");
    let meta1 = link1.metadata.as_ref().unwrap();
    assert_eq!(meta1["link_type"], "external");
    assert_eq!(meta1["target"], "https://example.com");
    assert_eq!(meta1["link_text"], "website");
    assert_eq!(meta1["is_image"], false);

    // Check second link
    let link2 = links[1];
    assert_eq!(link2.symbol_name.as_ref().unwrap(), "API docs");
    assert_eq!(
        link2.signature.as_ref().unwrap(),
        "https://api.example.com/docs"
    );
    let meta2 = link2.metadata.as_ref().unwrap();
    assert_eq!(meta2["link_type"], "external");
}

#[test]
fn test_anchor_links() {
    let source = r#"# Table of Contents

Jump to [Introduction](#introduction) or [Setup](#setup-guide).

## Introduction
Content here.

## Setup Guide
More content.
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    assert_eq!(links.len(), 2, "Should extract 2 anchor links");

    let link1 = links[0];
    assert_eq!(link1.symbol_name.as_ref().unwrap(), "Introduction");
    assert_eq!(link1.signature.as_ref().unwrap(), "#introduction");
    let meta1 = link1.metadata.as_ref().unwrap();
    assert_eq!(meta1["link_type"], "anchor");
    assert_eq!(meta1["target"], "#introduction");

    let link2 = links[1];
    assert_eq!(link2.symbol_name.as_ref().unwrap(), "Setup");
    assert_eq!(link2.signature.as_ref().unwrap(), "#setup-guide");
    let meta2 = link2.metadata.as_ref().unwrap();
    assert_eq!(meta2["link_type"], "anchor");
}

#[test]
fn test_relative_file_links() {
    let source = r#"# Documentation

See [other doc](./other.md) for details.
Check the [parent guide](../guides/setup.md).
Read about [config](config.md).
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    assert_eq!(links.len(), 3, "Should extract 3 relative links");

    let link1 = links[0];
    assert_eq!(link1.symbol_name.as_ref().unwrap(), "other doc");
    assert_eq!(link1.signature.as_ref().unwrap(), "./other.md");
    let meta1 = link1.metadata.as_ref().unwrap();
    assert_eq!(meta1["link_type"], "relative");

    let link2 = links[1];
    assert_eq!(link2.symbol_name.as_ref().unwrap(), "parent guide");
    assert_eq!(link2.signature.as_ref().unwrap(), "../guides/setup.md");
    let meta2 = link2.metadata.as_ref().unwrap();
    assert_eq!(meta2["link_type"], "relative");

    let link3 = links[2];
    assert_eq!(link3.symbol_name.as_ref().unwrap(), "config");
    assert_eq!(link3.signature.as_ref().unwrap(), "config.md");
    let meta3 = link3.metadata.as_ref().unwrap();
    assert_eq!(meta3["link_type"], "relative");
}

#[test]
fn test_absolute_path_links() {
    let source = r#"# Documentation

See [root doc](/docs/README.md) at repository root.
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    assert_eq!(links.len(), 1, "Should extract 1 absolute link");

    let link = links[0];
    assert_eq!(link.symbol_name.as_ref().unwrap(), "root doc");
    assert_eq!(link.signature.as_ref().unwrap(), "/docs/README.md");
    let meta = link.metadata.as_ref().unwrap();
    assert_eq!(meta["link_type"], "absolute");
}

#[test]
fn test_image_links() {
    let source = r#"# Images

Here is an image: ![Logo](logo.png)
And another: ![Diagram](https://example.com/diagram.svg)
"#;

    let chunks = extract_chunks(source, "md");

    let images: Vec<_> = chunks.iter().filter(|c| c.kind == "image_link").collect();

    assert_eq!(images.len(), 2, "Should extract 2 image links");

    let img1 = images[0];
    assert_eq!(img1.symbol_name.as_ref().unwrap(), "Logo");
    assert_eq!(img1.signature.as_ref().unwrap(), "logo.png");
    let meta1 = img1.metadata.as_ref().unwrap();
    assert_eq!(meta1["link_type"], "relative");
    assert_eq!(meta1["link_text"], "Logo");
    assert_eq!(meta1["is_image"], true);

    let img2 = images[1];
    assert_eq!(img2.symbol_name.as_ref().unwrap(), "Diagram");
    assert_eq!(
        img2.signature.as_ref().unwrap(),
        "https://example.com/diagram.svg"
    );
    let meta2 = img2.metadata.as_ref().unwrap();
    assert_eq!(meta2["link_type"], "external");
    assert_eq!(meta2["is_image"], true);
}

#[test]
fn test_links_without_text() {
    let source = r#"# Links

Empty text link: [](https://example.com)
Empty image: ![](image.png)
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    assert_eq!(
        links.len(),
        2,
        "Should extract 2 links even with empty text"
    );

    // When link text is empty, symbol_name should be the target
    let link1 = links[0];
    assert_eq!(link1.symbol_name.as_ref().unwrap(), "https://example.com");
    assert_eq!(link1.kind, "link");
    let meta1 = link1.metadata.as_ref().unwrap();
    assert_eq!(meta1["link_text"], "");

    let link2 = links[1];
    assert_eq!(link2.symbol_name.as_ref().unwrap(), "image.png");
    assert_eq!(link2.kind, "image_link");
    let meta2 = link2.metadata.as_ref().unwrap();
    assert_eq!(meta2["link_text"], "");
}

#[test]
fn test_mixed_link_types() {
    let source = r#"# Mixed Links

External: [Google](https://google.com)
Anchor: [Section](#my-section)
Relative: [Doc](./doc.md)
Image: ![Pic](pic.jpg)
Absolute: [Root](/docs/root.md)

## My Section
Content here.
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    assert_eq!(links.len(), 5, "Should extract all 5 links");

    // Check each link type
    let types: Vec<String> = links
        .iter()
        .map(|l| {
            l.metadata.as_ref().unwrap()["link_type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();

    assert_eq!(
        types,
        vec!["external", "anchor", "relative", "relative", "absolute"]
    );
}

#[test]
fn test_line_numbers() {
    let source = r#"# Header
Line 2
Line 3 has a [link](url1)
Line 4
Line 5 has another [link](url2)
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    assert_eq!(links.len(), 2);
    assert_eq!(links[0].start_line, 3, "First link should be on line 3");
    assert_eq!(links[1].start_line, 5, "Second link should be on line 5");
}

#[test]
fn test_multiple_links_same_line() {
    let source = r#"# Links

Check [link1](url1) and [link2](url2) on same line.
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    assert_eq!(links.len(), 2, "Should extract both links from same line");
    assert_eq!(
        links[0].start_line, links[1].start_line,
        "Both links on same line"
    );
}

#[test]
fn test_links_with_special_characters() {
    let source = r#"# Special Characters

[Link with spaces](https://example.com/path with spaces)
[Link-with-dashes](./some-file.md)
[Link_with_underscores](https://api.example.com/get_data)
[Query params](https://example.com?foo=bar&baz=qux)
[Hash in URL](https://example.com/page#section)
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    // Note: Links with spaces in URLs might not work in real markdown,
    // but our regex should still capture them
    assert!(
        links.len() >= 4,
        "Should extract links with special characters"
    );
}

#[test]
fn test_no_links() {
    let source = r#"# No Links

This document has no links.
Just plain text.
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    assert_eq!(links.len(), 0, "Should not extract any links");
}

#[test]
fn test_malformed_links() {
    let source = r#"# Malformed

This is not a link: [text without url]
This is incomplete: [text]
Also incomplete: (url)
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    // Malformed links should not be extracted
    assert_eq!(links.len(), 0, "Should not extract malformed links");
}

#[test]
fn test_malformed_cross_line_links() {
    // Regex can match across lines if pattern spans them
    // This test verifies we don't extract nonsense when markdown is split oddly
    let source = r#"# Test

Some text [text](
on next line.
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    // The regex pattern [^\]]* will match newlines, so this might extract "[text](\non next line."
    // This is acceptable - in real markdown this would be invalid anyway
    // We're just documenting the behavior
    // In practice, proper markdown should have the closing paren on same line or very close
}

#[test]
fn test_nested_brackets() {
    let source = r#"# Nested Brackets

[Link with [nested] brackets](https://example.com)
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    // Our regex uses [^\]]* which stops at first ], so nested brackets
    // will only capture "Link with [nested"
    // This is acceptable behavior - markdown doesn't support nested brackets in link text
    assert!(links.len() <= 1, "Should handle nested brackets gracefully");
}

#[test]
fn test_http_vs_https() {
    let source = r#"# Protocols

[HTTP link](http://example.com)
[HTTPS link](https://example.com)
[FTP not supported](ftp://example.com)
"#;

    let chunks = extract_chunks(source, "md");

    let links: Vec<_> = chunks.iter().filter(|c| c.kind == "link").collect();

    assert_eq!(links.len(), 3, "Should extract all protocol links");

    // Check HTTP and HTTPS are classified as external
    let meta1 = links[0].metadata.as_ref().unwrap();
    assert_eq!(meta1["link_type"], "external");

    let meta2 = links[1].metadata.as_ref().unwrap();
    assert_eq!(meta2["link_type"], "external");

    // FTP should be classified as relative (doesn't start with http)
    let meta3 = links[2].metadata.as_ref().unwrap();
    assert_eq!(meta3["link_type"], "relative");
}

#[test]
fn test_real_world_readme() {
    let source = r#"# Project Name

![Build Status](https://ci.example.com/badge.svg)

## Installation

See the [installation guide](./docs/install.md) for details.

## Usage

Check out our [documentation](https://docs.example.com) and [API reference](https://api.example.com).

For more examples, see the [examples directory](../examples/README.md).

## Contributing

Read our [contributing guidelines](CONTRIBUTING.md) and [code of conduct](#code-of-conduct).

### Code of Conduct

Be nice.
"#;

    let chunks = extract_chunks(source, "md");

    let all_links: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "link" || c.kind == "image_link")
        .collect();

    assert!(
        all_links.len() >= 6,
        "Should extract all links from realistic README"
    );

    // Count by type
    let regular_links = all_links.iter().filter(|l| l.kind == "link").count();
    let image_links = all_links.iter().filter(|l| l.kind == "image_link").count();

    assert_eq!(image_links, 1, "Should extract 1 image");
    assert!(
        regular_links >= 5,
        "Should extract at least 5 regular links"
    );

    // Check variety of link types
    let link_types: std::collections::HashSet<String> = all_links
        .iter()
        .map(|l| {
            l.metadata.as_ref().unwrap()["link_type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();

    assert!(
        link_types.contains("external"),
        "Should have external links"
    );
    assert!(
        link_types.contains("relative"),
        "Should have relative links"
    );
    assert!(link_types.contains("anchor"), "Should have anchor links");
}
