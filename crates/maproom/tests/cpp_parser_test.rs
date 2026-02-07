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
    // At this phase, empty chunks are expected - smoke test just ensures no panic
    // No panic is the success criteria
    assert!(chunks.is_empty());
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
    // Skeletal implementation returns empty chunks - this is expected in Phase 1
    // No panic is the success criteria
    assert!(chunks.is_empty());
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
    // Skeletal implementation returns empty chunks - this is expected in Phase 1
    // No panic is the success criteria
    assert!(chunks.is_empty());
}
