# ITERMCLN Ticket Index

**Project**: iTerm Spawn Command Cleanup
**Status**: Tickets Created, Ready for Execution
**Total Tickets**: 10

## Overview

This project **FIXES** the broken iTerm spawn command and cleans up ~1,750 lines of dead JSON-RPC code. The spawn command currently fails with 30-second timeout for iTerm users.

## Ticket Summary by Phase

| Phase | Tickets | Risk | Key Outcome |
|-------|---------|------|-------------|
| 1. Dead Code Removal | 2 | Medium | Remove broken bridge code |
| 2. ITermProvider Fix | 2 | Medium-High | **FIX broken spawn** |
| 3. Headless Messaging | 3 | Medium | stdin-based messaging |
| 4. Multi-Agent Spawn | 1 | Low | comma-separated agents |
| 5. Testing & Docs | 2 | Low | regression safety |

## Critical Path

**Phase 1 → Phase 2 must be done together** (spawn is broken until Phase 2 completes)

- ITERMCLN-1002 and ITERMCLN-2001 must be committed together (single atomic commit)

---

## Phase 1: Dead Code Removal (Medium Risk)

### ITERMCLN-1001: Delete Dead Python Bridge Code
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Delete ~1,650 lines of dead Python bridge code (10 files)
- **Dependencies**: None
- **File**: [ITERMCLN-1001_delete-python-dead-code.md](./ITERMCLN-1001_delete-python-dead-code.md)

### ITERMCLN-1002: Delete Dead TypeScript Bridge Code
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Delete ~663 lines of dead TypeScript code (3 files)
- **Dependencies**: ITERMCLN-1001
- **Critical**: Must be committed with ITERMCLN-2001
- **File**: [ITERMCLN-1002_delete-typescript-dead-code.md](./ITERMCLN-1002_delete-typescript-dead-code.md)

---

## Phase 2: ITermProvider Fix (Medium-High Risk)

### ITERMCLN-2001: Rewrite ITermProvider to Use Direct Script Calls
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: **BUG FIX** - Rewrite ITermProvider to fix broken spawn command
- **Dependencies**: ITERMCLN-1001
- **Critical**: Must be committed with ITERMCLN-1002
- **File**: [ITERMCLN-2001_rewrite-iterm-provider.md](./ITERMCLN-2001_rewrite-iterm-provider.md)

### ITERMCLN-2002: Verify Spawn Command Works (Manual Checkpoint)
- **Status**: [ ] Pending
- **Agent**: verify-ticket (manual)
- **Summary**: Verification checkpoint before proceeding to Phase 3
- **Dependencies**: ITERMCLN-1001, ITERMCLN-1002, ITERMCLN-2001
- **File**: [ITERMCLN-2002_verify-spawn-fix.md](./ITERMCLN-2002_verify-spawn-fix.md)

---

## Phase 3: Headless Messaging (Medium Risk)

### ITERMCLN-3001: Extend TerminalProvider Interface with Messaging Methods
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Add optional sendMessage() and listAgents() to interface
- **Dependencies**: ITERMCLN-2002
- **File**: [ITERMCLN-3001_extend-terminal-provider-interface.md](./ITERMCLN-3001_extend-terminal-provider-interface.md)

### ITERMCLN-3002: Add Messaging Support to HeadlessProvider
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Implement stdin pipe messaging for headless agents
- **Dependencies**: ITERMCLN-3001
- **File**: [ITERMCLN-3002_headless-provider-messaging.md](./ITERMCLN-3002_headless-provider-messaging.md)

### ITERMCLN-3003: Add Messaging Methods to ITermProvider
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Add sendMessage() and listAgents() using Python scripts
- **Dependencies**: ITERMCLN-3001, ITERMCLN-2001
- **File**: [ITERMCLN-3003_iterm-provider-messaging.md](./ITERMCLN-3003_iterm-provider-messaging.md)

---

## Phase 4: Multi-Agent Spawn (Low Risk)

### ITERMCLN-4001: Enable Multi-Agent Spawn with Comma Syntax
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Re-enable `crewchief spawn claude,gemini` command
- **Dependencies**: ITERMCLN-2001, ITERMCLN-2002
- **File**: [ITERMCLN-4001_multi-agent-spawn.md](./ITERMCLN-4001_multi-agent-spawn.md)

---

## Phase 5: Testing & Documentation (Low Risk)

### ITERMCLN-5001: Add Unit Tests for Terminal Providers
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Add tests for ITermProvider and HeadlessProvider
- **Dependencies**: ITERMCLN-2001, ITERMCLN-3002, ITERMCLN-3003
- **File**: [ITERMCLN-5001_provider-unit-tests.md](./ITERMCLN-5001_provider-unit-tests.md)

### ITERMCLN-5002: Update Documentation for Agent Commands
- **Status**: [ ] Pending
- **Agent**: general-development
- **Summary**: Update READMEs, remove bridge references
- **Dependencies**: ITERMCLN-1001, ITERMCLN-3002, ITERMCLN-4001
- **File**: [ITERMCLN-5002_update-documentation.md](./ITERMCLN-5002_update-documentation.md)

---

## Execution Order

Recommended ticket execution sequence:

1. **ITERMCLN-1001** - Delete Python dead code
2. **ITERMCLN-1002 + ITERMCLN-2001** - Delete TS dead code + Rewrite provider (commit together)
3. **ITERMCLN-2002** - Verify spawn works (checkpoint)
4. **ITERMCLN-3001** - Extend interface
5. **ITERMCLN-3002** - Headless messaging
6. **ITERMCLN-3003** - iTerm messaging
7. **ITERMCLN-4001** - Multi-agent spawn
8. **ITERMCLN-5001** - Unit tests
9. **ITERMCLN-5002** - Documentation

## Success Criteria

- [ ] `crewchief spawn claude` **WORKS** for iTerm users (was broken)
- [ ] `crewchief agent list` still works
- [ ] `crewchief agent message` still works for iTerm
- [ ] `crewchief agent message` works with headless agents (new)
- [ ] `crewchief spawn agent1,agent2` works (restored)
- [ ] ~1,750 lines of dead code removed
- [ ] Critical paths have test coverage

## Plan References

- [Analysis](../planning/analysis.md)
- [Architecture](../planning/architecture.md)
- [Plan](../planning/plan.md)
- [Quality Strategy](../planning/quality-strategy.md)
- [Security Review](../planning/security-review.md)
- [Project Review](../planning/project-review.md)
- [Review Updates](../planning/review-updates.md)
