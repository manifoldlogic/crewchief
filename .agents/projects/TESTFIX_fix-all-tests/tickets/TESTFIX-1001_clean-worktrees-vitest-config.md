# Ticket: TESTFIX-1001: Clean Stale Worktrees and Configure Vitest

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Remove stale test worktree directories that cause duplicate test discovery and create a vitest.config.ts for the CLI package to explicitly exclude `.crewchief` directories from test discovery.

## Background
The CLI package at `packages/cli` lacks a `vitest.config.ts` file, causing vitest to use default discovery and find test files in nested `.crewchief/worktrees/` directories. A stale worktree at `packages/cli/.crewchief/worktrees/variant-test-variant-minimal-*` is causing ~30 duplicate test failures. This is Phase 1 of the TESTFIX project - environment cleanup before systematic test fixes.

## Acceptance Criteria
- [ ] Stale worktree directory `packages/cli/.crewchief/worktrees/variant-test-*` is removed
- [ ] `packages/cli/vitest.config.ts` exists with explicit exclude patterns
- [ ] Running `pnpm test` in packages/cli no longer discovers tests from `.crewchief/` directories
- [ ] Vitest test count drops by ~30 (eliminating duplicates from nested worktree)

## Technical Requirements
- Create `packages/cli/vitest.config.ts` with this content:
```typescript
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.test.ts', 'tests/**/*.test.ts'],
    exclude: [
      '**/node_modules/**',
      '**/.crewchief/**',
      '**/dist/**',
    ],
  },
})
```
- Remove `packages/cli/.crewchief/worktrees/variant-test-*` directories
- Verify vitest only discovers tests from `src/` and `tests/` directories

## Implementation Notes
1. First check if `packages/cli/.crewchief/worktrees/` directory exists
2. Remove any `variant-test-*` directories inside it
3. Create the vitest.config.ts file
4. Run `pnpm test --reporter=verbose` to verify test discovery is clean
5. Compare test count before/after to confirm duplicates are eliminated

## Dependencies
- None (this is the first ticket in the project)

## Risk Assessment
- **Risk**: Vitest config may conflict with existing package.json test configuration
  - **Mitigation**: Check package.json for existing vitest configuration; vitest.config.ts takes precedence

- **Risk**: Removing worktrees might affect other tests
  - **Mitigation**: Only remove `variant-test-*` directories which are clearly stale from previous test runs

## Files/Packages Affected
- `packages/cli/vitest.config.ts` (create)
- `packages/cli/.crewchief/worktrees/variant-test-*` (remove)
