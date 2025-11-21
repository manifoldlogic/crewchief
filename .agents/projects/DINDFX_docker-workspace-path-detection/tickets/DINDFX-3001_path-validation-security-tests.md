# Ticket: DINDFX-3001: Add path validation and security tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add minimal path validation to warn about potentially dangerous paths (path traversal, relative paths) and create security test cases to verify mitigations work. Validation warns but doesn't block, as read-only mount is the primary security mitigation.

## Background
After implementing core workspace path detection functionality in Phase 2 (DINDFX-2004), we now add a lightweight security layer. This phase implements Phase 3 of the project plan: minimal validation that warns users about suspicious paths without blocking them, plus security test coverage to verify our mitigations (execFileSync, timeouts, buffer limits, read-only mount) work correctly.

The validation is intentionally minimal (MVP principle) - we warn but don't block because:
- Read-only mount (`:ro`) is the primary security control
- Users may have legitimate reasons for unusual path patterns
- We can't verify path existence from container (host vs container filesystem)

References:
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/plan.md` - Phase 3
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/architecture.md` - Security Considerations
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/security-review.md` - Phase 3 recommendations

## Acceptance Criteria
- [ ] Path validation function added to `resolveWorkspacePath()` in `bin/cli.cjs`
- [ ] Warns (console.warn) if path contains `..` (path traversal pattern)
- [ ] Warns if path doesn't start with `/` (relative path)
- [ ] Validation does NOT block or throw errors (just warns)
- [ ] Validation does NOT verify path exists (can't check host filesystem from container)
- [ ] Security test cases added to `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`
- [ ] Test case: malicious path with `..` triggers warning
- [ ] Test case: relative path triggers warning
- [ ] Test case: execFileSync with special characters in hostname is safe (no shell injection)
- [ ] All new security tests pass
- [ ] Verification: `pnpm test workspace-path-detection` shows all tests passing

## Technical Requirements

### Path Validation Function
Add to `resolveWorkspacePath()` before returning the path:

```javascript
function validateAndWarnPath(path) {
  // Check for path traversal patterns
  if (path.includes('..')) {
    console.warn(`⚠️  Workspace path contains ".." (path traversal risk): ${path}`);
    console.warn('    Proceeding with caution. Read-only mount limits risk.');
  }

  // Warn if relative path (not absolute)
  if (!path.startsWith('/')) {
    console.warn(`⚠️  Workspace path is not absolute: ${path}`);
    console.warn('    May cause unexpected behavior.');
  }

  // Don't verify path exists - container can't see host filesystem
  return path;
}

// Call validateAndWarnPath before returning in resolveWorkspacePath
```

### Security Test Cases
Add to `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`:

```typescript
describe('Security', () => {
  it('should warn about path traversal patterns', () => {
    process.env.WORKSPACE_HOST_PATH = '../../etc';
    const consoleSpy = vi.spyOn(console, 'warn');

    const result = resolveWorkspacePath();

    expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('..'));
    expect(result).toBe('../../etc'); // Doesn't block, just warns
  });

  it('should warn about relative paths', () => {
    process.env.WORKSPACE_HOST_PATH = 'relative/path';
    const consoleSpy = vi.spyOn(console, 'warn');

    const result = resolveWorkspacePath();

    expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('not absolute'));
  });

  it('should safely handle special characters in hostname', () => {
    // Mock hostname with shell metacharacters
    vi.mocked(execFileSync).mockReturnValueOnce('host; rm -rf /');
    vi.mocked(execFileSync).mockReturnValueOnce('/host_mnt/path');

    const result = getWorkspaceHostPath();

    // execFileSync with array args prevents shell injection
    expect(result).toBe('/host_mnt/path');
    expect(execFileSync).toHaveBeenCalledWith('docker', [
      'inspect',
      'host; rm -rf /',  // Passed as argument, not executed
      '--format',
      expect.any(String)
    ], expect.any(Object));
  });
});
```

### Test Execution
```bash
# Run workspace path detection tests
pnpm test workspace-path-detection

# Expected: all tests pass including new security tests
```

## Implementation Notes

**Design Principles:**
- Validation is MINIMAL by design (MVP principle)
- Warn but don't block - user override may have valid reasons for unusual paths
- Read-only mount (`:ro` in docker-compose.yml) is primary security mitigation
- execFileSync already prevents shell injection (implemented in Phase 2)
- Timeouts and buffer limits already configured (implemented in Phase 2)

**Security Layers (Defense in Depth):**
1. **Primary**: Read-only mount prevents writes to host
2. **Secondary**: execFileSync prevents shell injection
3. **Tertiary**: Timeouts and buffer limits prevent DoS
4. **Monitoring**: Path validation warns about suspicious patterns

**Why Not Block?**
- Can't distinguish malicious from legitimate unusual paths
- Container can't verify host filesystem paths
- User may have valid reasons (symlinks, custom mounts)
- Better to warn and let user decide

**Test Coverage:**
- Path traversal warning
- Relative path warning
- Shell injection prevention (verify execFileSync safety)
- All existing tests still pass

## Dependencies
- **DINDFX-2004** must be complete (integration complete, all Phase 2 tests passing)
- Requires existing test infrastructure in `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`

## Risk Assessment
- **Risk**: Over-engineering validation (scope creep)
  - **Mitigation**: Minimal checks only (just `..` and `/`), no blocking behavior

- **Risk**: False positives annoy users
  - **Mitigation**: Warnings not errors, user can proceed, clear messaging explains why

- **Risk**: Missing real vulnerabilities
  - **Mitigation**: execFileSync + read-only mount are primary defenses, validation is monitoring layer

- **Risk**: Tests don't catch real security issues
  - **Mitigation**: Test cases based on security-review.md recommendations, cover shell injection and path traversal

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Add validateAndWarnPath function and integrate into resolveWorkspacePath
- `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts` - Add security test suite
