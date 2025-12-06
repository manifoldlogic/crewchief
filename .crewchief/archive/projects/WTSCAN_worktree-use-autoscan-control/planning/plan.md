# Plan: Worktree Use Auto-Scan Control

## Overview

This execution plan delivers worktree auto-scan control in two focused phases. The entire project is estimated at 1-2 days for a small, contained change to config schema and conditional logic.

**Total Phases**: 2
**Estimated Duration**: 1-2 days
**Team Size**: 1 developer (general TypeScript agent)
**Complexity**: Low (single config field + conditional)

## Phases

### Phase 1: Config Schema and Core Logic

**Objective**: Add config field and implement conditional scan behavior in WorktreeService

**Duration**: 4-6 hours

**Deliverables**:
1. Config schema field `autoScanOnWorktreeUse` added to `WorktreeSchema`
2. Conditional scan logic in `WorktreeService.createWorktree()`
3. Unit tests for config field validation
4. Integration tests for scan behavior (enabled/disabled/error states)

**Agent Assignments**:
- `typescript-dev`:
  - Add `autoScanOnWorktreeUse: z.boolean().default(false)` to `WorktreeSchema` in `packages/cli/src/config/schema.ts`
  - Update `createWorktree()` method in `packages/cli/src/git/worktrees.ts` to check config before calling `runMaproomScan()`
  - Wrap config check in try-catch for error resilience
  - Add test suite for auto-scan behavior in `packages/cli/src/cli/__tests__/worktree-create.test.ts`

- `unit-test-runner`:
  - Run test suite to verify no regressions
  - Verify new tests pass
  - Check test coverage for new conditional logic

**Acceptance Criteria**:
- [ ] Config schema validates `autoScanOnWorktreeUse` as boolean
- [ ] Default value is `false`
- [ ] `createWorktree()` skips scan when config is false/undefined
- [ ] `createWorktree()` runs scan when config is true
- [ ] Config loaded once and reused for both copyIgnoredFiles and autoScan checks
- [ ] Config load errors don't break worktree creation
- [ ] All new tests pass
- [ ] Existing tests still pass (no regression)

**Dependencies**:
- No dependencies - extends existing `WorktreeSchema` following the `copyIgnoredFiles` pattern

**Risks**:
- **Config schema validation**: LOW - Zod handles validation automatically
- **Test mocking**: LOW - Existing test patterns already mock WorktreeService

---

### Phase 2: Documentation and Breaking Change Communication

**Objective**: Document the change, provide migration guide, and explain trade-offs

**Duration**: 2-4 hours

**Deliverables**:
1. README section explaining auto-scan configuration
2. Migration guide for existing users
3. Changelog entry documenting breaking change
4. Example config snippet in documentation

**Agent Assignments**:
- `docs-writer`:
  - Add "Auto-Scan Configuration" section to `packages/cli/README.md` (after "Semantic Code Search")
  - Document trade-offs: fast ops vs immediate searchability
  - Provide migration example: `autoScanOnWorktreeUse: true`
  - Add changelog entry explaining breaking change and migration path
  - Update any relevant JSDoc comments

- `verify-ticket`:
  - Verify documentation is clear and complete
  - Check that migration path is easy to follow
  - Confirm changelog entry properly warns about breaking change

**Acceptance Criteria**:
- [ ] README includes new section on auto-scan control
- [ ] Trade-offs clearly explained (speed vs convenience)
- [ ] Migration example shows exact config to restore old behavior
- [ ] Changelog entry prominently notes breaking change
- [ ] Documentation is accurate and grammatically correct

**Dependencies**:
- Phase 1 must be complete (implementation finished)

**Risks**:
- **Insufficient migration guidance**: MEDIUM - Breaking change must be clearly communicated
  - **Mitigation**: Include explicit config example in multiple places
- **User confusion about default**: LOW - Documentation explicitly states default and rationale

---

## Cross-Phase Dependencies

```
Phase 1 (Config + Logic) → Phase 2 (Documentation)
```

**Critical Path**: Phase 1 → Phase 2

**Parallel Work**: None (phases must be sequential)

**No External Dependencies**: This project extends existing config schema patterns and can proceed immediately.

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaking change breaks user workflows | Medium | High | Clear migration docs, prominent changelog, trivial one-line fix |
| Config validation edge cases | Low | Low | Zod schema handles validation, existing patterns proven |
| Test coverage gaps | Low | Medium | Follow existing test patterns, mock WorktreeService properly |
| Users don't read migration guide | Medium | Medium | Put example config in multiple places, make it copy-paste ready |
| Unclear documentation | Low | Medium | docs-writer and verify-ticket agents ensure clarity |

## Success Metrics

### Functional Metrics
- [ ] Config accepts boolean field with correct default
- [ ] Worktree creation skips scan by default
- [ ] Scan runs when explicitly enabled
- [ ] Error handling prevents worktree creation failure
- [ ] All tests pass (new and existing)

### Performance Metrics
- [ ] Default worktree creation completes in <1 second (down from 5-30s)
- [ ] Enabled auto-scan performance unchanged from current behavior
- [ ] Test suite execution time unchanged

### Documentation Metrics
- [ ] README includes auto-scan configuration section
- [ ] Migration guide provides copy-paste config example
- [ ] Changelog clearly notes breaking change
- [ ] Trade-offs explicitly documented

### User Experience Metrics
- [ ] Migration requires one line of config (measured by example simplicity)
- [ ] Documentation enables self-service migration in <5 minutes
- [ ] Default behavior matches user expectation (fast operations)

## Timeline

**Day 1**:
- Morning: Phase 1 implementation (config schema + logic)
- Afternoon: Phase 1 testing and verification
- End of day: Phase 1 complete, ready for review

**Day 2**:
- Morning: Phase 2 documentation
- Afternoon: Final review and verification
- End of day: Ready for commit and release

**Contingency**: +4 hours buffer for unexpected test issues or documentation iterations

## Rollout Strategy

### Pre-Release Checklist
- [ ] All tests passing
- [ ] Documentation complete and reviewed
- [ ] Changelog entry drafted
- [ ] Migration example verified
- [ ] Breaking change clearly communicated

### Release Notes Template

```markdown
## Breaking Change: Auto-Scan Now Opt-In

**What Changed**: Worktree creation no longer automatically triggers maproom scanning by default.

**Why**: This change dramatically improves worktree creation speed (from 5-30s to <1s) and gives users control over when indexing happens.

**Migration**: To restore automatic scanning, add one line to your `crewchief.config.js`:

```javascript
export default {
  worktree: {
    autoScanOnWorktreeUse: true, // Restore auto-scan behavior
  },
}
```

**Alternative**: Manually scan when needed: `crewchief maproom scan`

**Impact**: Users relying on automatic indexing must update config or manually scan.
```

### Post-Release Monitoring
- Watch for GitHub issues related to missing auto-scan
- Respond quickly with link to migration docs
- Consider adding warning message if user seems to expect auto-scan

## Definition of Done

**Phase 1 Complete When**:
- [ ] Code changes committed
- [ ] All tests passing
- [ ] verify-ticket agent confirms acceptance criteria met

**Phase 2 Complete When**:
- [ ] Documentation committed
- [ ] verify-ticket confirms docs are clear and complete
- [ ] Changelog entry drafted and reviewed

**Project Complete When**:
- [ ] Both phases complete
- [ ] All acceptance criteria met
- [ ] Breaking change clearly documented
- [ ] Ready for release with confidence

## Next Steps After Completion

1. **Create PR** with breaking change label
2. **Team Review** of breaking change impact
3. **Version Bump** (minor version with breaking change notes)
4. **Release** with prominent changelog entry
5. **Monitor** user feedback and issues
6. **Iterate** on documentation if users are confused

## Future Improvements

These enhancements are explicitly OUT OF SCOPE for MVP but may be considered later:

1. **CLI Flag Override**: `--scan` / `--no-scan` flags
   - Allows per-command override of config
   - Useful for scripts and automation

2. **Purpose-Based Auto-Scan**:
   - Auto-scan for agent worktrees
   - Skip scan for manual worktrees
   - Requires extending worktree metadata

3. **Background Scanning**:
   - Queue scan to run after worktree creation
   - Non-blocking user experience
   - Requires job queue infrastructure

4. **Smart Defaults**:
   - Auto-detect repo size
   - Enable auto-scan only for small repos
   - Disable for large repos

**Decision**: Ship simple boolean config first, gather user feedback, then iterate.
