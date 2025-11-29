# VSCDAEMN: VSCode Extension Cleanup

**Project Slug:** VSCDAEMN
**Status:** ⚠️ SCOPE REVISED - Simplified Cleanup Only
**Priority:** LOW (code cleanup, no functional changes)

## Decision Summary

**Original Plan**: Migrate VSCode scan to daemon-client pattern
**Review Finding**: daemon-client lacks scan/upsert/progress support; would require 3-5 weeks of prerequisite work
**Final Decision**: **Keep spawning for scan** (appropriate for one-time operations), perform simple cleanup

See `planning/project-review.md` for full analysis.

## Overview

The VSCDAEMN project was originally planned to migrate the VSCode extension's `scan` command to daemon-client. After comprehensive review, we determined that:

1. **Spawning is appropriate** for one-time scan operations (minimal overhead)
2. **daemon-client doesn't support scan** (only search/ping methods exist)
3. **Low value for high effort** (<5% improvement for 3-5 weeks of work)

**Revised Objectives:**
- ✅ Keep spawning for scan (it's the right pattern)
- ✅ Remove unused deprecated spawning utilities
- ✅ Document when spawning vs daemon is appropriate
- ✅ No functional changes to VSCode extension

## Problem Statement (Revised)

The VSCode extension has deprecated spawning utilities that are still needed for the `scan` command. The original assumption was that these utilities should be removed by migrating scan to daemon-client.

**Review Findings:**
1. **Spawning is optimal** for one-time operations like scan
2. **daemon-client lacks scan support** (would need 3-5 weeks to add)
3. **Performance gain minimal** (<5% for eliminating one-time spawn overhead)
4. **Some deprecated utilities truly unused** (can be safely removed)

## Revised Solution (Option 3: Simplified Cleanup)

Keep spawning for scan (it's the right pattern), but clean up truly unused utilities:

- **Keep**: Spawning for scan operations (one-time, optimal pattern)
- **Remove**: Deprecated utilities that are genuinely unused
- **Document**: When to use spawning vs daemon (clarity for future)
- **No performance claims**: Spawning overhead is negligible for one-time ops

### Revised Scope

**In Scope:**
- ✅ Audit deprecated spawning utilities for actual usage
- ✅ Remove genuinely unused utilities
- ✅ Document spawning vs daemon usage guidelines
- ✅ Verify VSCode extension still works correctly

**Out of Scope:**
- ❌ Migrating scan to daemon (spawning is appropriate)
- ❌ daemon-client enhancement (not needed)
- ❌ Performance improvements (none needed)
- ❌ Architectural changes (current design is correct)

## Benefits (Revised)

**Code Quality:**
- Remove genuinely unused utilities
- Reduce maintenance burden
- Clearer codebase

**Documentation:**
- Clear guidelines on spawning vs daemon
- Prevent future confusion
- Architectural clarity

**Pragmatism:**
- No wasted effort on low-value migration
- Keep working patterns working
- Focus on appropriate tool for each job

## Relevant Agents (Revised)

### Single-Phase Cleanup
- **general-purpose** - Audit utilities, remove unused code, update documentation

## Planning Documents

Comprehensive planning documents in `planning/` directory:

- **[analysis.md](planning/analysis.md)** - Problem analysis, existing solutions, research findings, risk assessment
- **[architecture.md](planning/architecture.md)** - System design, component changes, data flow, performance considerations
- **[quality-strategy.md](planning/quality-strategy.md)** - Testing approach, coverage targets, risk mitigation through testing
- **[security-review.md](planning/security-review.md)** - Threat model, attack vectors, MVP mitigations, compliance
- **[plan.md](planning/plan.md)** - Phased implementation, agent assignments, timeline, success metrics

## Success Metrics (Revised)

### Code Quality Targets
- Unused utilities identified and removed
- Documentation updated with usage guidelines
- Zero regressions in VSCode extension functionality

### Verification Targets
- VSCode extension still works correctly
- Scan operation unchanged (still uses spawning)
- All existing tests still pass

## Key Technical Decisions (Revised)

### Why Keep Spawning for Scan?
- ✅ One-time operation (spawn overhead negligible)
- ✅ Works correctly today (no need to change)
- ✅ daemon-client lacks scan support (would require weeks of work)
- ✅ Expected improvement <5% (not worth the effort)

### Why Not Migrate to Daemon?
- ❌ daemon-client only has search/ping (no scan/upsert/progress)
- ❌ Rust daemon RPC only handles search (no scan method)
- ❌ Would require 3-5 weeks of prerequisite work (25-30 tickets)
- ❌ Low value for high effort (spawning is fine for one-time ops)

### Why Simple Cleanup Instead?
- ✅ Pragmatic approach (remove genuinely unused code)
- ✅ No regression risk (keep working patterns)
- ✅ Clear documentation (prevent future confusion)
- ✅ Appropriate tool for the job (spawning vs daemon)

## Risks and Mitigations

### Technical Risks
- **Daemon fails to start**: Auto-restart with circuit breaker, user-friendly error messages
- **Progress events lost**: Comprehensive testing of progress callback
- **Extension activation slower**: Start daemon asynchronously (no blocking)

### Operational Risks
- **PostgreSQL unavailable**: postgres-checker prevents daemon start, show setup wizard
- **User confusion**: No user-facing changes, transparent migration

### Mitigation Strategy
- Fallback to user-friendly error messages (no spawning fallback needed)
- Comprehensive testing (unit, integration, regression)
- Phased rollout (internal → beta → stable)

## Timeline Estimate (Revised)

**Single Phase**: 1-2 days (audit, cleanup, documentation)

**Total**: 1-2 days (simple cleanup work)

**Original Estimate**: 4-6 days (turned out to be 3-5 weeks with prerequisite work)

## Next Steps (Revised)

1. ✅ **Review Complete**: project-review.md identified critical blockers
2. ✅ **Decision Made**: Option 3 (Simplified Cleanup) selected
3. ✅ **Scope Revised**: Simple cleanup instead of migration
4. **Execute Ticket**: Single ticket for cleanup work (VSCDAEMN-1001)
5. **Verify**: Confirm no regressions in VSCode extension
6. **Archive**: Move project to archive (work complete)

## Related Documentation

- **DAEMIGR Project**: `.crewchief/projects/DAEMIGR_daemon-client-migration/` (daemon-client package)
- **daemon-client README**: `packages/daemon-client/README.md` (API documentation, migration guide)
- **VSCode Extension**: `packages/vscode-maproom/` (current implementation)
- **MCP Server**: `packages/maproom-mcp/` (already migrated to daemon)

## Questions or Concerns?

See `planning/` directory for detailed analysis, architecture, quality strategy, security review, and implementation plan. All decisions are documented with rationale, alternatives considered, and trade-offs explained.

---

**Project Created:** 2025-01-22
**Planning Status:** ✅ COMPLETE
**Ready for Implementation:** ✅ YES
