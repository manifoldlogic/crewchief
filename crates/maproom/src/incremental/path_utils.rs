//! Path normalization utilities for incremental indexing.
//!
//! This module provides robust path normalization to convert absolute filesystem paths
//! to repository-relative paths. It prevents path format mismatches that can cause
//! database lookup failures and ensures security by rejecting path traversal attempts.

use anyhow::{bail, Context, Result};
use std::path::{Component, Path, PathBuf};

/// Normalizes an absolute filesystem path to a repository-relative path.
///
/// This function converts absolute paths (e.g., `/workspace/packages/cli/src/main.ts`)
/// to repository-relative paths (e.g., `packages/cli/src/main.ts`) by stripping the
/// repository root prefix.
///
/// # Security
///
/// This function explicitly rejects paths containing parent directory components (`..`)
/// to prevent path traversal attacks. Any attempt to access paths outside the repository
/// root will result in an error.
///
/// # Arguments
///
/// * `absolute_path` - The absolute filesystem path to normalize
/// * `repo_root` - The absolute path to the repository root
///
/// # Returns
///
/// A `Result` containing the normalized relative path, or an error if:
/// - The path is outside the repository root
/// - The path contains parent directory components (`..`)
/// - The path cannot be stripped of the repository root prefix
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use crewchief_maproom::incremental::normalize_to_relpath;
///
/// let repo_root = Path::new("/workspace");
/// let absolute_path = Path::new("/workspace/packages/cli/src/main.ts");
///
/// let relative = normalize_to_relpath(absolute_path, repo_root).unwrap();
/// assert_eq!(relative, Path::new("packages/cli/src/main.ts"));
/// ```
///
/// ```
/// use std::path::Path;
/// use crewchief_maproom::incremental::normalize_to_relpath;
///
/// let repo_root = Path::new("/workspace");
/// let outside_path = Path::new("/etc/passwd");
///
/// // This will return an error
/// assert!(normalize_to_relpath(outside_path, repo_root).is_err());
/// ```
///
/// # Cross-platform Compatibility
///
/// This function works on both Unix and Windows systems. On Windows, it correctly
/// handles both forward slashes and backslashes as path separators.
pub fn normalize_to_relpath(absolute_path: &Path, repo_root: &Path) -> Result<PathBuf> {
    // Strip the repository root prefix to get the relative path
    let relative_path = absolute_path.strip_prefix(repo_root).with_context(|| {
        format!(
            "Path '{}' is outside repository root '{}'",
            absolute_path.display(),
            repo_root.display()
        )
    })?;

    // Security check: reject any paths with parent directory components (..)
    // This prevents path traversal attacks
    for component in relative_path.components() {
        if matches!(component, Component::ParentDir) {
            bail!(
                "Path '{}' contains parent directory components (..), which is not allowed for security reasons",
                relative_path.display()
            );
        }
    }

    // Return the normalized relative path
    Ok(relative_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_simple_path_conversion() {
        let repo_root = Path::new("/workspace");
        let absolute_path = Path::new("/workspace/src/main.rs");

        let result = normalize_to_relpath(absolute_path, repo_root).unwrap();
        assert_eq!(result, Path::new("src/main.rs"));
    }

    #[test]
    fn test_nested_path_conversion() {
        let repo_root = Path::new("/workspace");
        let absolute_path = Path::new("/workspace/packages/cli/src/commands/index.ts");

        let result = normalize_to_relpath(absolute_path, repo_root).unwrap();
        assert_eq!(result, Path::new("packages/cli/src/commands/index.ts"));
    }

    #[test]
    fn test_path_outside_repo_root() {
        let repo_root = Path::new("/workspace");
        let outside_path = Path::new("/etc/passwd");

        let result = normalize_to_relpath(outside_path, repo_root);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("outside repository root"));
        assert!(err_msg.contains("/etc/passwd"));
        assert!(err_msg.contains("/workspace"));
    }

    #[test]
    fn test_path_with_parent_dir_components() {
        let repo_root = Path::new("/workspace");
        // Create a path that strips to a valid prefix but contains ..
        // Note: strip_prefix would fail on /workspace/../etc/passwd, so we test
        // a constructed relative path with .. components
        let absolute_path = Path::new("/workspace/src/../etc/passwd");

        // First normalize using standard library to remove ..
        let normalized = absolute_path
            .canonicalize()
            .unwrap_or_else(|_| absolute_path.to_path_buf());

        // This should fail because it's outside the repo root after normalization
        let result = normalize_to_relpath(&normalized, repo_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_with_trailing_slash() {
        let repo_root = Path::new("/workspace");
        let absolute_path = Path::new("/workspace/src/lib.rs/");

        let result = normalize_to_relpath(absolute_path, repo_root).unwrap();
        // PathBuf normalizes away trailing slashes
        assert_eq!(result, Path::new("src/lib.rs"));
    }

    #[test]
    fn test_repo_root_itself() {
        let repo_root = Path::new("/workspace");
        let absolute_path = Path::new("/workspace");

        let result = normalize_to_relpath(absolute_path, repo_root).unwrap();
        // Stripping a path from itself yields an empty relative path
        assert_eq!(result, Path::new(""));
    }

    #[test]
    fn test_deeply_nested_path() {
        let repo_root = Path::new("/home/user/projects/myrepo");
        let absolute_path =
            Path::new("/home/user/projects/myrepo/crates/maproom/src/incremental/path_utils.rs");

        let result = normalize_to_relpath(absolute_path, repo_root).unwrap();
        assert_eq!(
            result,
            Path::new("crates/maproom/src/incremental/path_utils.rs")
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_paths() {
        let repo_root = Path::new(r"C:\workspace");
        let absolute_path = Path::new(r"C:\workspace\packages\cli\src\main.ts");

        let result = normalize_to_relpath(absolute_path, repo_root).unwrap();
        assert_eq!(result, Path::new(r"packages\cli\src\main.ts"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_path_outside_repo() {
        let repo_root = Path::new(r"C:\workspace");
        let outside_path = Path::new(r"D:\other\file.txt");

        let result = normalize_to_relpath(outside_path, repo_root);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("outside repository root"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_unc_path() {
        let repo_root = Path::new(r"\\server\share\workspace");
        let absolute_path = Path::new(r"\\server\share\workspace\src\main.rs");

        let result = normalize_to_relpath(absolute_path, repo_root).unwrap();
        assert_eq!(result, Path::new(r"src\main.rs"));
    }
}
