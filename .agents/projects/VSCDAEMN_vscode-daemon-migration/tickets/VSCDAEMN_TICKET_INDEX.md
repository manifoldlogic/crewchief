# VSCDAEMN Ticket Index

**Project**: VSCDAEMN - VSCode Extension Cleanup
**Status**: ⚠️ SCOPE REVISED - Simplified Cleanup Only
**Decision**: Keep spawning for scan (appropriate pattern), perform simple cleanup

## Project Decision

**Original Plan**: Migrate VSCode scan to daemon-client
**Review Finding**: daemon-client lacks scan support; 3-5 weeks of prerequisite work required
**Final Decision**: **Option 3 - Simplified Cleanup** (keep spawning, remove unused utilities)

See `planning/project-review.md` for full analysis.

---

## Ticket List

### Cleanup Phase (1-2 days)

**VSCDAEMN-1001** - Cleanup Deprecated Spawning Utilities
- **Status**: 🟡 Ready for Work
- **Agent**: general-purpose
- **Effort**: 1-2 days
- **Description**: Audit spawning utilities, remove unused code, document spawning vs daemon guidelines
- **File**: `tickets/VSCDAEMN-1001_cleanup-spawning-utilities.md`

---

## Workflow

1. **Execute Ticket**: Complete VSCDAEMN-1001
2. **Verify**: Confirm no regressions in VSCode extension
3. **Archive**: Move project to archive (work complete)

---

## Notes

**Why One Ticket?**
- Original plan had 12 tickets for daemon migration
- After review, scope reduced to simple cleanup
- Single ticket sufficient for audit + cleanup + documentation

**Why Not More Tickets?**
- No daemon migration needed (spawning is appropriate)
- No daemon-client enhancement needed (not worth the effort)
- Simple cleanup work doesn't require phased approach

---

**Created**: 2025-01-22
**Last Updated**: 2025-01-22
