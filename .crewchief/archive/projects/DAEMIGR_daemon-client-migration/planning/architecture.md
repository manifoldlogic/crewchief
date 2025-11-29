# DAEMIGR Architecture Design

## Implementation Status

**Current State (as of 2025-11-22):**
- ✅ **Package Created:** `/workspace/packages/daemon-client/` exists
- ✅ **Core Modules Implemented:**
  - `src/client.ts` - DaemonClient class with search/ping methods
  - `src/lifecycle.ts` - DaemonLifecycle for process management
  - `src/rpc.ts` - JSON-RPC protocol handling
  - `src/errors.ts` - Error type hierarchy
- ✅ **Configuration Complete:** package.json, tsconfig.json, vitest config
- ⏳ **Pending Implementation:**
  - Unit tests (test files not yet created)
  - Integration with MCP server
  - Performance/stress testing
  - Documentation examples

**Implementation vs. Specification:**
- Core architecture matches this design document
- Need to verify existing code quality and completeness
- May require minor adjustments based on code review

## System Overview

The DAEMIGR project implements a **DaemonClient library** (`packages/daemon-client/`) that enables TypeScript/JavaScript applications to communicate with the `crewchief-maproom serve` daemon via JSON-RPC over stdin/stdout.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     MCP Server (TypeScript)                      │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                │
│  │  search    │  │  context   │  │  upsert    │                │
│  │   tool     │  │    tool    │  │    tool    │                │
│  └──────┬─────┘  └────────────┘  └────────────┘                │
│         │                                                        │
│         │ Uses DaemonClient                                     │
│         ▼                                                        │
│  ┌─────────────────────────────────────────┐                   │
│  │   DaemonClient (daemon-client package)  │                   │
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

New TypeScript package providing daemon lifecycle management and RPC communication.

#### Module Structure

```
packages/daemon-client/
├── src/
│   ├── client.ts          # Main DaemonClient class
│   ├── lifecycle.ts       # Process lifecycle management
│   ├── rpc.ts             # JSON-RPC protocol handling
│   ├── errors.ts          # Error types and handling
│   ├── types.ts           # TypeScript interfaces
│   └── index.ts           # Public exports
├── tests/
│   ├── client.test.ts     # DaemonClient tests
│   ├── lifecycle.test.ts  # Lifecycle tests
│   └── rpc.test.ts        # Protocol tests
└── package.json
```

#### `client.ts` - Main Client Class

**Responsibilities:**
- High-level search API
- Request/response matching
- Health checking
- Auto-restart coordination

**Interface:**
```typescript
export class DaemonClient {
  // Configuration
  private config: DaemonConfig
  private lifecycle: DaemonLifecycle
  private rpc: RpcProtocol

  // State
  private process?: ChildProcess
  private requestId = 0
  private pendingRequests: Map<number, PendingRequest>
  private isStarting = false
  private isHealthy = true

  constructor(config: DaemonConfig)

  // High-level API
  async ping(): Promise<string>
  async search(params: SearchParams): Promise<SearchResult>

  // Lifecycle
  async start(): Promise<void>
  async stop(): Promise<void>
  async restart(): Promise<void>
  async isHealthy(): Promise<boolean>

  // Low-level RPC (private)
  private async sendRequest(method: string, params?: any): Promise<any>
  private handleResponse(response: JsonRpcResponse): void
  private handleStdout(line: string): void
  private handleStderr(line: string): void
}
```

**Key Design Decisions:**

1. **Lazy Initialization**
   - Daemon starts on first search request, not on client construction
   - Reduces startup overhead for applications that may not search
   - Follows database connection pool pattern

2. **Request/Response Matching**
   - Each request assigned unique ID (sequential counter)
   - Pending requests stored in Map (ID → Promise resolver)
   - Response handler matches ID and resolves correct promise
   - Prevents race conditions with concurrent requests

3. **Health Checking**
   - Lightweight ping before each search
   - Detects stale/crashed daemon
   - Auto-restart on health check failure
   - Optional periodic health checks (configurable)

4. **Error Handling**
   - Typed errors (DaemonStartError, DaemonCrashError, RpcError)
   - Contextual error messages (include request ID, method)
   - Error propagation to caller (no silent failures)

#### `lifecycle.ts` - Process Lifecycle Management

**Responsibilities:**
- Process spawning and termination
- Crash detection and restart logic
- Exponential backoff for restart attempts
- Resource cleanup (streams, handles)

**Interface:**
```typescript
export class DaemonLifecycle {
  // State
  private process?: ChildProcess
  private restartAttempts = 0
  private lastRestartTime = 0
  private isShuttingDown = false

  // Configuration
  private config: LifecycleConfig

  async start(config: DaemonConfig): Promise<ChildProcess>
  async stop(process: ChildProcess, timeout?: number): Promise<void>
  async restart(config: DaemonConfig): Promise<ChildProcess>

  // Crash detection
  onExit(code: number | null, signal: string | null): void
  shouldRestart(): boolean
  getBackoffDelay(): number

  // Cleanup
  async cleanup(): Promise<void>
}
```

**Restart Strategy:**

```typescript
interface RestartConfig {
  maxAttempts: 5              // Max consecutive restart attempts
  backoffBase: 1000           // Base backoff delay (ms)
  backoffMultiplier: 2        // Exponential multiplier
  resetWindow: 60000          // Success window to reset attempts (ms)
}

// Backoff sequence: 1s, 2s, 4s, 8s, 16s
function getBackoffDelay(attempt: number): number {
  return backoffBase * Math.pow(backoffMultiplier, attempt)
}

// Reset counter if daemon runs successfully for 60s
function shouldResetAttempts(): boolean {
  return Date.now() - lastRestartTime > resetWindow
}
```

**Shutdown Sequence:**

```
1. Send SIGTERM (graceful shutdown request)
2. Wait up to shutdownTimeout (default: 5000ms)
3. If still running, send SIGKILL (force terminate)
4. Wait for process exit
5. Close stdin/stdout/stderr streams
6. Remove process event listeners
```

#### `rpc.ts` - JSON-RPC Protocol Handling

**Responsibilities:**
- JSON-RPC 2.0 request serialization
- Response parsing and validation
- Error code mapping
- Protocol validation

**Interface:**
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
  static createError(code: number, message: string, data?: any): JsonRpcError
}
```

**Protocol Details:**

- **Transport:** Line-delimited JSON over stdin/stdout
- **Format:** Strict JSON-RPC 2.0 compliance
- **Request IDs:** Sequential integers (1, 2, 3, ...)
- **Error Codes:** Standard JSON-RPC codes

**Error Code Mapping:**

```typescript
enum JsonRpcErrorCode {
  ParseError = -32700,      // Invalid JSON
  InvalidRequest = -32600,  // Malformed request
  MethodNotFound = -32601,  // Unknown method
  InvalidParams = -32602,   // Invalid parameters
  InternalError = -32603,   // Server error
  ServerError = -32000      // Application-defined errors
}
```

**Line Protocol:**

```
Request:  {"jsonrpc":"2.0","method":"search","params":{...},"id":1}\n
Response: {"jsonrpc":"2.0","result":{...},"id":1}\n
Error:    {"jsonrpc":"2.0","error":{"code":-32603,"message":"..."},"id":1}\n
```

#### `errors.ts` - Error Types

**Error Hierarchy:**

```typescript
export class DaemonError extends Error {
  constructor(
    message: string,
    public code: string,
    public cause?: Error
  )
}

// Lifecycle errors
export class DaemonStartError extends DaemonError {
  constructor(message: string, cause?: Error) {
    super(message, 'DAEMON_START_ERROR', cause)
  }
}

export class DaemonCrashError extends DaemonError {
  constructor(
    message: string,
    public exitCode: number | null,
    public signal: string | null
  ) {
    super(message, 'DAEMON_CRASH_ERROR')
  }
}

export class DaemonTimeoutError extends DaemonError {
  constructor(message: string, public timeoutMs: number) {
    super(message, 'DAEMON_TIMEOUT_ERROR')
  }
}

// Protocol errors
export class RpcError extends DaemonError {
  constructor(
    message: string,
    public rpcCode: number,
    public data?: any
  ) {
    super(message, 'RPC_ERROR')
  }
}

export class RpcTimeoutError extends RpcError {
  constructor(method: string, timeoutMs: number) {
    super(
      `RPC timeout after ${timeoutMs}ms: ${method}`,
      -32000,
      { method, timeoutMs }
    )
  }
}
```

### 2. MCP Server Integration (`packages/maproom-mcp/`)

#### Changes to `tools/search.ts`

**Before (Process Spawning):**

```typescript
// Lines 233-291: Binary spawning logic
const candidates = getBinaryCandidates()
const args = [
  command,
  '--repo', repo,
  '--query', query,
  '--limit', String(limit),
  // ... more args
]

const result = await trySpawnWithCandidates(candidates, args, {
  timeout: 30000,
  env: { ...process.env }
})

const rustOutput = JSON.parse(result.stdout)
```

**After (Daemon Client):**

```typescript
import { getDaemonClient } from '../daemon'

// Get singleton daemon client
const daemon = getDaemonClient()

// Execute search via daemon
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

**Migration Impact:**
- **Lines changed:** ~60 lines (spawning → daemon)
- **Lines preserved:** ~30 lines (chunk ID fetch, result transform)
- **Backward compatibility:** Old spawning code deprecated, not removed (VSCode needs it)

#### New File: `src/daemon.ts`

**Purpose:** Singleton management for DaemonClient

```typescript
import { DaemonClient } from '@crewchief/daemon-client'
import { findBinary } from './utils/process'

let daemonClient: DaemonClient | null = null

export function getDaemonClient(): DaemonClient {
  if (!daemonClient) {
    const binaryPath = findBinary()

    daemonClient = new DaemonClient({
      binaryPath,
      args: ['serve'],
      env: {
        // Database
        MAPROOM_DATABASE_URL: process.env.MAPROOM_DATABASE_URL,

        // Embedding providers
        OPENAI_API_KEY: process.env.OPENAI_API_KEY,
        ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY,
        OLLAMA_BASE_URL: process.env.OLLAMA_BASE_URL,

        // Logging
        RUST_LOG: process.env.RUST_LOG || 'info',
      },

      // Timeouts
      timeout: 30000,           // 30s request timeout
      startTimeout: 5000,       // 5s daemon start timeout
      shutdownTimeout: 5000,    // 5s graceful shutdown timeout

      // Restart behavior
      autoRestart: true,
      maxRestartAttempts: 5,
      restartBackoffMs: 1000,

      // Logging
      logger: console,
      logLevel: 'info'
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

// Shutdown hook (if MCP server supports graceful shutdown)
process.on('SIGTERM', async () => {
  await closeDaemonClient()
  process.exit(0)
})
```

**Design Decisions:**

1. **Singleton Pattern**
   - One daemon per MCP server instance
   - Shared across all search tool invocations
   - Lazy initialization on first search

2. **Binary Discovery**
   - Reuse existing `findBinary()` logic
   - Multi-platform path candidates
   - Error if binary not found

3. **Environment Propagation**
   - Pass critical env vars to daemon
   - Database URL, API keys, log level
   - Isolate from parent process env (explicit whitelist)

4. **Graceful Shutdown**
   - SIGTERM handler stops daemon
   - Prevents zombie processes
   - Clean resource cleanup

### 3. Daemon Binary (`crates/maproom/`)

**No changes required** - MAPDAEMON already implemented `serve` command.

**Relevant Code:** `crates/maproom/src/commands/serve.rs`

**Protocol:** JSON-RPC 2.0 over stdin/stdout (already implemented)

**Methods Supported:**
- `ping` → `{"result": "pong"}`
- `search` → `{"result": { hits: [...], total: N }}`

## Data Flow

### Search Request Flow

```
┌──────────────────────────────────────────────────────────────┐
│ 1. MCP Tool Handler (tools/search.ts)                        │
│    ├─ Validate params (Zod schema)                           │
│    ├─ Get daemon client (singleton)                          │
│    └─ Call daemon.search(params)                             │
└────────────────────┬─────────────────────────────────────────┘
                     │
┌────────────────────▼─────────────────────────────────────────┐
│ 2. DaemonClient (daemon-client package)                      │
│    ├─ Check if daemon running                                │
│    │  ├─ No → Start daemon (lifecycle.start())               │
│    │  └─ Yes → Send ping (health check)                      │
│    ├─ Generate request ID (sequential counter)               │
│    ├─ Create JSON-RPC request (rpc.createRequest())          │
│    ├─ Send to daemon via stdin                               │
│    ├─ Wait for response (with timeout)                       │
│    └─ Parse and return result                                │
└────────────────────┬─────────────────────────────────────────┘
                     │ stdin/stdout
┌────────────────────▼─────────────────────────────────────────┐
│ 3. Daemon (Rust - crewchief-maproom serve)                   │
│    ├─ Receive JSON-RPC request                               │
│    ├─ Resolve repo/worktree IDs                              │
│    ├─ Generate query embedding                               │
│    ├─ Execute vector search (pooled connection)              │
│    ├─ Format response                                        │
│    └─ Send JSON-RPC response via stdout                      │
└────────────────────┬─────────────────────────────────────────┘
                     │
┌────────────────────▼─────────────────────────────────────────┐
│ 4. DaemonClient (response handling)                          │
│    ├─ Parse response (rpc.parseResponse())                   │
│    ├─ Match to pending request (by ID)                       │
│    ├─ Resolve promise                                        │
│    └─ Return to MCP handler                                  │
└────────────────────┬─────────────────────────────────────────┘
                     │
┌────────────────────▼─────────────────────────────────────────┐
│ 5. MCP Tool Handler (result transformation)                  │
│    ├─ Fetch chunk IDs from database                          │
│    ├─ Transform results (add chunk IDs, format)              │
│    └─ Return to MCP client                                   │
└──────────────────────────────────────────────────────────────┘
```

### Error Flow

```
┌─────────────────────────────────────────────────────────┐
│ Error in Daemon (e.g., invalid query)                   │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ Daemon sends JSON-RPC error response                    │
│ {"jsonrpc":"2.0","error":{"code":-32602,"message":"Invalid│
│  params: repo not found"},"id":1}                       │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ DaemonClient receives error                             │
│    ├─ rpc.parseResponse() detects error                 │
│    ├─ Throws RpcError(-32602, "Invalid params...")      │
│    └─ Promise rejected                                  │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ MCP tool handler catches error                          │
│    ├─ formatSearchError() converts to MCP response      │
│    └─ Return user-friendly error to client              │
└─────────────────────────────────────────────────────────┘
```

### Crash Recovery Flow

```
┌─────────────────────────────────────────────────────────┐
│ Daemon crashes unexpectedly (segfault, panic, OOM)      │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ Process 'exit' event fired                              │
│    ├─ exitCode: 1 (error)                               │
│    └─ signal: null                                      │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ DaemonLifecycle.onExit(1, null) called                  │
│    ├─ Check shouldRestart()                             │
│    │  ├─ restartAttempts < maxAttempts? Yes (3 < 5)    │
│    │  └─ Return true                                    │
│    ├─ Calculate backoff: getBackoffDelay(3) = 8000ms    │
│    └─ Schedule restart after backoff                    │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ Wait 8 seconds (exponential backoff)                    │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ DaemonLifecycle.restart() called                        │
│    ├─ Spawn new daemon process                          │
│    ├─ Increment restartAttempts (3 → 4)                 │
│    ├─ Record lastRestartTime = Date.now()               │
│    └─ Return new process                                │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ Retry failed request                                    │
│    ├─ Send ping (health check)                          │
│    ├─ Resend original search request                    │
│    └─ Return result to caller                           │
└─────────────────────────────────────────────────────────┘
```

**Circuit Breaker (Max Attempts Exceeded):**

```
┌─────────────────────────────────────────────────────────┐
│ Daemon crashes 5 times in quick succession              │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ shouldRestart() returns false                           │
│    ├─ restartAttempts >= maxAttempts (5 >= 5)          │
│    └─ Circuit breaker triggered                         │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ Throw DaemonCrashError                                  │
│    ├─ message: "Daemon crashed, max restart attempts    │
│    │            exceeded (5)"                            │
│    └─ isHealthy = false                                 │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│ MCP tool handler receives error                         │
│    ├─ Log critical error                                │
│    ├─ Return error to client                            │
│    └─ [Optional] Fallback to process spawning           │
└─────────────────────────────────────────────────────────┘
```

## Configuration

### DaemonClient Configuration

```typescript
interface DaemonConfig {
  // Binary configuration
  binaryPath: string            // Path to crewchief-maproom binary
  args?: string[]               // Command args (default: ['serve'])
  env?: NodeJS.ProcessEnv       // Environment variables for daemon

  // Timeouts (milliseconds)
  timeout?: number              // Request timeout (default: 30000)
  startTimeout?: number         // Daemon start timeout (default: 5000)
  shutdownTimeout?: number      // Graceful shutdown timeout (default: 5000)

  // Restart behavior
  autoRestart?: boolean         // Auto-restart on crash (default: true)
  maxRestartAttempts?: number   // Max restart attempts (default: 5)
  restartBackoffMs?: number     // Initial backoff (default: 1000)
  restartResetWindowMs?: number // Success window to reset attempts (default: 60000)

  // Health checking
  healthCheckInterval?: number  // Periodic ping interval (default: 0 = disabled)
  healthCheckBeforeRequest?: boolean // Ping before each request (default: true)

  // Logging
  logger?: Logger               // Custom logger (default: console)
  logLevel?: 'debug' | 'info' | 'warn' | 'error' // Log level (default: 'info')
}
```

### Environment Variables (Daemon)

**Required:**
```bash
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
```

**Optional (Embedding Providers):**
```bash
# OpenAI
OPENAI_API_KEY=sk-...

# Anthropic
ANTHROPIC_API_KEY=...

# Ollama (local)
OLLAMA_BASE_URL=http://localhost:11434
```

**Logging:**
```bash
RUST_LOG=info  # Rust log level (error, warn, info, debug, trace)
```

## Connection Pool Configuration

### Pool Sizing Strategy

The Rust daemon maintains a PostgreSQL connection pool (using `deadpool-postgres`) to serve concurrent search requests efficiently.

**Default Configuration:**
- Pool size: 5 connections (default)
- Connection timeout: 30 seconds
- Idle timeout: 10 minutes

**Sizing Formula:**

```
pool_size >= concurrent_requests / 2
```

**Rationale:**
- Search queries are fast (3-25ms on average)
- Connections are released quickly after query execution
- 50% utilization factor accounts for query execution overhead
- Provides buffer for burst traffic

**Examples:**
- Low concurrency (1-2 concurrent): 2-3 connections sufficient
- Medium concurrency (3-6 concurrent): 4-5 connections (default)
- High concurrency (10-15 concurrent): 8-10 connections

**Pool Exhaustion Behavior:**

When all pool connections are in use:
1. **Request queues** in daemon (Tokio task awaits available connection)
2. **Timeout applies** - request fails if connection not available within `timeout` period
3. **Error returned** - `RpcError` with code -32000 (Server Error)
4. **No retry** - Client receives error, can retry if desired

**Configuration (Future Enhancement):**

Currently pool size is hardcoded in Rust daemon. Phase 2 may add configuration:

```rust
// crates/maproom/src/config.rs
pub struct PoolConfig {
    pub max_size: usize,        // Default: 5
    pub timeout_secs: u64,      // Default: 30
    pub idle_timeout_secs: u64, // Default: 600
}
```

### Error Serialization Format

**DaemonError → JSON-RPC Error Mapping:**

TypeScript DaemonError types are serialized to JSON-RPC 2.0 error objects when transmitted over stdio.

**Mapping Table:**

| TypeScript Error | JSON-RPC Code | JSON-RPC Message | Error Data |
|------------------|---------------|------------------|------------|
| `DaemonStartError` | -32000 | "Daemon failed to start" | `{ cause: string }` |
| `DaemonCrashError` | -32000 | "Daemon crashed" | `{ exitCode: number \| null, signal: string \| null }` |
| `DaemonTimeoutError` | -32000 | "Request timeout" | `{ timeoutMs: number, method: string }` |
| `RpcError` | (varies) | (from daemon) | (from daemon) |
| `RpcTimeoutError` | -32000 | "RPC timeout" | `{ timeoutMs: number, method: string }` |

**Standard JSON-RPC Error Codes (from Daemon):**

```typescript
-32700  ParseError      Invalid JSON received
-32600  InvalidRequest  JSON not valid request object
-32601  MethodNotFound  Method does not exist
-32602  InvalidParams   Invalid method parameters
-32603  InternalError   Internal JSON-RPC error
-32000  ServerError     Application-defined error (search failures, DB errors)
```

**Example Error Response:**

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params: repository 'unknown-repo' not found",
    "data": {
      "repo": "unknown-repo",
      "available_repos": ["crewchief", "example"]
    }
  },
  "id": 42
}
```

**Client Error Handling:**

```typescript
try {
  const result = await daemon.search(params)
} catch (error) {
  if (error instanceof RpcError) {
    // JSON-RPC error from daemon
    console.error(`RPC Error ${error.rpcCode}: ${error.message}`)
    console.error('Data:', error.data)
  } else if (error instanceof DaemonCrashError) {
    // Daemon process crashed
    console.error(`Daemon crashed with code ${error.exitCode}`)
  } else if (error instanceof DaemonTimeoutError) {
    // Request timeout
    console.error(`Request timed out after ${error.timeoutMs}ms`)
  }
}
```

### Graceful Shutdown Behavior

**Shutdown Sequence:**

When `DaemonClient.stop()` is called:

1. **Mark as shutting down** - `isShuttingDown = true` (prevents new requests)
2. **Wait for in-flight requests** - All pending requests in `pendingRequests` Map
3. **Send SIGTERM** - Graceful shutdown signal to daemon process
4. **Wait for process exit** - Up to `shutdownTimeout` (default: 5s)
5. **Force kill if needed** - Send SIGKILL if still running after timeout
6. **Clean up resources** - Close streams, remove listeners, clear state

**In-Flight Request Handling:**

```typescript
async stop(): Promise<void> {
  this.isShuttingDown = true

  // Wait for pending requests to complete (with timeout)
  const pendingPromises = Array.from(this.pendingRequests.values())
    .map(req => req.promise)

  await Promise.race([
    Promise.allSettled(pendingPromises),
    sleep(this.config.shutdownTimeout || 5000)
  ])

  // Stop daemon process
  await this.lifecycle.stop(this.process)

  // Clear state
  this.pendingRequests.clear()
  this.process = null
}
```

**Behavior Details:**

- **Pending requests during shutdown:**
  - Complete normally if daemon responds before timeout
  - Rejected with `DaemonTimeoutError` if timeout expires
  - No silent failures (all promises resolved or rejected)

- **New requests during shutdown:**
  - Rejected immediately with error: "Daemon is shutting down"
  - No queuing or waiting

- **Daemon process:**
  - SIGTERM allows daemon to flush buffers and close DB connections gracefully
  - SIGKILL as last resort (only if SIGTERM fails)
  - Process exit awaited before returning from `stop()`

**MCP Server Shutdown Hook:**

```typescript
// packages/maproom-mcp/src/daemon.ts
process.on('SIGTERM', async () => {
  console.log('MCP server shutting down, stopping daemon...')
  await closeDaemonClient()
  process.exit(0)
})
```

### Request ID Collision Handling

**ID Generation Strategy:**

Request IDs are sequential integers starting from 1:

```typescript
class DaemonClient {
  private requestId = 0

  private getNextRequestId(): number {
    this.requestId++

    // Handle overflow (rollover to 1)
    if (this.requestId > Number.MAX_SAFE_INTEGER) {
      this.requestId = 1
    }

    return this.requestId
  }
}
```

**Collision Prevention:**

- **Sequential IDs** - Monotonically increasing, no random collisions
- **Rollover strategy** - Resets to 1 at `Number.MAX_SAFE_INTEGER`
- **Pending request tracking** - Map tracks all in-flight requests by ID

**Rollover Safety:**

```typescript
Number.MAX_SAFE_INTEGER = 9007199254740991  // ~9 quadrillion

// At 1000 requests/sec, rollover occurs after:
9007199254740991 / 1000 / 60 / 60 / 24 / 365 = 285,616 years
```

**Practical Considerations:**

- Rollover is effectively impossible in practice
- Even at 1M requests/sec, would take 285 years
- No collision detection needed for sequential IDs

**Response Matching:**

```typescript
private handleResponse(response: JsonRpcResponse): void {
  const requestId = response.id
  const pending = this.pendingRequests.get(requestId)

  if (!pending) {
    // Orphaned response (request already timed out or cancelled)
    console.warn(`Received response for unknown request ID ${requestId}`)
    return
  }

  // Match response to request, resolve promise
  this.pendingRequests.delete(requestId)

  if (response.error) {
    pending.reject(new RpcError(response.error.message, response.error.code, response.error.data))
  } else {
    pending.resolve(response.result)
  }
}
```

**Edge Case: Orphaned Responses**

Responses can arrive after request timeout:

```
1. Client sends request ID=42 with 30s timeout
2. Daemon is slow, takes 35s to respond
3. Client timeout fires at 30s, request rejected with DaemonTimeoutError
4. Client removes ID=42 from pendingRequests
5. Daemon sends response for ID=42 at 35s
6. handleResponse() finds no pending request for ID=42
7. Log warning, discard response (no error)
```

This is safe - no memory leak, no crash, just a logged warning.

## Performance Characteristics

### Latency Analysis

**Cold Start (First Request):**
```
Component                 Latency
────────────────────────────────
Process spawn             50-100ms
Rust initialization       100-250ms
DB connection pool        50-150ms
────────────────────────────────
Subtotal (daemon start)   200-500ms

Query embedding           5-20ms
Vector search             3-25ms
────────────────────────────────
Subtotal (search exec)    10-50ms

Total (cold)              210-550ms
```

**Warm Request (Subsequent):**
```
Component                 Latency
────────────────────────────────
JSON serialization        0.1-0.3ms
IPC (stdio)               0.2-0.4ms
JSON parsing              0.2-0.3ms
────────────────────────────────
Subtotal (RPC overhead)   0.5-1ms

Query embedding           5-20ms
Vector search (pooled)    3-25ms
────────────────────────────────
Subtotal (search exec)    10-50ms

Total (warm)              10.5-51ms
```

**Improvement Factor:**
- Cold start: ~1x (similar to spawning)
- Warm requests: 20-50x faster than spawning

### Resource Usage

**Memory:**
- Daemon process: 50-80MB (Rust binary + connection pool)
- DaemonClient: 5-10MB (Node.js overhead, request queue)
- **Total additional:** 55-90MB

**CPU:**
- Daemon idle: ~0%
- Per request: 5-15% spike (2-core system)
- Concurrent requests: Handled async, minimal contention

**Database Connections:**
- **Current (spawning):** N connections (one per process)
- **With daemon:** 1 connection pool (default 5 connections)
- **Improvement:** 10x fewer connections under load

### Concurrency

**Supported Concurrency:**
- Daemon handles async requests (Tokio runtime)
- No artificial concurrency limit
- Limited by PostgreSQL connection pool size

**Backpressure Handling:**
- Request queue in DaemonClient (unbounded)
- Database pool blocks if exhausted
- Timeout prevents indefinite waits

## Technology Choices

### Why JSON-RPC 2.0?
- **Standard:** Well-defined spec, many implementations
- **Simple:** Text-based, easy to debug
- **Proven:** Used by LSP, VSCode extensions
- **No Ports:** Stdin/stdout, no port management

**Alternatives Considered:**
- gRPC: Rejected (port management, network overhead)
- MessagePack: Rejected (binary, harder to debug)
- Custom protocol: Rejected (reinvent wheel)

### Why stdio (vs HTTP/sockets)?
- **Security:** No network exposure, no authentication needed
- **Simplicity:** No port conflicts, no firewall issues
- **Performance:** Low latency for local IPC
- **Debuggability:** Text-based, pipe to file/console

**Alternatives Considered:**
- HTTP: Rejected (port management, security)
- Unix sockets: Rejected (platform-specific, permissions)
- Named pipes: Rejected (Windows/Linux differences)

### Why Process-per-Instance?
- **Isolation:** Client crashes don't affect others
- **Lifecycle:** Simple ownership (client owns daemon)
- **Migration:** Gradual (one client at a time)
- **Debugging:** Clear parent-child relationship

**Alternatives Considered:**
- Shared daemon: Rejected for Phase 1 (complexity, coupling)
- Native module: Rejected (build complexity, crash propagation)

## Constraints and Trade-offs

### Constraints

**Platform Support:**
- Node.js >= 18 (DaemonClient requires ESM)
- PostgreSQL >= 13 (Maproom schema compatibility)
- Rust binary availability (platform-specific builds)

**Performance:**
- Cold start acceptable (similar to spawning)
- Warm requests target < 50ms
- Memory overhead acceptable (< 100MB)

**Reliability:**
- Zero data corruption (strict priority)
- Auto-restart on daemon crash
- Graceful degradation (error messages, fallback)

### Trade-offs

#### Trade-off 1: Cold Start Latency vs Simplicity
**Decision:** Accept cold start latency for lazy initialization

**Rationale:**
- Reduces startup overhead for applications that may not search
- Follows database connection pool pattern (proven)
- First request pays cost, all subsequent requests benefit

**Alternative:** Pre-start daemon on client init
- Pros: First request is fast
- Cons: Wasted resources if no search, longer client init

#### Trade-off 2: Process-per-Instance vs Shared Daemon
**Decision:** Process-per-instance for Phase 1

**Rationale:**
- Simple lifecycle management (client owns daemon)
- Isolated failures (one client crash doesn't affect others)
- Easier debugging (clear parent-child relationship)
- Gradual migration (one client at a time)

**Alternative:** Shared daemon across all clients
- Pros: Better resource usage (one daemon for all)
- Cons: Complex IPC (socket coordination), coupled lifecycles, harder debugging

#### Trade-off 3: Auto-restart vs Manual Recovery
**Decision:** Auto-restart with exponential backoff

**Rationale:**
- Better user experience (transparent recovery)
- Proven pattern (PM2, systemd)
- Circuit breaker prevents restart loops

**Alternative:** Manual recovery (error, user restarts)
- Pros: Simpler implementation
- Cons: Poor UX (requires user intervention)

## Extensibility

### Future Enhancements

**Phase 2 Candidates:**

1. **VSCode Scan Migration**
   - Migrate `scan` command to daemon
   - Reuse daemon-client package
   - Medium impact (infrequent operation)

2. **Shared Daemon Exploration**
   - If resource usage justifies
   - Socket-based IPC (Unix domain socket)
   - Daemon lifecycle management (systemd/PM2)

3. **Additional Methods via Daemon**
   - `context` tool (if latency-sensitive)
   - `upsert` tool (if latency-sensitive)
   - Batch operations (multiple queries in one request)

**Backward Compatibility:**

- Daemon-client API versioned (semver)
- JSON-RPC protocol versioned (negotiate version)
- MCP server can fallback to spawning (safety net)

---

**Architecture Designed:** 2025-11-22
**Status:** Ready for implementation
