# WTCLEAN Ticket Index

## Overview

This index tracks all tickets for the WTCLEAN (Enhanced Worktree Clean) project.

**Project:** Enhanced Worktree Clean
**Total Tickets:** 11
**Phases:** 3

## Phase 1: Binary Resolution and Maproom Integration (3 tickets)

**Objective:** Enable TypeScript CLI to discover and invoke maproom binary for database cleanup

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| WTCLEAN-1001 | Implement Binary Discovery Utility | ⬜ Not Started | typescript-engineer |
| WTCLEAN-1002 | Create Maproom Cleanup Helper Function | ⬜ Not Started | typescript-engineer |
| WTCLEAN-1003 | Add Unit Tests for Binary Resolution | ⬜ Not Started | typescript-engineer |

**Dependencies:**
- WTCLEAN-1002 depends on WTCLEAN-1001
- WTCLEAN-1003 depends on WTCLEAN-1001 and WTCLEAN-1002

**Success Criteria:**
- Binary discovery finds packaged, dev, and PATH binaries
- Cleanup function calls `db cleanup-stale --confirm` successfully
- Tests cover all fallback strategies
- Graceful handling when binary not found

## Phase 2: Enhanced Clean Command (6 tickets)

**Objective:** Integrate maproom cleanup and branch deletion into `worktree clean` command

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| WTCLEAN-2001 | Add CLI Flags for Opt-Out Behavior | ⬜ Not Started | typescript-engineer |
| WTCLEAN-2002 | Integrate Maproom Cleanup in Clean Command | ⬜ Not Started | typescript-engineer |
| WTCLEAN-2003 | Integrate Branch Deletion in Clean Command | ⬜ Not Started | typescript-engineer |
| WTCLEAN-2004a | Add Error Handling for Maproom Cleanup | ⬜ Not Started | typescript-engineer |
| WTCLEAN-2004b | Add Error Handling for Branch Deletion | ⬜ Not Started | typescript-engineer |
| WTCLEAN-2004c | Add Logging for All Cleanup Steps | ⬜ Not Started | typescript-engineer |

**Dependencies:**
- WTCLEAN-2002 depends on WTCLEAN-1001, WTCLEAN-1002, WTCLEAN-2001
- WTCLEAN-2003 depends on WTCLEAN-2001, WTCLEAN-2002
- WTCLEAN-2004a depends on WTCLEAN-2002
- WTCLEAN-2004b depends on WTCLEAN-2003
- WTCLEAN-2004c depends on WTCLEAN-2002, WTCLEAN-2003, WTCLEAN-2004a, WTCLEAN-2004b

**Success Criteria:**
- Clean command deletes directory, metadata, branch, and database records
- Branch name extracted BEFORE worktree removal (critical sequencing)
- Failures logged as warnings, don't block cleanup
- Clear feedback for each step
- `--keep-branch` and `--keep-maproom` flags work correctly
- Manual recovery instructions provided for each failure scenario

**Critical Requirement (WTCLEAN-2003):**
Branch name MUST be extracted from worktree metadata BEFORE calling `removeWorktree()`. Once worktree is removed, git metadata is gone and branch name cannot be determined.

## Phase 3: Testing and Documentation (2 tickets)

**Objective:** Comprehensive test coverage and user documentation

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| WTCLEAN-3001 | Add Integration Tests for Cleanup Workflow | ⬜ Not Started | typescript-engineer |
| WTCLEAN-3002 | Add Failure Scenario Tests | ⬜ Not Started | typescript-engineer |
| WTCLEAN-3003 | Update README Documentation | ⬜ Not Started | typescript-engineer |

**Dependencies:**
- WTCLEAN-3001 depends on all Phase 2 tickets
- WTCLEAN-3002 depends on WTCLEAN-2004a, WTCLEAN-2004b, WTCLEAN-2004c
- WTCLEAN-3003 depends on all Phase 1 and Phase 2 tickets

**Success Criteria:**
- Tests cover happy path (all cleanup succeeds)
- Tests cover partial failure (maproom missing, branch protected)
- Tests verify graceful degradation
- README documents new flags and behavior
- All tests pass

## Execution Order

**Recommended execution order:**

1. **Phase 1 (sequential):**
   - WTCLEAN-1001 → WTCLEAN-1002 → WTCLEAN-1003

2. **Phase 2 (some parallelization possible):**
   - WTCLEAN-2001 (can start early, no dependencies)
   - WTCLEAN-2002 (after 1001, 1002, 2001)
   - WTCLEAN-2003 (after 2001, 2002)
   - WTCLEAN-2004a (after 2002)
   - WTCLEAN-2004b (after 2003)
   - WTCLEAN-2004c (after 2002, 2003, 2004a, 2004b)

3. **Phase 3 (parallel possible):**
   - WTCLEAN-3001 (after all Phase 2)
   - WTCLEAN-3002 (after 2004a, 2004b, 2004c)
   - WTCLEAN-3003 (after all implementation complete)

**Critical Path:**
```
1001 → 1002 → 2002 → 2003 → 2004a/2004b → 2004c → 3001/3002 → 3003
         ↓     ↑
       1003   2001
```

## Ticket Sizing

All tickets are sized for 2-8 hour scope:

| Phase | Ticket Count | Estimated Time |
|-------|--------------|----------------|
| Phase 1 | 3 tickets | 0.5-1 day |
| Phase 2 | 6 tickets | 1.5-2 days |
| Phase 3 | 2 tickets | 0.5-1 day |
| **Total** | **11 tickets** | **2.5-4 days** |

## Status Legend

- ⬜ Not Started
- 🔄 In Progress
- ✅ Completed
- ⚠️ Blocked

## Notes

### Code Duplication (Temporary)

Phase 1 (tickets 1001-1003) copies `findMaproomBinary()` from maproom-mcp as a pragmatic MVP decision. This creates temporary code duplication that will be consolidated when the MRBIN (Binary Resolution) project completes. A follow-up ticket should be created after MRBIN delivers to replace this copied code with the shared utility.

### Critical Sequencing (WTCLEAN-2003)

Ticket WTCLEAN-2003 has a critical sequencing requirement: branch name MUST be extracted BEFORE worktree removal. This is easy to miss and must be verified during ticket review. The integration test in WTCLEAN-3001 validates this sequencing.

### Error Handling Split (2004a-c)

The original plan had a single ticket WTCLEAN-2004, but it was split into three smaller tickets (2004a, 2004b, 2004c) for better agent sizing and clearer scope:
- 2004a: Maproom error handling
- 2004b: Branch deletion error handling
- 2004c: Logging for all steps

This ensures each ticket stays within 2-8 hour scope and has clear acceptance criteria.

### Batch Cleanup Performance

Maproom cleanup uses batch cleanup (`db cleanup-stale --confirm`) which scans all worktrees in the database. For databases with 50+ worktrees, cleanup may take 2-5 seconds. This is acceptable for MVP as cleanup is an infrequent operation. Manual testing should include databases with 50+ worktrees to validate performance.

## References

- **Plan:** `.crewchief/projects/WTCLEAN_enhanced-worktree-clean/planning/plan.md`
- **Architecture:** `.crewchief/projects/WTCLEAN_enhanced-worktree-clean/planning/architecture.md`
- **Quality Strategy:** `.crewchief/projects/WTCLEAN_enhanced-worktree-clean/planning/quality-strategy.md`
- **Project README:** `.crewchief/projects/WTCLEAN_enhanced-worktree-clean/README.md`
