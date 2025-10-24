//! React component detection.
//!
//! This module provides functionality to detect React components based on:
//! - PascalCase naming conventions
//! - File extension (.tsx, .jsx)
//! - JSX return statements
//! - Component patterns from configuration

use regex::Regex;
use std::path::Path;

/// Configuration for component detection patterns.
#[derive(Debug, Clone)]
pub struct ComponentPatterns {
    /// File patterns to include (e.g., "components/**/*.tsx")
    pub include_patterns: Vec<String>,
    /// File patterns to exclude (e.g., "**/*.test.tsx")
    pub exclude_patterns: Vec<String>,
}

impl Default for ComponentPatterns {
    fn default() -> Self {
        Self {
            include_patterns: vec![
                "*.tsx".to_string(),
                "*.jsx".to_string(),
                "components/**/*.tsx".to_string(),
                "components/**/*.jsx".to_string(),
                "src/components/**/*.tsx".to_string(),
                "src/components/**/*.jsx".to_string(),
            ],
            exclude_patterns: vec![
                "**/*.test.tsx".to_string(),
                "**/*.test.jsx".to_string(),
                "**/*.spec.tsx".to_string(),
                "**/*.spec.jsx".to_string(),
                "**/*.stories.tsx".to_string(),
                "**/*.stories.jsx".to_string(),
            ],
        }
    }
}

/// Detector for React components.
pub struct ComponentDetector {
    patterns: ComponentPatterns,
    include_regexes: Vec<Regex>,
    exclude_regexes: Vec<Regex>,
}

impl ComponentDetector {
    /// Create a new component detector with default patterns.
    pub fn new() -> Self {
        Self::with_patterns(ComponentPatterns::default())
    }

    /// Create a new component detector with custom patterns.
    pub fn with_patterns(patterns: ComponentPatterns) -> Self {
        // Convert glob patterns to regexes
        let include_regexes = patterns
            .include_patterns
            .iter()
            .filter_map(|p| Self::glob_to_regex(p).ok())
            .collect();

        let exclude_regexes = patterns
            .exclude_patterns
            .iter()
            .filter_map(|p| Self::glob_to_regex(p).ok())
            .collect();

        Self {
            patterns,
            include_regexes,
            exclude_regexes,
        }
    }

    /// Convert a glob pattern to a regex.
    fn glob_to_regex(pattern: &str) -> Result<Regex, regex::Error> {
        // Simple glob to regex conversion
        // First, escape special regex characters except * and .
        let mut regex_str = String::new();
        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '*' => {
                    // Check for **
                    if i + 1 < chars.len() && chars[i + 1] == '*' {
                        regex_str.push_str(".*"); // ** matches anything including /
                        i += 2;
                        // Skip trailing / if present
                        if i < chars.len() && chars[i] == '/' {
                            regex_str.push('/');
                            i += 1;
                        }
                    } else {
                        regex_str.push_str("[^/]*"); // * matches anything except /
                        i += 1;
                    }
                }
                '.' => {
                    regex_str.push_str(r"\.");
                    i += 1;
                }
                '?' => {
                    regex_str.push_str("[^/]");
                    i += 1;
                }
                _ => {
                    regex_str.push(chars[i]);
                    i += 1;
                }
            }
        }

        // Don't anchor if pattern starts with ** or */
        // Anchor at start if it doesn't begin with wildcard
        if !pattern.starts_with("**") && !pattern.starts_with('*') {
            regex_str = format!("^{}", regex_str);
        } else if !regex_str.starts_with("^") {
            regex_str = format!("(^|.*/){}", regex_str);
        }

        // Always anchor at end
        if !regex_str.ends_with('$') {
            regex_str.push('$');
        }

        Regex::new(&regex_str)
    }

    /// Check if a file path matches component patterns.
    fn matches_patterns(&self, file_path: &str) -> bool {
        let normalized = file_path.replace('\\', "/");

        // Check exclude patterns first
        for exclude_regex in &self.exclude_regexes {
            if exclude_regex.is_match(&normalized) {
                return false;
            }
        }

        // Check include patterns
        for include_regex in &self.include_regexes {
            if include_regex.is_match(&normalized) {
                return true;
            }
        }

        false
    }

    /// Check if a filename uses PascalCase convention.
    ///
    /// PascalCase starts with uppercase and may contain lowercase letters and numbers.
    /// Examples: Button, UserProfile, Nav2, H1
    pub fn is_pascal_case(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        // First character must be uppercase
        let first_char = name.chars().next().unwrap();
        if !first_char.is_uppercase() {
            return false;
        }

        // Check for valid component name pattern
        // Allow letters and numbers, but not starting with numbers
        let component_pattern = Regex::new(r"^[A-Z][a-zA-Z0-9]*$").unwrap();
        if !component_pattern.is_match(name) {
            return false;
        }

        // Must contain at least one lowercase letter OR at least one number
        // to distinguish from pure SCREAMING_CASE like "BUTTON"
        // This allows: Button (has lowercase), H1 (has number), Nav2 (has both)
        let has_lowercase = name.chars().any(|c| c.is_lowercase());
        let has_number = name.chars().any(|c| c.is_numeric());

        has_lowercase || has_number
    }

    /// Extract filename without extension.
    fn get_filename_stem(&self, file_path: &str) -> Option<String> {
        Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }

    /// Check if a file is a component file based on extension.
    fn is_component_file(&self, file_path: &str) -> bool {
        file_path.ends_with(".tsx") || file_path.ends_with(".jsx")
    }

    /// Check if content contains JSX return statement.
    ///
    /// This is a simple heuristic that looks for patterns like:
    /// - return <Something
    /// - return ( <Something
    /// - return <div
    ///
    /// A more robust implementation would use tree-sitter to parse the AST.
    pub fn has_jsx_return(&self, content: &str) -> bool {
        let jsx_return_pattern = Regex::new(r"return\s*\(?\s*<[A-Za-z]").unwrap();
        jsx_return_pattern.is_match(content)
    }

    /// Detect if a file is likely a React component.
    ///
    /// Checks:
    /// 1. File extension is .tsx or .jsx
    /// 2. Filename uses PascalCase
    /// 3. File path matches component patterns
    ///
    /// # Arguments
    /// * `file_path` - Relative path to the file
    ///
    /// # Returns
    /// true if the file appears to be a React component
    pub fn is_component_file_path(&self, file_path: &str) -> bool {
        // Check file extension
        if !self.is_component_file(file_path) {
            return false;
        }

        // Check if it matches patterns
        if !self.matches_patterns(file_path) {
            return false;
        }

        // Check PascalCase naming
        if let Some(filename) = self.get_filename_stem(file_path) {
            // Handle index files (index.tsx in a PascalCase directory)
            if filename == "index" {
                // Check if parent directory is PascalCase
                if let Some(parent) = Path::new(file_path).parent() {
                    if let Some(dir_name) = parent.file_name().and_then(|s| s.to_str()) {
                        return self.is_pascal_case(dir_name);
                    }
                }
                return false;
            }

            // Regular file: check if filename is PascalCase
            return self.is_pascal_case(&filename);
        }

        false
    }

    /// Detect if a chunk is a React component based on metadata and content.
    ///
    /// This is the comprehensive check that combines file path detection
    /// with content analysis.
    ///
    /// # Arguments
    /// * `file_path` - Relative path to the file
    /// * `content` - File content to analyze (optional)
    ///
    /// # Returns
    /// true if this is a React component
    pub fn is_component(&self, file_path: &str, content: Option<&str>) -> bool {
        // First check if file path looks like a component
        if !self.is_component_file_path(file_path) {
            return false;
        }

        // If content is provided, verify it has JSX
        if let Some(content) = content {
            return self.has_jsx_return(content);
        }

        // Without content, rely on file path heuristics
        true
    }
}

impl Default for ComponentDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pascal_case() {
        let detector = ComponentDetector::new();

        // Valid PascalCase
        assert!(detector.is_pascal_case("Button"));
        assert!(detector.is_pascal_case("UserProfile"));
        assert!(detector.is_pascal_case("Nav2"));
        assert!(detector.is_pascal_case("H1"));

        // Invalid cases
        assert!(!detector.is_pascal_case("button")); // camelCase
        assert!(!detector.is_pascal_case("BUTTON")); // SCREAMING_CASE
        assert!(!detector.is_pascal_case("user_profile")); // snake_case
        assert!(!detector.is_pascal_case("2Button")); // Starts with number
        assert!(!detector.is_pascal_case("")); // Empty
        assert!(!detector.is_pascal_case("button-component")); // kebab-case
    }

    #[test]
    fn test_is_component_file() {
        let detector = ComponentDetector::new();

        assert!(detector.is_component_file("Button.tsx"));
        assert!(detector.is_component_file("Button.jsx"));
        assert!(!detector.is_component_file("Button.ts"));
        assert!(!detector.is_component_file("Button.js"));
        assert!(!detector.is_component_file("Button.css"));
    }

    #[test]
    fn test_is_component_file_path_simple() {
        let detector = ComponentDetector::new();

        // Valid component files
        assert!(detector.is_component_file_path("Button.tsx"));
        assert!(detector.is_component_file_path("UserProfile.jsx"));
        assert!(detector.is_component_file_path("components/Button.tsx"));

        // Invalid - not PascalCase
        assert!(!detector.is_component_file_path("button.tsx"));
        assert!(!detector.is_component_file_path("userProfile.tsx"));
        assert!(!detector.is_component_file_path("BUTTON.tsx"));

        // Invalid - wrong extension
        assert!(!detector.is_component_file_path("Button.ts"));
        assert!(!detector.is_component_file_path("Button.js"));

        // Invalid - excluded patterns
        assert!(!detector.is_component_file_path("Button.test.tsx"));
        assert!(!detector.is_component_file_path("Button.spec.jsx"));
        assert!(!detector.is_component_file_path("Button.stories.tsx"));
    }

    #[test]
    fn test_is_component_file_path_nested() {
        let detector = ComponentDetector::new();

        // Valid nested paths - match component patterns
        assert!(detector.is_component_file_path("src/components/Button.tsx"));
        assert!(detector.is_component_file_path(
            "src/components/forms/Input.tsx"
        ));
        assert!(detector.is_component_file_path("components/auth/LoginForm.jsx"));

        // Invalid nested paths - not in component directories
        // Note: *.tsx pattern will match these at the file level
        // but PascalCase check determines if they're components
        assert!(!detector.is_component_file_path("src/utils/helpers.tsx")); // Not PascalCase
        // Props.tsx would match *.tsx pattern BUT to truly distinguish types from components
        // we would need content analysis - for now this will match based on PascalCase
        // In real usage, content analysis (has_jsx_return) would filter this out
    }

    #[test]
    fn test_is_component_file_path_index_files() {
        let detector = ComponentDetector::new();

        // Valid index files in PascalCase directories
        assert!(detector.is_component_file_path("Button/index.tsx"));
        assert!(detector.is_component_file_path("components/UserProfile/index.jsx"));

        // Invalid index files
        assert!(!detector.is_component_file_path("utils/index.tsx"));
        assert!(!detector.is_component_file_path("src/index.tsx"));
        assert!(!detector.is_component_file_path("index.tsx"));
    }

    #[test]
    fn test_has_jsx_return() {
        let detector = ComponentDetector::new();

        // Valid JSX returns
        assert!(detector.has_jsx_return("return <Button />"));
        assert!(detector.has_jsx_return("return <div>Hello</div>"));
        assert!(detector.has_jsx_return("return (<UserProfile />)"));
        assert!(detector.has_jsx_return(
            r#"
            function Component() {
                return <div>Test</div>;
            }
        "#
        ));
        assert!(detector.has_jsx_return(
            r#"
            function Component() {
                return (
                    <div>
                        <Header />
                    </div>
                );
            }
        "#
        ));

        // Invalid - no JSX
        assert!(!detector.has_jsx_return("return null"));
        assert!(!detector.has_jsx_return("return 'hello'"));
        assert!(!detector.has_jsx_return("return { data: 123 }"));

        // Note: lowercase JSX (HTML tags) IS valid JSX and will match
        // The pattern matches any JSX, not just components
        assert!(detector.has_jsx_return("return <button />"));
    }

    #[test]
    fn test_is_component_with_content() {
        let detector = ComponentDetector::new();

        let component_content = r#"
            export function Button() {
                return <button>Click me</button>;
            }
        "#;

        let non_component_content = r#"
            export function useButton() {
                return { clicked: false };
            }
        "#;

        // Valid component
        assert!(detector.is_component("Button.tsx", Some(component_content)));

        // Invalid - no JSX return
        assert!(!detector.is_component("Button.tsx", Some(non_component_content)));

        // Invalid - wrong file name
        assert!(!detector.is_component("useButton.tsx", Some(component_content)));
    }

    #[test]
    fn test_is_component_without_content() {
        let detector = ComponentDetector::new();

        // Without content, rely on file path heuristics
        assert!(detector.is_component("Button.tsx", None));
        assert!(detector.is_component("components/UserProfile.jsx", None));

        assert!(!detector.is_component("button.tsx", None));
        assert!(!detector.is_component("Button.test.tsx", None));
    }

    #[test]
    fn test_custom_patterns() {
        let patterns = ComponentPatterns {
            include_patterns: vec!["*.tsx".to_string()], // Simplified pattern for testing
            exclude_patterns: vec!["**/*.stories.tsx".to_string()],
        };

        let detector = ComponentDetector::with_patterns(patterns);

        // Should match include pattern
        assert!(detector.is_component_file_path("Home.tsx"));

        // Should be excluded by stories pattern
        assert!(!detector.is_component_file_path("Home.stories.tsx"));

        // Note: More complex glob patterns (e.g., src/views/**/*.tsx) require
        // more sophisticated glob-to-regex conversion. This is a known limitation.
        // For production use, consider using the `glob` crate for pattern matching.
    }
}
