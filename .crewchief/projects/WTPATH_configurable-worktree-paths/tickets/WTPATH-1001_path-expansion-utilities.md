# Ticket: [WTPATH-1001]: Path Expansion Utilities

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-dev
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create reusable path expansion utility functions with comprehensive test coverage for tilde expansion, repository name extraction, placeholder replacement, and full path expansion.

## Background
The current worktree system only supports relative paths (e.g., `.crewchief/worktrees`). To enable absolute paths with tilde (`~`) and repository name placeholder (`<repo-name>`), we need tested path expansion utilities that handle all edge cases before integrating into the WorktreeService.

This ticket implements Phase 1 of the Configurable Worktree Paths project, creating pure functions that can be tested in isolation without changing any existing behavior.

**Planning References:**
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/plan.md` (Phase 1)
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/architecture.md` (Path Expansion Utility section)

## Acceptance Criteria
- [x] `expandTilde('~')` returns home directory
- [x] `expandTilde('~/foo')` returns `$HOME/foo`
- [x] `expandTilde('/abs/path')` returns path unchanged
- [x] `getRepositoryName()` extracts name from `git@github.com:org/repo.git` format
- [x] `getRepositoryName()` extracts name from `https://github.com/org/repo.git` format
- [x] `getRepositoryName()` extracts name from URLs without `.git` suffix
- [x] `getRepositoryName()` falls back to directory basename when git command fails
- [x] `getRepositoryName()` sanitizes special characters (/, \, :, *, ?, ", <, >, |)
- [x] `expandRepoPlaceholder()` replaces all `<repo-name>` occurrences
- [x] `expandWorktreePath()` chains all expansions correctly
- [x] System directories (/, /etc, /usr) are rejected with clear error messages
- [x] Git operations timeout after 5 seconds and fall back to directory basename
- [x] Error messages include: (1) rejected path, (2) reason, (3) example valid path
- [x] All unit tests pass with 100% line coverage for paths.ts
- [x] Tests include Windows-specific scenarios (USERPROFILE, C:\\ paths)

## Technical Requirements

### File Creation
Create new file: `/workspace/packages/cli/src/utils/paths.ts`

### Function Signatures
```typescript
export function expandTilde(pathStr: string): string
export async function getRepositoryName(cwd?: string): Promise<string>
export async function expandRepoPlaceholder(pathStr: string, cwd?: string): Promise<string>
export async function expandWorktreePath(pathStr: string, cwd?: string): Promise<string>
```

### Repository Name Extraction Logic
Per project-review.md recommendations, implement:
```typescript
// Use simple-git library with timeout (already in codebase)
const git = simpleGit({ timeout: { block: 5000 } })
const output = await git.raw(['config', '--get', 'remote.origin.url'])

// Regex patterns for URL parsing:
// git@github.com:org/repo.git → "repo"
// https://github.com/org/repo.git → "repo"
// https://github.com/org/repo → "repo"
const regex = /[/:]([^/:]+?)(\.git)?$/
const match = output.match(regex)
const repoName = match ? match[1] : path.basename(cwd || process.cwd())

// Sanitization: Replace dangerous characters with hyphens
return repoName.replace(/[/\\:*?"<>|]/g, '-').slice(0, 255)
```

**Timeout Implementation:** Configure simple-git with 5 second timeout using `{ timeout: { block: 5000 } }` option. If git operation exceeds timeout or fails, fall back to directory basename gracefully.

### Error Handling Strategy
```typescript
// Git command errors → fall back to directory basename (graceful degradation)
// Timeout: 5 seconds for git operations
// Non-git directories: use directory basename (don't throw)
// Empty sanitized names: throw with clear error message
// System directories: throw with clear error message
```

### System Directory Validation
Reject these paths with error:
- `/` (root)
- `/etc`, `/etc/*`
- `/usr`, `/usr/*`
- `/System` (macOS)
- `C:\Windows` (Windows)

### Test Coverage Requirements
Create test file: `/workspace/packages/cli/src/utils/__tests__/paths.test.ts`

Test cases must cover:
1. Tilde expansion (various formats)
2. Repository name extraction (both git formats + fallback)
3. Placeholder replacement (single, multiple, none)
4. Full path expansion chain
5. Error cases (system directories, invalid input)
6. Windows-specific paths (if process.platform === 'win32')

## Implementation Notes

### Port from Rust Implementation
Reference existing tilde expansion in `/workspace/crates/maproom/src/db/connection.rs:25-39` for patterns, but implement in TypeScript using Node.js APIs.

### Use Node.js Built-in APIs
- `os.homedir()` for home directory (cross-platform)
- `path.resolve()` for absolute path resolution
- `path.basename()` for directory name fallback

### Use Existing Libraries
- `simple-git` library (already in codebase) for git operations
- No new dependencies required

### Incremental Design
Each function does one thing:
- `expandTilde()` - only handles tilde
- `getRepositoryName()` - only extracts repo name
- `expandRepoPlaceholder()` - only replaces placeholder
- `expandWorktreePath()` - orchestrates all expansions

### Example Full Expansion Flow
```typescript
// Input: "~/.crewchief/worktrees/<repo-name>"

// Step 1: expandTilde
// → "/home/user/.crewchief/worktrees/<repo-name>"

// Step 2: expandRepoPlaceholder (calls getRepositoryName)
// → "/home/user/.crewchief/worktrees/crewchief"

// Step 3: path.resolve (make absolute)
// → "/home/user/.crewchief/worktrees/crewchief"

// Step 4: validate (not a system directory)
// → return "/home/user/.crewchief/worktrees/crewchief"
```

## Dependencies
- None (Phase 1 is foundational)

## Risk Assessment
- **Risk**: Repository name extraction regex doesn't cover all git URL formats
  - **Mitigation**: Fallback to directory basename always works; comprehensive test cases validate common formats

- **Risk**: Sanitization results in empty or invalid names
  - **Mitigation**: Throw clear error with example of valid format; fallback to directory basename when possible

- **Risk**: Windows path handling differs from Unix
  - **Mitigation**: Use `path.resolve()` which handles platform differences; test on Windows if available

## Files/Packages Affected
- `/workspace/packages/cli/src/utils/paths.ts` (new file)
- `/workspace/packages/cli/src/utils/__tests__/paths.test.ts` (new file)

## Verification Notes

Verify-ticket agent should check:
- [ ] All acceptance criteria checkboxes are met
- [ ] Test file exists and contains comprehensive test cases
- [ ] Test output shows all tests passing
- [ ] Code coverage for paths.ts is 100%
- [ ] Error messages are clear and actionable
- [ ] TypeScript types are correct (no `any` types)
- [ ] Functions handle async properly (getRepositoryName)
- [ ] Edge cases tested (empty strings, special characters, Windows paths)
