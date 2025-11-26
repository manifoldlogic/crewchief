# DOCKERUP Ticket Index

**Project**: DOCKERUP - Automatic Docker Container Startup
**Status**: Ready for Implementation
**Total Tickets**: 1
**Estimated Time**: 2-3 hours

## Overview

This is a single-ticket project that wires existing DockerManager infrastructure into the VSCode extension activation flow. All heavy lifting was completed in VSMAP-1001 (Nov 16). This project is pure integration - connecting existing, tested components.

## Ticket List

### Phase 1: Integration (Single Ticket)

| Ticket ID | Title | Status | Agent | Estimated Time | Priority |
|-----------|-------|--------|-------|----------------|----------|
| DOCKERUP-1001 | Wire DockerManager into Extension Activation Flow | ⏳ Pending | vscode-extension-specialist | 2-3 hours | High |

## Ticket Details

### DOCKERUP-1001: Wire DockerManager into Extension Activation Flow

**File**: `DOCKERUP-1001_wire-docker-into-activation.md`
**Phase**: 1 (Integration)
**Agent**: vscode-extension-specialist

**Summary**: Create `ensureDockerRunning()` function and integrate into extension activation flow (initializeServices, runFirstTimeSetup). Automatically start Docker containers before PostgreSQL check and watch processes.

**Key Deliverables**:
- New function: `ensureDockerRunning()` (~30 lines)
- Integration: `initializeServices()` (+2 lines)
- Integration: `runFirstTimeSetup()` (+2 lines)
- Unit tests: ~300 lines with >90% coverage
- Documentation: README + CHANGELOG updates

**Acceptance Criteria** (15 checkboxes):
- ✅ Functional: 6 scenarios (fresh install, Docker not running, error handling, setup wizard, deactivation, multi-workspace)
- ✅ Quality: 6 requirements (unit tests, flow verification, manual tests, no regressions)
- ✅ Documentation: 3 updates (README, troubleshooting, CHANGELOG)

**Dependencies**:
- VSMAP-1001 (DockerManager) - ✅ Complete
- VSMAP-1003 (ProcessOrchestrator) - ✅ Complete
- MCPINIT-1001 (MCPConfigWriter) - ✅ Complete
- MCPINIT-1002 (SetupWizard) - ✅ Complete

**Risk Level**: 🟢 Low (reusing tested components)

**Plan Reference**: See `planning/plan.md` lines 81-203 for implementation details

## Workflow

```
DOCKERUP-1001 (vscode-extension-specialist)
    ↓
unit-test-runner (execute tests)
    ↓
verify-ticket (check acceptance criteria)
    ↓
commit-ticket (create commit)
```

## Success Metrics

**MVP Success Criteria**:
- [ ] Extension starts Docker automatically (no manual commands)
- [ ] Watch processes start after Docker ready
- [ ] Clear error when Docker not running
- [ ] Containers stop on deactivation
- [ ] All unit tests passing (>90% coverage)
- [ ] Manual test checklist complete (5 scenarios)

## Planning Documents

- [README](../README.md) - Project overview
- [Analysis](../planning/analysis.md) - Problem definition
- [Architecture](../planning/architecture.md) - Integration design
- [Plan](../planning/plan.md) - Implementation details
- [Quality Strategy](../planning/quality-strategy.md) - Testing approach
- [Security Review](../planning/security-review.md) - Security assessment
- [Project Review](../planning/project-review.md) - Pre-ticket review

## Execution Instructions

1. **Prepare**: Read all planning documents
2. **Execute**: Run `/single-ticket DOCKERUP-1001`
3. **Verify**: Complete manual test checklist
4. **Commit**: Create Conventional Commit via commit-ticket agent
5. **Release**: Bump version, publish to marketplace

## Notes

**Why Single Ticket?**
- Trivial integration task (~50 lines of production code)
- All infrastructure exists from previous projects
- No complex dependencies or sequencing
- 2-3 hours focused work fits single agent execution

**What Makes This Simple?**
- DockerManager.ensureServicesRunning() does all the work
- Just calling existing methods in correct order
- Zero new logic, pure function calls
- Comprehensive tests already exist for DockerManager

**Key Insight**:
> "This is a trivial integration task masquerading as a project. The hard work was already done in VSMAP-1001 (DockerManager implementation)." - from planning/analysis.md

---

**Created**: 2025-01-24
**Last Updated**: 2025-01-24
**Tickets Created**: 1/1
**Ready for Execution**: ✅ Yes
