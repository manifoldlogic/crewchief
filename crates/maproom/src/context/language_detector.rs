//! Language detection for context assembly strategy selection.
//!
//! This module provides language detection based on file extensions,
//! chunk metadata, and content analysis. The detected language is used
//! to select the appropriate assembly strategy.

use std::collections::HashMap;
use std::path::Path;

/// Detected programming language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// TypeScript
    TypeScript,
    /// JavaScript
    JavaScript,
    /// Python
    Python,
    /// Ruby
    Ruby,
    /// Rust
    Rust,
    /// Go
    Go,
    /// Java
    Java,
    /// C/C++
    Cpp,
    /// C#
    CSharp,
    /// Unknown or unsupported language
    Unknown,
}

impl Language {
    /// Get the string representation of the language.
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Ruby => "ruby",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Java => "java",
            Language::Cpp => "cpp",
            Language::CSharp => "csharp",
            Language::Unknown => "unknown",
        }
    }

    /// Parse a language from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "typescript" | "ts" => Language::TypeScript,
            "javascript" | "js" => Language::JavaScript,
            "python" | "py" => Language::Python,
            "ruby" | "rb" => Language::Ruby,
            "rust" | "rs" => Language::Rust,
            "go" => Language::Go,
            "java" => Language::Java,
            "cpp" | "c++" | "c" => Language::Cpp,
            "csharp" | "c#" | "cs" => Language::CSharp,
            _ => Language::Unknown,
        }
    }
}

/// Language detector that uses file extensions and metadata.
#[derive(Debug, Clone)]
pub struct LanguageDetector {
    /// Cache of file paths to detected languages
    cache: HashMap<String, Language>,
}

impl LanguageDetector {
    /// Create a new language detector.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Detect language from a file path.
    ///
    /// This method uses file extensions as the primary detection method.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file
    ///
    /// # Returns
    ///
    /// The detected language, or `Language::Unknown` if unable to detect.
    ///
    /// # Examples
    ///
    /// ```
    /// use crewchief_maproom::context::language_detector::{Language, LanguageDetector};
    ///
    /// let detector = LanguageDetector::new();
    ///
    /// assert_eq!(detector.detect_from_path("src/main.rs"), Language::Rust);
    /// assert_eq!(detector.detect_from_path("src/app.ts"), Language::TypeScript);
    /// assert_eq!(detector.detect_from_path("main.py"), Language::Python);
    /// ```
    pub fn detect_from_path(&self, file_path: &str) -> Language {
        let path = Path::new(file_path);

        if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("rs") => Language::Rust,
                Some("py") | Some("pyi") => Language::Python,
                Some("rb") | Some("rake") => Language::Ruby,
                Some("ts") | Some("mts") | Some("cts") => Language::TypeScript,
                Some("tsx") => Language::TypeScript, // React TypeScript
                Some("js") | Some("mjs") | Some("cjs") => Language::JavaScript,
                Some("jsx") => Language::JavaScript, // React JavaScript
                Some("go") => Language::Go,
                Some("java") => Language::Java,
                Some("c") | Some("cc") | Some("cpp") | Some("cxx") | Some("h") | Some("hpp") => {
                    Language::Cpp
                }
                Some("cs") => Language::CSharp,
                _ => Language::Unknown,
            }
        } else {
            Language::Unknown
        }
    }

    /// Detect language with caching for performance.
    ///
    /// This method caches detection results per file path.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file
    ///
    /// # Returns
    ///
    /// The detected language, or `Language::Unknown` if unable to detect.
    pub fn detect_cached(&mut self, file_path: &str) -> Language {
        if let Some(&lang) = self.cache.get(file_path) {
            return lang;
        }

        let lang = self.detect_from_path(file_path);
        self.cache.insert(file_path.to_string(), lang);
        lang
    }

    /// Detect language from chunk metadata.
    ///
    /// This method can be used as a fallback when file extension is ambiguous.
    ///
    /// # Arguments
    ///
    /// * `kind` - The chunk kind (e.g., "func", "class", "impl")
    ///
    /// # Returns
    ///
    /// The detected language, or `Language::Unknown` if unable to infer.
    pub fn detect_from_kind(&self, kind: &str) -> Language {
        match kind {
            "impl" | "trait" | "mod" => Language::Rust,
            "def" | "class" if kind.contains("py") => Language::Python,
            "module" => Language::Ruby, // Ruby-specific
            _ => Language::Unknown,
        }
    }

    /// Detect language from file content analysis.
    ///
    /// This is a more expensive operation that analyzes the actual content
    /// for language-specific patterns. Use as a last resort.
    ///
    /// # Arguments
    ///
    /// * `content` - The file content to analyze
    ///
    /// # Returns
    ///
    /// The detected language, or `Language::Unknown` if unable to detect.
    pub fn detect_from_content(&self, content: &str) -> Language {
        // Ruby patterns (check before Python to avoid false positives)
        if content.contains("class") && content.contains("<") && content.contains("end") {
            return Language::Ruby;
        }
        if content.contains("def") && content.contains("end") {
            return Language::Ruby;
        }
        if content.contains("module") && content.contains("end") {
            return Language::Ruby;
        }

        // C# detection patterns
        if content.contains("namespace ")
            && (content.contains("class ") || content.contains("interface "))
        {
            return Language::CSharp;
        }
        if content.contains("using System") || content.contains("using static") {
            return Language::CSharp;
        }

        // Python patterns
        if content.contains("def ") && content.contains("import ") {
            return Language::Python;
        }

        // Rust patterns
        if content.contains("fn ") && (content.contains("impl ") || content.contains("pub ")) {
            return Language::Rust;
        }

        // TypeScript patterns
        if content.contains("interface ")
            || content.contains(": string")
            || content.contains(": number")
        {
            return Language::TypeScript;
        }

        // JavaScript patterns (must come after TypeScript)
        if content.contains("function ") || content.contains("const ") {
            return Language::JavaScript;
        }

        // Go patterns
        if content.contains("func ") && content.contains("package ") {
            return Language::Go;
        }

        Language::Unknown
    }

    /// Clear the detection cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the size of the detection cache.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::TypeScript.as_str(), "typescript");
        assert_eq!(Language::JavaScript.as_str(), "javascript");
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("rust"), Language::Rust);
        assert_eq!(Language::from_str("Rust"), Language::Rust);
        assert_eq!(Language::from_str("rs"), Language::Rust);
        assert_eq!(Language::from_str("python"), Language::Python);
        assert_eq!(Language::from_str("py"), Language::Python);
        assert_eq!(Language::from_str("typescript"), Language::TypeScript);
        assert_eq!(Language::from_str("ts"), Language::TypeScript);
        assert_eq!(Language::from_str("unknown_lang"), Language::Unknown);
    }

    #[test]
    fn test_detect_from_path_rust() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.detect_from_path("src/main.rs"), Language::Rust);
        assert_eq!(
            detector.detect_from_path("src/lib/module.rs"),
            Language::Rust
        );
    }

    #[test]
    fn test_detect_from_path_python() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.detect_from_path("main.py"), Language::Python);
        assert_eq!(
            detector.detect_from_path("src/utils/__init__.py"),
            Language::Python
        );
        assert_eq!(detector.detect_from_path("types.pyi"), Language::Python);
    }

    #[test]
    fn test_detect_from_path_typescript() {
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
    }

    #[test]
    fn test_detect_from_path_javascript() {
        let detector = LanguageDetector::new();
        assert_eq!(
            detector.detect_from_path("src/app.js"),
            Language::JavaScript
        );
        assert_eq!(
            detector.detect_from_path("src/Component.jsx"),
            Language::JavaScript
        );
    }

    #[test]
    fn test_detect_from_path_other_languages() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.detect_from_path("main.go"), Language::Go);
        assert_eq!(detector.detect_from_path("Main.java"), Language::Java);
        assert_eq!(detector.detect_from_path("main.cpp"), Language::Cpp);
        assert_eq!(detector.detect_from_path("header.h"), Language::Cpp);
    }

    #[test]
    fn test_detect_from_path_unknown() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.detect_from_path("README.md"), Language::Unknown);
        assert_eq!(detector.detect_from_path("config.json"), Language::Unknown);
    }

    #[test]
    fn test_detect_cached() {
        let mut detector = LanguageDetector::new();

        let path = "src/main.rs";
        assert_eq!(detector.cache_size(), 0);

        let lang = detector.detect_cached(path);
        assert_eq!(lang, Language::Rust);
        assert_eq!(detector.cache_size(), 1);

        // Second call should use cache
        let lang2 = detector.detect_cached(path);
        assert_eq!(lang2, Language::Rust);
        assert_eq!(detector.cache_size(), 1);
    }

    #[test]
    fn test_clear_cache() {
        let mut detector = LanguageDetector::new();

        detector.detect_cached("file1.rs");
        detector.detect_cached("file2.py");
        assert_eq!(detector.cache_size(), 2);

        detector.clear_cache();
        assert_eq!(detector.cache_size(), 0);
    }

    #[test]
    fn test_detect_from_kind() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.detect_from_kind("impl"), Language::Rust);
        assert_eq!(detector.detect_from_kind("trait"), Language::Rust);
        assert_eq!(detector.detect_from_kind("mod"), Language::Rust);
    }

    #[test]
    fn test_detect_from_content_python() {
        let detector = LanguageDetector::new();
        let content = "import os\n\ndef main():\n    pass";
        assert_eq!(detector.detect_from_content(content), Language::Python);
    }

    #[test]
    fn test_detect_from_content_rust() {
        let detector = LanguageDetector::new();
        let content = "pub fn main() {\n    impl Foo {}\n}";
        assert_eq!(detector.detect_from_content(content), Language::Rust);
    }

    #[test]
    fn test_detect_from_content_typescript() {
        let detector = LanguageDetector::new();
        let content = "interface User { name: string; age: number; }";
        assert_eq!(detector.detect_from_content(content), Language::TypeScript);
    }

    #[test]
    fn test_detect_from_content_javascript() {
        let detector = LanguageDetector::new();
        let content = "function foo() { const bar = 42; }";
        assert_eq!(detector.detect_from_content(content), Language::JavaScript);
    }

    #[test]
    fn test_detect_from_content_go() {
        let detector = LanguageDetector::new();
        let content = "package main\n\nfunc main() {}";
        assert_eq!(detector.detect_from_content(content), Language::Go);
    }

    #[test]
    fn test_ruby_detection() {
        let detector = LanguageDetector::new();

        // Path detection
        assert_eq!(detector.detect_from_path("app.rb"), Language::Ruby);
        assert_eq!(detector.detect_from_path("tasks.rake"), Language::Ruby);

        // String conversion
        assert_eq!(Language::from_str("ruby"), Language::Ruby);
        assert_eq!(Language::from_str("rb"), Language::Ruby);
        assert_eq!(Language::Ruby.as_str(), "ruby");

        // Content detection
        let ruby_source = "class Greeter < Base\n  def greet\n  end\nend";
        assert_eq!(detector.detect_from_content(ruby_source), Language::Ruby);

        // Kind detection
        assert_eq!(detector.detect_from_kind("module"), Language::Ruby);
    }

    #[test]
    fn test_detect_csharp_from_path() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.detect_from_path("MyClass.cs"), Language::CSharp);
        assert_eq!(
            detector.detect_from_path("src/Program.cs"),
            Language::CSharp
        );
        assert_eq!(
            detector.detect_from_path("Models/User.cs"),
            Language::CSharp
        );
    }

    #[test]
    fn test_csharp_string_conversion() {
        assert_eq!(Language::from_str("csharp"), Language::CSharp);
        assert_eq!(Language::from_str("c#"), Language::CSharp);
        assert_eq!(Language::from_str("cs"), Language::CSharp);
        assert_eq!(Language::from_str("CSharp"), Language::CSharp);
        assert_eq!(Language::from_str("C#"), Language::CSharp);
        assert_eq!(Language::CSharp.as_str(), "csharp");
    }

    #[test]
    fn test_detect_csharp_from_content() {
        let detector = LanguageDetector::new();

        // Test namespace + class pattern
        let content = r#"
using System;
namespace MyNamespace
{
    public class MyClass { }
}
"#;
        assert_eq!(detector.detect_from_content(content), Language::CSharp);

        // Test using System pattern
        let content2 = "using System.Collections.Generic;";
        assert_eq!(detector.detect_from_content(content2), Language::CSharp);

        // Test using static pattern (C# 6+ feature)
        let content3 = "using static System.Math;";
        assert_eq!(detector.detect_from_content(content3), Language::CSharp);

        // Test namespace + interface pattern
        let content4 = r#"
namespace MyApp
{
    public interface IService { }
}
"#;
        assert_eq!(detector.detect_from_content(content4), Language::CSharp);
    }

    #[test]
    fn test_csharp_end_to_end() {
        use crate::indexer::parser;
        use std::path::Path;

        // Detect language from path
        let path = Path::new("Test.cs");
        let detector = LanguageDetector::new();
        let lang = detector.detect_from_path(path.to_str().unwrap());
        assert_eq!(lang, Language::CSharp);

        // Extract chunks
        let source = r#"
namespace Test
{
    public class Calculator
    {
        public int Add(int a, int b) => a + b;
    }
}
"#;
        let chunks = parser::extract_chunks(source, "cs");

        // Verify extraction
        assert!(!chunks.is_empty());
        let class = chunks.iter().find(|c| c.kind == "class");
        assert!(class.is_some());
        assert_eq!(class.unwrap().symbol_name.as_ref().unwrap(), "Calculator");
    }
}
