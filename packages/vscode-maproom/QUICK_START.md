# DockerManager Quick Start

## Installation

The DockerManager is ready to use - no installation needed beyond having the files.

## Basic Usage (30 seconds)

```typescript
import * as vscode from 'vscode'
import { DockerManager } from './src/docker/index.js'

// In your activate() function:
const outputChannel = vscode.window.createOutputChannel('Maproom')
const docker = new DockerManager(outputChannel, context.extensionPath)

// Start services
await docker.ensureServicesRunning()  // Idempotent - safe to call multiple times

// Stop services (call in deactivate or cleanup)
await docker.stop()
```

## With Error Handling (60 seconds)

```typescript
import { DockerManager, DockerError } from './src/docker/index.js'

try {
  await docker.ensureServicesRunning()
  vscode.window.showInformationMessage('Services ready!')
} catch (error) {
  if (error instanceof DockerError) {
    // Show user-friendly error based on code
    switch (error.code) {
      case 'DOCKER_NOT_FOUND':
        vscode.window.showErrorMessage('Install Docker Desktop')
        break
      case 'DOCKER_DAEMON_NOT_RUNNING':
        vscode.window.showErrorMessage('Start Docker Desktop')
        break
      default:
        vscode.window.showErrorMessage(error.message)
    }
  }
}
```

## Complete Extension Integration (2 minutes)

```typescript
import * as vscode from 'vscode'
import { DockerManager, DockerError } from './src/docker/index.js'

export async function activate(context: vscode.ExtensionContext) {
  const outputChannel = vscode.window.createOutputChannel('Maproom')
  const docker = new DockerManager(outputChannel, context.extensionPath)

  // Optional: Show progress notification
  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: 'Starting Maproom...',
    },
    async () => await docker.ensureServicesRunning()
  )

  // Register cleanup
  context.subscriptions.push(
    new vscode.Disposable(async () => {
      await docker.stop()
    })
  )

  return { docker }
}

export async function deactivate() {
  // Cleanup handled by Disposable
}
```

## Testing Your Integration

1. **Install Docker Desktop** (if not already installed)

2. **Start Docker Desktop**

3. **Run your extension** (F5 in VSCode)

4. **Check the output channel**:
   - View → Output
   - Select "Maproom" from dropdown
   - Look for "All services are healthy and ready"

5. **Verify services are running**:
   ```bash
   docker ps
   # Should show: maproom-postgres, maproom-ollama, maproom-mcp
   ```

## Troubleshooting (1 minute)

### "Docker command not found"
→ Install Docker Desktop, restart VSCode

### "Docker daemon is not running"
→ Start Docker Desktop, wait for it to be ready

### "Health check timeout"
→ Check Docker Desktop has enough resources (Settings → Resources)

### Services stuck starting
```bash
# Check logs
docker logs maproom-postgres
docker logs maproom-ollama
docker logs maproom-mcp

# Reset everything
docker compose -f config/docker-compose.yml down
```

## Files Reference

- **Implementation**: `src/docker/manager.ts`
- **Tests**: `src/docker/manager.test.ts`
- **Examples**: `src/docker/example-usage.ts`
- **Docs**: `src/docker/README.md`
- **Config**: `config/docker-compose.yml`

## Key Features

- ✓ Idempotent (safe to call multiple times)
- ✓ Health checks with exponential backoff
- ✓ Clear error messages with codes
- ✓ Graceful shutdown (SIGTERM → SIGKILL)
- ✓ No process leaks
- ✓ Cross-platform (Linux, macOS, Windows)

## What Services Are Started?

1. **PostgreSQL** (port 5433) - Vector database with pgvector
2. **Ollama** (port 11434) - Embedding generation
3. **Maproom MCP** - Semantic search server

All services are configured with health checks and will auto-restart if they crash.

## Next Steps

1. Read `src/docker/README.md` for complete API docs
2. Check `src/docker/example-usage.ts` for advanced patterns
3. Run tests: `pnpm test src/docker/manager.test.ts`
4. Integrate into your extension's activate() function

## Need Help?

- Check output channel for detailed logs
- Read the full README: `src/docker/README.md`
- Check implementation summary: `IMPLEMENTATION_SUMMARY.md`
- Review test cases: `src/docker/manager.test.ts`
