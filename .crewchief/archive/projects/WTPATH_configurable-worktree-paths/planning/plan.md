# Plan: Configurable Worktree Paths

## Overview

Phased approach to minimize risk and enable incremental testing:

1. **Phase 1**: Path expansion utilities (pure functions, easy to test)
2. **Phase 2**: WorktreeService integration (core logic change)
3. **Phase 3**: Config schema update and documentation (breaking change rollout)

## Phases

### Phase 1: Path Expansion Utilities

**Objective**: Create tested, reusable path expansion functions without changing any behavior

**Deliverables**:
- New file `packages/cli/src/utils/paths.ts` with expansion functions
- Test file `packages/cli/src/utils/__tests__/paths.test.ts` with comprehensive coverage
- All edge cases handled (no git repo, special characters, system paths)

**Agent Assignments**:
- typescript-dev: Implement `expandTilde`, `getRepositoryName`, `expandRepoPlaceholder`, `expandWorktreePath`
- unit-test-runner: Write and execute tests for path utilities
- verify-ticket: Verify all tests pass and edge cases covered

**Acceptance Criteria**:
- [ ] `expandTilde('~')` returns home directory
- [ ] `expandTilde('~/foo')` returns `$HOME/foo`
- [ ] `expandTilde('/abs/path')` returns path unchanged
- [ ] `getRepositoryName()` extracts name from git remote URL (both `git@` and `https://`)
- [ ] `getRepositoryName()` falls back to directory basename
- [ ] `expandRepoPlaceholder()` replaces all `<repo-name>` occurrences
- [ ] `expandWorktreePath()` chains all expansions correctly
- [ ] System directories (/, /etc, /usr) are rejected with clear error
- [ ] Repo names are sanitized (remove /, \, :, etc.)
- [ ] All tests pass

**Estimated Effort**: 2-3 hours

**Dependencies**: None

---

### Phase 2: WorktreeService Integration

**Objective**: Use path expansion in worktree creation without changing default config

**Deliverables**:
- Update `packages/cli/src/git/worktrees.ts` to use `expandWorktreePath()`
- Update `packages/cli/src/cli/__tests__/worktree-create.test.ts` with mocked expansion
- Add integration test for expanded paths
- Verify backward compatibility with relative paths

**Agent Assignments**:
- typescript-dev: Modify WorktreeService.createWorktree() to call expansion utility
- unit-test-runner: Update mocks and add integration tests
- verify-ticket: Verify worktrees create correctly with expansion

**Acceptance Criteria**:
- [ ] WorktreeService calls `expandWorktreePath()` before constructing worktree path
- [ ] Relative paths still work (`.crewchief/worktrees`)
- [ ] Absolute paths work without joining to cwd
- [ ] Tilde paths expand correctly
- [ ] `<repo-name>` placeholder expands correctly
- [ ] Existing worktree tests pass with mocked expansion
- [ ] Integration test creates real worktree with expanded path

**Estimated Effort**: 2-3 hours

**Dependencies**: Phase 1 complete

---

### Phase 3: Config Schema and Documentation

**Objective**: Change default and communicate breaking change to users

**Deliverables**:
- Update `packages/cli/src/config/schema.ts` default to `~/.crewchief/worktrees/<repo-name>`
- Update example configs with comments about new default
- Update `packages/cli/README.md` with migration guide
- Update tests to use new default

**Agent Assignments**:
- typescript-dev: Update config schema default
- docs-writer: Update README, example configs, migration guide
- verify-ticket: Verify documentation is clear and complete

**Acceptance Criteria**:
- [ ] Config schema default is `~/.crewchief/worktrees/<repo-name>`
- [ ] `crewchief.config.example.js` documents new default with comment
- [ ] `crewchief.config.js` documents new default with comment
- [ ] `.devcontainer/scripts/post-create.sh` uses new default
- [ ] README includes migration section
- [ ] Migration guide covers accepting new default OR reverting to old behavior
- [ ] Tests updated to use new default in mocks
- [ ] All tests pass with new default

**Estimated Effort**: 1-2 hours

**Dependencies**: Phase 2 complete

## Dependencies

**Cross-Phase**:
- Phase 2 depends on Phase 1 (utilities must exist)
- Phase 3 depends on Phase 2 (integration must work before changing default)

**External**:
- None - self-contained changes

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Path expansion breaks on Windows | Low | Medium | Use `path.resolve()` which handles Windows correctly; Node.js abstracts platform differences |
| Users don't see migration guide | Medium | Medium | Document in multiple places (README, config examples, release notes); old behavior is 1-line config change |
| Repository name detection fails | Low | Low | Fallback to directory basename always works |
| Existing worktrees become orphaned | Low | Low | Existing worktrees continue to work; users can keep both locations or manually migrate |
| Breaking change causes user friction | Medium | Medium | Clear migration documentation; easy opt-out to old behavior |

## Success Metrics

- [ ] All acceptance criteria met across phases
- [ ] Path expansion utilities have 100% line coverage
- [ ] All existing tests pass without modification (except mock updates)
- [ ] Migration guide tested manually on fresh install
- [ ] Documentation reviewed for clarity
- [ ] Zero-config workflow improved (worktrees outside repo by default)
