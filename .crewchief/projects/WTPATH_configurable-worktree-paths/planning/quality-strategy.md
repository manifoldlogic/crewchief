# Quality Strategy: Configurable Worktree Paths

## Testing Philosophy

**Test for confidence, not coverage**: Focus on critical paths and edge cases that prevent real bugs. High-value tests over ceremonial coverage metrics.

**MVP Pragmatism**:
- Unit tests for path expansion (pure functions, easy to test thoroughly)
- Integration tests for worktree creation with expanded paths
- Minimal manual testing for platform verification
- No Windows-specific automated tests (rely on Node.js `path` module correctness)

## Test Types

### Unit Tests

**Scope**: Path expansion utility functions in isolation

**Tools**: Vitest (existing test framework)

**Coverage Target**: 100% line coverage for `utils/paths.ts` (small, critical utility)

**Test File**: `packages/cli/src/utils/__tests__/paths.test.ts`

**Critical Test Cases**:

1. **Tilde Expansion**:
   - `~` expands to home directory
   - `~/path` expands to `$HOME/path`
   - Absolute paths unchanged
   - Relative paths unchanged
   - Invalid home directory throws

2. **Repository Name Detection**:
   - Extract from `git@github.com:org/repo.git`
   - Extract from `https://github.com/org/repo.git`
   - Extract from URLs without `.git` suffix
   - Fallback to directory basename when git fails
   - Sanitize special characters (/, \, :, etc.)

3. **Placeholder Expansion**:
   - Single `<repo-name>` replaced
   - Multiple `<repo-name>` replaced
   - No placeholder returns unchanged
   - Placeholder with no git repo uses directory name

4. **Full Path Expansion**:
   - Chain tilde + placeholder + resolution
   - Relative paths resolve against cwd
   - Absolute paths remain absolute
   - System directories rejected (/, /etc, /usr)
   - Unresolved placeholder throws with helpful message

### Integration Tests

**Scope**: Worktree creation with real path expansion

**Approach**: Create actual worktrees in temp directories with various path configurations

**Test File**: `packages/cli/src/git/__tests__/worktrees.integration.test.ts`

**Critical Paths**:

1. **Create worktree with tilde path**:
   - Config: `~/test-worktrees`
   - Verify: worktree created in home directory
   - Cleanup: remove worktree after test

2. **Create worktree with repo placeholder**:
   - Config: `/tmp/<repo-name>-wt`
   - Verify: placeholder replaced with actual repo name
   - Cleanup: remove worktree after test

3. **Backward compatibility**:
   - Config: `.crewchief/worktrees` (old default)
   - Verify: worktree created relative to repo root
   - Cleanup: remove worktree after test

### End-to-End Tests

**Scope**: Not required for this project

**Rationale**: Worktree creation is already tested in integration tests. CLI commands delegate to WorktreeService which is fully tested.

## Critical Paths

The following paths MUST be tested:

1. **Default new user workflow**: No config, worktree created in `~/.crewchief/worktrees/<repo-name>/`
2. **Backward compatibility**: Config with `.crewchief/worktrees` still works
3. **Custom absolute path**: Config with `/custom/path` works without cwd join
4. **Tilde expansion**: Config with `~/custom` expands correctly
5. **Repository isolation**: Multiple repos don't conflict in shared directory
6. **Error handling**: Invalid paths produce helpful errors

## Test Data Strategy

**Mocking**:
- Mock `os.homedir()` for predictable home directory in tests
- Mock `simpleGit` for repository name detection without requiring real git remotes
- Use temp directories for integration tests (cleanup after each test)

**Fixtures**:
- No fixtures needed - path expansion is deterministic given inputs

## Quality Gates

Before verification:
- [ ] All unit tests pass
- [ ] Path expansion utilities have 100% line coverage
- [ ] Integration tests pass
- [ ] No TypeScript errors
- [ ] Linting passes (`pnpm lint`)
- [ ] Formatting passes (`pnpm format`)
- [ ] Existing worktree tests pass with updated mocks
- [ ] Manual testing checklist completed (see below)

## Manual Testing Checklist

**Pre-Merge**:

1. **Fresh install with new default**:
   ```bash
   rm crewchief.config.js
   crewchief worktree create test-feature
   ls -la ~/.crewchief/worktrees/
   ```
   Expected: worktree in `~/.crewchief/worktrees/crewchief/test-feature/`

2. **Backward compatibility**:
   ```javascript
   // crewchief.config.js
   export default {
     repository: { worktreeBasePath: '.crewchief/worktrees' }
   }
   ```
   ```bash
   crewchief worktree create test-old
   ```
   Expected: worktree in `.crewchief/worktrees/test-old/`

3. **Custom absolute path**:
   ```javascript
   export default {
     repository: { worktreeBasePath: '/tmp/my-wt/<repo-name>' }
   }
   ```
   ```bash
   crewchief worktree create test-custom
   ```
   Expected: worktree in `/tmp/my-wt/crewchief/test-custom/`

4. **Error handling**:
   ```javascript
   export default {
     repository: { worktreeBasePath: '/etc' }
   }
   ```
   ```bash
   crewchief worktree create test-error
   ```
   Expected: Clear error about system directory

**Post-Merge**:
- Monitor GitHub issues for path-related problems
- Check CI for cross-platform failures
