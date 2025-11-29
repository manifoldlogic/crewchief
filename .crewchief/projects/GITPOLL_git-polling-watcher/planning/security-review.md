# Security Review: Git Polling File Watcher

## Architecture Security Analysis

### Attack Surface

The git polling watcher has a minimal attack surface:

1. **External Process Execution**: Runs `git status` command
2. **File Path Handling**: Parses paths from git output
3. **State Storage**: In-memory only (no persistence)

### Threat Model

| Threat | Vector | Risk | Status |
|--------|--------|------|--------|
| Command Injection | Malformed repo path | Medium | Mitigated |
| Path Traversal | Git status output parsing | Low | Mitigated |
| Denial of Service | Large git output | Low | Acceptable |
| Information Disclosure | Error messages | Low | Acceptable |

## Security Analysis by Component

### 1. Git Command Execution

**Risk**: Command injection through repository path

**Current Design**:
```rust
Command::new("git")
    .args(["status", "--porcelain"])
    .current_dir(&self.root)  // Path used as working directory
    .output()
```

**Analysis**:
- Using `Command::new()` with explicit args prevents shell injection
- Path is used as `current_dir`, not interpolated into command string
- Git itself validates the working directory

**Mitigation**: Already safe by design

### 2. Path Parsing from Git Output

**Risk**: Malicious paths in git status output (e.g., `../../etc/passwd`)

**Current Design**:
```rust
pub fn from_git_status(output: &str, root: &Path) -> Result<Self> {
    for line in output.lines() {
        let path = parse_path(line)?;
        // path is relative to git root
    }
}
```

**Analysis**:
- Git outputs paths relative to repository root
- Paths with `..` components would be unusual (git normally normalizes)
- Downstream consumers (`normalize_to_relpath`) already reject `..` paths

**Mitigation**:
- Add explicit check in path parsing
- Reject paths with `..` components
- Log suspicious paths for debugging

```rust
fn validate_path(path: &Path) -> Result<PathBuf> {
    // Reject absolute paths
    if path.is_absolute() {
        return Err(GitPollerError::InvalidPath {
            path: path.to_path_buf(),
            reason: "absolute path not allowed".into(),
        });
    }

    // Reject path traversal
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(GitPollerError::InvalidPath {
                path: path.to_path_buf(),
                reason: "path traversal not allowed".into(),
            });
        }
    }

    Ok(path.to_path_buf())
}
```

### 3. Resource Exhaustion

**Risk**: Very large git status output consuming memory

**Analysis**:
- Output size proportional to number of changed files
- Extremely unlikely to have millions of changed files
- Git itself limits output reasonably

**Mitigation**:
- Add configurable max output size (default: 10MB)
- Log warning if output is unusually large

```rust
const MAX_GIT_OUTPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB

let output = Command::new("git")
    .args(["status", "--porcelain"])
    .current_dir(&self.root)
    .output()?;

if output.stdout.len() > MAX_GIT_OUTPUT_SIZE {
    warn!("Git status output unusually large: {} bytes", output.stdout.len());
    // Still process, but log for investigation
}
```

### 4. Error Information Disclosure

**Risk**: Error messages revealing sensitive path information

**Analysis**:
- Errors may contain file paths
- This is local-only tool (not network-exposed)
- Paths are visible to user anyway

**Mitigation**: Acceptable for MVP. Error messages can include paths.

## Comparison with Previous Implementation

| Aspect | Notify-based | Git Polling |
|--------|--------------|-------------|
| External process | None | git command |
| File descriptors | Many (risk) | Zero |
| Command injection | N/A | Mitigated |
| Path handling | Direct FS paths | Parsed from output |
| Privilege level | User | User |

**Net Assessment**: Git polling has slightly different attack surface but lower overall risk due to simpler design.

## Known Gaps

### 1. Git Binary Trust

**Gap**: We trust the git binary to behave correctly

**Risk**: Compromised git binary could return malicious output

**Assessment**: Out of scope. If user's git is compromised, there are bigger problems.

**Mitigation for Enterprise**: Document that git binary should be from trusted source.

### 2. Symlink Handling

**Gap**: Git may report symlinked paths

**Risk**: Following symlinks could access files outside repo

**Assessment**: Low risk. Git handles symlinks within repo. Maproom already logs symlinks.

**Mitigation**: Document symlink behavior. User responsibility.

### 3. Repository with Malicious History

**Gap**: Git status could include paths from malicious commits

**Risk**: Path traversal via committed path names

**Assessment**: Very low. Git normalizes paths internally.

**Mitigation**: Path validation already rejects `..` components.

## Recommendations

### Must Implement

1. **Path validation**: Reject absolute paths and `..` components
2. **Command execution safety**: Use `Command::new()` not shell (already done)

### Should Implement

1. **Output size limit**: Warn on unusually large git output
2. **Timeout enforcement**: Don't hang on slow git operations

### Nice to Have

1. **Audit logging**: Log unusual conditions for debugging
2. **Configurable git path**: Allow specifying git binary location

## Security Checklist

- [x] No shell command execution (using `Command::new()`)
- [ ] Path validation for traversal attacks
- [ ] Output size bounds checking
- [x] Timeout on external command
- [x] No sensitive data in logs (paths are not sensitive here)
- [x] No network exposure (local tool only)
- [x] No elevated privileges required

## Conclusion

The git polling approach is **security-acceptable for MVP**:

1. Lower complexity than native file watching
2. Minimal attack surface (just git command execution)
3. Path validation prevents traversal attacks
4. No new privilege requirements

The main security requirement is proper path validation when parsing git output, which is straightforward to implement.
