# Ticket: MCP_CORE-1002: Open Tool Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the Open tool for the MCP server, providing file reading capabilities with git integration for historical versions, line range extraction, and comprehensive error handling.

## Background
The Open tool is a foundational MCP tool that enables clients to retrieve file contents from the current worktree or from historical git commits. This tool supports reading entire files or specific line ranges, and intelligently determines whether to read from the filesystem (for checked-out commits) or from git history (for non-checked-out commits).

This is part of Phase 1, Week 1 of the MCP_CORE project, focusing on core tool implementations that provide essential functionality for code navigation and retrieval.

## Acceptance Criteria
- [ ] File reading logic correctly reads files from filesystem
- [ ] Git integration retrieves files from historical commits using `git show`
- [ ] Range extraction accurately returns specific line ranges when requested
- [ ] Error handling covers all edge cases (file not found, invalid ranges, git errors, path traversal)
- [ ] Commit checkout detection works correctly
- [ ] Tool validates all parameters using Zod schema
- [ ] Unit tests cover all functionality with >80% coverage
- [ ] Integration tests verify git history retrieval

## Technical Requirements
- **Parameters** (Zod schema):
  - `relpath` (required): Relative path to file from repository root
  - `range` (optional): Object with `start` and `end` line numbers
  - `worktree` (optional): Worktree identifier for multi-worktree support
  - `commit` (optional): Git commit SHA to retrieve file from
- **Return Type**: `FileContent` object with:
  - `content`: File contents as string
  - `relpath`: Echo back the relative path
  - `range`: Echo back the range if provided
- **Git Integration**:
  - Check if commit is currently checked out before reading
  - Use filesystem read for checked-out commits (performance)
  - Use `git show <commit>:<relpath>` for historical commits
- **Range Extraction**:
  - Support inclusive line ranges (e.g., lines 10-20)
  - Handle edge cases (start > end, out of bounds)
- **Security**:
  - Prevent path traversal attacks (e.g., `../../etc/passwd`)
  - Validate file paths are within repository boundaries
  - Enforce max file size limit from config (1MB default)

## Implementation Notes
### Architecture Reference
See `/workspace/crewchief_context/maproom/MCP_CORE/MCP_CORE_ARCHITECTURE.md` lines 81-104 for the Open Tool specification and lines 166-192 for error handling patterns.

### Key Implementation Details
1. **Commit Detection**:
   ```typescript
   async isCommitCheckedOut(commit?: string): Promise<boolean> {
     if (!commit) return true;
     const currentCommit = await execGit(['rev-parse', 'HEAD']);
     return currentCommit.trim() === commit.trim();
   }
   ```

2. **Git File Retrieval**:
   ```typescript
   async getFileFromGit(commit: string, relpath: string): Promise<string> {
     return await execGit(['show', `${commit}:${relpath}`]);
   }
   ```

3. **Range Extraction**:
   ```typescript
   extractRange(content: string, start: number, end: number): string {
     const lines = content.split('\n');
     if (start < 1 || end > lines.length || start > end) {
       throw new ValidationError('Invalid range');
     }
     return lines.slice(start - 1, end).join('\n');
   }
   ```

4. **Error Handling**:
   - File not found → `FILE_NOT_FOUND` error
   - Invalid range → `INVALID_RANGE` error
   - Git errors → `GIT_ERROR` error with details
   - Path traversal → `INVALID_PATH` error
   - File too large → `FILE_TOO_LARGE` error

### Testing Strategy
- Unit tests for each function (commit detection, range extraction, path validation)
- Mock filesystem and git operations for isolated testing
- Integration tests with real git repository
- Edge case testing (empty files, single-line files, binary files)

## Dependencies
- None (foundational tool, no prerequisites)

## Risk Assessment
- **Risk**: Path traversal vulnerabilities allowing access to files outside repository
  - **Mitigation**: Strict path validation, normalize paths, check resolved path is within repo root
- **Risk**: Performance issues with large files
  - **Mitigation**: Enforce max file size limit, consider streaming for large files in future
- **Risk**: Git operations may be slow for large repositories
  - **Mitigation**: Use `isCommitCheckedOut` check to prefer filesystem reads when possible
- **Risk**: Binary files may cause encoding issues
  - **Mitigation**: Detect binary files and return appropriate error or handle encoding explicitly

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/open.ts` - Main Open tool handler implementation
- `packages/maproom-mcp/src/tools/open_schema.ts` - Zod schema for parameter validation
- `packages/maproom-mcp/src/utils/git.ts` - Git utility functions (create if doesn't exist)
- `packages/maproom-mcp/src/utils/validation.ts` - Path validation utilities (create if doesn't exist)
- `packages/maproom-mcp/src/types.ts` - Type definitions for FileContent and OpenParams
- `packages/maproom-mcp/tests/tools/open_test.ts` - Unit tests for Open tool
- `packages/maproom-mcp/tests/tools/open.int.test.ts` - Integration tests with git
