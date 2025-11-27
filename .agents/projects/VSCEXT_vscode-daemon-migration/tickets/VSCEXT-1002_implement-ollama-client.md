# Ticket: VSCEXT-1002: Implement OllamaClient class

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
Create an HTTP client class for Ollama API operations, building on the existing `detectOllama()` pattern from setupWizard.ts. This enables automatic model management before watch starts.

## Background
The extension needs to check if Ollama is running and if the required embedding model is available. If the model is missing, it should be pulled automatically. The existing `detectOllama()` function in setupWizard.ts provides the detection pattern to extend.

Reference: planning/plan.md - Phase 1, Ticket 1002
Reference: planning/architecture.md - Ollama Model Management section

## Acceptance Criteria
- [ ] OllamaClient class created at `src/ollama/client.ts`
- [ ] `isRunning()` method detects running Ollama (2s timeout)
- [ ] `hasModel(name)` method checks model existence via `/api/tags` endpoint
- [ ] `pullModel(name, onProgress)` method streams progress via NDJSON callback
- [ ] Hardcoded to localhost:11434 (security requirement - not configurable)
- [ ] Model name validated with regex before API calls
- [ ] Unit tests with mocked HTTP pass

## Technical Requirements
- **Security**: Base URL MUST be hardcoded to `http://127.0.0.1:11434`
- Reuse pattern from `detectOllama()` in `src/ui/setupWizard.ts`
- Model name validation regex: `/^[a-z0-9][a-z0-9._-]*(?::[a-z0-9._-]+)?$/i`
- API endpoints:
  - `/api/tags` - GET, returns `{ models: [{ name: string }] }`
  - `/api/pull` - POST, body `{ name: string }`, streams NDJSON progress
- Use 2 second timeout for health checks
- Stream NDJSON for model pull progress

## Implementation Notes

```typescript
// src/ollama/client.ts
export class OllamaClient {
  // SECURITY: Hardcoded to localhost, not configurable
  private readonly baseUrl = 'http://127.0.0.1:11434'

  async isRunning(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/api/tags`, {
        signal: AbortSignal.timeout(2000)
      })
      return response.ok
    } catch {
      return false
    }
  }

  async hasModel(name: string): Promise<boolean> {
    const response = await fetch(`${this.baseUrl}/api/tags`)
    const data = await response.json()
    return data.models?.some((m: { name: string }) =>
      m.name === name || m.name === `${name}:latest`
    )
  }

  async pullModel(name: string, onProgress?: (status: string) => void): Promise<void> {
    // Validate model name format (SECURITY)
    if (!/^[a-z0-9][a-z0-9._-]*(?::[a-z0-9._-]+)?$/i.test(name)) {
      throw new Error('Invalid model name format')
    }

    const response = await fetch(`${this.baseUrl}/api/pull`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
    })

    // Stream NDJSON progress
    const reader = response.body!.getReader()
    const decoder = new TextDecoder()

    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      const text = decoder.decode(value)
      const lines = text.split('\n').filter(Boolean)
      for (const line of lines) {
        try {
          const event = JSON.parse(line)
          onProgress?.(event.status || 'Downloading...')
        } catch { /* ignore malformed lines */ }
      }
    }
  }
}
```

## Dependencies
- None (can be developed in parallel with VSCEXT-1001)

## Risk Assessment
- **Risk**: Ollama API format might change
  - **Mitigation**: Using standard, stable Ollama API endpoints
- **Risk**: NDJSON parsing edge cases
  - **Mitigation**: Graceful error handling for malformed lines

## Files/Packages Affected
- `packages/vscode-maproom/src/ollama/client.ts` - New file
- `packages/vscode-maproom/src/ollama/index.ts` - New barrel export
- `packages/vscode-maproom/src/ollama/client.test.ts` - Unit tests
