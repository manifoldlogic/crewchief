# Ticket: BRANCHX-1004: Implement git tree SHA and diff-tree functions

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create Rust functions to get git tree SHA and find changed files between two tree states, enabling incremental update optimization.

## Background
This is Phase 2, Step 2.1 of BRANCHX. After establishing the worktree tracking schema (Phase 1), we need git integration to detect changes. Git's tree SHA represents the entire repository state as a hash—if tree SHA matches, nothing changed. The `git diff-tree` command efficiently identifies which files changed between two tree states.

These functions are the core of the incremental update optimization: check tree SHA (instant), and if different, find changed files (fast).

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 2.1

## Acceptance Criteria
- [ ] New file `crates/maproom/src/git.rs` created with git integration functions
- [ ] `get_git_tree_sha(repo_path)` returns current git tree SHA
- [ ] `git_diff_tree(old_tree, new_tree, repo_path)` returns list of changed files with status (A/M/D)
- [ ] `get_current_branch(repo_path)` returns current branch name
- [ ] Unit tests pass for all git functions
- [ ] Functions handle git command errors gracefully (return Result)
- [ ] Module exported in `crates/maproom/src/lib.rs`

## Technical Requirements
- Use `std::process::Command` to call git (no libgit2 dependency for simplicity)
- `get_git_tree_sha` executes `git rev-parse HEAD^{tree}`
- `git_diff_tree` executes `git diff-tree -r --no-commit-id --name-status --diff-filter=AMD`
- Parse diff-tree output format: `<status><tab><filename>`
- FileStatus enum: Added, Modified, Deleted
- FileChange struct: {status, path}
- Error handling: bail! if git commands fail with stderr
- `get_current_branch` executes `git rev-parse --abbrev-ref HEAD`

## Implementation Notes

Create new file `crates/maproom/src/git.rs`:

```rust
// crates/maproom/src/git.rs
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{bail, Result};

pub fn get_git_tree_sha(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!("Failed to get git tree SHA: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub fn get_current_branch(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!("Failed to get current branch: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub status: FileStatus,
    pub path: PathBuf,
}

pub fn git_diff_tree(old_tree: &str, new_tree: &str, repo_path: &Path) -> Result<Vec<FileChange>> {
    let output = Command::new("git")
        .args([
            "diff-tree",
            "-r",
            "--no-commit-id",
            "--name-status",
            "--diff-filter=AMD",
            old_tree,
            new_tree,
        ])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        bail!("git diff-tree failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    parse_diff_tree_output(&String::from_utf8(output.stdout)?)
}

fn parse_diff_tree_output(output: &str) -> Result<Vec<FileChange>> {
    let mut changes = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let status = match parts[0] {
            "A" => FileStatus::Added,
            "M" => FileStatus::Modified,
            "D" => FileStatus::Deleted,
            _ => continue,
        };

        changes.push(FileChange {
            status,
            path: PathBuf::from(parts[1]),
        });
    }

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_git_tree_sha() {
        // Test in current repo
        let repo_path = Path::new(".");
        let result = get_git_tree_sha(repo_path);
        assert!(result.is_ok());
        let tree_sha = result.unwrap();
        assert_eq!(tree_sha.len(), 40); // Git SHA is 40 chars
    }

    #[test]
    fn test_get_current_branch() {
        let repo_path = Path::new(".");
        let result = get_current_branch(repo_path);
        assert!(result.is_ok());
        // Branch name should be non-empty
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_diff_tree_output() {
        let output = "A\tsrc/new_file.rs\nM\tsrc/existing.rs\nD\tsrc/old.rs\n";
        let result = parse_diff_tree_output(output);
        assert!(result.is_ok());

        let changes = result.unwrap();
        assert_eq!(changes.len(), 3);

        assert_eq!(changes[0].status, FileStatus::Added);
        assert_eq!(changes[0].path, PathBuf::from("src/new_file.rs"));

        assert_eq!(changes[1].status, FileStatus::Modified);
        assert_eq!(changes[1].path, PathBuf::from("src/existing.rs"));

        assert_eq!(changes[2].status, FileStatus::Deleted);
        assert_eq!(changes[2].path, PathBuf::from("src/old.rs"));
    }
}
```

Update `crates/maproom/src/lib.rs` to add:
```rust
pub mod git;
```

See `architecture.md` section "Git Integration Functions" (lines 131-208) for complete design rationale.

## Dependencies
- Phase 1 complete (BRANCHX-1001, BRANCHX-1002, BRANCHX-1003)
- Git available in PATH

## Risk Assessment
- **Risk**: Git commands fail in environments without git
  - **Mitigation**: Return clear error message, document git requirement in README
- **Risk**: diff-tree output format changes across git versions
  - **Mitigation**: Comprehensive parsing tests with various output formats, handle edge cases gracefully
- **Risk**: Repository in detached HEAD state
  - **Mitigation**: `get_current_branch` should handle detached HEAD (will return "HEAD")

## Files/Packages Affected
- `crates/maproom/src/git.rs` (new)
- `crates/maproom/src/lib.rs` (add `pub mod git;`)

## Planning References
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 2.1
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Git Integration Functions (lines 131-208)
