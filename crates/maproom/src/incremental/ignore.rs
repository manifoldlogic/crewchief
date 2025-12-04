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
}
