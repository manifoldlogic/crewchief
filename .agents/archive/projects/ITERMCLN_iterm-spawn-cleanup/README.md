# ITERMCLN: iTerm Spawn Command Cleanup

**Status**: Complete ✅
**Completed**: 2025-11-27
**Tickets**: 10 tickets (all verified)

## Summary

**FIX** the broken iTerm spawn command and clean up the CrewChief CLI's agent spawning system by removing ~1,750 lines of dead JSON-RPC code, fixing the terminal provider, and adding headless messaging support.

## Critical Finding

**`crewchief spawn claude` is BROKEN for iTerm users.** The spawn command attempts to start a non-functional JSON-RPC bridge and fails after 30-second timeout. This project fixes that bug while also cleaning up the dead code.

## Problem Statement

The codebase contains two parallel implementations for agent terminal management:

1. **JSON-RPC Bridge** (abandoned, BROKEN) - Complex Python bridge server with HTTP RPC, never worked
2. **Direct Script Calls** (working) - Simple synchronous Python script invocation (used by agent.ts)

This creates:
- **BROKEN** spawn command for iTerm users (30-second timeout)
- Confusion about which code paths are active
- ~1,750 lines of dead code
- No headless messaging support
- Disabled multi-agent spawn feature

## Proposed Solution

1. **FIX spawn** - Rewrite ITermProvider to use direct script calls (like ITermSimpleService)
2. **Remove dead code** - Delete all JSON-RPC related files (~1,750 lines)
3. **Add headless messaging** - Enable `agent message` for headless agents via stdin pipe
4. **Re-enable multi-agent** - Allow `crewchief spawn claude,gemini`
5. **Add tests** - Cover critical spawn/message/list paths

## Scope

### In Scope
- TypeScript terminal provider cleanup
- Python scripts cleanup
- Headless provider messaging
- Multi-agent spawn re-enablement
- Basic test coverage

### Out of Scope
- Agent CLI changes (claude, gemini, etc.)
- New orchestration features
- Cross-platform terminal support beyond iTerm2/headless

## Phases

| Phase | Description | Risk | Key Outcome |
|-------|-------------|------|-------------|
| 1 | Dead Code Removal | Medium | Remove broken bridge code |
| 2 | ITermProvider Fix | Medium-High | **FIX broken spawn** |
| 3 | Headless Messaging | Medium | stdin-based messaging |
| 4 | Multi-Agent Spawn | Low | comma-separated agents |
| 5 | Testing & Docs | Low | regression safety |

**Total**: 10 tickets

**Critical Path**: Phase 1 → Phase 2 must be done together (spawn is broken until Phase 2)

## Key Metrics

- **Bug Fixed**: `crewchief spawn claude` works again
- **Lines Removed**: ~1,750 (TypeScript + Python)
- **Files Deleted**: ~13
- **New Capability**: Headless agent messaging via stdin
- **Restored Feature**: Multi-agent spawn

## Tickets

- [Ticket Index](./tickets/ITERMCLN_TICKET_INDEX.md) - All tickets with status and dependencies

## Planning Documents

- [Analysis](./planning/analysis.md) - Problem space and current state
- [Architecture](./planning/architecture.md) - Target design and decisions
- [Quality Strategy](./planning/quality-strategy.md) - Testing approach
- [Security Review](./planning/security-review.md) - Risk assessment
- [Plan](./planning/plan.md) - Phased execution plan
- [Project Review](./planning/project-review.md) - Critical review findings
- [Review Updates](./planning/review-updates.md) - Changes made based on review

## Relevant Agents

No specialized agents required. General TypeScript/Python development work.

## Dependencies

- None (self-contained cleanup project)

## Success Criteria (All Met ✅)

- [x] `crewchief spawn claude` **WORKS** for iTerm users (was broken)
- [x] `crewchief agent list` still works
- [x] `crewchief agent message` still works for iTerm
- [x] `crewchief agent message` works with headless agents (new)
- [x] `crewchief spawn agent1,agent2` works (restored)
- [x] ~1,750 lines of dead code removed
- [x] Critical paths have test coverage
