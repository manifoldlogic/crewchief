//! File loading and line range extraction utilities.

use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

use super::types::LineRange;

/// File loader for reading and extracting specific line ranges from source files.
///
/// Handles UTF-8 encoding, line-based extraction, and graceful error handling
/// for missing files or invalid ranges.
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::context::{FileLoader, LineRange};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let loader = FileLoader::new("/workspace");
///     let range = LineRange::new(10, 20);
///     let content = loader.load_range("src/main.rs", range).await?;
///     println!("Lines 10-20: {}", content);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FileLoader {
    /// Base directory for resolving relative paths
    base_path: String,
}

impl FileLoader {
    /// Create a new file loader with the specified base path.
    ///
    /// All relative paths will be resolved relative to this base path.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::FileLoader;
    ///
    /// let loader = FileLoader::new("/workspace/my-project");
    /// ```
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Load the entire file content.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File does not exist
    /// - File cannot be read
    /// - File is not valid UTF-8
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::context::FileLoader;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let loader = FileLoader::new("/workspace");
    ///     let content = loader.load("src/main.rs").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn load(&self, relpath: &str) -> Result<String> {
        let full_path = Path::new(&self.base_path).join(relpath);

        fs::read_to_string(&full_path)
            .await
            .with_context(|| format!("Failed to read file: {}", full_path.display()))
    }

    /// Load a specific line range from a file.
    ///
    /// Lines are 1-indexed and inclusive (line 1 is the first line).
    /// If the range extends beyond the file, only available lines are returned.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File does not exist
    /// - File cannot be read
    /// - File is not valid UTF-8
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::context::{FileLoader, LineRange};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let loader = FileLoader::new("/workspace");
    ///     let range = LineRange::new(10, 20);
    ///     let content = loader.load_range("src/main.rs", range).await?;
    ///     println!("Lines 10-20:\n{}", content);
    ///     Ok(())
    /// }
    /// ```
    pub async fn load_range(&self, relpath: &str, range: LineRange) -> Result<String> {
        let content = self.load(relpath).await?;
        Ok(Self::extract_range(&content, range))
    }

    /// Extract a line range from text content.
    ///
    /// Lines are 1-indexed. If the range extends beyond available lines,
    /// only the available lines are returned.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::{FileLoader, LineRange};
    ///
    /// let text = "line 1\nline 2\nline 3\nline 4\nline 5";
    /// let range = LineRange::new(2, 4);
    /// let extracted = FileLoader::extract_range(text, range);
    /// assert_eq!(extracted, "line 2\nline 3\nline 4");
    /// ```
    pub fn extract_range(content: &str, range: LineRange) -> String {
        let lines: Vec<&str> = content.lines().collect();

        // Lines are 1-indexed, so line 1 is at index 0
        // If start is 0 or negative, clamp to 1 (index 0)
        let start_line = if range.start <= 0 { 1 } else { range.start };
        let end_line = if range.end <= 0 { 0 } else { range.end };

        // Convert 1-indexed to 0-indexed and clamp to valid range
        let start = (start_line.saturating_sub(1) as usize).min(lines.len());
        let end = (end_line as usize).min(lines.len());

        if start >= end || start >= lines.len() {
            return String::new();
        }

        lines[start..end].join("\n")
    }

    /// Check if a file exists at the given relative path.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::context::FileLoader;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let loader = FileLoader::new("/workspace");
    ///     if loader.exists("src/main.rs").await {
    ///         println!("File exists!");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn exists(&self, relpath: &str) -> bool {
        let full_path = Path::new(&self.base_path).join(relpath);
        fs::metadata(&full_path).await.is_ok()
    }

    /// Get the full absolute path for a relative path.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::FileLoader;
    ///
    /// let loader = FileLoader::new("/workspace");
    /// let full_path = loader.full_path("src/main.rs");
    /// assert!(full_path.ends_with("src/main.rs"));
    /// ```
    pub fn full_path(&self, relpath: &str) -> String {
        Path::new(&self.base_path)
            .join(relpath)
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_range_normal() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let range = LineRange::new(2, 4);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "line 2\nline 3\nline 4");
    }

    #[test]
    fn test_extract_range_single_line() {
        let content = "line 1\nline 2\nline 3";
        let range = LineRange::new(2, 2);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "line 2");
    }

    #[test]
    fn test_extract_range_first_line() {
        let content = "line 1\nline 2\nline 3";
        let range = LineRange::new(1, 1);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "line 1");
    }

    #[test]
    fn test_extract_range_beyond_end() {
        let content = "line 1\nline 2\nline 3";
        let range = LineRange::new(2, 100);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "line 2\nline 3");
    }

    #[test]
    fn test_extract_range_all_lines() {
        let content = "line 1\nline 2\nline 3";
        let range = LineRange::new(1, 3);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "line 1\nline 2\nline 3");
    }

    #[test]
    fn test_extract_range_invalid_start() {
        let content = "line 1\nline 2\nline 3";
        let range = LineRange::new(10, 20);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "");
    }

    #[test]
    fn test_extract_range_zero_start() {
        let content = "line 1\nline 2\nline 3";
        // start=0 should be clamped to line 1 (index 0)
        let range = LineRange::new(0, 2);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "line 1\nline 2");
    }

    #[test]
    fn test_extract_range_empty_content() {
        let content = "";
        let range = LineRange::new(1, 5);
        let result = FileLoader::extract_range(content, range);
        assert_eq!(result, "");
    }

    #[test]
    fn test_full_path() {
        let loader = FileLoader::new("/workspace");
        let full = loader.full_path("src/main.rs");
        assert!(full.contains("workspace"));
        assert!(full.ends_with("src/main.rs"));
    }

    #[tokio::test]
    async fn test_load_file_integration() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();

        let temp_path = temp_file.path();
        let temp_dir = temp_path.parent().unwrap();
        let file_name = temp_path.file_name().unwrap().to_str().unwrap();

        // Load the file
        let loader = FileLoader::new(temp_dir);
        let content = loader.load(file_name).await.unwrap();
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
        assert!(content.contains("line 3"));
    }

    #[tokio::test]
    async fn test_load_range_integration() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();
        writeln!(temp_file, "line 4").unwrap();
        writeln!(temp_file, "line 5").unwrap();

        let temp_path = temp_file.path();
        let temp_dir = temp_path.parent().unwrap();
        let file_name = temp_path.file_name().unwrap().to_str().unwrap();

        // Load a range
        let loader = FileLoader::new(temp_dir);
        let range = LineRange::new(2, 4);
        let content = loader.load_range(file_name, range).await.unwrap();

        assert!(content.contains("line 2"));
        assert!(content.contains("line 3"));
        assert!(content.contains("line 4"));
        assert!(!content.contains("line 1"));
        assert!(!content.contains("line 5"));
    }

    #[tokio::test]
    async fn test_exists() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();
        let temp_dir = temp_path.parent().unwrap();
        let file_name = temp_path.file_name().unwrap().to_str().unwrap();

        let loader = FileLoader::new(temp_dir);
        assert!(loader.exists(file_name).await);
        assert!(!loader.exists("nonexistent_file.txt").await);
    }
}
