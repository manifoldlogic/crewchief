# Ticket: WTSRCH-1001: Implement Git Branch Detection with Caching

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add git branch detection capability to existing git utilities in packages/maproom-mcp. This foundational ticket implements `getCurrentBranch()` function with LRU caching to minimize git subprocess overhead.

## Background
Currently, Maproom search returns results from all indexed worktrees, causing massive duplication (15+ copies of the same code from different branches). Users expect search to find code in their current working context. This ticket provides the foundation for auto-detecting which worktree the user is currently working in.

The solution follows the existing pattern in `packages/maproom-mcp/src/utils/git.ts` which uses `execa` for safe subprocess execution.

This ticket implements Phase 1 of the WTSRCH project as defined in `.crewchief/projects/WTSRCH_worktree-scoped-search/planning/plan.md`.

## Acceptance Criteria
- [x] `lru-cache` dependency installed in package.json (version ^10.0.0 or latest stable)
- [x] `getCurrentBranch(cwd?: string): Promise<string>` function implemented in `src/utils/git.ts`
- [x] Function returns correct branch name when in git repository
- [x] Function handles detached HEAD state gracefully (returns commit SHA or throws clear error)
- [x] Function throws clear error when not in git repository
- [x] LRU cache implemented with 60-second TTL to reduce git subprocess calls
- [x] Cache demonstrates >95% hit rate in tests (multiple calls within TTL window)
- [x] Unit tests pass for all scenarios: normal branch, detached HEAD, non-git directory, cache behavior

## Technical Requirements

### 1. Dependency Installation
```bash
cd packages/maproom-mcp
pnpm add lru-cache
```

### 2. Function Signature
```typescript
export async function getCurrentBranch(cwd?: string): Promise<string>
```

### 3. Implementation Pattern
- Use existing `execGit()` helper from same file
- Git command: `git rev-parse --abbrev-ref HEAD`
- Follow existing error handling pattern (try/catch with clear error messages)
- Cache key: absolute path of `cwd` (default to `process.cwd()`)
- Cache structure: `LRUCache<string, string>` with max 100 entries, 60s TTL

### 4. Edge Cases
- **Detached HEAD**: `git rev-parse --abbrev-ref HEAD` returns "HEAD" - handle this case
- **Not in git repo**: git command fails - throw clear error "Not in a git repository"
- **Invalid cwd**: Let git command fail naturally with its error

### 5. Cache Configuration
```typescript
const branchCache = new LRUCache<string, string>({
  max: 100,        // Max 100 different directories cached
  ttl: 60_000,     // 60 second TTL
})
```

## Implementation Notes

### File: `packages/maproom-mcp/src/utils/git.ts`

Add to existing file (which already has `execGit`, `getFileFromGit`, `isCommitCheckedOut`).

**Suggested approach:**
1. Import `LRUCache` from 'lru-cache'
2. Create module-level cache instance
3. Implement `getCurrentBranch()` following existing patterns:
   - Check cache first
   - On cache miss, call `execGit(['rev-parse', '--abbrev-ref', 'HEAD'], cwd)`
   - Trim whitespace from result
   - Store in cache before returning
   - Proper error handling with descriptive messages

### Test File: `packages/maproom-mcp/tests/unit/git.test.ts` (new file)

Use Vitest framework (not Jest). Test cases:
1. Returns branch name in normal git repo
2. Handles detached HEAD state
3. Throws error when not in git repo
4. Cache reduces subprocess calls (mock `execGit`, verify called once for multiple `getCurrentBranch` calls)
5. Cache expires after TTL (use `vi.useFakeTimers()`)

**Reference:** Examine existing git utilities pattern in `packages/maproom-mcp/src/utils/git.ts` for consistent error handling and async patterns.

## Dependencies
- None (first ticket in Phase 1)

## Risk Assessment
- **Risk**: Git command behavior varies across platforms (Linux/macOS/Windows)
  - **Mitigation**: Use well-established git command, test on Linux + macOS minimum
- **Risk**: Cache staleness if user switches branches within 60s window
  - **Mitigation**: Acceptable trade-off, users can pass explicit worktree parameter

## Files/Packages Affected
- `packages/maproom-mcp/package.json` - Add lru-cache dependency
- `packages/maproom-mcp/src/utils/git.ts` - Add getCurrentBranch() function and cache
- `packages/maproom-mcp/tests/unit/git.test.ts` - NEW: Unit tests

## Documentation References
- Existing git utilities pattern: `packages/maproom-mcp/src/utils/git.ts`
- LRU cache docs: https://github.com/isaacs/node-lru-cache
- Git rev-parse docs: https://git-scm.com/docs/git-rev-parse
- Project architecture: `.crewchief/projects/WTSRCH_worktree-scoped-search/planning/architecture.md`
