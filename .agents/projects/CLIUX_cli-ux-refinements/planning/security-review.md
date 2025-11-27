# Security Review: CLI UX Refinements

## Scope Assessment

This project modifies CLI command behavior without introducing new:
- Network connections
- File system operations (beyond existing)
- Authentication/authorization
- Data storage
- External dependencies

**Security impact: Minimal**

## Architecture Security Analysis

### Changes Under Review

1. **Worktree use** - No longer creates worktrees
2. **Worktree create** - Default output change (path instead of subshell)
3. **Spawn command** - Moved under `agent` subcommand

### Security Considerations

#### 1. Path Output to stdout

**Change**: Commands now output file paths to stdout by default.

**Analysis**:
- Paths are already available via `--print` flag
- No new information exposure
- Standard CLI pattern (pwd, git rev-parse, etc.)

**Risk**: None

#### 2. Subshell Spawning

**Current**: `spawn(shell, { stdio: 'inherit', cwd: targetPath })`

**Change**: Now opt-in via `--shell` flag

**Analysis**:
- Same spawning logic, different trigger
- Shell is from `process.env.SHELL` (user's configured shell)
- No command injection possible (no user input in spawn args)
- `cwd` is validated worktree path

**Risk**: None (no change to mechanism)

#### 3. Command Registration

**Change**: `spawn` moved from top-level to `agent spawn`

**Analysis**:
- Same code, different registration point
- No new capabilities
- No permission changes

**Risk**: None

## Input Validation

### Existing Validation (Unchanged)

| Input | Validation | Location |
|-------|------------|----------|
| Worktree name | Matched against existing worktrees | worktree.ts |
| Branch name | Git validates | WorktreeService |
| Base path | Resolved to absolute | worktree.ts |
| Agent type | Validated by Scheduler | spawn logic |

### New Validation Requirements

None. All inputs already validated by existing code.

## Potential Attack Vectors

### 1. Path Traversal
**Scenario**: Malicious worktree name like `../../etc/passwd`

**Current mitigation**: Git worktree names are validated by git itself. Our matching logic uses `path.resolve()` and compares against known worktrees.

**Status**: Already mitigated

### 2. Shell Injection
**Scenario**: Inject commands via worktree name

**Current mitigation**: Worktree names become branch names in git. Git sanitizes branch names. Our code uses `spawn()` with array arguments, not shell string interpolation.

**Status**: Already mitigated

### 3. Environment Variable Leakage
**Scenario**: Sensitive env vars exposed

**Analysis**: We pass `process.env` to spawned shells (unchanged). This is expected behavior for subshells.

**Status**: Accepted (expected behavior)

## Dependency Analysis

No new dependencies introduced. Using existing:
- `child_process` (Node.js built-in)
- `commander` (CLI framework)
- `chalk` (terminal colors)

## Secrets and Credentials

This project does not handle:
- API keys
- Passwords
- Tokens
- Personal data

No secrets management changes.

## Known Gaps

### Existing Gap: Shell Environment
The CLI inherits and passes full environment to subshells. This includes potentially sensitive variables. This is standard CLI behavior and not introduced by this project.

**Recommendation**: Document that subshells inherit full environment. Out of scope for this project.

## Security Checklist

- [x] No new network connections
- [x] No new file system operations outside worktrees
- [x] No new dependencies with known vulnerabilities
- [x] Input validation unchanged/sufficient
- [x] No credential handling changes
- [x] No authentication changes
- [x] Path handling uses safe methods (path.resolve)
- [x] Process spawning uses safe patterns (spawn with arrays)

## Conclusion

**Security Status**: ✅ Approved

This project makes behavioral changes to existing CLI commands without introducing new security concerns. All existing mitigations remain in place. No security-related code changes required.

## Recommendations

1. **Documentation**: Note in help text that `--shell` spawns with inherited environment
2. **Future consideration**: Consider `--clean-env` flag for sensitive environments (out of scope)
