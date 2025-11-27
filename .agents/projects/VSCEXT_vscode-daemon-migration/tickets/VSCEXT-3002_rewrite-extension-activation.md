# Ticket: VSCEXT-3002: Rewrite extension activation

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
Rewrite the extension activation flow to remove Docker dependencies and integrate the new Ollama model management and startup reconciliation. The sync portion must complete in under 500ms.

## Background
The current activation flow starts Docker containers and spawns dual watch processes. The new flow should: check Ollama → ensure model → reconcile → start single watch. No Docker involvement.

Reference: planning/plan.md - Phase 3, Ticket 3002
Reference: planning/architecture.md - Simplified Extension Flow

## Acceptance Criteria
- [ ] No Docker containers started during activation
- [ ] Ollama model checked/pulled before watch (ollama provider only)
- [ ] Reconciliation runs before watch starts
- [ ] Activation sync portion completes < 500ms
- [ ] Background initialization shows progress in status bar
- [ ] Error states handled with user-friendly messages
- [ ] Non-Ollama providers (OpenAI, Google) skip model management

## Technical Requirements
- Remove all `ensureDockerRunning()` calls
- Add `ensureOllamaModel()` call (only for ollama provider)
- Add `reconcileChanges()` call before watch
- Update orchestrator instantiation for single watch
- Preserve fast sync activation (commands, status bar)

**New Activation Flow**:
```
activate() → fast sync setup → return (< 500ms)
  ↓ (background via setImmediate or setTimeout(0))
Check provider → Ensure Ollama model → Reconcile → Start watch → Ready
```

## Implementation Notes

```typescript
// src/extension.ts
export async function activate(context: vscode.ExtensionContext) {
  // 1. Fast sync setup (< 100ms)
  const outputChannel = vscode.window.createOutputChannel('Maproom')
  const statusBar = new StatusBarManager(context)
  statusBar.setState('starting')
  registerCommands(context)

  // 2. Background initialization (non-blocking)
  void initializeAsync(context, outputChannel, statusBar)
}

async function initializeAsync(
  context: vscode.ExtensionContext,
  outputChannel: vscode.OutputChannel,
  statusBar: StatusBarManager
) {
  try {
    // 1. Check/configure provider
    const provider = getConfiguredProvider(context)
    if (!provider) {
      const selectedProvider = await runSetupWizard(context)
      if (!selectedProvider) return // User cancelled
    }

    // 2. Ensure Ollama model (ONLY for ollama provider)
    if (provider === 'ollama') {
      await ensureOllamaModel('nomic-embed-text')
    }

    // 3. Run startup reconciliation
    statusBar.setState('reconciling')
    await reconcileChanges(context)

    // 4. Start unified watch process
    const orchestrator = new ProcessOrchestrator(outputChannel, {
      extensionRoot: context.extensionPath,
      workspaceRoot: getWorkspaceRoot(),
      databaseUrlOverride: getDatabaseUrl(),
      provider,
    })

    await orchestrator.startWatching()
    statusBar.setState('watching')
    statusBar.connectOrchestrator(orchestrator)

    // Store orchestrator for deactivation
    context.subscriptions.push({
      dispose: () => orchestrator.stop()
    })

  } catch (error) {
    statusBar.setState('error', error.message)

    if (error instanceof OllamaNotRunningError) {
      const action = await vscode.window.showErrorMessage(
        'Ollama is not running. Please start Ollama or install it.',
        'Install Ollama',
        'Retry'
      )
      if (action === 'Install Ollama') {
        vscode.env.openExternal(vscode.Uri.parse('https://ollama.ai'))
      } else if (action === 'Retry') {
        void initializeAsync(context, outputChannel, statusBar)
      }
    } else {
      vscode.window.showErrorMessage(`Maproom: ${error.message}`)
    }
  }
}
```

## Dependencies
- VSCEXT-1003 (Model management flow)
- VSCEXT-3001 (Startup reconciliation)

## Risk Assessment
- **Risk**: Breaking existing user workflows
  - **Mitigation**: Comprehensive testing, preserve SQLite guidance for first run
- **Risk**: Activation timeout in large workspaces
  - **Mitigation**: Background initialization keeps sync fast

## Files/Packages Affected
- `packages/vscode-maproom/src/extension.ts` - Main activation rewrite
- `packages/vscode-maproom/src/extension.test.ts` - Update integration tests
