//! Integration tests for language detection.

use crewchief_maproom::context::language_detector::{Language, LanguageDetector};

#[test]
fn test_detect_rust_files() {
    let detector = LanguageDetector::new();

    assert_eq!(detector.detect_from_path("src/main.rs"), Language::Rust);
    assert_eq!(detector.detect_from_path("lib.rs"), Language::Rust);
    assert_eq!(
        detector.detect_from_path("crates/maproom/src/context/strategy.rs"),
        Language::Rust
    );
}

#[test]
fn test_detect_python_files() {
    let detector = LanguageDetector::new();

    assert_eq!(detector.detect_from_path("main.py"), Language::Python);
    assert_eq!(
        detector.detect_from_path("src/utils/__init__.py"),
        Language::Python
    );
    assert_eq!(detector.detect_from_path("types.pyi"), Language::Python);
    assert_eq!(
        detector.detect_from_path("tests/test_main.py"),
        Language::Python
    );
}

#[test]
fn test_detect_typescript_files() {
    let detector = LanguageDetector::new();

    assert_eq!(
        detector.detect_from_path("src/app.ts"),
        Language::TypeScript
    );
    assert_eq!(
        detector.detect_from_path("src/Component.tsx"),
        Language::TypeScript
    );
    assert_eq!(detector.detect_from_path("index.mts"), Language::TypeScript);
    assert_eq!(detector.detect_from_path("types.d.ts"), Language::TypeScript);
}

#[test]
fn test_detect_javascript_files() {
    let detector = LanguageDetector::new();

    assert_eq!(
        detector.detect_from_path("src/app.js"),
        Language::JavaScript
    );
    assert_eq!(
        detector.detect_from_path("src/Component.jsx"),
        Language::JavaScript
    );
    assert_eq!(detector.detect_from_path("index.mjs"), Language::JavaScript);
}

#[test]
fn test_detect_other_languages() {
    let detector = LanguageDetector::new();

    assert_eq!(detector.detect_from_path("main.go"), Language::Go);
    assert_eq!(detector.detect_from_path("Main.java"), Language::Java);
    assert_eq!(detector.detect_from_path("main.cpp"), Language::Cpp);
    assert_eq!(detector.detect_from_path("main.c"), Language::Cpp);
    assert_eq!(detector.detect_from_path("header.hpp"), Language::Cpp);
}

#[test]
fn test_detect_unknown_extensions() {
    let detector = LanguageDetector::new();

    assert_eq!(detector.detect_from_path("README.md"), Language::Unknown);
    assert_eq!(detector.detect_from_path("config.json"), Language::Unknown);
    assert_eq!(detector.detect_from_path("data.xml"), Language::Unknown);
    assert_eq!(
        detector.detect_from_path("Dockerfile"),
        Language::Unknown
    );
}

#[test]
fn test_detect_cached_performance() {
    let mut detector = LanguageDetector::new();

    // First detection
    let path = "src/main.rs";
    let start = std::time::Instant::now();
    let lang1 = detector.detect_cached(path);
    let duration1 = start.elapsed();

    // Second detection (should use cache)
    let start = std::time::Instant::now();
    let lang2 = detector.detect_cached(path);
    let duration2 = start.elapsed();

    assert_eq!(lang1, Language::Rust);
    assert_eq!(lang2, Language::Rust);

    // Cache should be faster (though micro-optimization, just checking cache works)
    assert!(duration2 <= duration1);
    assert_eq!(detector.cache_size(), 1);
}

#[test]
fn test_cache_multiple_files() {
    let mut detector = LanguageDetector::new();

    detector.detect_cached("file1.rs");
    detector.detect_cached("file2.py");
    detector.detect_cached("file3.ts");
    detector.detect_cached("file4.js");

    assert_eq!(detector.cache_size(), 4);

    // Accessing same files again should not increase cache size
    detector.detect_cached("file1.rs");
    detector.detect_cached("file2.py");

    assert_eq!(detector.cache_size(), 4);
}

#[test]
fn test_clear_cache() {
    let mut detector = LanguageDetector::new();

    detector.detect_cached("file1.rs");
    detector.detect_cached("file2.py");
    detector.detect_cached("file3.ts");

    assert_eq!(detector.cache_size(), 3);

    detector.clear_cache();
    assert_eq!(detector.cache_size(), 0);

    // After clearing, can still detect
    let lang = detector.detect_cached("file1.rs");
    assert_eq!(lang, Language::Rust);
    assert_eq!(detector.cache_size(), 1);
}

#[test]
fn test_language_string_conversion() {
    assert_eq!(Language::Rust.as_str(), "rust");
    assert_eq!(Language::Python.as_str(), "python");
    assert_eq!(Language::TypeScript.as_str(), "typescript");
    assert_eq!(Language::JavaScript.as_str(), "javascript");
    assert_eq!(Language::Go.as_str(), "go");
    assert_eq!(Language::Java.as_str(), "java");
    assert_eq!(Language::Cpp.as_str(), "cpp");
    assert_eq!(Language::Unknown.as_str(), "unknown");
}

#[test]
fn test_language_from_string() {
    assert_eq!(Language::from_str("rust"), Language::Rust);
    assert_eq!(Language::from_str("Rust"), Language::Rust);
    assert_eq!(Language::from_str("RUST"), Language::Rust);
    assert_eq!(Language::from_str("rs"), Language::Rust);

    assert_eq!(Language::from_str("python"), Language::Python);
    assert_eq!(Language::from_str("py"), Language::Python);

    assert_eq!(Language::from_str("typescript"), Language::TypeScript);
    assert_eq!(Language::from_str("ts"), Language::TypeScript);

    assert_eq!(Language::from_str("javascript"), Language::JavaScript);
    assert_eq!(Language::from_str("js"), Language::JavaScript);

    assert_eq!(Language::from_str("go"), Language::Go);
    assert_eq!(Language::from_str("java"), Language::Java);
    assert_eq!(Language::from_str("cpp"), Language::Cpp);
    assert_eq!(Language::from_str("c++"), Language::Cpp);

    assert_eq!(Language::from_str("unknown_lang"), Language::Unknown);
}

#[test]
fn test_detect_from_content_python() {
    let detector = LanguageDetector::new();

    let python_code = r#"
import os
import sys

def main():
    """Main function"""
    print("Hello, World!")

if __name__ == "__main__":
    main()
"#;

    assert_eq!(detector.detect_from_content(python_code), Language::Python);
}

#[test]
fn test_detect_from_content_rust() {
    let detector = LanguageDetector::new();

    let rust_code = r#"
pub fn main() {
    println!("Hello, World!");
}

impl MyStruct {
    pub fn new() -> Self {
        Self {}
    }
}
"#;

    assert_eq!(detector.detect_from_content(rust_code), Language::Rust);
}

#[test]
fn test_detect_from_content_typescript() {
    let detector = LanguageDetector::new();

    let typescript_code = r#"
interface User {
    name: string;
    age: number;
}

function greet(user: User): string {
    return `Hello, ${user.name}!`;
}
"#;

    assert_eq!(
        detector.detect_from_content(typescript_code),
        Language::TypeScript
    );
}

#[test]
fn test_detect_from_content_javascript() {
    let detector = LanguageDetector::new();

    let javascript_code = r#"
function greet(name) {
    const message = `Hello, ${name}!`;
    return message;
}
"#;

    assert_eq!(
        detector.detect_from_content(javascript_code),
        Language::JavaScript
    );
}

#[test]
fn test_detect_from_content_go() {
    let detector = LanguageDetector::new();

    let go_code = r#"
package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}
"#;

    assert_eq!(detector.detect_from_content(go_code), Language::Go);
}

#[test]
fn test_detect_from_content_ambiguous() {
    let detector = LanguageDetector::new();

    // Content with no clear language indicators
    let ambiguous_code = "// Just a comment";

    assert_eq!(
        detector.detect_from_content(ambiguous_code),
        Language::Unknown
    );
}

#[test]
fn test_detect_from_kind_rust() {
    let detector = LanguageDetector::new();

    assert_eq!(detector.detect_from_kind("impl"), Language::Rust);
    assert_eq!(detector.detect_from_kind("trait"), Language::Rust);
    assert_eq!(detector.detect_from_kind("mod"), Language::Rust);
}

#[test]
fn test_detect_from_kind_unknown() {
    let detector = LanguageDetector::new();

    assert_eq!(detector.detect_from_kind("func"), Language::Unknown);
    assert_eq!(detector.detect_from_kind("class"), Language::Unknown);
}

#[test]
fn test_language_detection_accuracy() {
    let detector = LanguageDetector::new();

    // Test a mix of real-world file paths
    let test_cases = vec![
        ("src/main.rs", Language::Rust),
        ("lib/utils.py", Language::Python),
        ("app/components/Button.tsx", Language::TypeScript),
        ("src/index.js", Language::JavaScript),
        ("cmd/server/main.go", Language::Go),
        ("src/Main.java", Language::Java),
        ("include/header.h", Language::Cpp),
        ("README.md", Language::Unknown),
    ];

    for (path, expected) in test_cases {
        let detected = detector.detect_from_path(path);
        assert_eq!(
            detected, expected,
            "Failed to detect {} for path: {}",
            expected.as_str(),
            path
        );
    }
}

#[test]
fn test_nested_path_detection() {
    let detector = LanguageDetector::new();

    // Test deeply nested paths
    assert_eq!(
        detector.detect_from_path("a/b/c/d/e/f/file.rs"),
        Language::Rust
    );
    assert_eq!(
        detector.detect_from_path("deeply/nested/path/to/module.py"),
        Language::Python
    );
}

#[test]
fn test_filename_with_multiple_dots() {
    let detector = LanguageDetector::new();

    assert_eq!(
        detector.detect_from_path("file.test.ts"),
        Language::TypeScript
    );
    assert_eq!(
        detector.detect_from_path("config.dev.js"),
        Language::JavaScript
    );
    assert_eq!(
        detector.detect_from_path("module.spec.py"),
        Language::Python
    );
}
