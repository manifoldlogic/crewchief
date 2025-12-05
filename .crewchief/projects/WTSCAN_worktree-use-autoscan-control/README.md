# Project: Worktree Use Auto-Scan Control

**Slug:** WTSCAN
**Status:** Planning
**Created:** 2025-12-05
**Priority:** Medium
**Effort:** S (1-2 days)

## Summary

Add `worktree.autoScanOnWorktreeUse` config option (default: false) to control whether `worktree create` automatically triggers maproom scans. This makes fast worktree operations the default behavior while allowing users to opt-in to automatic indexing.

**Note**: This only affects `worktree create`. The `worktree use` command (which switches between existing worktrees) does not trigger scanning and is not modified by this project.

## Problem Statement

CrewChief currently performs automatic maproom scanning after every worktree creation, causing 5-30 second delays (depending on repository size). This unconditional behavior:

- Creates unexpected delays when users just want to quickly create a worktree
- Provides no escape hatch for users who don't need semantic search
- Slows down CI/CD and automated workflows unnecessarily
- Violates the principle of least surprise

**Core Issue**: Auto-scan is a policy decision masquerading as a feature, with no user control.

## Proposed Solution

**Add Config Field**: `autoScanOnWorktreeUse: boolean` (default: `false`)

**Breaking Change**: Auto-scan becomes opt-in instead of default behavior.

**Implementation**:
1. Add config field to `WorktreeSchema` in `packages/cli/src/config/schema.ts`
2. Update `WorktreeService.createWorktree()` to load config once and check before calling `runMaproomScan()`
3. Wrap config loading in try-catch for error resilience
4. Add comprehensive tests for config-gated behavior
5. Document migration path and trade-offs clearly

**Value Proposition**: Worktree creation becomes instant by default (< 1 second vs 5-30 seconds), while users who want auto-indexing can enable it with one config line.

**Scope Note**: Auto-scan configuration only affects `worktree create`. Other worktree commands (`use`, `merge`, `clean`) never trigger scans and are not modified.

## Key Design Decisions

1. **Default to `false`**: Fast operations are more important than automatic indexing
2. **Scope limited to `create`**: Only affects `worktree create` command (use/merge/clean unchanged)
3. **Breaking change accepted**: Better UX justifies the breaking change
4. **Clear migration path**: One-line config change to restore old behavior
5. **Error resilience**: Config errors must never break worktree creation
6. **Single config load**: Load config once, reuse for both copyIgnoredFiles and autoScan checks

## Relevant Agents

**Planning Phase**:
- project-planner ✅ (this document)

**Implementation Phase** (Phase 1):
- typescript-dev (config schema + conditional logic + tests)
- unit-test-runner (test execution and verification)
- verify-ticket (acceptance criteria verification)
- commit-ticket (commit changes)

**Documentation Phase** (Phase 2):
- docs-writer (README updates, migration guide, changelog)
- verify-ticket (documentation review)
- commit-ticket (commit documentation)

## Planning Documents

All planning documents are complete and ready for review:

- [analysis.md](planning/analysis.md) - Comprehensive problem analysis with codebase research
- [architecture.md](planning/architecture.md) - Solution design with implementation details
- [plan.md](planning/plan.md) - 2-phase execution plan (1-2 day timeline)
- [quality-strategy.md](planning/quality-strategy.md) - Pragmatic testing approach focused on confidence
- [security-review.md](planning/security-review.md) - Security assessment (LOW risk, APPROVED)

## Dependencies

**No Dependencies**: This is an isolated change that extends existing `WorktreeSchema` following the established `copyIgnoredFiles` pattern. Can proceed immediately without waiting for other projects.

## Deliverables

### Phase 1: Config Schema and Core Logic (4-6 hours)
1. ✅ Config field `autoScanOnWorktreeUse` added to `WorktreeSchema`
2. ✅ Conditional scan logic in `WorktreeService.createWorktree()`
3. ✅ Unit tests for config validation
4. ✅ Integration tests for scan behavior (5 test cases)

### Phase 2: Documentation and Breaking Change Communication (2-4 hours)
1. ✅ README section explaining auto-scan configuration
2. ✅ Migration guide with copy-paste config example
3. ✅ Changelog entry documenting breaking change
4. ✅ Trade-offs clearly explained

## Acceptance Criteria

### Functional
- [ ] Config accepts `autoScanOnWorktreeUse: boolean` field
- [ ] Default value is `false` (opt-in scanning)
- [ ] `worktree create` skips scan by default
- [ ] `worktree create` runs scan when config is true
- [ ] Config loaded once and reused for both operations
- [ ] Config load errors don't break worktree creation

### Technical
- [ ] All new tests pass
- [ ] All existing tests still pass (no regression)
- [ ] TypeScript compiles without errors
- [ ] ESLint passes with no new warnings

### Documentation
- [ ] README includes auto-scan configuration section
- [ ] Trade-offs clearly explained (speed vs convenience)
- [ ] Migration example shows exact config to restore old behavior
- [ ] Changelog entry prominently notes breaking change

### Performance
- [ ] Worktree creation time < 1 second by default (down from 5-30s)
- [ ] No performance regression when auto-scan is enabled

## Breaking Change Communication

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

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaking change breaks user workflows | Medium | High | Clear migration docs, prominent changelog, trivial one-line fix |
| Config validation edge cases | Low | Low | Zod schema handles validation, existing patterns proven |
| Users don't read migration guide | Medium | Medium | Put example config in multiple places, make it copy-paste ready |

**Overall Risk**: LOW - Simple change with clear migration path and no external dependencies.

## Timeline

**Estimated Duration**: 1-2 days

**Day 1**: Phase 1 implementation and testing
**Day 2**: Phase 2 documentation and final review

**Contingency**: +4 hours buffer for unexpected issues

## Next Steps

1. **Run `/workstream:project-review WTSCAN`** to validate planning completeness
2. Address any review feedback
3. **Run `/workstream:project-tickets WTSCAN`** to generate implementation tickets
4. Execute Phase 1 (config + logic + tests)
5. Execute Phase 2 (documentation)
6. Create PR with breaking change label
7. Release with prominent changelog entry

## Success Metrics

- Worktree creation time: <1 second for default operation (down from 5-30s)
- Migration simplicity: One line of config (verified)
- Test coverage: 100% of new conditional logic
- Breaking change impact: Zero production failures due to clear migration path
- User satisfaction: Fast operations + clear opt-in path

## Future Enhancements (Out of Scope)

These are explicitly NOT in this project but may be considered later:

1. CLI flag override (`--scan` / `--no-scan`)
2. Purpose-based auto-scan (agent worktrees vs manual)
3. Background scanning (non-blocking)
4. Smart defaults based on repo size

**Decision**: Ship simple MVP first, gather feedback, iterate.

---

**Project Status**: Planning complete, ready for review and ticket creation.

**Confidence Level**: HIGH - Small, well-understood change with comprehensive planning.
