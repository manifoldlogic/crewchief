# MRPROG Ticket Index

**Project:** Maproom Progress UX Enhancement
**Status:** Ready for Implementation
**Total Tickets:** 16 (7 Phase 1 + 4 Phase 2 + 5 Phase 3)

## Overview

This index organizes all MRPROG tickets by phase and tracks dependencies. Tickets should be executed sequentially within each phase, and phases must complete in order (Phase 1 → Phase 2 → Phase 3).

---

## Phase 1: Progress Tracking Foundation (7 tickets)

**Goal:** Add real-time progress indicator to scan command

**Estimated Duration:** 2-3 days

| Ticket ID | Title | Dependencies | Agent | Status |
|-----------|-------|--------------|-------|--------|
| MRPROG-1001 | Create ProgressTracker module | None | general-purpose | ☐ Not Started |
| MRPROG-1002 | Integrate ProgressTracker with scan_worktree | MRPROG-1001 | general-purpose | ☐ Not Started |
| MRPROG-1003 | Add --verbose flag and wire up ProgressTracker in scan command | MRPROG-1001, MRPROG-1002 | general-purpose | ☐ Not Started |
| MRPROG-1004 | Write ProgressTracker unit tests | MRPROG-1001 | general-purpose | ☐ Not Started |
| MRPROG-1005 | Add performance benchmarks for scan progress tracking | MRPROG-1002, MRPROG-1003 | general-purpose | ☐ Not Started |
| MRPROG-1006 | Update scan command help text and documentation | MRPROG-1003 | general-purpose | ☐ Not Started |
| MRPROG-1007 | Phase 1 integration test and validation | MRPROG-1001 through MRPROG-1006 | general-purpose, unit-test-runner | ☐ Not Started |

**Phase 1 Success Criteria:**
- ✅ `maproom scan` shows real-time progress during indexing
- ✅ Progress updates appear every 200-500ms
- ✅ Final output shows "Completed in X.Xs" prominently
- ✅ TTY mode uses line overwriting, non-TTY uses periodic updates
- ✅ Performance overhead <5% (verified via benchmark)
- ✅ Unit tests pass with >80% coverage of ProgressTracker

---

## Phase 2: Watch Minimal Output (4 tickets)

**Goal:** Make watch command output minimal and glanceable by default

**Estimated Duration:** 1-2 days

**Prerequisites:** Phase 1 complete (MRPROG-1007 passing)

| Ticket ID | Title | Dependencies | Agent | Status |
|-----------|-------|--------------|-------|--------|
| MRPROG-2001 | Implement minimal output mode for watch_worktree | MRPROG-1001, MRPROG-1007 | general-purpose | ☐ Not Started |
| MRPROG-2002 | Add --verbose flag to watch command CLI | MRPROG-2001 | general-purpose | ☐ Not Started |
| MRPROG-2003 | Write integration tests for watch output modes | MRPROG-2001, MRPROG-2002 | general-purpose, integration-tester | ☐ Not Started |
| MRPROG-2004 | Manual testing across terminal environments for watch command | MRPROG-2001, MRPROG-2002, MRPROG-2003 | general-purpose, verify-ticket | ☐ Not Started |

**Phase 2 Success Criteria:**
- ✅ `maproom watch` shows minimal output by default
- ✅ Change events display: "🔄 N files changed"
- ✅ Indexing shows: "Indexing: ....." (one dot per file)
- ✅ Completion shows: "✅ Done in X.Xs"
- ✅ `--verbose` flag restores old detailed output
- ✅ Integration tests verify both output modes
- ✅ Works correctly in 5+ terminal environments

---

## Phase 3: Polish & Documentation (5 tickets)

**Goal:** Final polish, comprehensive testing, documentation updates

**Estimated Duration:** 1 day

**Prerequisites:** Phases 1 & 2 complete (MRPROG-2004 passing)

| Ticket ID | Title | Dependencies | Agent | Status |
|-----------|-------|--------------|-------|--------|
| MRPROG-3001 | Update maproom-mcp README with progress UX features | MRPROG-1007, MRPROG-2004 | general-purpose | ☐ Not Started |
| MRPROG-3002 | Ensure CI runs all tests and benchmarks | MRPROG-1004, MRPROG-1005, MRPROG-2003 | general-purpose | ☐ Not Started |
| MRPROG-3003 | Performance validation on realistic codebase | MRPROG-1005, MRPROG-1007 | general-purpose | ☐ Not Started |
| MRPROG-3004 | Write changelog entry for progress UX features | All Phase 1 & 2 tickets, MRPROG-3003 | general-purpose | ☐ Not Started |
| MRPROG-3005 | Final project validation and completion sign-off | ALL tickets (1001-3004) | verify-ticket, general-purpose | ☐ Not Started |

**Phase 3 Success Criteria:**
- ✅ Help text clearly documents default directory behavior
- ✅ `--help` output shows examples of new features
- ✅ CI runs all tests and benchmarks
- ✅ Manual testing completed in 5+ environments
- ✅ Documentation updated with new UX features
- ✅ Changelog entry written
- ✅ Performance validated on large codebase (<5% overhead)
- ✅ Final sign-off for merge

---

## Dependency Graph

```
Phase 1 (Foundation):
1001 ──┬──> 1002 ──┬──> 1003 ──┬──> 1007 (Validation)
       │            │           │
       └──> 1004    └──> 1005   └──> 1006

Phase 2 (Watch):
1007 ──> 2001 ──> 2002 ──┬──> 2003 ──> 2004 (Validation)
                         │
                         └────────────^

Phase 3 (Polish):
1007 ──┬──> 3001
2004 ──┘

1004 ──┬
1005 ──┼──> 3002
2003 ──┘

1005 ──┬
1007 ──┴──> 3003

All ──> 3004 ──> 3005 (Final Sign-off)
```

---

## Implementation Order

**Recommended Execution Sequence:**

### Week 1: Phase 1
1. **Day 1-2:** MRPROG-1001, 1002, 1003 (core implementation)
2. **Day 2-3:** MRPROG-1004, 1005 (testing)
3. **Day 3:** MRPROG-1006, 1007 (docs + validation)

### Week 1-2: Phase 2
4. **Day 4:** MRPROG-2001, 2002 (watch implementation)
5. **Day 4-5:** MRPROG-2003, 2004 (testing + validation)

### Week 2: Phase 3
6. **Day 5-6:** MRPROG-3001, 3002, 3003 (docs + perf validation)
7. **Day 6:** MRPROG-3004, 3005 (changelog + final sign-off)

**Total Timeline:** 5-7 days (single developer, full-time)

---

## Ticket Workflow

Each ticket follows the standard workflow:

1. **Implementation:** Assigned agent completes the work
2. **Testing:** unit-test-runner or integration-tester verifies functionality
3. **Verification:** verify-ticket checks acceptance criteria
4. **Commit:** commit-ticket creates conventional commit

---

## File Locations

All tickets are located in:
```
.agents/projects/MRPROG_maproom-progress-ux/tickets/
├── MRPROG-1001_create-progress-tracker-module.md
├── MRPROG-1002_integrate-progress-tracker-scan.md
├── MRPROG-1003_add-verbose-flag-cli-integration.md
├── MRPROG-1004_write-progress-tracker-unit-tests.md
├── MRPROG-1005_performance-benchmarks-progress-tracking.md
├── MRPROG-1006_update-scan-help-documentation.md
├── MRPROG-1007_phase1-integration-test-validation.md
├── MRPROG-2001_minimal-output-mode-watch-worktree.md
├── MRPROG-2002_add-verbose-flag-watch-cli.md
├── MRPROG-2003_watch-integration-tests.md
├── MRPROG-2004_manual-testing-terminal-environments.md
├── MRPROG-3001_update-maproom-mcp-readme.md
├── MRPROG-3002_ensure-ci-runs-all-tests-benchmarks.md
├── MRPROG-3003_performance-validation-realistic-codebase.md
├── MRPROG-3004_write-changelog-entry.md
└── MRPROG-3005_final-project-validation-completion-signoff.md
```

---

## Progress Tracking

**Phase Completion:**
- [ ] Phase 1: Progress Tracking Foundation (7 tickets)
- [ ] Phase 2: Watch Minimal Output (4 tickets)
- [ ] Phase 3: Polish & Documentation (5 tickets)

**Overall Status:** 0/16 tickets complete (0%)

**Next Action:** Begin with MRPROG-1001 (Create ProgressTracker module)

---

## Planning Documents Reference

Comprehensive planning available in `planning/` directory:

- **[analysis.md](../planning/analysis.md)** - Deep problem understanding and requirements
- **[architecture.md](../planning/architecture.md)** - Technical design and implementation approach
- **[quality-strategy.md](../planning/quality-strategy.md)** - Pragmatic testing strategy
- **[security-review.md](../planning/security-review.md)** - Security assessment (minimal risk)
- **[plan.md](../planning/plan.md)** - Phased implementation plan with detailed tasks

---

## Quick Commands

```bash
# Start working on the project
/work-on-project MRPROG

# Work on a single ticket
/single-ticket MRPROG-1001

# Review all tickets
/review-tickets MRPROG
```

---

**Last Updated:** 2025-01-05
**Project Status:** Ready for Implementation
**Next Milestone:** Complete Phase 1 (MRPROG-1007)
