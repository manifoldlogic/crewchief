# VSCEXT Ticket Index

**Project**: VSCode Extension Daemon Migration
**Created**: 2025-11-27
**Total Tickets**: 12

## Phase 1: Event Types & Ollama Client (3 tickets)

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| VSCEXT-1001 | Add BranchSwitchedEvent to events.ts | Pending | None |
| VSCEXT-1002 | Implement OllamaClient class | Pending | None |
| VSCEXT-1003 | Implement model management flow | Pending | VSCEXT-1002 |

## Phase 2: ProcessOrchestrator Refactor (2 tickets)

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| VSCEXT-2001 | Refactor ProcessOrchestrator for single watch | Pending | VSCEXT-1001 |
| VSCEXT-2002 | Update StatusBarManager integration | Pending | VSCEXT-2001 |

## Phase 3: Extension Flow Update (3 tickets)

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| VSCEXT-3001 | Implement startup reconciliation | Pending | VSCEXT-2001 |
| VSCEXT-3002 | Rewrite extension activation | Pending | VSCEXT-1003, VSCEXT-3001 |
| VSCEXT-3003 | Update setup wizard | Pending | VSCEXT-3002 |

## Phase 4: Cleanup (2 tickets)

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| VSCEXT-4001 | Remove Docker code | Pending | VSCEXT-3002 |
| VSCEXT-4002 | Remove PostgreSQL code and settings | Pending | VSCEXT-4001 |

## Phase 5: Testing & Verification (2 tickets)

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| VSCEXT-5001 | Unit and integration tests | Pending | VSCEXT-4002 |
| VSCEXT-5002 | Manual testing and verification | Pending | VSCEXT-5001 |

## Dependency Graph

```
Phase 1 (parallel start):
  VSCEXT-1001 (BranchSwitchedEvent) ──┐
  VSCEXT-1002 (OllamaClient) ─────────┼──▶ VSCEXT-1003 (Model management)
                                      │
Phase 2:                              │
  VSCEXT-1001 ──▶ VSCEXT-2001 (ProcessOrchestrator) ──▶ VSCEXT-2002 (StatusBar)
                                      │
Phase 3:                              │
  VSCEXT-2001 ──▶ VSCEXT-3001 (Reconciliation) ─┐
  VSCEXT-1003 ──────────────────────────────────┴──▶ VSCEXT-3002 (Activation) ──▶ VSCEXT-3003 (Setup)

Phase 4:
  VSCEXT-3002 ──▶ VSCEXT-4001 (Docker removal) ──▶ VSCEXT-4002 (PostgreSQL removal)

Phase 5:
  VSCEXT-4002 ──▶ VSCEXT-5001 (Tests) ──▶ VSCEXT-5002 (Manual testing)
```

## Plan Reference

See [planning/plan.md](../planning/plan.md) for detailed implementation specifications.

## Agent Assignments

| Agent | Tickets |
|-------|---------|
| vscode-extension-specialist | VSCEXT-1001 through VSCEXT-4002 |
| unit-test-runner | VSCEXT-5001 |
| verify-ticket | All tickets |
| commit-ticket | All tickets |
