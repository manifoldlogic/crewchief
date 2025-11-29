# Ticket: BRWATCH-1003: Implement branch name parsing from .git/HEAD

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (15 tests, all pass)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create utility function to parse the current branch name from .git/HEAD file content, handling both branch references and detached HEAD states.

## Background
This ticket implements Step 1.3 from the implementation plan. The .git/HEAD file contains either a branch reference (e.g., "ref: refs/heads/main") or a commit SHA (detached HEAD). We need reliable parsing to extract the branch name for worktree identification.

From architecture.md lines 164-194:
- Branch refs: "ref: refs/heads/main\n" → "main"
- Feature branches: "ref: refs/heads/feature/auth\n" → "feature/auth"
- Detached HEAD: "abc123def...\n" → "abc123de" (short SHA)

**Planning Reference**: `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 1.3

## Acceptance Criteria
- [x] `get_current_branch()` function implemented in watcher.rs
- [x] Correctly parses standard branch refs (e.g., "main", "feature/auth")
- [x] Handles detached HEAD by returning first 8 characters of commit SHA
- [x] Returns error for invalid .git/HEAD format
- [x] Trims whitespace from parsed values
- [x] Unit tests pass for all scenarios: branch ref, feature branch, detached HEAD, invalid format
- [x] Function documented with rustdoc comments and examples

## Technical Requirements
- Function signature: `fn get_current_branch(repo_path: &Path) -> Result<String>`
- Read .git/HEAD file content
- Strip "ref: refs/heads/" prefix if present
- For detached HEAD, return first 8 chars of SHA
- Handle edge cases: empty file, corrupt content, missing file
- Use anyhow::Result for error handling
- Include comprehensive unit tests

## Implementation Notes

From architecture.md lines 164-194:

```rust
fn get_current_branch(repo_path: &Path) -> Result<String> {
    let head_path = repo_path.join(".git/HEAD");
    let content = fs::read_to_string(&head_path)?;

    // Parse "ref: refs/heads/main" or commit SHA
    if let Some(branch_ref) = content.strip_prefix("ref: refs/heads/") {
        Ok(branch_ref.trim().to_string())
    } else {
        // Detached HEAD (commit SHA)
        let sha = content.trim();
        if sha.len() >= 8 {
            Ok(sha[..8].to_string()) // Short SHA
        } else {
            anyhow::bail!("Invalid HEAD content: {}", content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_branch_ref() {
        // Create test with "ref: refs/heads/main\n" content
        let branch = parse_head_content("ref: refs/heads/main\n").unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_parse_feature_branch() {
        let branch = parse_head_content("ref: refs/heads/feature/auth-system\n").unwrap();
        assert_eq!(branch, "feature/auth-system");
    }

    #[test]
    fn test_parse_detached_head() {
        let branch = parse_head_content("abc123def456789012345678901234567890abcd\n").unwrap();
        assert_eq!(branch, "abc123de"); // Short SHA
    }

    #[test]
    fn test_parse_invalid_format() {
        let result = parse_head_content("invalid format");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_sha() {
        let result = parse_head_content("abc\n");
        assert!(result.is_err()); // SHA too short
    }
}
```

Add helper function for testing:
```rust
#[cfg(test)]
fn parse_head_content(content: &str) -> Result<String> {
    if let Some(branch_ref) = content.strip_prefix("ref: refs/heads/") {
        Ok(branch_ref.trim().to_string())
    } else {
        let sha = content.trim();
        if sha.len() >= 8 {
            Ok(sha[..8].to_string())
        } else {
            anyhow::bail!("Invalid HEAD content: too short")
        }
    }
}
```

## Dependencies
- BRWATCH-1002 complete (watcher.rs file exists)

## Risk Assessment
- **Risk**: .git/HEAD format varies across git versions
  - **Mitigation**: Test with multiple git versions, handle edge cases
- **Risk**: File read race condition during git operations
  - **Mitigation**: Retry on file read errors, log transient failures
- **Risk**: Unexpected HEAD formats (submodules, worktrees)
  - **Mitigation**: Comprehensive unit tests, graceful error messages

## Files/Packages Affected
- `/workspace/crates/maproom/src/watcher.rs` (add get_current_branch function and tests)
