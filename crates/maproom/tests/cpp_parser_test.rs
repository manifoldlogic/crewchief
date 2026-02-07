use crewchief_maproom::indexer::parser;

#[test]
fn test_cpp_smoke() {
    let source = r#"
class Foo {
public:
    void bar() {}
};
"#;
    let chunks = parser::extract_chunks(source, "cpp");
    // Phase 2: Parser now extracts actual chunks
    assert!(!chunks.is_empty(), "Expected chunks to be extracted");

    // Should extract class Foo and method bar
    let class_chunk = chunks.iter().find(|c| c.kind == "class");
    assert!(class_chunk.is_some(), "Expected class chunk");
    assert_eq!(class_chunk.unwrap().symbol_name, Some("Foo".to_string()));

    let method_chunk = chunks.iter().find(|c| c.kind == "method");
    assert!(method_chunk.is_some(), "Expected method chunk");
    assert_eq!(method_chunk.unwrap().symbol_name, Some("bar".to_string()));
}

#[test]
fn test_cpp_namespace() {
    let source = r#"
namespace myapp {
    class Widget {
    public:
        void render();
    };
}
"#;
    let chunks = parser::extract_chunks(source, "cpp");
    // Phase 2: Parser now extracts actual chunks
    assert!(!chunks.is_empty(), "Expected chunks to be extracted");

    // Should extract namespace myapp
    let namespace_chunk = chunks.iter().find(|c| c.kind == "namespace");
    assert!(namespace_chunk.is_some(), "Expected namespace chunk");
    assert_eq!(namespace_chunk.unwrap().symbol_name, Some("myapp".to_string()));

    // Should extract class Widget
    let class_chunk = chunks.iter().find(|c| c.kind == "class");
    assert!(class_chunk.is_some(), "Expected class chunk");
    assert_eq!(class_chunk.unwrap().symbol_name, Some("Widget".to_string()));
}

#[test]
fn test_cpp_includes() {
    let source = r#"
#include <iostream>
#include "myheader.h"

int main() {
    return 0;
}
"#;
    let chunks = parser::extract_chunks(source, "cpp");
    // Phase 2: Parser now extracts actual chunks
    assert!(!chunks.is_empty(), "Expected chunks to be extracted");

    // Should extract function main
    let func_chunk = chunks.iter().find(|c| c.kind == "func");
    assert!(func_chunk.is_some(), "Expected function chunk");
    assert_eq!(func_chunk.unwrap().symbol_name, Some("main".to_string()));

    // Should extract __imports__ chunk with includes
    let imports_chunk = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports_chunk.is_some(), "Expected imports chunk");
    assert_eq!(imports_chunk.unwrap().symbol_name, Some("__imports__".to_string()));

    // Verify import metadata contains both includes
    let metadata = imports_chunk.unwrap().metadata.as_ref().unwrap();
    let imports = metadata.get("imports").and_then(|v| v.as_array());
    assert!(imports.is_some(), "Expected imports array in metadata");
    assert_eq!(imports.unwrap().len(), 2, "Expected 2 includes");
}
