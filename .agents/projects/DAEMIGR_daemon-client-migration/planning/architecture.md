# DAEMIGR Architecture Design

## System Overview

The DAEMIGR project implements a **DaemonClient library** that enables TypeScript/JavaScript applications to communicate with the `crewchief-maproom serve` daemon via JSON-RPC over stdio.

```
┌─────────────────────────────────────────────────────────────────┐
│                        MCP Server (TypeScript)                   │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                │
│  │  search    │  │  context   │  │  upsert    │                │
│  │   tool     │  │    tool    │  │    tool    │                │
│  └──────┬─────┘  └────────────┘  └────────────┘                │
│         │                                                        │
│         │ Uses DaemonClient                                     │
│         ▼                                                        │
│  ┌─────────────────────────────────────────┐                   │
│  │      DaemonClient (daemon-client pkg)   │                   │
│  │  ┌────────────┐  ┌────────────────┐    │                   │
│  │  │ Lifecycle  │  │  RPC Protocol  │    │                   │
│  │  │ Management │  │    Handling    │    │                   │
│  │  └────────────┘  └────────────────┘    │                   │
│  └──────────────────┬──────────────────────┘                   │
└─────────────────────┼──────────────────────────────────────────┘
                      │
                      │ stdin/stdout
                      │ JSON-RPC 2.0
                      │
                      ▼
         ┌────────────────────────────────────┐
         │  crewchief-maproom serve (Rust)    │
         │  ┌──────────┐  ┌──────────────┐   │
         │  │  Event   │  │   Search     │   │
         │  │   Loop   │  │   Executor   │   │
         │  └──────────┘  └──────────────┘   │
         │  ┌──────────────────────────────┐ │
         │  │   Connection Pool (pgpool)   │ │
         │  └──────────────────────────────┘ │
         └────────────────┬───────────────────┘
                          │
                          ▼
                   ┌──────────────┐
                   │  PostgreSQL  │
                   │   (maproom)  │
                   └──────────────┘
```

## Component Architecture

### 1. DaemonClient Library (`packages/daemon-client/`)

**Purpose:** Manage daemon lifecycle and provide high-level search API

**Modules:**

#### `client.ts` - Main Client Class
```typescript
export class DaemonClient {
  private process?: ChildProcess
  private config: DaemonConfig
  private requestId = 0
  private pendingRequests: Map<number, PendingRequest>
  private isStarting = false
  
  constructor(config: DaemonConfig)
  
  // High-level API
  async ping(): Promise<string>
  async search(params: SearchParams): Promise<SearchResult>
  
  // Lifecycle
  async start(): Promise<void>
  async stop(): Promise<void>
  async restart(): Promise<void>
  async isHealthy(): Promise<boolean>
  
  // Low-level RPC
  private async sendRequest(method: string, params?: any): Promise<any>
  private handleResponse(response: JsonRpcResponse): void
  private handleStdout(line: string): void
}
```

**Key Design Decisions:**
- **Lazy initialization:** Daemon starts on first search request
- **Request queue:** Pending requests tracked by ID for response matching
- **Health checking:** Ping before search to detect stale daemon
- **Auto-restart:** Crash detection with exponential backoff

#### `lifecycle.ts` - Process Lifecycle Management
```typescript
export class DaemonLifecycle {
  private process?: ChildProcess
  private restartAttempts = 0
  private lastRestartTime = 0
  
  async start(config: DaemonConfig): Promise<ChildProcess>
  async stop(process: ChildProcess): Promise<void>
  async restart(config: DaemonConfig): Promise<ChildProcess>
  
  // Crash detection
  onExit(code: number, signal: string): void
  shouldRestart(): boolean
  getBackoffDelay(): number
}
```

**Restart Strategy:**
- **Max attempts:** 5 restarts
- **Backoff:** Exponential (1s, 2s, 4s, 8s, 16s)
- **Reset window:** 60s (success resets attempt counter)
- **Circuit breaker:** Give up after max attempts

#### `rpc.ts` - JSON-RPC Protocol Handling
```typescript
export interface JsonRpcRequest {
  jsonrpc: '2.0'
  method: string
  params?: any
  id: number
}

export interface JsonRpcResponse {
  jsonrpc: '2.0'
  result?: any
  error?: JsonRpcError
  id: number | null
}

export interface JsonRpcError {
  code: number
  message: string
  data?: any
}

export class RpcProtocol {
  static createRequest(method: string, params: any, id: number): JsonRpcRequest
  static parseResponse(line: string): JsonRpcResponse
  static isError(response: JsonRpcResponse): boolean
}
```

**Protocol Details:**
- **Transport:** Line-delimited JSON over stdin/stdout
- **Format:** JSON-RPC 2.0 spec
- **Request IDs:** Sequential counter (1, 2, 3, ...)
- **Error codes:** Standard JSON-RPC codes (-32700, -32600, etc.)

#### `errors.ts` - Error Types
```typescript
export class DaemonError extends Error {
  constructor(message: string, public code: string, public cause?: Error)
}

export class DaemonStartError extends DaemonError {}
export class DaemonCrashError extends DaemonError {}
export class DaemonTimeoutError extends DaemonError {}
export class RpcError extends DaemonError {
  constructor(message: string, public rpcCode: number, public data?: any)
}
```

### 2. MCP Server Integration (`packages/maproom-mcp/`)

**Changes to `tools/search.ts`:**

```typescript
// OLD: Spawning approach (lines 233-291)
const candidates = getBinaryCandidates()
const args = [command, '--repo', repo, '--query', query, ...]
result = await trySpawnWithCandidates(candidates, args, {...})
rustOutput = JSON.parse(result.stdout)

// NEW: Daemon approach
import { getDaemonClient } from '../daemon'

const daemon = getDaemonClient() // Singleton per MCP instance
const searchResult = await daemon.search({
  query,
  repo,
  worktree,
  limit,
  mode,
  debug
})

// Chunk ID fetching remains unchanged (lines 307-343)
const chunkIdMap = await fetchChunkIds(client, repo, searchResult.hits)
```

**Daemon Singleton Management:**
```typescript
// packages/maproom-mcp/src/daemon.ts (new file)
let daemonClient: DaemonClient | null = null

export function getDaemonClient(): DaemonClient {
  if (!daemonClient) {
    const binaryPath = findBinary() // Reuse existing binary discovery
    daemonClient = new DaemonClient({
      binaryPath,
      env: {
        DATABASE_URL: process.env.MAPROOM_DATABASE_URL,
        OPENAI_API_KEY: process.env.OPENAI_API_KEY,
        // ... other env vars
      },
      timeout: 30000,
      autoRestart: true
    })
  }
  return daemonClient
}

export async function closeDaemonClient(): Promise<void> {
  if (daemonClient) {
    await daemonClient.stop()
    daemonClient = null
  }
}
```

## Data Flow

### Search Request Flow

```
1. MCP Tool Handler
   ├─ Validate params (Zod schema)
   ├─ Get daemon client (singleton)
   └─ Call daemon.search(params)
       │
2. DaemonClient
   ├─ Check if daemon running
   │  ├─ No → Start daemon
   │  └─ Yes → Send ping (health check)
   ├─ Generate request ID
   ├─ Create JSON-RPC request
   ├─ Send to daemon via stdin
   ├─ Wait for response (with timeout)
   └─ Parse and return result
       │
3. Daemon (Rust)
   ├─ Receive JSON-RPC request
   ├─ Resolve repo/worktree IDs
   ├─ Generate query embedding
   ├─ Execute vector search (pooled connection)
   ├─ Format response
   └─ Send JSON-RPC response via stdout
       │
4. DaemonClient
   ├─ Parse response
   ├─ Match to pending request (by ID)
   ├─ Resolve promise
   └─ Return to MCP handler
       │
5. MCP Tool Handler
   ├─ Fetch chunk IDs from database
   ├─ Transform results
   └─ Return to MCP client
```

### Error Flow

```
Error in Daemon
   ↓
Daemon sends JSON-RPC error response
   ↓
DaemonClient receives error
   ↓
Throws RpcError with code/message
   ↓
MCP tool handler catches
   ↓
formatSearchError() converts to MCP response
   ↓
Client receives user-friendly error
```

### Crash Recovery Flow

```
Daemon crashes unexpectedly
   ↓
DaemonLifecycle.onExit() called
   ↓
Check shouldRestart()
   ├─ Attempts < 5 → Yes
   │  ├─ Wait backoff delay (exponential)
   │  ├─ Restart daemon
   │  └─ Retry failed request
   │
   └─ Attempts >= 5 → No
      ├─ Mark daemon as unhealthy
      ├─ Throw DaemonCrashError
      └─ [Optional] Fallback to process spawning
```

## Configuration

### DaemonClient Configuration

```typescript
interface DaemonConfig {
  // Binary location
  binaryPath: string
  
  // Environment variables for daemon
  env: NodeJS.ProcessEnv
  
  // Timeouts
  timeout?: number          // Request timeout (default: 30000ms)
  startTimeout?: number     // Daemon start timeout (default: 5000ms)
  shutdownTimeout?: number  // Graceful shutdown timeout (default: 5000ms)
  
  // Restart behavior
  autoRestart?: boolean     // Auto-restart on crash (default: true)
  maxRestartAttempts?: number // Max restart attempts (default: 5)
  restartBackoffMs?: number // Initial backoff (default: 1000ms)
  
  // Health checking
  healthCheckInterval?: number // Ping interval (default: 0 = disabled)
  
  // Logging
  logger?: Logger          // Custom logger (default: console)
  logLevel?: 'debug' | 'info' | 'warn' | 'error'
}
```

### Environment Variables (Daemon)

```bash
# PostgreSQL connection
MAPROOM_DATABASE_URL=postgres://user:pass@localhost/maproom

# Embedding provider (if vector search is used)
OPENAI_API_KEY=sk-...
# Or
ANTHROPIC_API_KEY=...
# Or
OLLAMA_BASE_URL=http://localhost:11434

# Logging
RUST_LOG=info
```

## Performance Characteristics

### Latency Breakdown

**Cold Start (First Request):**
```
Daemon start: ~200-500ms
  ├─ Process spawn: ~50-100ms
  ├─ DB connection: ~50-150ms
  └─ Rust initialization: ~100-250ms

Search execution: ~10-50ms
  ├─ Resolve repo/worktree: ~2-5ms
  ├─ Generate embedding: ~5-20ms (depends on provider)
  └─ Vector search: ~3-25ms (depends on index size)

Total: ~210-550ms (similar to current spawning approach)
```

**Warm Request (Subsequent):**
```
JSON-RPC overhead: ~0.5-1ms
  ├─ Serialization: ~0.1-0.3ms
  ├─ IPC: ~0.2-0.4ms
  └─ Parsing: ~0.2-0.3ms

Search execution: ~10-50ms
  ├─ Resolve repo/worktree: ~2-5ms (cached)
  ├─ Generate embedding: ~5-20ms
  └─ Vector search: ~3-25ms (pooled connection)

Total: ~10.5-51ms (20-50x improvement vs spawning)
```

### Resource Usage

**Memory:**
- Daemon process: ~50-80MB (Rust binary + connection pool)
- DaemonClient: ~5-10MB (Node.js overhead)
- Total: ~55-90MB additional memory

**CPU:**
- Daemon idle: ~0%
- Per request: ~5-15% spike (2-core system)
- Concurrent requests: Handled async, minimal contention

**Connections:**
- Database: 1 connection pool (default 5 connections)
- vs Current: N connections (one per spawned process)

## Security Considerations

### Process Communication
- **Transport:** stdin/stdout (local IPC, no network exposure)
- **Authentication:** Not needed (same-machine communication)
- **Authorization:** Inherited from parent process permissions

### Environment Variables
- **Credentials:** Passed via env vars (standard practice)
- **Visibility:** Environment visible in `/proc/<pid>/environ`
- **Mitigation:** Use secrets management for production

### Resource Limits
- **Memory:** No explicit limits (relies on OS)
- **CPU:** No throttling (daemon uses async I/O)
- **Connections:** Pool size limits DB connections

## Testing Strategy

### Unit Tests (daemon-client)
- Process lifecycle (start, stop, restart)
- JSON-RPC protocol handling
- Error scenarios (timeouts, parse errors, crashes)
- Request/response matching

### Integration Tests (maproom-mcp)
- End-to-end search via daemon
- Concurrent requests handling
- Daemon crash recovery
- Error propagation

### Performance Tests
- Cold start latency
- Warm request latency
- Concurrent load (10, 50, 100 requests)
- Memory leak detection (1000 requests)

---

**Architecture designed:** 2025-11-21  
**Status:** Ready for implementation
