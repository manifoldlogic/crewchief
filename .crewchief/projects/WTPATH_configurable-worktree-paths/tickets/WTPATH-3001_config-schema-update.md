# Ticket: [WTPATH-3001]: Config Schema Update

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
- typescript-dev
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update the worktreeBasePath config schema default from `.crewchief/worktrees` to `~/.crewchief/worktrees/<repo-name>`, introducing a breaking change that moves worktrees outside the repository by default.

## Background
Phases 1 and 2 implemented path expansion capability. Now we change the default configuration to take advantage of this capability, moving worktrees to a centralized location outside the repository. This is a breaking change that requires careful rollout and documentation.

This ticket implements Phase 3 (Config Schema) of the Configurable Worktree Paths project.

**Planning References:**
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/plan.md` (Phase 3)
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/architecture.md` (Config Schema Update section)

## Acceptance Criteria
- [x] Config schema default is `~/.crewchief/worktrees/<repo-name>`
- [x] Schema validation still allows custom paths
- [x] Example config files document new default with explanatory comments
- [x] Test mocks updated to use new default
- [x] All existing tests pass with new default
- [x] TypeScript types reflect new default value
- [x] Config loading correctly uses new default when no config file present

## Technical Requirements

### Update Config Schema
File: `/workspace/packages/cli/src/config/schema.ts`

**Current code** (approximate):
```typescript
worktreeBasePath: z.string().default('.crewchief/worktrees'),
```

**New code**:
```typescript
worktreeBasePath: z.string().default('~/.crewchief/worktrees/<repo-name>'),
```

Add JSDoc comment explaining the breaking change:
```typescript
/**
 * Base path for worktrees. Supports:
 * - Tilde expansion: ~/worktrees → /home/user/worktrees
 * - Repository placeholder: <repo-name> → actual repo name
 * - Absolute paths: /custom/path
 * - Relative paths: .crewchief/worktrees (legacy)
 *
 * @default '~/.crewchief/worktrees/<repo-name>' (changed from '.crewchief/worktrees' in v1.x)
 */
worktreeBasePath: z.string().default('~/.crewchief/worktrees/<repo-name>'),
```

### Update Example Config Files

**File: `/workspace/crewchief.config.example.js`**

Add comment documenting new default and opt-out:
```javascript
export default {
  repository: {
    // Default (v2.0+): worktrees outside repo
    // worktreeBasePath: '~/.crewchief/worktrees/<repo-name>',

    // Legacy (v1.x): worktrees inside repo
    // worktreeBasePath: '.crewchief/worktrees',

    // Custom: absolute path with repo isolation
    // worktreeBasePath: '/mnt/ssd/worktrees/<repo-name>',
  }
}
```

**File: `/workspace/crewchief.config.js`** (if exists, check first)

Apply same documentation updates.

**File: `/workspace/.devcontainer/scripts/post-create.sh`** (if it sets config)

Verify uses new default or explicitly sets path. Update if needed.

### Update Test Mocks
Files: Any test file that mocks config

Update mocked `worktreeBasePath` to new default:
```typescript
const mockConfig = {
  repository: {
    worktreeBasePath: '~/.crewchief/worktrees/<repo-name>' // updated
  }
}
```

Or mock expansion to return predictable paths for tests.

## Implementation Notes

### Breaking Change Strategy
This is a **breaking change** because:
- New worktrees will be created in different location by default
- Users upgrading will have worktrees in two locations
- Old worktrees continue to work (git manages them), but users might be confused

### Migration Path
Users have two options:
1. **Accept new default**: Do nothing, new worktrees go to new location
2. **Revert to old behavior**: Add config override:
   ```javascript
   export default {
     repository: { worktreeBasePath: '.crewchief/worktrees' }
   }
   ```

### Existing Worktrees Continue Working
Git worktrees are managed by path. Old worktrees in `.crewchief/worktrees/` will continue to function correctly. Users can:
- Keep both locations (old worktrees + new worktrees)
- Manually migrate old worktrees (move directories, update git references)
- Delete old worktrees and recreate in new location

### Test Updates
Some tests may need mock adjustments if they:
- Assert on exact worktree paths
- Check directory structure
- Mock config defaults

Use `expandWorktreePath` mock from Phase 2 to make tests deterministic.

### Version Note
Document in schema that this changed in v2.0 (or appropriate version number).

## Dependencies
- **WTPATH-2001** (WorktreeService Integration) - Must be completed first
- **WTPATH-3002** (Documentation Updates) - Should be completed in parallel or immediately after

## Risk Assessment
- **Risk**: Users don't notice breaking change until they have worktrees in two locations
  - **Mitigation**: Clear documentation in WTPATH-3002; release notes; runtime warning (optional)

- **Risk**: Tests fail due to hardcoded path expectations
  - **Mitigation**: Update test mocks to use new default; use predictable paths in tests

- **Risk**: Users want to migrate existing worktrees but don't know how
  - **Mitigation**: Migration guide in WTPATH-3002 documents manual migration steps

## Files/Packages Affected
- `/workspace/packages/cli/src/config/schema.ts` (modify default)
- `/workspace/crewchief.config.example.js` (add documentation)
- `/workspace/crewchief.config.js` (add documentation if exists)
- `/workspace/.devcontainer/scripts/post-create.sh` (verify/update if needed)
- All test files with config mocks (update mocks)

## Verification Notes

Verify-ticket agent should check:
- [ ] All acceptance criteria checkboxes are met
- [ ] Schema default is exactly `~/.crewchief/worktrees/<repo-name>`
- [ ] JSDoc comment explains the breaking change and features
- [ ] Example config files show all three patterns (new default, legacy, custom)
- [ ] All tests pass with new default
- [ ] Test mocks use new default or mock expansion appropriately
- [ ] TypeScript compilation succeeds
- [ ] No hardcoded `.crewchief/worktrees` paths remain in production code (tests can have them in mocks)
- [ ] Config loading works correctly with no config file (uses new default)
