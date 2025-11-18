# Ticket: OPNFIX-2001: Add Symlink Validation to File Reading

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement symlink detection and validation in readFileFromFilesystem() to ensure symlink targets stay within repository boundaries. Allows symlinks within the repository but rejects those pointing outside.

## Background
The current implementation follows symlinks automatically via `fs.readFile()`, creating a security risk where a symlink in the repository could point to sensitive files like `/etc/passwd` or other locations outside the repository boundary.

This security threat was identified in `.agents/projects/OPNFIX_open-path-fix/planning/security-review.md` as **Threat 3: Symlink Escape**. While symlinks within a repository are legitimate use cases (e.g., linking to shared configuration files), symlinks that escape repository boundaries must be blocked.

This ticket implements **Security Improvement 3: Symlink Detection** from the security review, adding defense-in-depth protection against symlink-based path traversal attacks.

**Reference:** OPNFIX Phase 2: Security Enhancements (security-review.md lines 241-286)

## Acceptance Criteria
- [x] Symlinks are detected before file reading (using fs.lstat)
- [x] Symlink targets are resolved to absolute paths (using fs.realpath)
- [x] Targets are validated against repository root (validateWithinRepo)
- [x] Symlinks within repository are allowed and read successfully
- [x] Symlinks outside repository throw ValidationError with clear message
- [x] Debug logs show symlink detection and resolution
- [x] No performance impact for non-symlink files (lstat is fast)

## Technical Requirements
- Use `fs.lstat()` to get stats without following symlinks
- Check `stats.isSymbolicLink()` to detect symlinks
- Use `fs.realpath()` to resolve symlink target to absolute path
- Use existing `validateWithinRepo(path, repoRoot)` from utils/validation.ts
- Error message format: "Symlink target escapes repository boundary"
- Maintain existing function signature for backward compatibility
- Follow existing error handling patterns (ValidationError)
- Add debug logging for symlink detection and validation

## Implementation Notes

### Current Code Location
File: `packages/maproom-mcp/src/tools/open.ts`
Function: `readFileFromFilesystem()`
Lines: 96-129

### Algorithm
1. Construct absolute path: `path.join(worktreePath, relpath)`
2. Validate path within repo (existing validation)
3. **NEW:** Check if path is a symlink using `fs.lstat()`
4. **NEW:** If symlink detected:
   - Resolve target using `fs.realpath()`
   - Validate target with `validateWithinRepo(realPath, worktreePath)`
   - Log debug message with original path and resolved target
5. Check file size (existing validation)
6. Read file content

### Implementation Pattern
```typescript
async function readFileFromFilesystem(
  worktreePath: string,
  relpath: string,
  config: OpenToolConfig
): Promise<string> {
  const absolutePath = path.join(worktreePath, relpath)

  // Validate within repo (existing)
  validateWithinRepo(absolutePath, worktreePath)

  // NEW: Check if it's a symlink
  const stats = await fs.lstat(absolutePath)
  if (stats.isSymbolicLink()) {
    const realPath = await fs.realpath(absolutePath)
    validateWithinRepo(realPath, worktreePath)
    log.debug({ path: absolutePath, target: realPath }, 'Following symlink within repository')
  }

  // Check file size (existing)
  await validateFileSize(absolutePath, config.maxFileSize)

  // Read file (existing)
  const content = await fs.readFile(absolutePath, 'utf8')
  return content
}
```

### Security Considerations
- **Defense in depth**: Validates symlink targets even after path validation
- **Legitimate use cases**: Symlinks within repository are allowed (e.g., shared config files)
- **Attack prevention**: Symlinks escaping repository boundaries are blocked
- **Performance**: `fs.lstat()` is fast; negligible impact for non-symlink files
- **Symlink chains**: `fs.realpath()` resolves nested symlinks automatically

### Error Handling
- Use existing `ValidationError` from `utils/validation.ts`
- Error thrown by `validateWithinRepo()` when symlink target escapes
- Error message will be: "Path escapes repository boundary: [target]"
- No special error handling needed; let existing error propagate

### Logging
- Use existing `log` instance from open.ts
- Log level: `debug` (not warning - symlinks within repo are normal)
- Log format: `{ path: originalPath, target: resolvedPath }`
- Message: "Following symlink within repository"

## Dependencies
- **Requires**: OPNFIX-1001 (path validation infrastructure working)
- **Requires**: Existing `validateWithinRepo()` function in utils/validation.ts
- **Blocks**: OPNFIX-3002 (security test suite needs this implementation)

## Risk Assessment
- **Risk**: Breaking existing workflows that use symlinks within repository
  - **Mitigation**: Allow symlinks within repository boundaries; only reject escaping symlinks
- **Risk**: Performance impact from extra filesystem calls
  - **Mitigation**: `fs.lstat()` is fast (~1ms); only adds `fs.realpath()` for actual symlinks
- **Risk**: Nested symlink chains could cause issues
  - **Mitigation**: `fs.realpath()` handles nested symlinks correctly; validate final target only

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/open.ts` - readFileFromFilesystem function (lines 96-129)
- `packages/maproom-mcp/src/utils/validation.ts` - import validateWithinRepo (already imported)

## Testing Notes

### Manual Testing Scenarios
1. **Normal file** (no symlink) - Should work unchanged
2. **Symlink within repo** - Should log debug message and read successfully
3. **Symlink to /etc/passwd** - Should throw ValidationError
4. **Symlink to ../../../etc** - Should throw ValidationError
5. **Nested symlinks** (symlink → symlink → file) - Should validate final target

### Unit Test Requirements
Unit tests for these scenarios should be created in Phase 3 (OPNFIX-3002).
This ticket focuses on implementation; comprehensive security testing is separate.
