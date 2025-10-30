use tree_sitter::{Language, Parser};

fn lang_markdown() -> Language { tree_sitter_md::language() }

#[test]
fn debug_markdown_tree() {
    let source = r#"# Hello World

Some content here.

```rust
fn main() {
    println!("test");
}
```

## Section

More content with a [link](https://example.com).
"#;

    let mut parser = Parser::new();
    parser.set_language(&lang_markdown()).unwrap();

    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    println!("Root node: {:?}", root.kind());
    println!("Root child count: {}", root.child_count());

    fn print_tree(node: tree_sitter::Node, source: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        let text = node.utf8_text(source.as_bytes()).unwrap_or("???");
        let preview = if text.len() > 50 {
            format!("{}...", &text[..50])
        } else {
            text.to_string()
        };
        println!("{}{} [{}:{}] {:?}",
            indent,
            node.kind(),
            node.start_position().row,
            node.end_position().row,
            preview.replace("\n", "\\n")
        );

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                print_tree(child, source, depth + 1);
            }
        }
    }

    print_tree(root, source, 0);
}
