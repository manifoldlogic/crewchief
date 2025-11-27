# Ticket: VSMAP-0001: Create process-management-specialist agent

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (agent definition, no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create a specialized agent for spawning and monitoring child processes in Node.js. This agent will handle all interaction with the `crewchief-maproom` Rust binary (spawning watch/branch-watch/scan processes, parsing NDJSON stdout, crash recovery).

## Background
The VSMAP extension spawns long-running Rust processes (`crewchief-maproom watch` and `branch-watch`) and parses their stdout to update the UI. We need an agent skilled in Node.js process management, NDJSON parsing, and error recovery patterns.

This ticket is part of Phase 0 (Agent Creation) of the VSMAP project, which creates the specialized agents needed before implementation begins.

## Acceptance Criteria
- [x] Agent definition file created at `.claude/agents/specialized/process-management-specialist.md`
- [x] Agent includes training on child_process.spawn() API
- [x] Agent includes NDJSON parsing examples
- [x] Agent includes exponential backoff patterns
- [x] Agent includes platform-specific binary selection logic
- [x] Agent can be invoked via Task tool

## Technical Requirements
- Follow agent template from `.agents/reference/agent-template.md`
- Include Node.js child_process examples
- Document NDJSON parsing approach
- Include crash recovery patterns (exponential backoff, circuit breaker)
- Document platform detection (process.platform, process.arch)
- Include stdout/stderr stream handling examples

## Implementation Notes
- Reference `packages/cli/src/git/worktrees.ts` for existing binary spawning pattern (lines 25-92)
- The agent should understand Rust binary outputs NDJSON format
- Include examples of graceful process shutdown
- Document environment variable passing to child processes
- Focus on long-running process management, not one-off command execution
- Include signal handling for cleanup (SIGTERM, SIGINT)

## Dependencies
None

## Risk Assessment
- **Risk**: Agent may not understand NDJSON parsing specifics
  - **Mitigation**: Include concrete examples in training data with sample input/output

## Files/Packages Affected
- `.claude/agents/specialized/process-management-specialist.md` (new file)
