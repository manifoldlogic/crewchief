# Ticket: VSMAP-0003: Test agents with simple extension creation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Validate both agents by having them create a simple "Hello World" VSCode extension with status bar and a spawned echo process. This verifies agents understand their domains before using them on VSMAP.

## Background
Before using new agents on the real project, we should test them with a simple task to verify they understand VSCode Extension API and process spawning patterns correctly. This throwaway test extension validates that both specialized agents can execute independently.

This ticket is part of Phase 0 (Agent Creation) of the VSMAP project, which validates the specialized agents before implementation begins.

## Acceptance Criteria
- [x] Simple test extension created in `test/agents/hello-world/`
- [x] Extension activates in <500ms
- [x] Status bar shows "Hello World"
- [x] Extension spawns `echo "test"` process successfully
- [x] Process stdout parsed and logged
- [x] Test extension can be packaged as VSIX
- [x] Both agents successfully complete their test tasks

## Technical Requirements
- Use vscode-extension-specialist to create extension scaffold
- Use process-management-specialist to add process spawning
- Test extension must follow VSCode Extension API best practices
- Must use TypeScript
- Must include package.json with activation events
- Must demonstrate proper cleanup on deactivation

## Implementation Notes
- This is a throwaway test extension (not the real VSMAP extension)
- Focus: Can agents execute independently?
- Success metric: Extension works as specified without human intervention
- Delete test extension after validation
- The extension should use `echo "test"` as the spawned process (simple, cross-platform)
- Verify status bar updates after process completes
- Include basic error handling for process spawn failures

## Dependencies
- VSMAP-0001 (process-management-specialist must exist)
- VSMAP-0002 (vscode-extension-specialist must exist)

## Risk Assessment
- **Risk**: Agents may not work correctly on first try
  - **Mitigation**: This test catches issues before main implementation
- **Risk**: Extension may not activate properly in test environment
  - **Mitigation**: Use minimal activation event (onStartupFinished) and test manually if needed

## Files/Packages Affected
- `test/agents/hello-world/package.json` (new file)
- `test/agents/hello-world/src/extension.ts` (new file)
- `test/agents/hello-world/tsconfig.json` (new file)
- `test/agents/hello-world/.vscodeignore` (new file)
