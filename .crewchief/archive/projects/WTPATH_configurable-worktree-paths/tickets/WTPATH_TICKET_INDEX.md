# WTPATH Ticket Index

**Project**: Configurable Worktree Paths
**Total Tickets**: 4
**Status**: Ready for execution

## Overview

This project enables flexible worktree path configuration with tilde expansion (`~`), repository name placeholders (`<repo-name>`), and absolute paths. The implementation is phased to minimize risk and enable incremental testing.

## Phases

### Phase 1: Path Expansion Utilities (Foundation)
Create tested utility functions without changing behavior.

**Tickets**:
- **WTPATH-1001**: Path Expansion Utilities
  - Status: Not started
  - Estimated: 2-3 hours
  - Agent: typescript-dev
  - Dependencies: None

### Phase 2: WorktreeService Integration (Core Logic)
Integrate expansion utilities into worktree creation.

**Tickets**:
- **WTPATH-2001**: WorktreeService Integration
  - Status: Not started
  - Estimated: 2-3 hours
  - Agent: typescript-dev
  - Dependencies: WTPATH-1001

### Phase 3: Config Schema and Documentation (Breaking Change)
Change default and communicate to users.

**Tickets**:
- **WTPATH-3001**: Config Schema Update
  - Status: Not started
  - Estimated: 1-2 hours
  - Agent: typescript-dev
  - Dependencies: WTPATH-2001

- **WTPATH-3002**: Documentation and Migration Guide
  - Status: Not started
  - Estimated: 1-2 hours
  - Agent: docs-writer
  - Dependencies: WTPATH-3001 (or parallel)

## Total Effort

**Estimated**: 6-10 hours total
**Phases**: 3
**Breaking Changes**: Yes (Phase 3)

## Execution Order

```
WTPATH-1001 (utilities)
    ↓
WTPATH-2001 (integration)
    ↓
WTPATH-3001 (config) ← → WTPATH-3002 (docs)
```

**Note**: WTPATH-3001 and WTPATH-3002 can be executed in parallel or sequentially.

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| Phase-based numbering | Clear dependency tracking (1xxx → 2xxx → 3xxx) |
| Pure functions first | Test utilities in isolation before integration |
| Breaking change last | Only change default after capability proven |
| Comprehensive docs | Breaking change requires clear migration guide |

## Success Criteria

- [ ] All 4 tickets completed and verified
- [ ] Path expansion utilities have 100% line coverage
- [ ] All existing tests pass
- [ ] Integration tests validate real worktree creation
- [ ] Documentation covers migration and troubleshooting
- [ ] Breaking change clearly communicated

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Path expansion breaks on Windows | Use Node.js `path` module; test Windows scenarios |
| Repository name detection fails | Fallback to directory basename always works |
| Users confused by breaking change | Comprehensive migration guide in prominent location |
| Old worktrees orphaned | Document that they continue to work; provide migration steps |

## Planning References

- **Plan**: `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/plan.md`
- **Architecture**: `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/architecture.md`
- **Quality Strategy**: `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/quality-strategy.md`
- **Project Review**: `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/project-review.md`

## Ticket Summaries

### WTPATH-1001: Path Expansion Utilities
Create reusable path expansion functions with comprehensive test coverage. Implements tilde expansion, repository name extraction, placeholder replacement, and full path expansion.

**Key Files**:
- `packages/cli/src/utils/paths.ts` (new)
- `packages/cli/src/utils/__tests__/paths.test.ts` (new)

**Key Requirements**:
- 100% line coverage
- Repository name regex patterns specified
- Error handling with clear messages
- Windows-specific test cases

### WTPATH-2001: WorktreeService Integration
Integrate path expansion into WorktreeService.createWorktree() method. Add integration tests that create real worktrees with expanded paths.

**Key Files**:
- `packages/cli/src/git/worktrees.ts` (modify)
- `packages/cli/src/cli/__tests__/worktree-create.test.ts` (add mocks)
- `packages/cli/src/git/__tests__/worktrees.integration.test.ts` (new)

**Key Requirements**:
- Backward compatibility maintained
- Integration tests for all path types
- Error messages include expanded paths

### WTPATH-3001: Config Schema Update
Change default from `.crewchief/worktrees` to `~/.crewchief/worktrees/<repo-name>`. Update example configs and test mocks.

**Key Files**:
- `packages/cli/src/config/schema.ts` (modify default)
- `crewchief.config.example.js` (add documentation)
- All test mocks (update defaults)

**Key Requirements**:
- JSDoc explains breaking change
- Example configs show all patterns
- All tests pass with new default

### WTPATH-3002: Documentation and Migration Guide
Create comprehensive documentation including migration guide, path patterns explanation, examples, and troubleshooting.

**Key Files**:
- `packages/cli/README.md` (add sections)

**Key Requirements**:
- Migration guide covers all options
- Troubleshooting addresses project-review.md concerns
- Examples are concrete and copy-pasteable
- Repository rename behavior documented

## Notes

### Addressing Project Review Recommendations

All recommendations from `project-review.md` have been incorporated:

1. **Repository name extraction spec**: Added explicit regex patterns and logic to WTPATH-1001
2. **Error handling strategy**: Specified timeout, fallback behavior, and error messages in WTPATH-1001
3. **Windows test cases**: Added to acceptance criteria for WTPATH-1001

### Breaking Change Communication

The breaking change (new default path) is communicated through:
- WTPATH-3001: Config schema JSDoc and example comments
- WTPATH-3002: Prominent migration guide in README
- Release notes: (to be added when releasing)

### Test Strategy

- **Unit tests**: Pure function testing with mocks (Phase 1)
- **Integration tests**: Real worktree creation in temp directories (Phase 2)
- **Manual testing**: Platform verification checklist (quality-strategy.md)

### Version Note

This is a **breaking change** suitable for a major version bump (e.g., v1.x → v2.0).
