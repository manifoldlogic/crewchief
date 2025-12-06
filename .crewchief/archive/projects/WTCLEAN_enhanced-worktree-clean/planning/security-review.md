# Security Review: Enhanced Worktree Clean

## Security Assessment

### Threat Model

**What we're protecting:**
- User's git repository and worktrees
- User's file system
- Git branch data
- Maproom database integrity

**Who might attack:**
- Malicious code in user's repository
- Malicious plugins or extensions
- Compromised dependencies

**What they might do:**
- Delete unintended files/directories
- Delete important git branches
- Corrupt maproom database
- Execute arbitrary commands

**Risk level:** LOW - This is a local developer tool, not a network service. User already has full access to their own file system and git repository.

### Authentication & Authorization

**Not applicable** - This is a local CLI tool with no authentication or authorization requirements.

**Access control:**
- Tool runs with user's permissions (no elevation)
- Can only access files user can access
- Cannot escape user's permission boundary

**Risk:** None. User has full control over their own environment.

### Data Protection

**Sensitive data handled:**
- Git worktree paths
- Git branch names
- Maproom database location

**Protection measures:**
- No data transmitted over network
- No data logged to external services
- No credentials or secrets handled
- All operations are local file system

**Risk:** None. No sensitive data beyond what user already controls.

### Input Validation

**User inputs:**
1. **Worktree selector** (branch name, path, or basename)
2. **CLI flags** (`--keep-branch`, `--keep-maproom`, etc.)

**Validation approach:**

1. **Worktree selector:**
   - Existing validation: Resolved against known worktrees only
   - Path traversal prevention: Uses `path.resolve()` for canonicalization
   - Current directory protection: Prevents removing CWD
   - Ambiguity detection: Errors if selector matches multiple worktrees

2. **CLI flags:**
   - Type validation by Commander.js
   - Boolean flags (safe - no injection risk)
   - No user-supplied command arguments

**Existing safeguards:**
```typescript
// Prevents deleting current directory
const isCwdInsideTarget = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel))
if (isCwdInsideTarget) {
  logger.error('Refusing to remove the current working tree.')
  process.exitCode = 1
  return
}
```

**New code does NOT introduce new input vectors:**
- Maproom cleanup: Fixed command `['db', 'cleanup-stale', '--confirm']`
- Branch deletion: Uses existing `GitMergeService.deleteBranch()` (already validated)

**Risk:** LOW. Input validation already exists and is sufficient.

### Command Injection

**Potential vectors:**

1. **Maproom binary execution:**
   ```typescript
   spawnSync(maproomBin, ['db', 'cleanup-stale', '--confirm'])
   ```
   - Binary path found via `findMaproomBinary()` (controlled, not user input)
   - Arguments are hardcoded (no user input)
   - **Risk:** NONE

2. **Git branch deletion:**
   ```typescript
   await this.git.raw(['branch', '-d', branch])
   ```
   - Uses `simple-git` library (parameterized, not shell)
   - Branch name from git metadata (not arbitrary user input)
   - **Risk:** NONE

**Shell injection:**
- No shell invocation (`spawnSync` without `shell: true`)
- No string interpolation into commands
- All arguments passed as arrays (parameterized)

**Risk:** NONE. No command injection vectors.

### Path Traversal

**Potential vectors:**

1. **Directory deletion:**
   ```typescript
   removeDirSync(targetPath)
   ```
   - `targetPath` resolved from worktree list (git metadata)
   - Canonicalized with `path.resolve()` and `fs.realpathSync()`
   - Validated to not be CWD

2. **Binary discovery:**
   ```typescript
   fs.existsSync(binPath)
   ```
   - Checks existence only (read-only)
   - Paths are constructed, not user-supplied

**Risk:** NONE. Paths are validated and canonicalized.

### Privilege Escalation

**Does this code require elevated privileges?** NO

**Can it be tricked into elevating?** NO
- Runs with user's permissions only
- No `sudo`, `setuid`, or similar mechanisms
- No privilege escalation vectors

**Risk:** NONE.

### Denial of Service

**Potential vectors:**

1. **Filesystem exhaustion:** Deleting directories frees space (opposite of DoS)
2. **Database corruption:** Maproom handles locking, we catch errors
3. **Git repository corruption:** Git operations are atomic

**Risk:** NONE. No DoS vectors introduced.

### Data Integrity

**Concerns:**

1. **Accidental deletion of wrong worktree**
   - Mitigated by: Selector disambiguation
   - Mitigated by: Current directory protection
   - Mitigated by: Safe branch deletion (`-d` requires merge)

2. **Maproom database corruption**
   - Mitigated by: Maproom uses SQLite transactions
   - Mitigated by: We don't modify database directly (call binary)
   - Mitigated by: Best-effort cleanup (failure doesn't stop other steps)

3. **Git repository corruption**
   - Mitigated by: Use `simple-git` library (tested, safe)
   - Mitigated by: Git operations are atomic
   - Mitigated by: Safe branch deletion prevents data loss

**Risk:** LOW. Multiple safeguards prevent accidental deletion.

### Dependency Security

**New dependencies:** NONE
- Uses existing dependencies (simple-git, Commander.js, fs, path)
- Binary discovery is new code, but no new deps

**Existing dependencies:**
- `simple-git`: Well-maintained, widely used
- `commander`: Well-maintained, widely used
- Node built-ins: `fs`, `path`, `child_process` (no security concerns)

**Supply chain risk:** LOW
- No new dependencies introduced
- Existing dependencies already vetted

**Mitigation:**
- Regular `pnpm audit` (existing process)
- Dependabot updates (existing process)

### Known Gaps

| Gap | Risk Level | Impact | Mitigation | Status |
|-----|------------|--------|------------|--------|
| Unmerged branch deletion | Low | User could force-delete unmerged branch | Use `git branch -d` (safe delete), not `-D` (force) | Accepted |
| Binary path injection | None | User could set malicious `CREWCHIEF_MAPROOM_BIN` | User already has full system access | Accepted |
| Database locking errors | Low | Maproom cleanup might fail if DB locked | Log warning, continue with other cleanup steps | Accepted |
| Symlink confusion | Low | Symlinked worktrees might confuse path resolution | Use `fs.realpathSync()` for canonicalization | Accepted |

### Risk Assessment Summary

| Category | Risk Level | Justification |
|----------|------------|---------------|
| Command Injection | NONE | No shell invocation, parameterized commands |
| Path Traversal | NONE | Paths validated and canonicalized |
| Privilege Escalation | NONE | Runs with user permissions only |
| Data Integrity | LOW | Multiple safeguards, safe delete by default |
| Denial of Service | NONE | No DoS vectors |
| Supply Chain | LOW | No new dependencies |
| **Overall Risk** | **LOW** | Local tool, user-controlled environment |

## MVP Security Scope

### In Scope (MVP)

- [x] Path validation (existing, not changed)
- [x] Command injection prevention (parameterized commands)
- [x] Current directory protection (existing, not changed)
- [x] Safe branch deletion (use `-d`, not `-D`)
- [x] Best-effort cleanup (failures don't cascade)
- [x] Clear error messages (no sensitive info leakage)

### Out of Scope (Future/Never)

- Authentication/Authorization (not applicable for local CLI)
- Network security (no network operations)
- Encryption (no sensitive data)
- Audit logging (not required for local tool)
- Multi-user isolation (single-user tool)

### Deferred to Future

- `--force` flag for branch deletion (could add later with clear warnings)
- Dry-run mode (preview deletions before executing)

## Security Checklist

### Code Security

- [x] No hardcoded secrets
- [x] No hardcoded credentials
- [x] No API keys or tokens
- [x] Input validation on all external inputs
- [x] Proper error handling (no info leakage)
- [x] No SQL injection vulnerabilities (don't touch SQL directly)
- [x] No command injection vulnerabilities (parameterized commands)
- [x] No path traversal vulnerabilities (path validation)
- [x] No XSS vulnerabilities (not applicable - CLI tool)

### Dependency Security

- [x] No new dependencies introduced
- [x] Existing dependencies are up to date (per normal process)
- [x] Dependencies are from trusted sources (npm)
- [x] No vulnerable dependencies (per `pnpm audit`)

### Operational Security

- [x] Tool runs with minimal privileges (user permissions)
- [x] No network operations (local only)
- [x] No data exfiltration
- [x] Clear error messages (guide user, don't leak secrets)
- [x] Graceful degradation (failures don't cascade)

### Documentation Security

- [x] Document `--keep-branch` flag (safe default)
- [x] Warn about data loss (branch deletion)
- [x] Clear examples (prevent misuse)
- [x] No security-sensitive information in docs

## Threat Scenarios

### Scenario 1: Malicious Binary in PATH

**Threat:** User has malicious `crewchief-maproom` in PATH
**Attack:** Binary executes malicious code when called
**Impact:** Malicious code runs with user permissions
**Likelihood:** LOW (user already compromised)
**Mitigation:** None needed - user already has full system access
**Status:** ACCEPTED RISK

**Why accepted:** If user's PATH is compromised, they have bigger problems. This tool doesn't escalate privileges or do anything the user couldn't do themselves.

### Scenario 2: Accidental Wrong Worktree Deletion

**Threat:** User accidentally deletes wrong worktree
**Attack:** Typo in selector, ambiguous match
**Impact:** Lost work (if not committed)
**Likelihood:** MEDIUM (user error)
**Mitigation:**
- Selector disambiguation (existing)
- Current directory protection (existing)
- Safe branch deletion (prevents loss of unmerged work)
**Status:** MITIGATED

### Scenario 3: Database Corruption

**Threat:** Maproom database corrupted during cleanup
**Attack:** Concurrent access, disk full, power loss
**Impact:** Search results incorrect or unavailable
**Likelihood:** LOW (SQLite is robust)
**Mitigation:**
- Maproom uses SQLite transactions
- We don't modify database directly
- User can rebuild database with `scan`
**Status:** ACCEPTED RISK

### Scenario 4: Git Repository Corruption

**Threat:** Git repository corrupted during cleanup
**Attack:** Interrupted operation, disk failure
**Impact:** Lost git metadata or history
**Likelihood:** VERY LOW (git is atomic)
**Mitigation:**
- Use `simple-git` library (safe wrappers)
- Git operations are atomic
- User can recover from git reflog
**Status:** ACCEPTED RISK

## Comparison with Existing Code

**Similar operations in codebase:**

1. **`worktree merge` command:**
   - Already deletes branches
   - Already removes directories
   - Same risk profile
   - Well-tested and trusted

2. **`worktree create` command:**
   - Already calls maproom binary (scan)
   - Same binary discovery logic
   - Same risk profile

3. **`worktree clean --all` mode:**
   - Already removes multiple worktrees
   - Same directory deletion logic
   - Same risk profile

**Conclusion:** This enhancement follows established patterns and introduces NO NEW security risks beyond what already exists.

## Security Sign-Off

**Can we ship this without meaningful security concerns?** YES

**Rationale:**
1. No new attack vectors introduced
2. Follows existing secure patterns
3. Local tool (not network-exposed)
4. User-controlled environment
5. Multiple safeguards against accidental deletion
6. Graceful error handling prevents cascading failures
7. No privilege escalation
8. No sensitive data handling

**Recommended actions before ship:**
- [x] Code review (normal process)
- [x] Test on multiple platforms (manual)
- [x] Verify path validation (existing tests)
- [x] Verify error handling (new tests)
- [x] Document new flags (README update)

**No additional security review required.**

## Future Security Considerations

**If adding these features, reassess security:**

1. **`--force` flag for branch deletion**
   - Could enable unintended data loss
   - Add clear warnings in docs
   - Consider requiring confirmation

2. **Remote worktree cleanup**
   - Network operations introduce new risks
   - Requires authentication/authorization
   - Out of scope for MVP

3. **Automated cleanup (cron, watch)**
   - Could delete worktrees user is still using
   - Requires smarter detection
   - Out of scope for MVP

## Summary

This enhancement has **LOW security risk** and introduces **NO NEW attack vectors**.

**Key security properties:**
- No command injection (parameterized commands)
- No path traversal (validated paths)
- No privilege escalation (user permissions)
- No data exfiltration (local only)
- Safe by default (soft delete branches, graceful errors)

**Ship confidently:** Multiple safeguards, follows existing patterns, user-controlled environment.
