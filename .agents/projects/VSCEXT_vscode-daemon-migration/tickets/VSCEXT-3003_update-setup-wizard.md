# Ticket: VSCEXT-3003: Update setup wizard

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Simplify the setup wizard for the SQLite + Ollama flow. Remove any PostgreSQL provider references and Docker-dependent flows while preserving the existing first-run user guidance.

## Background
The setup wizard previously handled PostgreSQL configuration and Docker setup. With the move to SQLite-only, the wizard should focus on embedding provider selection (Ollama, OpenAI, Google) and validation.

Reference: planning/plan.md - Phase 3, Ticket 3003
Reference: planning/analysis.md - Reusable Extension Infrastructure

## Acceptance Criteria
- [ ] Setup wizard works for Ollama, OpenAI, and Google providers
- [ ] No Docker or PostgreSQL references in wizard
- [ ] First run shows SQLite guidance (preserve `showNoSqliteGuidance()`)
- [ ] Re-run setup works correctly (command palette: "Maproom: Setup")
- [ ] Provider validation before saving selection
- [ ] API key input for OpenAI/Google providers

## Technical Requirements
- Keep: `showNoSqliteGuidance()` function for first-run UX
- Keep: `detectOllama()` function (now part of OllamaClient)
- Remove: PostgreSQL provider option (if present)
- Remove: Docker health check calls
- Remove: Container startup prompts

**Wizard Flow**:
```
1. Select provider (Ollama / OpenAI / Google)
2. If Ollama: validate running → validate model
3. If OpenAI/Google: prompt for API key → validate key
4. Save configuration
5. Return provider selection
```

## Implementation Notes
1. Review current `setupWizard.ts` implementation
2. Remove Docker-related imports and calls
3. Simplify provider selection to three options
4. For Ollama: can use new `OllamaClient.isRunning()` instead of `detectOllama()`
5. For API key providers: use existing `SecretsManager` for secure storage
6. Ensure `showNoSqliteGuidance()` is still called on first run

```typescript
// Simplified wizard structure
export async function runSetupWizard(context: vscode.ExtensionContext): Promise<string | undefined> {
  // Show guidance on first run
  if (!hasConfiguredProvider(context)) {
    await showNoSqliteGuidance()
  }

  // Provider selection
  const provider = await vscode.window.showQuickPick([
    { label: 'Ollama', description: 'Local embeddings (free)', value: 'ollama' },
    { label: 'OpenAI', description: 'Cloud embeddings (requires API key)', value: 'openai' },
    { label: 'Google', description: 'Cloud embeddings (requires API key)', value: 'google' },
  ], { placeHolder: 'Select embedding provider' })

  if (!provider) return undefined

  // Provider-specific validation
  if (provider.value === 'ollama') {
    const client = new OllamaClient()
    if (!await client.isRunning()) {
      // Show error with install link
      return undefined
    }
  } else {
    // Prompt for API key
    const apiKey = await vscode.window.showInputBox({
      prompt: `Enter your ${provider.label} API key`,
      password: true,
    })
    if (!apiKey) return undefined

    await secretsManager.setApiKey(provider.value, apiKey)
  }

  // Save provider selection
  await context.globalState.update('maproom.provider', provider.value)
  return provider.value
}
```

## Dependencies
- VSCEXT-3002 (Rewritten activation calls setup wizard)

## Risk Assessment
- **Risk**: Existing users with PostgreSQL config lose access
  - **Mitigation**: SQLite is a fresh index; no data migration needed
- **Risk**: API key validation fails silently
  - **Mitigation**: Show clear error messages for invalid keys

## Files/Packages Affected
- `packages/vscode-maproom/src/ui/setupWizard.ts` - Simplify wizard
- `packages/vscode-maproom/src/ui/setupWizard.test.ts` - Update tests
