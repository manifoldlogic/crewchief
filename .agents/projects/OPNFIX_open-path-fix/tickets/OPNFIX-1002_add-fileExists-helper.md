# Ticket: OPNFIX-1002: Add fileExists Helper Function to Validation Utils

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create an async fileExists() helper function in validation.ts that checks if a file exists and is readable using fs.access(). Returns boolean instead of throwing errors.

## Background
The getWorktreePath function needs to validate candidate paths against the filesystem to recover from database pollution (multiple worktree entries with inconsistent abs_path values).

Existing validation.ts has validatePath(), validateWithinRepo(), and validateFileSize(), but lacks a simple async boolean check for file existence. This helper enables the multi-candidate fallback strategy implemented in OPNFIX-1001.

This ticket provides the foundational utility for Phase 1 of the OPNFIX project.

## Acceptance Criteria
- [ ] Function correctly detects existing readable files (returns true)
- [ ] Function returns false for non-existent paths
- [ ] Function returns false for inaccessible files (no read permission)
- [ ] Function does NOT throw errors (catches and returns false)
- [ ] JSDoc documentation is complete with examples
- [ ] Function is exported from validation.ts

## Technical Requirements
- Use Node.js fs.promises.access() or fs/promises import
- Use fs.constants.R_OK flag (read permission check)
- Catch ALL errors and return false (ENOENT, EACCES, etc.)
- Keep function pure (no side effects, no logging)
- Type signature: `async function fileExists(path: string): Promise<boolean>`
- Add comprehensive JSDoc with @param and @returns tags
- Export function for use in other modules

## Implementation Notes

### File Location
`packages/maproom-mcp/src/utils/validation.ts`

### Implementation
```typescript
import { access, constants } from 'fs/promises'

/**
 * Check if a file exists and is readable
 *
 * This function performs a non-throwing filesystem check to determine if a file
 * exists and has read permissions. Used for validating candidate paths during
 * worktree path resolution.
 *
 * @param filePath - Absolute path to file to check
 * @returns true if file exists and is readable, false otherwise
 *
 * @example
 * ```typescript
 * if (await fileExists('/path/to/file.ts')) {
 *   // File exists and is readable
 * }
 * ```
 */
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath, constants.R_OK)
    return true
  } catch {
    return false
  }
}
```

### Design Rationale
- **No errors thrown**: Enables simple conditional logic in calling code
- **Read permission check**: Uses R_OK flag to ensure file is actually accessible
- **Pure function**: No side effects, no logging - just a boolean check
- **Async**: Matches Node.js filesystem API patterns

### Integration
This function will be imported and used in:
- `packages/maproom-mcp/src/tools/open.ts` (getWorktreePath function)
- Potentially other tools that need path validation

## Dependencies
- **Requires**: None (independent utility function)
- **Blocks**: OPNFIX-1001 (getWorktreePath needs this function)

## Risk Assessment
- **Risk**: Function could have performance impact if called frequently
  - **Mitigation**: Filesystem checks are fast; only called during path resolution (not hot path)
- **Risk**: Different error codes might need different handling
  - **Mitigation**: For our use case, any error (ENOENT, EACCES, etc.) means "not usable" - return false is correct

## Files/Packages Affected
- `packages/maproom-mcp/src/utils/validation.ts` (add new function)
