# Ticket: GITPOLL-1001: Implement GitState Module

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Implement the `GitState` module that represents the state of files in a git repository and provides parsing/diffing capabilities. This is the foundational data structure for the git polling system.

## Background

The current `notify`-based file watcher causes "too many open files" errors on large repositories because it creates file descriptors for every watched directory. The git polling approach eliminates this by using `git status --porcelain` output instead.

This ticket implements the core data structures and parsing logic that will be used by the GitPoller (GITPOLL-1002) to detect file changes.

Reference: [architecture.md](../planning/architecture.md) - GitState Component section

## Acceptance Criteria

- [x] `FileStatus` enum implemented with Clean, Modified, New, Deleted, Renamed variants
- [x] `GitState` struct with HashMap<PathBuf, FileStatus> storage
- [x] `from_git_status()` parser handles all git status codes (M, A, D, R, ??, etc.)
- [x] `diff()` method compares two states and returns Vec<FileEvent>
- [x] Path validation rejects absolute paths and `..` traversal
- [x] Handles quoted paths with spaces and unicode characters

## Technical Requirements

- Create new file: `crates/maproom/src/incremental/git_state.rs`
- Export from `crates/maproom/src/incremental/mod.rs`
- Use existing `FileEvent` enum from `events.rs` for diff output
- Implement `Default` for `GitState` (empty state)
- Use `thiserror` for error types consistent with codebase patterns

### FileStatus Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Clean,
    Modified,
    New,
    Deleted,
    Renamed { from: PathBuf },
}
```

### GitState Struct

```rust
#[derive(Debug, Clone, Default)]
pub struct GitState {
    files: HashMap<PathBuf, FileStatus>,
    captured_at: Option<Instant>,
}
```

### Git Status Code Mapping

| Git Code | FileStatus |
|----------|------------|
| `M ` (staged modified) | Modified |
| ` M` (unstaged modified) | Modified |
| `MM` (both) | Modified |
| `A ` (staged add) | New |
| `??` (untracked) | New |
| `D ` (staged delete) | Deleted |
| ` D` (unstaged delete) | Deleted |
| `R ` (renamed) | Renamed { from } |

### Path Validation

Use existing `normalize_to_relpath` from `path_utils.rs` pattern:

```rust
fn validate_path(path: &Path) -> Result<PathBuf, GitStateError> {
    if path.is_absolute() {
        return Err(GitStateError::InvalidPath {
            path: path.to_path_buf(),
            reason: "absolute path not allowed".into()
        });
    }
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(GitStateError::InvalidPath {
                path: path.to_path_buf(),
                reason: "path traversal not allowed".into()
            });
        }
    }
    Ok(path.to_path_buf())
}
```

## Implementation Notes

### Parsing Git Porcelain Output

Git porcelain format: `XY PATH` or `XY PATH -> NEWPATH` for renames
- X = index status
- Y = worktree status
- PATH may be quoted if contains special chars

Example parsing:
```rust
pub fn from_git_status(output: &str, _root: &Path) -> Result<Self, GitStateError> {
    let mut files = HashMap::new();

    for line in output.lines() {
        if line.len() < 3 { continue; }

        let status_chars = &line[0..2];
        let path_part = &line[3..];

        let (path, status) = parse_status_line(status_chars, path_part)?;
        let validated_path = validate_path(&path)?;
        files.insert(validated_path, status);
    }

    Ok(Self { files, captured_at: Some(Instant::now()) })
}
```

### Diff Logic

The `diff()` method compares old state to new state:
1. Files in new but not old → Modified event (new file)
2. Files in old but not new → Deleted event
3. Files in both with status change → Modified event
4. Renamed files → Renamed event

```rust
pub fn diff(&self, new: &GitState) -> Vec<FileEvent> {
    let mut events = Vec::new();

    // Check for new/modified files
    for (path, new_status) in &new.files {
        match self.files.get(path) {
            None => events.push(FileEvent::Modified(path.clone())),
            Some(old_status) if old_status != new_status => {
                events.push(FileEvent::Modified(path.clone()));
            }
            _ => {}
        }
    }

    // Check for deleted files
    for path in self.files.keys() {
        if !new.files.contains_key(path) {
            events.push(FileEvent::Deleted(path.clone()));
        }
    }

    events
}
```

### Handling Quoted Paths

Git quotes paths containing spaces or special characters:
```
" M \"path with spaces/file.rs\""
```

Implement `unquote_path()` helper to handle:
- Simple paths (no quotes)
- Quoted paths with escape sequences
- Unicode escapes

## Dependencies

- None (first ticket in sequence)

## Risk Assessment

- **Risk**: Git status format variations across git versions
  - **Mitigation**: Use `--porcelain` which is stable across versions. Test with git 2.20+.

- **Risk**: Unicode path handling edge cases
  - **Mitigation**: Add explicit test cases for unicode paths. Use `OsString` where needed.

## Files/Packages Affected

- `crates/maproom/src/incremental/git_state.rs` (NEW)
- `crates/maproom/src/incremental/mod.rs` (export new module)
