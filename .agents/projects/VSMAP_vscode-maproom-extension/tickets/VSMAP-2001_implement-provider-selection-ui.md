# Ticket: VSMAP-2001: Implement provider selection QuickPick UI

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create setup wizard with QuickPick for selecting embedding provider (Ollama/OpenAI/Google). Detect running Ollama instance and show recommendations.

## Background
This implements Phase 2 (Setup Wizard) of the VSMAP plan. Users need a first-run configuration experience to select their embedding provider. The wizard should intelligently detect if Ollama is running locally and recommend it as the preferred option. This is the entry point for the setup flow, which continues with credential storage (VSMAP-2002) and initial scan (VSMAP-2003).

Reference: VSMAP_PLAN.md Phase 2 "Setup Wizard - Provider Selection"

## Acceptance Criteria
- [ ] QuickPick shown on first activation when no saved config exists
- [ ] Three provider options displayed: Ollama (recommended if running), OpenAI, Google
- [ ] Selection persisted to VSCode workspace settings
- [ ] Ollama detection works via HTTP ping to localhost:11434
- [ ] Setup wizard re-runnable via command palette command `Maproom: Setup`
- [ ] QuickPick items include detail text with provider descriptions

## Technical Requirements
- Use `vscode.window.showQuickPick()` API for provider selection
- Implement Ollama detection by checking `http://localhost:11434` (HTTP request)
- Save selected provider to workspace state: `workspaceState.update('maproom.provider', selected)`
- QuickPick items should include label, detail, and description properties
- Command registered in package.json: `maproom.setup`
- Handle network errors gracefully during Ollama detection (timeout 2s)

## Implementation Notes
The setup wizard should be friendly and informative. QuickPick structure:
```typescript
{
  label: "$(zap) Ollama (Recommended)",
  detail: "Running locally - fast and private",
  value: "ollama"
}
```

For Ollama detection, use a simple HTTP GET request with short timeout. If Ollama responds, mark it as recommended in the QuickPick. If detection fails or times out, still show Ollama as an option but without the "recommended" badge.

The wizard should be invocable both:
1. Automatically on first activation (check workspace state for existing config)
2. Manually via command palette (`Maproom: Setup`)

Store provider selection in workspace state (not global settings) so different workspaces can use different providers.

## Dependencies
- VSMAP-1006 (extension activation wiring) must be complete
- No external dependencies

## Risk Assessment
- **Risk**: Ollama detection may fail due to firewall/network issues
  - **Mitigation**: Use short timeout (2s) and fallback to showing all options without recommendation
- **Risk**: Users may select provider without understanding requirements (API keys)
  - **Mitigation**: Include clear detail text in QuickPick items explaining what's needed

## Files/Packages Affected
- `src/ui/setupWizard.ts` (new file, ~100 lines)
- `src/extension.ts` (integrate wizard on activation)
- `package.json` (add `maproom.setup` command)
- `src/test/setupWizard.test.ts` (new test file)
