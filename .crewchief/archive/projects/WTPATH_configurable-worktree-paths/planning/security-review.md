# Security Review: Configurable Worktree Paths

## Security Assessment

**Overall Risk Level**: LOW

This project involves file system operations but has limited security implications because:
- Users explicitly configure paths (not accepting external input)
- Git commands provide additional validation
- Existing safety checks prevent accidental deletion

### Authentication & Authorization

Not applicable - this is a local development tool with no authentication layer.

### Data Protection

**Worktree Paths**: Stored in config file as plaintext. Not sensitive information.

**No Encryption Needed**: Paths are local file system locations, not secrets.

### Input Validation

**Config File Execution**: Config files are JavaScript that executes with user permissions. This is standard for Node.js tooling (same as `package.json` scripts, webpack config, etc.).

**Path Validation**:
1. **System Directory Check**: Reject `/`, `/usr`, `/etc`, `/bin`, `/sbin`, `/System`, `C:\Windows`
2. **Repository Name Sanitization**: Remove path separators (/, \) and dangerous characters (:, *, ?, ", <, >, |)
3. **Home Directory Validation**: Verify `os.homedir()` returns valid path (not `/` or empty)
4. **Tilde Expansion**: Only expand leading `~` (matches Rust maproom implementation)

**Implementation**:
```typescript
const forbidden = ['/', '/usr', '/etc', '/bin', '/sbin', '/System', 'C:\\Windows']
if (forbidden.includes(resolved)) {
  throw new Error('Invalid worktree base path: system directory')
}
```

## Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| Malicious config file | Low | User must trust repository (same as npm scripts) | Accepted |
| Symlink following in parent paths | Low | Expected behavior for file system tools | Accepted |
| No environment variable expansion | None | Out of scope for MVP | Open |

## MVP Security Scope

**In Scope**:
- Path validation to reject system directories
- Repository name sanitization
- Home directory validation
- Clear error messages for invalid paths
- Existing symlink safety checks (realpathSync before deletion)

**Out of Scope**:
- Sandboxing (not a sandbox tool)
- SELinux/AppArmor integration
- Cryptographic validation of config files
- Rate limiting or DOS prevention
- Multi-user scenarios
- Encrypted path storage

**Rationale**: MVP is for trusted developer workflows. Users control repositories and config files. Security model matches other developer tools.

## Security Threats Analyzed

### 1. Path Traversal

**Threat**: Config could specify path outside repository to delete sensitive directories

**Mitigation**:
- System directory validation rejects `/`, `/etc`, `/usr`, etc.
- Git validation (git worktree commands fail on invalid paths)
- Existing safety check prevents deleting current working directory
- Multiple layers of defense

**Code Reference**: `packages/cli/src/git/worktrees.ts:206-213`

**Risk**: LOW (multiple protections)

### 2. Symlink Attacks

**Threat**: Symlink in config path tricks deletion into removing sensitive files

**Mitigation**:
- `fs.realpathSync()` resolves symlinks before deletion (existing)
- Check if resolved path contains current directory (existing)
- Only removes directories git recognizes as worktrees
- No additional changes needed

**Risk**: LOW (existing protections adequate)

### 3. Home Directory Confusion

**Threat**: `os.homedir()` returns unexpected value

**Mitigation**:
- Node.js `os.homedir()` is documented to return valid home directory or throw
- Add validation to reject invalid values (/, empty string)
- Fail fast with clear error

**Risk**: VERY LOW (Node.js API guarantees + validation)

### 4. Repository Name Injection

**Threat**: Git remote URL contains path traversal characters

**Mitigation**:
- Sanitize repository name: remove /, \, :, *, ?, ", <, >, |
- Use only last path segment from URL
- Limit length to 255 characters
- Defense in depth

**Risk**: LOW (URL structure makes injection unlikely + sanitization)

### 5. Race Conditions (TOCTOU)

**Threat**: Path changes between validation and use

**Mitigation**:
- Git validates paths before writing
- Tool runs with user permissions (can't escalate)
- Directory creation doesn't follow symlinks in last component
- Not designed for root usage

**Risk**: VERY LOW (requires file system access + limited impact)

### 6. Configuration File Trust

**Threat**: Malicious config in untrusted repository

**Mitigation**:
- User responsibility (same as package.json scripts)
- No automatic execution (user must run crewchief commands)
- Clear error messages show what path would be used
- Industry-standard trust model

**Risk**: ACCEPTABLE (same as npm scripts, webpack, etc.)

## Security Checklist

- [x] No hardcoded secrets
- [x] Input validation on config paths
- [x] Proper error handling (helpful messages without info leakage)
- [x] Dependencies up to date (no new dependencies added)
- [x] No SQL injection (no database)
- [x] No XSS vulnerabilities (not a web app)
- [x] Path traversal prevention
- [x] Symlink safety (existing checks)
- [x] System directory protection

## Security Testing

**Validation Tests** (Phase 1):
```typescript
describe('Security: Path Validation', () => {
  it('rejects system directories', async () => {
    await expect(expandWorktreePath('/', '/pwd'))
      .rejects.toThrow('system directory')
  })

  it('sanitizes repository name', async () => {
    // Mock git remote with path separators
    const name = await getRepositoryName()
    expect(name).not.toContain('/')
    expect(name).not.toContain('\\')
  })
})
```

**Manual Security Testing**:
- Try creating worktrees in `/`, `/etc`, `/usr`
- Try config with `../../../../../../etc` traversal
- Try repository name with special characters
- Verify error messages don't leak sensitive information

## User-Facing Security Documentation

Add to README:

```markdown
## Security Considerations

CrewChief executes JavaScript config files with your user permissions. Only use config files from trusted sources.

Worktree paths are validated to prevent accidental system directory operations. The tool refuses to create or delete worktrees in system directories like `/`, `/etc`, `/usr`, or `C:\Windows`.

CrewChief uses standard git worktree commands which provide additional path validation.
```

## Conclusion

**Recommendation**: Ship without meaningful security concerns

This project maintains existing security properties while adding configurable paths:
- Users control configuration (not external input)
- Multiple layers of validation (tool + git)
- Standard trust model for developer tools
- Existing safety checks preserved
- No new attack surfaces introduced

**Action Items**:
- Implement path validation as specified
- Add security tests for edge cases
- Document security considerations in README
