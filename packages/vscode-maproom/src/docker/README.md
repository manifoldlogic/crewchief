# Docker Manager for VSCode Extension

Robust Docker Compose service management for the Maproom VSCode extension.

## Overview

The `DockerManager` class handles the complete lifecycle of Docker Compose services required by Maproom:

- **PostgreSQL database** (`maproom-postgres`) - Vector database with pgvector
- **Ollama** (`maproom-ollama`) - Embedding generation service
- **MCP Server** (`maproom-mcp`) - Model Context Protocol server

## Features

### Robust Process Management

- **Idempotent operations**: Safe to call `ensureServicesRunning()` multiple times
- **Graceful shutdown**: SIGTERM → SIGKILL cascade with 5s timeout
- **Event listener cleanup**: No memory leaks from orphaned listeners
- **Comprehensive error handling**: Specific error codes for different failure modes

### Health Checking

- **Exponential backoff**: 1s, 2s, 4s, 8s, 16s delays between attempts
- **PostgreSQL health**: Uses `pg_isready` inside container
- **Timeout protection**: 30s total timeout prevents indefinite hangs
- **Detailed logging**: Every health check attempt logged to output channel

### Error Reporting

- **Docker daemon detection**: Clear message when Docker is not running
- **Binary not found**: Helpful message to install Docker Desktop
- **User-friendly errors**: All errors logged to VSCode output channel
- **Structured error types**: `DockerError` with error codes

## Usage

### Basic Integration

```typescript
import * as vscode from 'vscode'
import { DockerManager, DockerError } from './docker/manager.js'

export async function activate(context: vscode.ExtensionContext) {
  const outputChannel = vscode.window.createOutputChannel('Maproom Docker')
  const dockerManager = new DockerManager(outputChannel, context.extensionPath)

  // Start services
  try {
    await dockerManager.ensureServicesRunning()
    vscode.window.showInformationMessage('Maproom ready!')
  } catch (error) {
    if (error instanceof DockerError) {
      vscode.window.showErrorMessage(error.message)
    }
  }

  // Register cleanup
  context.subscriptions.push(
    new vscode.Disposable(async () => {
      await dockerManager.stop()
    })
  )
}
```

### Error Handling

```typescript
try {
  await dockerManager.ensureServicesRunning()
} catch (error) {
  if (error instanceof DockerError) {
    switch (error.code) {
      case 'DOCKER_NOT_FOUND':
        // Docker not installed
        showInstallDockerMessage()
        break
      case 'DOCKER_DAEMON_NOT_RUNNING':
        // Docker Desktop not running
        showStartDockerMessage()
        break
      case 'HEALTH_CHECK_TIMEOUT':
        // Services didn't become healthy
        showHealthCheckFailedMessage()
        break
      default:
        // Other errors
        showGenericError(error.message)
    }
  }
}
```

### With Progress Notification

```typescript
await vscode.window.withProgress(
  {
    location: vscode.ProgressLocation.Notification,
    title: 'Starting Maproom services...',
    cancellable: false,
  },
  async (progress) => {
    progress.report({ message: 'Checking Docker...' })
    await dockerManager.ensureServicesRunning()
    progress.report({ message: 'Services ready!' })
  }
)
```

## API Reference

### `DockerManager`

```typescript
class DockerManager {
  constructor(outputChannel: OutputChannel, extensionRoot?: string)

  /**
   * Ensure services are running (idempotent)
   * @throws DockerError if startup fails
   */
  ensureServicesRunning(): Promise<void>

  /**
   * Stop all services gracefully
   * @throws DockerError if shutdown fails
   */
  stop(): Promise<void>
}
```

### `DockerError`

```typescript
class DockerError extends Error {
  code: string        // Error code (e.g., 'DOCKER_NOT_FOUND')
  exitCode?: number   // Process exit code if applicable
  stderr?: string     // stderr output from command
}
```

### Error Codes

| Code | Description | User Action |
|------|-------------|-------------|
| `DOCKER_NOT_FOUND` | Docker command not in PATH | Install Docker Desktop |
| `DOCKER_DAEMON_NOT_RUNNING` | Docker daemon not running | Start Docker Desktop |
| `DOCKER_CHECK_FAILED` | Could not check Docker status | Check Docker installation |
| `COMPOSE_UP_FAILED` | docker compose up failed | Check logs, verify config |
| `COMPOSE_DOWN_FAILED` | docker compose down failed | Check logs |
| `HEALTH_CHECK_TIMEOUT` | Services didn't become healthy | Check container logs |
| `HEALTH_CHECK_FAILED` | Health checks exhausted | Check PostgreSQL logs |
| `TIMEOUT` | Command timed out | Increase timeout or check system |
| `SPAWN_ERROR` | Failed to spawn process | Check system resources |

## Implementation Details

### Process Spawning

- Uses `spawn()` not `exec()` for streaming output
- Explicit `stdio: ['ignore', 'pipe', 'pipe']` for control
- No `shell: true` to avoid security issues
- Environment variables passed through from parent process

### Signal Handling

```typescript
// 1. Send SIGTERM for graceful shutdown
child.kill('SIGTERM')

// 2. Wait 5 seconds
setTimeout(() => {
  // 3. Force kill if still running
  if (!child.killed) {
    child.kill('SIGKILL')
  }
}, 5000)
```

### Health Check Algorithm

```typescript
for (let attempt = 1; attempt <= 10; attempt++) {
  // Check total timeout (30s)
  if (elapsed >= 30000) throw timeout error

  // Try health check
  const result = await dockerExec('pg_isready ...')
  if (result.code === 0) return healthy

  // Exponential backoff: min(1000 * 2^(attempt-1), 16000)
  const delay = Math.min(1000 * Math.pow(2, attempt - 1), 16000)
  await sleep(delay)
}
throw health check failed
```

### Resource Cleanup

Every spawned process:
1. Captures stdout/stderr via event handlers
2. Registers 'close' and 'error' handlers
3. Removes ALL event listeners after completion
4. Clears timeout handles
5. No orphaned processes or memory leaks

## Testing

```bash
# Run tests (requires Docker running)
pnpm test src/docker/manager.test.ts

# Watch mode
pnpm test:watch src/docker/manager.test.ts
```

Tests verify:
- Initialization and logging
- Service startup and health checks
- Idempotent operations
- Graceful shutdown
- Error handling and error codes

## Configuration

The manager expects:
- **docker-compose.yml** at `config/docker-compose.yml` (relative to extension root)
- **Docker command** in system PATH
- **PostgreSQL container** named `maproom-postgres`

### Custom Extension Root

```typescript
// Auto-detected (default)
const manager = new DockerManager(outputChannel)

// Explicit path
const manager = new DockerManager(outputChannel, '/path/to/extension')
```

## Platform Support

- **Linux**: Full support
- **macOS**: Full support
- **Windows**: Full support (uses Docker Desktop)

SIGTERM/SIGKILL work on all platforms. Windows uses Docker Desktop's Linux VM internally.

## Troubleshooting

### "Docker command not found"

**Cause**: Docker not installed or not in PATH

**Solution**:
1. Install Docker Desktop
2. Ensure `docker` is in PATH
3. Restart VSCode

### "Docker daemon is not running"

**Cause**: Docker Desktop not started

**Solution**:
1. Launch Docker Desktop
2. Wait for it to fully start
3. Retry operation

### Health check timeout

**Cause**: Services taking longer than 30s to start

**Solutions**:
1. Check Docker Desktop has enough resources (CPU/RAM)
2. Check container logs: `docker logs maproom-postgres`
3. Verify docker-compose.yml is correct
4. Try `docker compose down && docker compose up -d` manually

### Permission denied

**Cause**: Docker requires sudo on Linux

**Solution**:
1. Add user to `docker` group: `sudo usermod -aG docker $USER`
2. Log out and log back in
3. Verify: `docker ps` should work without sudo

## See Also

- [example-usage.ts](./example-usage.ts) - Complete integration examples
- [manager.test.ts](./manager.test.ts) - Test suite
- [../../config/docker-compose.yml](../../config/docker-compose.yml) - Service configuration
