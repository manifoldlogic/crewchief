//! Ignore pattern handling for file watching.
//!
//! This module provides functionality to filter out files that should not trigger
//! incremental indexing, based on .gitignore patterns and default ignore rules.

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Default ignore patterns that should always be excluded from indexing.
const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "*.log",
    ".git/**",
    "**/node_modules",
    "**/node_modules/**",
    "**/dist",
    "**/dist/**",
    "**/target",
    "**/target/**",
    "**/.crewchief",
    "**/.crewchief/**",
    "**/.DS_Store",
    "**/Thumbs.db",
];

/// Load ignore patterns from .maproomignore file in repository root.
///
/// Reads `.maproomignore` from the repository root and combines patterns with defaults.
/// Returns only default patterns if the file doesn't exist (not an error).
/// Fails if the file exists but contains invalid glob patterns.
///
/// # Arguments
/// * `root` - Repository root path where .maproomignore should be located
///
/// # Returns
/// * `Ok(Vec<String>)` - Combined default and user patterns
/// * `Err` - If file I/O fails or patterns are invalid
pub fn load_ignore_patterns(root: &Path) -> Result<Vec<String>> {
    let mut patterns = DEFAULT_IGNORE_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let maproomignore_path = root.join(".maproomignore");
    if maproomignore_path.exists() {
        let content = std::fs::read_to_string(&maproomignore_path).with_context(|| {
            format!("Failed to read .maproomignore at {:?}", maproomignore_path)
        })?;

        for line in content.lines() {
            let line = line.trim();
            // Skip empty lines and comments (lines starting with #)
            if !line.is_empty() && !line.starts_with('#') {
                // Validate glob pattern before adding
                Glob::new(line)
                    .with_context(|| format!("Invalid glob pattern in .maproomignore: {}", line))?;
                patterns.push(line.to_string());
            }
        }
    }

    Ok(patterns)
}

/// Manages ignore patterns for filtering file system events.
pub struct IgnorePatternMatcher {
    glob_set: GlobSet,
}

impl IgnorePatternMatcher {
    /// Create a new matcher with default ignore patterns.
    pub fn new() -> Result<Self> {
        Self::with_patterns(DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()))
    }

    /// Create a new matcher with custom patterns.
    pub fn with_patterns<I, S>(patterns: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut builder = GlobSetBuilder::new();

        for pattern in patterns {
            let glob = Glob::new(pattern.as_ref())
                .with_context(|| format!("Invalid glob pattern: {}", pattern.as_ref()))?;
            builder.add(glob);
        }

        let glob_set = builder.build().context("Failed to build glob set")?;

        Ok(Self { glob_set })
    }

    /// Create a matcher by reading .gitignore file and combining with defaults.
    pub fn from_gitignore(gitignore_path: &Path) -> Result<Self> {
        let mut patterns: Vec<String> = DEFAULT_IGNORE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Try to read .gitignore if it exists
        if gitignore_path.exists() {
            let gitignore_content = std::fs::read_to_string(gitignore_path)
                .context("Failed to read .gitignore file")?;

            for line in gitignore_content.lines() {
                let line = line.trim();
                // Skip comments and empty lines
                if !line.is_empty() && !line.starts_with('#') {
                    patterns.push(line.to_string());
                }
            }
        }

        Self::with_patterns(patterns)
    }

    /// Create a matcher by reading .maproomignore from repository root.
    ///
    /// Loads ignore patterns from `.maproomignore` in the repository root and combines
    /// them with default patterns. Returns matcher with only defaults if file doesn't exist.
    ///
    /// # Arguments
    /// * `root` - Repository root path where .maproomignore should be located
    ///
    /// # Returns
    /// * `Ok(IgnorePatternMatcher)` - Matcher configured with loaded patterns
    /// * `Err` - If pattern loading fails or patterns are invalid
    pub fn from_repository(root: &Path) -> Result<Self> {
        let patterns = load_ignore_patterns(root)?;
        Self::with_patterns(patterns)
    }

    /// Check if a path should be ignored.
    pub fn should_ignore(&self, path: &Path) -> bool {
        // Convert to string for globset matching
        let path_str = path.to_string_lossy();
        self.glob_set.is_match(&*path_str)
    }

    /// Check if a path should be watched (inverse of should_ignore).
    pub fn should_watch(&self, path: &Path) -> bool {
        !self.should_ignore(path)
    }
}

impl Default for IgnorePatternMatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create default ignore matcher")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Test data constants
    const BASIC_PATTERNS: &str = "test/**\n*.tmp\nbuild/\n";
    const WITH_COMMENTS: &str =
        "# Skip test fixtures\ntest-fixtures/**\n\n# Build outputs\nbuild/\n";
    const INVALID_PATTERN: &str = "[invalid\n*.tmp\n";

    /// Helper function to create a test repository with .maproomignore
    fn create_test_repo_with_maproomignore(patterns: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        let ignore_file = dir.path().join(".maproomignore");
        std::fs::write(&ignore_file, patterns).unwrap();
        dir
    }

    #[test]
    fn test_default_patterns() {
        let matcher = IgnorePatternMatcher::new().unwrap();

        // Should ignore
        assert!(matcher.should_ignore(&PathBuf::from("test.log")));
        assert!(matcher.should_ignore(&PathBuf::from(".git/config")));
        assert!(matcher.should_ignore(&PathBuf::from("node_modules/package/index.js")));
        assert!(matcher.should_ignore(&PathBuf::from("dist/bundle.js")));
        assert!(matcher.should_ignore(&PathBuf::from("target/release/binary")));
        assert!(matcher.should_ignore(&PathBuf::from(".crewchief/worktree")));

        // Should not ignore
        assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
        assert!(!matcher.should_ignore(&PathBuf::from("README.md")));
        assert!(!matcher.should_ignore(&PathBuf::from("package.json")));
    }

    #[test]
    fn test_custom_patterns() {
        let patterns = vec!["*.tmp", "build/**"];
        let matcher = IgnorePatternMatcher::with_patterns(patterns).unwrap();

        assert!(matcher.should_ignore(&PathBuf::from("test.tmp")));
        assert!(matcher.should_ignore(&PathBuf::from("build/output.js")));
        assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
    }

    #[test]
    fn test_should_watch() {
        let matcher = IgnorePatternMatcher::new().unwrap();

        assert!(matcher.should_watch(&PathBuf::from("src/main.rs")));
        assert!(!matcher.should_watch(&PathBuf::from("node_modules/pkg/index.js")));
    }

    #[test]
    fn test_gitignore_parsing() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "# This is a comment").unwrap();
        writeln!(temp_file, "*.tmp").unwrap();
        writeln!(temp_file, "").unwrap();
        writeln!(temp_file, "build/**").unwrap();
        temp_file.flush().unwrap();

        let matcher = IgnorePatternMatcher::from_gitignore(temp_file.path()).unwrap();

        // Custom patterns from .gitignore
        assert!(matcher.should_ignore(&PathBuf::from("test.tmp")));
        assert!(matcher.should_ignore(&PathBuf::from("build/output.js")));

        // Default patterns still work
        assert!(matcher.should_ignore(&PathBuf::from("node_modules/pkg/index.js")));

        // Normal files not ignored
        assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
    }

    // ========== Pattern Loading Tests ==========

    #[test]
    fn test_load_ignore_patterns_missing_file() {
        // Setup: temp dir without .maproomignore
        let dir = TempDir::new().unwrap();

        // Action: call load_ignore_patterns()
        let patterns = load_ignore_patterns(dir.path()).unwrap();

        // Assert: returns Ok with only default patterns
        assert_eq!(patterns.len(), DEFAULT_IGNORE_PATTERNS.len());
        for default_pattern in DEFAULT_IGNORE_PATTERNS {
            assert!(
                patterns.contains(&default_pattern.to_string()),
                "Missing default pattern: {}",
                default_pattern
            );
        }
    }

    #[test]
    fn test_load_ignore_patterns_with_comments() {
        // Setup: .maproomignore with comment lines
        let dir = create_test_repo_with_maproomignore(WITH_COMMENTS);

        // Action: call load_ignore_patterns()
        let patterns = load_ignore_patterns(dir.path()).unwrap();

        // Assert: comments skipped, patterns loaded
        assert!(patterns.contains(&"test-fixtures/**".to_string()));
        assert!(patterns.contains(&"build/".to_string()));

        // Verify comments were not added
        for pattern in &patterns {
            assert!(
                !pattern.starts_with('#'),
                "Comment line was not skipped: {}",
                pattern
            );
        }

        // Verify default patterns still present
        assert!(patterns.contains(&"*.log".to_string()));
    }

    #[test]
    fn test_load_ignore_patterns_empty_file() {
        // Setup: empty .maproomignore
        let dir = create_test_repo_with_maproomignore("");

        // Action: call load_ignore_patterns()
        let patterns = load_ignore_patterns(dir.path()).unwrap();

        // Assert: returns Ok with only default patterns
        assert_eq!(patterns.len(), DEFAULT_IGNORE_PATTERNS.len());
        for default_pattern in DEFAULT_IGNORE_PATTERNS {
            assert!(
                patterns.contains(&default_pattern.to_string()),
                "Missing default pattern: {}",
                default_pattern
            );
        }
    }

    #[test]
    fn test_load_ignore_patterns_invalid_glob() {
        // Setup: .maproomignore with "[invalid" pattern
        let dir = create_test_repo_with_maproomignore(INVALID_PATTERN);

        // Action: call load_ignore_patterns()
        let result = load_ignore_patterns(dir.path());

        // Assert: returns Err (fail-fast on invalid patterns)
        assert!(result.is_err(), "Expected error for invalid glob pattern");
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(
                err_msg.contains("Invalid glob pattern") || err_msg.contains("[invalid"),
                "Error message should mention invalid pattern: {}",
                err_msg
            );
        }
    }

    // ========== Matcher Construction Tests ==========

    #[test]
    fn test_from_repository_reads_maproomignore() {
        // Setup: .maproomignore with "test/**"
        let dir = create_test_repo_with_maproomignore("test/**\n");

        // Action: IgnorePatternMatcher::from_repository()
        let matcher = IgnorePatternMatcher::from_repository(dir.path()).unwrap();

        // Assert: matcher created successfully, patterns loaded
        assert!(matcher.should_ignore(&PathBuf::from("test/file.rs")));
        assert!(matcher.should_ignore(&PathBuf::from("test/nested/file.rs")));
        assert!(!matcher.should_ignore(&PathBuf::from("src/test.rs")));
    }

    #[test]
    fn test_from_repository_combines_with_defaults() {
        // Setup: .maproomignore with custom pattern
        let dir = create_test_repo_with_maproomignore("custom-dir/**\n");

        // Action: from_repository()
        let matcher = IgnorePatternMatcher::from_repository(dir.path()).unwrap();

        // Assert: both default patterns AND custom patterns present
        // Custom pattern works
        assert!(matcher.should_ignore(&PathBuf::from("custom-dir/file.rs")));

        // Default patterns still work
        assert!(matcher.should_ignore(&PathBuf::from("node_modules/pkg/index.js")));
        assert!(matcher.should_ignore(&PathBuf::from("test.log")));
        assert!(matcher.should_ignore(&PathBuf::from(".git/config")));

        // Non-ignored files
        assert!(!matcher.should_ignore(&PathBuf::from("src/main.rs")));
    }

    #[test]
    fn test_from_repository_fails_on_invalid() {
        // Setup: .maproomignore with invalid pattern
        let dir = create_test_repo_with_maproomignore(INVALID_PATTERN);

        // Action: from_repository()
        let result = IgnorePatternMatcher::from_repository(dir.path());

        // Assert: returns Err with clear message
        assert!(result.is_err(), "Expected error for invalid glob pattern");
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(
                err_msg.contains("Invalid glob pattern") || err_msg.contains("[invalid"),
                "Error message should mention invalid pattern: {}",
                err_msg
            );
        }
    }

    // ========== Matching Behavior Tests ==========

    #[test]
    fn test_should_ignore_matches_pattern() {
        // Setup: matcher with "*.tmp" pattern
        let patterns = vec!["*.tmp".to_string()];
        let matcher = IgnorePatternMatcher::with_patterns(patterns).unwrap();

        // Action: should_ignore("file.tmp")
        // Assert: returns true
        assert!(matcher.should_ignore(&PathBuf::from("file.tmp")));
        assert!(matcher.should_ignore(&PathBuf::from("data.tmp")));

        // Action: should_ignore("file.rs")
        // Assert: returns false
        assert!(!matcher.should_ignore(&PathBuf::from("file.rs")));
        assert!(!matcher.should_ignore(&PathBuf::from("README.md")));
    }

    #[test]
    fn test_should_ignore_relative_paths() {
        // Setup: matcher with "test/**" pattern
        let patterns = vec!["test/**".to_string()];
        let matcher = IgnorePatternMatcher::with_patterns(patterns).unwrap();

        // Action: should_ignore("test/file.rs")
        // Assert: returns true
        assert!(matcher.should_ignore(&PathBuf::from("test/file.rs")));
        assert!(matcher.should_ignore(&PathBuf::from("test/nested/deep/file.rs")));

        // Action: should_ignore("src/test/file.rs")
        // Assert: returns false (pattern is relative to root)
        assert!(!matcher.should_ignore(&PathBuf::from("src/test/file.rs")));
        assert!(!matcher.should_ignore(&PathBuf::from("src/test.rs")));
    }
}
