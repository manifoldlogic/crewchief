//! Git repository state tracking for polling-based change detection.
//!
//! This module provides state representation and diffing capabilities for
//! git-based file change detection. It parses `git status --porcelain` output
//! and compares states to emit [`FileEvent`]s for detected changes.
//!
//! # Security
//!
//! Path validation rejects:
//! - Absolute paths
//! - Path traversal attempts (`..` components)
//!
//! # Example
//!
//! ```ignore
//! use crewchief_maproom::incremental::git_state::GitState;
//! use std::path::Path;
//!
//! let output = "M  src/main.rs\n?? new_file.rs\n";
//! let state = GitState::from_git_status(output)?;
//!
//! // Compare with previous state to detect changes
//! let events = old_state.diff(&state);
//! ```

use super::events::FileEvent;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;
use thiserror::Error;

/// Errors that can occur when working with git state.
#[derive(Debug, Error)]
pub enum GitStateError {
    /// Invalid path encountered in git status output.
    #[error("invalid path '{path}': {reason}")]
    InvalidPath { path: PathBuf, reason: String },

    /// Failed to parse git status output line.
    #[error("parse error on line '{line}': {reason}")]
    ParseError { line: String, reason: String },
}

/// Status of a file in the git repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File is tracked and unmodified.
    Clean,
    /// File is modified (staged or unstaged).
    Modified,
    /// File is new (added or untracked).
    New,
    /// File is staged for deletion or deleted from worktree.
    Deleted,
    /// File was renamed from another path.
    Renamed { from: PathBuf },
}

/// Represents the state of files in a git repository at a point in time.
///
/// GitState tracks file statuses parsed from `git status --porcelain` output
/// and supports diffing against another state to detect changes.
#[derive(Debug, Clone)]
pub struct GitState {
    /// Map of relative path to file status.
    files: HashMap<PathBuf, FileStatus>,
    /// When this state was captured.
    captured_at: Option<Instant>,
}

impl Default for GitState {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
            captured_at: None,
        }
    }
}

impl GitState {
    /// Create a new empty GitState.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse git status porcelain output into a GitState.
    ///
    /// Expects output from `git status --porcelain` or `git status --porcelain -M`.
    ///
    /// # Format
    ///
    /// The porcelain format is: `XY PATH` or `XY PATH -> NEWPATH` for renames
    /// - X = index status
    /// - Y = worktree status
    /// - PATH may be quoted if contains special characters
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - A path is absolute
    /// - A path contains `..` components (path traversal)
    /// - A line cannot be parsed
    pub fn from_git_status(output: &str) -> Result<Self, GitStateError> {
        let mut files = HashMap::new();

        for line in output.lines() {
            // Skip empty or whitespace-only lines
            let line = line.trim_end();
            if line.is_empty() || line.len() < 3 {
                continue;
            }

            // Skip lines that are just whitespace
            if line.chars().all(|c| c.is_whitespace()) {
                continue;
            }

            let status_chars = &line[0..2];
            let path_part = &line[3..];

            let (path, status) = parse_status_line(status_chars, path_part)?;
            let validated_path = validate_path(&path)?;
            files.insert(validated_path, status);
        }

        Ok(Self {
            files,
            captured_at: Some(Instant::now()),
        })
    }

    /// Compare this state with a newer state and return the differences as FileEvents.
    ///
    /// The comparison logic:
    /// - Files with Deleted status → Deleted event (git shows file was deleted)
    /// - Files in `new` but not in `self` → Modified event (new file appeared)
    /// - Files in `self` but not in `new` → Deleted event (file disappeared from git status)
    /// - Files in both with different status → Modified event
    /// - Renamed files → Renamed event
    pub fn diff(&self, new: &GitState) -> Vec<FileEvent> {
        let mut events = Vec::new();

        // Check for new/modified/deleted files and renames
        for (path, new_status) in &new.files {
            match new_status {
                FileStatus::Renamed { from } => {
                    // Emit rename event
                    events.push(FileEvent::Renamed(from.clone(), path.clone()));
                }
                FileStatus::Deleted => {
                    // Git shows file as deleted - always emit Deleted event
                    // This handles committed files that were removed from worktree
                    events.push(FileEvent::Deleted(path.clone()));
                }
                _ => {
                    match self.files.get(path) {
                        None => {
                            // File is new (wasn't in previous state)
                            events.push(FileEvent::Modified(path.clone()));
                        }
                        Some(old_status) if old_status != new_status => {
                            // Status changed
                            events.push(FileEvent::Modified(path.clone()));
                        }
                        _ => {
                            // No change
                        }
                    }
                }
            }
        }

        // Check for deleted files (in old state but not in new)
        for (path, old_status) in &self.files {
            // Skip if this was the source of a rename
            let is_rename_source = new.files.values().any(|s| {
                matches!(s, FileStatus::Renamed { from } if from == path)
            });

            if is_rename_source {
                continue;
            }

            if !new.files.contains_key(path) {
                // Check if the old status was Deleted - if so, it's already gone
                if !matches!(old_status, FileStatus::Deleted) {
                    events.push(FileEvent::Deleted(path.clone()));
                }
            }
        }

        events
    }

    /// Returns the number of tracked files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns true if no files are tracked.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get the status of a specific file.
    pub fn get(&self, path: &Path) -> Option<&FileStatus> {
        self.files.get(path)
    }

    /// Returns when this state was captured.
    pub fn captured_at(&self) -> Option<Instant> {
        self.captured_at
    }

    /// Insert a file status (primarily for testing).
    #[cfg(test)]
    pub fn insert(&mut self, path: PathBuf, status: FileStatus) {
        self.files.insert(path, status);
    }
}

/// Validate a path for security concerns.
fn validate_path(path: &Path) -> Result<PathBuf, GitStateError> {
    // Reject absolute paths
    if path.is_absolute() {
        return Err(GitStateError::InvalidPath {
            path: path.to_path_buf(),
            reason: "absolute path not allowed".into(),
        });
    }

    // Reject path traversal attempts
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(GitStateError::InvalidPath {
                path: path.to_path_buf(),
                reason: "path traversal not allowed".into(),
            });
        }
    }

    Ok(path.to_path_buf())
}

/// Parse a single status line from git status output.
fn parse_status_line(status_chars: &str, path_part: &str) -> Result<(PathBuf, FileStatus), GitStateError> {
    let chars: Vec<char> = status_chars.chars().collect();
    if chars.len() != 2 {
        return Err(GitStateError::ParseError {
            line: format!("{}{}", status_chars, path_part),
            reason: "invalid status code length".into(),
        });
    }

    let index_status = chars[0];
    let worktree_status = chars[1];

    // Handle renames: "R  old.rs -> new.rs" or "RM old.rs -> new.rs"
    if index_status == 'R' {
        return parse_rename(path_part);
    }

    // Handle copies similarly to adds (C  original.rs -> copy.rs)
    if index_status == 'C' {
        return parse_copy(path_part);
    }

    // Unquote the path if needed
    let path = unquote_path(path_part)?;

    // Determine file status based on status codes
    let status = match (index_status, worktree_status) {
        // Deleted
        ('D', _) | (_, 'D') => FileStatus::Deleted,
        // Added (staged) or untracked
        ('A', _) | ('?', '?') => FileStatus::New,
        // Modified (any combination of M in index or worktree)
        ('M', _) | (_, 'M') | ('T', _) | (_, 'T') => FileStatus::Modified,
        // Updated in index
        ('U', _) | (_, 'U') => FileStatus::Modified,
        // Ignored (should not appear with default options, but handle gracefully)
        ('!', '!') => FileStatus::Clean,
        // Anything else is treated as modified if there's any status
        (' ', ' ') => FileStatus::Clean,
        _ => FileStatus::Modified,
    };

    Ok((path, status))
}

/// Parse a rename line: "old.rs -> new.rs" or quoted variants.
fn parse_rename(path_part: &str) -> Result<(PathBuf, FileStatus), GitStateError> {
    // Look for " -> " separator
    if let Some(arrow_pos) = path_part.find(" -> ") {
        let old_path_str = &path_part[..arrow_pos];
        let new_path_str = &path_part[arrow_pos + 4..];

        let old_path = unquote_path(old_path_str)?;
        let new_path = unquote_path(new_path_str)?;

        // Validate both paths
        validate_path(&old_path)?;
        validate_path(&new_path)?;

        Ok((new_path, FileStatus::Renamed { from: old_path }))
    } else {
        Err(GitStateError::ParseError {
            line: path_part.to_string(),
            reason: "rename missing ' -> ' separator".into(),
        })
    }
}

/// Parse a copy line: "original.rs -> copy.rs" - treat as new file.
fn parse_copy(path_part: &str) -> Result<(PathBuf, FileStatus), GitStateError> {
    // Look for " -> " separator
    if let Some(arrow_pos) = path_part.find(" -> ") {
        let new_path_str = &path_part[arrow_pos + 4..];
        let new_path = unquote_path(new_path_str)?;
        Ok((new_path, FileStatus::New))
    } else {
        Err(GitStateError::ParseError {
            line: path_part.to_string(),
            reason: "copy missing ' -> ' separator".into(),
        })
    }
}

/// Unquote a path from git status output.
///
/// Git quotes paths containing spaces or special characters:
/// - Simple paths: `src/main.rs`
/// - Quoted paths: `"path with spaces/file.rs"`
/// - Escaped quotes: `"file\"quoted\".rs"`
/// - Unicode escapes: `"\303\251"` for é
fn unquote_path(path_str: &str) -> Result<PathBuf, GitStateError> {
    let path_str = path_str.trim();

    // Check if the path is quoted
    if path_str.starts_with('"') && path_str.ends_with('"') && path_str.len() >= 2 {
        let inner = &path_str[1..path_str.len() - 1];
        let unescaped = unescape_git_path(inner)?;
        Ok(PathBuf::from(unescaped))
    } else {
        Ok(PathBuf::from(path_str))
    }
}

/// Unescape a git-quoted path string.
fn unescape_git_path(s: &str) -> Result<String, GitStateError> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                // Octal escape sequences (e.g., \303\251 for UTF-8)
                Some(c1) if c1.is_ascii_digit() => {
                    let mut octal = String::from(c1);
                    // Git uses up to 3 octal digits
                    for _ in 0..2 {
                        if let Some(&next) = chars.peek() {
                            if next.is_ascii_digit() {
                                octal.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                    }
                    if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                        result.push(byte as char);
                    } else {
                        // If we can't parse as octal, just include literally
                        result.push('\\');
                        result.push_str(&octal);
                    }
                }
                Some(other) => {
                    // Unknown escape, keep as-is
                    result.push('\\');
                    result.push(other);
                }
                None => {
                    // Trailing backslash
                    result.push('\\');
                }
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============== Parsing Tests ==============

    #[test]
    fn test_parse_modified_staged() {
        let output = "M  src/main.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_modified_unstaged() {
        let output = " M src/main.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_modified_both() {
        let output = "MM src/main.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn test_parse_added_staged() {
        let output = "A  new-file.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("new-file.rs")), Some(&FileStatus::New));
    }

    #[test]
    fn test_parse_untracked() {
        let output = "?? untracked.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("untracked.rs")), Some(&FileStatus::New));
    }

    #[test]
    fn test_parse_deleted_staged() {
        let output = "D  deleted.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("deleted.rs")), Some(&FileStatus::Deleted));
    }

    #[test]
    fn test_parse_deleted_unstaged() {
        let output = " D deleted.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("deleted.rs")), Some(&FileStatus::Deleted));
    }

    #[test]
    fn test_parse_renamed() {
        let output = "R  old.rs -> new.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(
            state.get(Path::new("new.rs")),
            Some(&FileStatus::Renamed { from: PathBuf::from("old.rs") })
        );
    }

    #[test]
    fn test_parse_copied() {
        let output = "C  original.rs -> copy.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.get(Path::new("copy.rs")), Some(&FileStatus::New));
    }

    #[test]
    fn test_parse_multiple_files() {
        let output = "M  modified.rs\n?? untracked.rs\n D deleted.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert_eq!(state.len(), 3);
        assert_eq!(state.get(Path::new("modified.rs")), Some(&FileStatus::Modified));
        assert_eq!(state.get(Path::new("untracked.rs")), Some(&FileStatus::New));
        assert_eq!(state.get(Path::new("deleted.rs")), Some(&FileStatus::Deleted));
    }

    #[test]
    fn test_parse_empty_output() {
        let output = "";
        let state = GitState::from_git_status(output).unwrap();
        assert!(state.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let output = "   \n\n";
        let state = GitState::from_git_status(output).unwrap();
        assert!(state.is_empty());
    }

    // ============== Path Handling Tests ==============

    #[test]
    fn test_parse_path_with_spaces() {
        let output = " M \"path with spaces/file.rs\"\n";
        let state = GitState::from_git_status(output).unwrap();
        assert!(state.get(Path::new("path with spaces/file.rs")).is_some());
    }

    #[test]
    fn test_parse_path_with_quotes_in_name() {
        // Git escapes quotes in paths
        let output = " M \"file\\\"quoted\\\".rs\"\n";
        let state = GitState::from_git_status(output).unwrap();
        assert!(state.get(Path::new("file\"quoted\".rs")).is_some());
    }

    #[test]
    fn test_reject_absolute_path() {
        let output = " M /etc/passwd\n";
        let result = GitState::from_git_status(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, GitStateError::InvalidPath { .. }));
    }

    #[test]
    fn test_reject_path_traversal() {
        let output = " M ../outside/file.rs\n";
        let result = GitState::from_git_status(output);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, GitStateError::InvalidPath { .. }));
    }

    #[test]
    fn test_reject_hidden_traversal() {
        let output = " M foo/../bar/../../etc/passwd\n";
        let result = GitState::from_git_status(output);
        assert!(result.is_err());
    }

    // ============== State Diff Tests ==============

    #[test]
    fn test_diff_new_file() {
        let old = GitState::default();
        let mut new = GitState::default();
        new.insert(PathBuf::from("new.rs"), FileStatus::New);

        let events = old.diff(&new);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], FileEvent::Modified(p) if p == Path::new("new.rs")));
    }

    #[test]
    fn test_diff_deleted_file() {
        let mut old = GitState::default();
        old.insert(PathBuf::from("deleted.rs"), FileStatus::Clean);
        let new = GitState::default();

        let events = old.diff(&new);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], FileEvent::Deleted(p) if p == Path::new("deleted.rs")));
    }

    #[test]
    fn test_diff_file_with_deleted_status() {
        // When git status shows a file as deleted (e.g., ` D file.rs`),
        // we should emit a Deleted event even if the file wasn't in the old state.
        // This happens when a committed file is removed from the worktree.
        let old = GitState::default();
        let mut new = GitState::default();
        new.insert(PathBuf::from("deleted.rs"), FileStatus::Deleted);

        let events = old.diff(&new);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], FileEvent::Deleted(p) if p == Path::new("deleted.rs")));
    }

    #[test]
    fn test_diff_modified_file() {
        let mut old = GitState::default();
        old.insert(PathBuf::from("file.rs"), FileStatus::Clean);
        let mut new = GitState::default();
        new.insert(PathBuf::from("file.rs"), FileStatus::Modified);

        let events = old.diff(&new);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], FileEvent::Modified(p) if p == Path::new("file.rs")));
    }

    #[test]
    fn test_diff_renamed_file() {
        let mut old = GitState::default();
        old.insert(PathBuf::from("old.rs"), FileStatus::Clean);
        let mut new = GitState::default();
        new.insert(PathBuf::from("new.rs"), FileStatus::Renamed { from: PathBuf::from("old.rs") });

        let events = old.diff(&new);
        // Should emit rename event
        assert!(events.iter().any(|e| matches!(e, FileEvent::Renamed(o, n) if o == Path::new("old.rs") && n == Path::new("new.rs"))));
    }

    #[test]
    fn test_diff_no_changes() {
        let mut old = GitState::default();
        old.insert(PathBuf::from("file.rs"), FileStatus::Clean);
        let mut new = GitState::default();
        new.insert(PathBuf::from("file.rs"), FileStatus::Clean);

        let events = old.diff(&new);
        assert!(events.is_empty());
    }

    #[test]
    fn test_diff_multiple_changes() {
        let mut old = GitState::default();
        old.insert(PathBuf::from("existing.rs"), FileStatus::Clean);
        old.insert(PathBuf::from("to-delete.rs"), FileStatus::Clean);

        let mut new = GitState::default();
        new.insert(PathBuf::from("existing.rs"), FileStatus::Modified);
        new.insert(PathBuf::from("new.rs"), FileStatus::New);

        let events = old.diff(&new);
        assert_eq!(events.len(), 3); // modified + deleted + new
    }

    // ============== GitState Methods Tests ==============

    #[test]
    fn test_gitstate_default() {
        let state = GitState::default();
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
        assert!(state.captured_at().is_none());
    }

    #[test]
    fn test_gitstate_from_git_status_sets_timestamp() {
        let output = "M  file.rs\n";
        let state = GitState::from_git_status(output).unwrap();
        assert!(state.captured_at().is_some());
    }

    #[test]
    fn test_unquote_simple_path() {
        let path = unquote_path("src/main.rs").unwrap();
        assert_eq!(path, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_unquote_quoted_path() {
        let path = unquote_path("\"path with spaces.rs\"").unwrap();
        assert_eq!(path, PathBuf::from("path with spaces.rs"));
    }

    #[test]
    fn test_unquote_escaped_quotes() {
        let path = unquote_path("\"file\\\"name\\\".rs\"").unwrap();
        assert_eq!(path, PathBuf::from("file\"name\".rs"));
    }
}
