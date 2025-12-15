# Project: Maproom Binary Configuration

**Slug:** MRBIN
**Status:** Complete ✓
**Created:** 2025-12-15
**Completed:** 2025-12-15
**Effort:** S (1 day)

## Summary

Complete the partially-implemented `maproomBinaryPath` configuration feature by ensuring all CLI commands consistently use config-based binary resolution and documenting the development workflow.

**Key finding:** The feature is 90% complete. The config schema exists, resolution logic works, and tests are comprehensive. Only one call site (`cleanMaproomRecords`) needs updating, and developer documentation needs to be added.

## Problem Statement

Developers working on the Rust maproom binary currently must set `CREWCHIEF_MAPROOM_BIN` environment variable to use their local builds. The `maproomBinaryPath` config option exists but is:
- **Inconsistently used**: `cleanMaproomRecords()` function doesn't pass config to binary resolution
- **Underdocumented**: Missing from local development workflow documentation

This causes confusion and creates an incomplete feature that appears to work (most commands respect it) but fails in edge cases (database cleanup operations).

## Proposed Solution

**Minimal, focused changes:**

1. **Update one function** - Make `cleanMaproomRecords()` load and use config
2. **Add developer docs** - Document config-based workflow in `local-development.md`
3. **Verify tests** - Ensure test coverage for config usage in cleanup scenarios

**Not changing:**
- Config schema (already correct)
- Binary resolution logic (already correct)
- Resolution priority order (already correct: env > config > global > packaged)
- MCP package (intentionally different, serves IDE integration use case)

**Rationale:** This is a completion project, not a redesign. The architecture is sound; we're just filling the gaps.

## Relevant Agents

**Planning:**
- project-planner (this document)

**Implementation:**
- typescript-specialist (update cleanMaproomRecords function)
- unit-test-specialist (add/verify test coverage)
- documentation-specialist (add config method to local-development.md)

**Quality Assurance:**
- unit-test-runner (run test suite)
- verify-ticket (check acceptance criteria)

**Finalization:**
- commit-ticket (create commit)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis with code inspection
- [architecture.md](planning/architecture.md) - Solution design decisions
- [plan.md](planning/plan.md) - Phased execution plan (4 phases, 6 hours)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach
- [security-review.md](planning/security-review.md) - Security assessment (LOW risk)

## Key Insights from Planning

### What Already Works

✅ **Config schema** - `maproomBinaryPath: z.string().optional()` exists
✅ **Binary resolution** - `findMaproomBinary()` implements correct priority order
✅ **Test coverage** - 20+ test cases covering all scenarios
✅ **User documentation** - README.md documents the config option
✅ **Two call sites** - `maproom.ts` and `runMaproomScan()` already pass config

### What Needs Completion

❌ **One call site** - `cleanMaproomRecords()` at worktrees.ts:242 doesn't use config
❌ **Developer docs** - local-development.md missing config-based method
⚠️ **Test coverage** - May need 2-3 tests for cleanMaproomRecords scenarios

### Design Decisions

**Decision 1:** No changes to resolution order (already correct)
**Decision 2:** MCP package unchanged (serves different use case)
**Decision 3:** No shared utility extraction (implementations intentionally different)
**Decision 4:** Config file location via optional parameter (no breaking changes)
**Decision 5:** Fix cleanMaproomRecords only (minimal scope)

### Acceptance Criteria Status

From project summary:

- [x] Config accepts `maproomBinaryPath` setting (already implemented)
- [x] Env var still takes highest precedence (already implemented)
- [x] Global install checked before packaged binary (already correct)
- [ ] Config path takes precedence over packaged binary (needs verification in tests)
- [ ] Binary resolution is consistent across all commands (fix cleanMaproomRecords)
- [ ] Development workflow documented (add to local-development.md)

**3 of 6 complete, 3 remaining**

## Tickets

Tickets will be generated via `/workstream:project-tickets MRBIN` after planning review.

**Expected tickets:**
1. **MRBIN-1001**: Update cleanMaproomRecords to use config (Phase 1)
2. **MRBIN-1002**: Add/verify test coverage for config usage (Phase 2)
3. **MRBIN-1003**: Document config-based development workflow (Phase 3)
4. **MRBIN-1004**: Verification and quality gates (Phase 4)

## Dependencies

**None.** This project is completely self-contained:
- No blocking dependencies on other projects
- All required infrastructure exists (schema, resolution, tests, docs)
- Can start immediately after planning review

## Timeline

**Total effort:** 6 hours (S - 1 day)

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 1. Code Integration | 1h | cleanMaproomRecords updated |
| 2. Test Coverage | 2h | Tests verified/added |
| 3. Documentation | 1.5h | Config method documented |
| 4. Verification | 1.5h | All acceptance criteria met |

**Buffer:** 0.5 hours for unexpected issues

## Risk Assessment

**Overall Risk: LOW**

**Mitigations in place:**
- Minimal code changes (one function signature update)
- Comprehensive existing tests catch regressions
- Backwards compatible (config parameter optional)
- Well-understood problem space

**No blockers identified.**

## Security Assessment

**Risk Level: LOW**

No new security concerns:
- No new attack surface (config already loads JavaScript)
- No network communication
- No sensitive data handling
- Trust model unchanged (developer controls their config)

**See [security-review.md](planning/security-review.md) for detailed analysis.**

## Success Metrics

**Feature complete when:**
- [ ] All 3 CLI call sites pass config to findMaproomBinary
- [ ] Test coverage includes cleanMaproomRecords scenarios
- [ ] local-development.md documents config-based workflow
- [ ] All tests pass (pnpm test)
- [ ] No linting errors (pnpm lint)
- [ ] Manual verification confirms config works end-to-end

## Next Steps

1. **Review planning documents** - Run `/workstream:project-review MRBIN`
2. **Address review findings** - If any issues identified
3. **Generate tickets** - Run `/workstream:project-tickets MRBIN`
4. **Execute tickets** - Work through 4 phases sequentially
5. **Verify completion** - Check all acceptance criteria met
6. **Commit and PR** - Standard workflow

## Notes

**Key constraint:** This is a 1-day effort. Scope is tightly controlled:
- No MCP package changes (intentionally different)
- No shared utility extraction (not needed)
- No resolution order changes (already correct)
- No new config fields (using existing maproomBinaryPath)

**Focus:** Complete the existing feature, don't redesign it.
