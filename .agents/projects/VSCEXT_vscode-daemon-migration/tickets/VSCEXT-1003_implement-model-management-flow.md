# Ticket: VSCEXT-1003: Implement model management flow

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the orchestration flow that ensures the required Ollama model is available before the watch process starts. This includes VSCode progress notifications during model download.

## Background
When the extension activates with the Ollama provider, it needs to verify the embedding model exists and pull it if missing. This should happen transparently with a progress notification, not require user intervention.

Reference: planning/plan.md - Phase 1, Ticket 1003
Reference: planning/architecture.md - Simplified Extension Flow

## Acceptance Criteria
- [x] `ensureOllamaModel(modelName)` function created in `src/ollama/model-manager.ts`
- [x] Shows VSCode progress notification during model pull
- [x] Handles "Ollama not running" with helpful error and "Install Ollama" link
- [x] Handles network errors with retry option
- [x] Skips pull gracefully if model already exists
- [x] Custom error types for different failure modes

## Technical Requirements
- Create `OllamaNotRunningError` custom error class
- Use `vscode.window.withProgress` for download notifications
- Progress location: `vscode.ProgressLocation.Notification`
- Install link: `https://ollama.ai`
- Default model: `nomic-embed-text`

## Implementation Notes

```typescript
// src/ollama/model-manager.ts
import * as vscode from 'vscode'
import { OllamaClient } from './client'

export class OllamaNotRunningError extends Error {
  constructor() {
    super('Ollama is not running')
    this.name = 'OllamaNotRunningError'
  }
}

export async function ensureOllamaModel(modelName: string): Promise<void> {
  const client = new OllamaClient()

  if (!await client.isRunning()) {
    throw new OllamaNotRunningError()
  }

  if (await client.hasModel(modelName)) {
    return // Model already exists
  }

  await vscode.window.withProgress({
    location: vscode.ProgressLocation.Notification,
    title: 'Downloading embedding model...',
    cancellable: false,
  }, async (progress) => {
    await client.pullModel(modelName, (status) => {
      progress.report({ message: status })
    })
  })
}

// Error handling in extension.ts:
// catch (error) {
//   if (error instanceof OllamaNotRunningError) {
//     const action = await vscode.window.showErrorMessage(
//       'Ollama is not running. Please start Ollama or install it.',
//       'Install Ollama',
//       'Retry'
//     )
//     if (action === 'Install Ollama') {
//       vscode.env.openExternal(vscode.Uri.parse('https://ollama.ai'))
//     }
//   }
// }
```

## Dependencies
- VSCEXT-1002 (OllamaClient class)

## Risk Assessment
- **Risk**: Model download takes too long
  - **Mitigation**: Progress notification keeps user informed; download is one-time
- **Risk**: User closes VSCode during download
  - **Mitigation**: Model download continues server-side; next activation retries

## Files/Packages Affected
- `packages/vscode-maproom/src/ollama/model-manager.ts` - New file
- `packages/vscode-maproom/src/ollama/errors.ts` - New file for custom errors
- `packages/vscode-maproom/src/ollama/index.ts` - Update barrel exports
- `packages/vscode-maproom/src/ollama/model-manager.test.ts` - Unit tests
