# Ticket: OPNFIX-4001: Update Tool Documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update documentation for the open tool to reflect the new multi-candidate fallback behavior, enhanced error messages, and symlink validation features implemented in Phase 1-2.

## Background
The open tool has been enhanced with significant improvements to path resolution:
- Multi-candidate fallback mechanism for handling database pollution
- Symlink validation for security
- Enhanced error messages with troubleshooting guidance

This ticket implements the documentation requirements from Phase 4, Ticket 4.1 of the OPNFIX project plan. Users and developers need clear documentation to understand the new behavior, interpret error messages, and troubleshoot path resolution issues.

## Acceptance Criteria
- [ ] README.md explains the open tool's multi-candidate fallback behavior
- [ ] README.md documents all error messages and their meanings
- [ ] README.md includes a troubleshooting guide for common path resolution issues
- [ ] JSDoc comments in open.ts are complete and accurate for all modified functions
- [ ] Documentation follows existing formatting and style conventions
- [ ] All new features (symlink validation, path validation) are documented

## Technical Requirements
- Update `packages/maproom-mcp/README.md` with:
  - Description of multi-candidate fallback behavior
  - Error message reference table
  - Troubleshooting guide for path errors
  - Security features (symlink validation)
- Update JSDoc in `packages/maproom-mcp/src/tools/open.ts` for:
  - `getWorktreePath()` function
  - `fileExists()` helper (if added in Phase 1)
  - Any other modified functions
- Documentation should include:
  - When fallback behavior triggers
  - What each error message means
  - How to resolve common issues
  - Security implications of symlink validation

## Implementation Notes
**Documentation Sections to Add:**

1. **README.md - Open Tool Behavior Section:**
   - Explain that open tool tries multiple database candidates in order
   - Document that it returns the first valid worktree path
   - Explain ordering (most recent worktree ID first)
   - Note that this handles database pollution gracefully

2. **README.md - Error Messages Section:**
   - "No worktrees found with name X" - no matching worktrees in database
   - "All N candidates failed validation" - multiple worktrees found but none have valid paths
   - "File Y not found in repository Z" - file doesn't exist in resolved worktree
   - Include recommended actions for each error

3. **README.md - Troubleshooting Section:**
   - How to identify database pollution (error mentions "N candidates")
   - When to run `maproom db cleanup-stale`
   - How to verify worktree paths
   - Common causes of path resolution failures

4. **JSDoc Updates:**
   - Document parameters including new optional ones
   - Document return values and error conditions
   - Include examples of usage
   - Note security validations performed

**Style Guidelines:**
- Use existing README.md formatting
- Keep explanations concise but complete
- Include code examples where helpful
- Use tables for error reference

## Dependencies
- Phase 1 tickets (OPNFIX-1001, OPNFIX-1002, OPNFIX-1003) must be completed
- Phase 2 tickets (OPNFIX-2001, OPNFIX-2002) must be completed
- Code changes must be finalized before documentation

## Risk Assessment
- **Risk**: Documentation may become outdated if implementation details change
  - **Mitigation**: Document behavior, not implementation; review docs after any code changes
- **Risk**: Error messages in docs may not match actual error messages in code
  - **Mitigation**: Copy exact error message strings from code; verify accuracy before commit

## Files/Packages Affected
- `packages/maproom-mcp/README.md`
- `packages/maproom-mcp/src/tools/open.ts` (JSDoc comments only)
