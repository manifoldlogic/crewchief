# Ticket: OPNFIX-4002: Add Debug Logging

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
Add comprehensive debug logging to the open tool to help troubleshoot path resolution issues, showing the decision-making process as the tool tries multiple candidates and validates paths.

## Background
The open tool now implements multi-candidate fallback and symlink validation (Phase 1-2). To help developers and users troubleshoot path resolution issues, we need debug logging that shows:
- Each candidate being tried
- Why validation fails for each candidate
- When path resolution succeeds
- Symlink resolution details

This ticket implements Phase 4, Ticket 4.2 of the OPNFIX project plan. Debug logging is critical for diagnosing database pollution issues and understanding why certain paths are rejected.

## Acceptance Criteria
- [ ] Debug log added when trying each candidate path
- [ ] Debug log added for validation failures (with reason)
- [ ] Debug log added for successful path resolution
- [ ] Debug log added for symlink resolution steps
- [ ] No sensitive data (absolute paths outside repo) in logs
- [ ] Logs use appropriate levels (debug/info/warn/error)
- [ ] All logs follow existing logging conventions in the codebase
- [ ] Tests verify logging behavior

## Technical Requirements
- Add debug logging to `packages/maproom-mcp/src/tools/open.ts`:
  - Log each candidate being tried (with index/count)
  - Log validation failure reasons
  - Log successful path resolution
  - Log symlink detection and resolution
- Use appropriate log levels:
  - **debug**: Candidate attempts, validation steps
  - **info**: Successful resolution
  - **warn**: Validation failures, potential issues
  - **error**: Fatal errors, security violations
- Protect sensitive information:
  - Only log relative paths within repo
  - Redact or omit absolute filesystem paths
  - Don't log full database connection strings
- Follow existing logging patterns in maproom-mcp

## Implementation Notes
**Logging Points to Add:**

1. **Starting Path Resolution:**
   ```
   debug: `Resolving path for relpath=${relpath}, worktree=${worktreeName}`
   ```

2. **Multiple Candidates Found:**
   ```
   debug: `Found ${candidates.length} worktree candidates, trying in order`
   ```

3. **Trying Each Candidate:**
   ```
   debug: `Trying candidate ${index + 1}/${total}: worktree_id=${id}, abs_path=${abs_path}`
   ```

4. **Validation Failures:**
   ```
   debug: `Candidate ${index + 1} failed: file does not exist at ${fullPath}`
   debug: `Candidate ${index + 1} failed: path outside repository boundary`
   ```

5. **Symlink Detection:**
   ```
   debug: `Detected symlink at ${relpath}, resolving target`
   debug: `Symlink target: ${targetPath}, validating against repo root`
   ```

6. **Success:**
   ```
   info: `Resolved path successfully: worktree=${worktreeName}, relpath=${relpath}`
   ```

7. **All Candidates Failed:**
   ```
   warn: `All ${count} candidates failed validation for ${worktreeName}/${relpath}`
   ```

**Security Considerations:**
- Never log full absolute paths that might expose system structure
- Log relative paths only
- Consider using path.basename() for filenames in sensitive contexts
- Ensure debug logs can be disabled in production

**Testing:**
- Verify logs appear when debug level is enabled
- Verify logs are suppressed when debug is disabled
- Verify no sensitive data in log output
- Test log output for all code paths (success, failure, symlinks)

## Dependencies
- Phase 1 tickets (OPNFIX-1001, OPNFIX-1002, OPNFIX-1003) must be completed
- Phase 2 tickets (OPNFIX-2001, OPNFIX-2002) must be completed
- Logging infrastructure must be available in maproom-mcp

## Risk Assessment
- **Risk**: Excessive logging may impact performance
  - **Mitigation**: Use debug level for verbose logs; ensure logs can be disabled
- **Risk**: Logs may expose sensitive file system information
  - **Mitigation**: Only log relative paths; review all log messages for security
- **Risk**: Log format changes may break log parsing tools
  - **Mitigation**: Follow existing logging conventions; use structured logging if available

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/open.ts`
- `packages/maproom-mcp/tests/tools/open.e2e.test.ts` (add logging verification)
