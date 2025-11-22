# @maproom/daemon-client

TypeScript client library for communicating with the `crewchief-maproom` daemon via JSON-RPC 2.0.

## Features

- 🚀 **High Performance**: Eliminates process spawning overhead (50-100x faster for warm requests)
- 🔄 **Auto-Restart**: Automatically restarts daemon on crash with exponential backoff
- 🏥 **Health Checking**: Built-in health checks and timeout handling
- 📦 **Type-Safe**: Full TypeScript support with comprehensive type definitions
- 🧪 **Well-Tested**: Extensive unit and integration tests

## Installation

```bash
npm install @maproom/daemon-client
```

## Quick Start

```typescript
import { DaemonClient } from '@maproom/daemon-client'

// Create client
const client = new DaemonClient({
  binaryPath: '/path/to/crewchief-maproom',
  env: {
    MAPROOM_DATABASE_URL: 'postgresql://localhost/maproom',
    OPENAI_API_KEY: 'sk-...',
  },
  timeout: 30000, // 30s timeout
  autoRestart: true,
})

// Search (daemon auto-starts on first request)
const results = await client.search({
  query: 'function parseConfig',
  repo: 'my-repo',
  worktree: 'main',
  limit: 10,
})

console.log(`Found ${results.total} results`)
for (const hit of results.hits) {
  console.log(`${hit.file_path}:${hit.start_line} (score: ${hit.score})`)
}

// Cleanup when done
await client.stop()
```

## API Reference

### `DaemonClient`

Main class for daemon communication.

#### Constructor

```typescript
new DaemonClient(config: DaemonConfig)
```

**Config options:**
- `binaryPath` (string, required): Path to `crewchief-maproom` binary
- `env` (object, optional): Environment variables for daemon
- `timeout` (number, optional): Request timeout in ms (default: 30000)
- `startTimeout` (number, optional): Daemon start timeout in ms (default: 5000)
- `shutdownTimeout` (number, optional): Graceful shutdown timeout in ms (default: 5000)
- `autoRestart` (boolean, optional): Auto-restart on crash (default: true)
- `maxRestartAttempts` (number, optional): Max restart attempts (default: 5)
- `restartBackoffMs` (number, optional): Initial backoff delay in ms (default: 1000)

#### Methods

##### `search(params: SearchParams): Promise<SearchResult>`

Perform vector search.

**Parameters:**
- `query` (string): Search query text
- `repo` (string): Repository name
- `worktree` (string, optional): Worktree name to filter results
- `limit` (number, optional): Number of results (default: 10)
- `threshold` (number, optional): Similarity threshold 0.0-1.0
- `debug` (boolean, optional): Enable debug output

**Returns:** `SearchResult` with hits array

##### `ping(): Promise<string>`

Health check (returns "pong").

##### `start(): Promise<void>`

Explicitly start daemon (optional - auto-starts on first request).

##### `stop(): Promise<void>`

Stop daemon gracefully.

##### `restart(): Promise<void>`

Restart daemon.

##### `isHealthy(): Promise<boolean>`

Check if daemon is healthy and responsive.

## Error Handling

The library provides specific error types for different failure modes:

```typescript
import {
  DaemonError,          // Base error class
  DaemonStartError,     // Failed to start
  DaemonCrashError,     // Daemon crashed
  DaemonTimeoutError,   // Request timed out
  RpcError,             // JSON-RPC protocol error
  DaemonUnhealthyError,  // Daemon not healthy
} from '@maproom/daemon-client'

try {
  await client.search({ query: 'test', repo: 'my-repo' })
} catch (error) {
  if (error instanceof DaemonTimeoutError) {
    console.error('Request timed out')
  } else if (error instanceof RpcError) {
    console.error(`RPC error (code ${error.rpcCode}): ${error.message}`)
  } else if (error instanceof DaemonCrashError) {
    console.error(`Daemon crashed (exit code: ${error.exitCode})`)
  }
}
```

## Performance

| Operation | Process Spawning | With Daemon| Improvement |
|-----------|------------------|---------------|-------------|
| **First request (cold)** | ~200-500ms | ~200-500ms | Same |
| **Subsequent requests (warm)** | ~200-500ms | ~10-50ms | **20-50x faster** |
| **Concurrent requests** | N processes | 1 daemon | **Massive savings** |

## Architecture

```
TypeScript Client
      ↓
DaemonClient.search()
      ↓
JSON-RPC request via stdin
      ↓
crewchief-maproom serve (daemon)
      ├─ Connection Pool
      └─ Vector Search
      ↓
JSON-RPC response via stdout
      ↓
Parse and return
```

## License

MIT
