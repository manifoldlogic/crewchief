# Ticket: VSMAP-0002: Create vscode-extension-specialist agent

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (agent definition, no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create a specialized agent for VSCode Extension API work. This agent will handle extension activation, status bar updates, SecretStorage integration, QuickPick UI, and VSIX packaging.

## Background
The VSMAP extension needs VSCode-specific UI components (status bar, setup wizard) and integration with VSCode APIs (SecretStorage, extension lifecycle). We need an agent skilled in the VSCode Extension API with focus on performance (<500ms activation).

This ticket is part of Phase 0 (Agent Creation) of the VSMAP project, which creates the specialized agents needed before implementation begins.

## Acceptance Criteria
- [x] Agent definition file created at `.claude/agents/specialized/vscode-extension-specialist.md`
- [x] Agent includes training on VSCode Extension API
- [x] Agent includes StatusBarItem examples
- [x] Agent includes SecretStorage usage patterns
- [x] Agent includes QuickPick UI examples
- [x] Agent includes extension activation best practices
- [x] Agent can be invoked via Task tool

## Technical Requirements
- Follow agent template from `.agents/reference/agent-template.md`
- Include VSCode Extension API documentation references
- Document activation performance requirements (<500ms)
- Include SecretStorage API examples
- Document QuickPick and UI patterns
- Include VSIX packaging steps

## Implementation Notes
- The agent should NOT include FileSystemWatcher knowledge (Rust binary handles file watching)
- Focus on simple orchestration, not complex state machines
- Include examples of lazy loading for performance
- Document proper extension deactivation cleanup
- Include activation event patterns (e.g., `onStartupFinished`)
- Document configuration management (workspace vs user settings)
- Include examples of output channel logging for debugging

## Dependencies
None

## Risk Assessment
- **Risk**: Agent may try to implement file watching (out of scope)
  - **Mitigation**: Explicitly state Rust binary handles watching in agent definition
- **Risk**: Agent may suggest complex patterns when simple orchestration is needed
  - **Mitigation**: Emphasize thin wrapper architecture in agent definition

## Files/Packages Affected
- `.claude/agents/specialized/vscode-extension-specialist.md` (new file)
