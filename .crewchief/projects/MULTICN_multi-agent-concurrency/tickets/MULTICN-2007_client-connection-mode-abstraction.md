# Ticket: MULTICN-2007: Client Connection Mode Abstraction

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

Update DaemonClient to support multiple connection modes (socket/stdio/auto) with Connection interface abstraction. Refactor existing stdio logic into StdioConnection class, add auto-detection (Windows→stdio, Unix→socket with stdio fallback), and support MAPROOM_CONNECTION_MODE environment variable override.

## Background

To enable dual-mode support (socket vs stdio), we need a clean abstraction that allows DaemonClient to work with either transport without code duplication. This ticket completes the TypeScript client updates by tying together SocketConnection (MULTICN-2005) and the existing stdio logic into a unified interface.

The auto-detection logic ensures backward compatibility while enabling new socket mode.

Reference: [architecture.md](../planning/architecture.md) - Connection Abstraction Layer section

## Acceptance Criteria

- [ ] Connection interface defined with 4 methods: sendRequest, close, isConnected, on
- [ ] SocketConnection and StdioConnection both implement Connection interface
- [ ] Windows platform automatically uses stdio mode
- [ ] `MAPROOM_CONNECTION_MODE=stdio` forces stdio mode on Unix
- [ ] `MAPROOM_CONNECTION_MODE=socket` forces socket mode
- [ ] `MAPROOM_CONNECTION_MODE=auto` or unset triggers auto-detection
- [ ] Auto mode tries socket, falls back to stdio on failure
- [ ] Existing error classes extended (not replaced)
- [ ] Test: Both connection modes pass identical test suite
- [ ] Test: Mode detection logic correct on different platforms

## Technical Requirements

Update `packages/daemon-client/src/client.ts` and create new abstraction files.

### Connection Interface (Already Created in MULTICN-2005)

```typescript
// packages/daemon-client/src/connection.ts

export interface Connection {
  sendRequest<T = unknown>(method: string, params?: unknown): Promise<T>;
  close(): Promise<void>;
  isConnected(): boolean;
  on(event: 'error' | 'close', handler: (err?: Error) => void): void;
}

export enum ConnectionMode {
  Socket = 'socket',
  Stdio = 'stdio',
  Auto = 'auto',
}

export interface ConnectionConfig {
  mode: ConnectionMode;
  socketPath?: string;
  binaryPath?: string;
  startupTimeout?: number;
}
```

### StdioConnection Implementation

Refactor existing DaemonClient stdio logic into standalone class:

```typescript
// packages/daemon-client/src/stdio.ts

import { ChildProcess, spawn } from 'child_process';
import { Connection } from './connection';
import { JsonRpcRequest, JsonRpcResponse, RequestId } from './rpc';
import { DaemonLifecycle } from './lifecycle';
import { DaemonStartupError, DaemonCommunicationError } from './errors';

interface PendingRequest {
  resolve: (value: any) => void;
  reject: (error: Error) => void;
  method: string;
  timeout?: NodeJS.Timeout;
}

export class StdioConnection implements Connection {
  private daemonProcess: ChildProcess | null = null;
  private pendingRequests = new Map<RequestId, PendingRequest>();
  private nextId = 1;
  private connected = false;
  private lifecycle: DaemonLifecycle;
  private errorHandlers: Array<(err?: Error) => void> = [];
  private closeHandlers: Array<(err?: Error) => void> = [];

  constructor(private binaryPath: string) {
    this.lifecycle = new DaemonLifecycle({
      restartOnFailure: true,
      maxFailures: 3,
      resetWindow: 60000,
    });
  }

  async connect(): Promise<void> {
    this.daemonProcess = spawn(this.binaryPath, ['serve'], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    this.setupHandlers();
    this.connected = true;

    // Wait for daemon to be ready
    await this.waitForReady();
  }

  private setupHandlers(): void {
    if (!this.daemonProcess) return;

    // stdout: JSON-RPC responses
    this.daemonProcess.stdout?.on('data', (data: Buffer) => {
      const lines = data.toString().split('\n').filter((l) => l.trim());
      for (const line of lines) {
        try {
          const message = JSON.parse(line) as JsonRpcResponse;
          this.handleMessage(message);
        } catch (err) {
          console.error('Failed to parse JSON-RPC response:', err);
        }
      }
    });

    // stderr: daemon logs (optional logging)
    this.daemonProcess.stderr?.on('data', (data: Buffer) => {
      console.error('[daemon]', data.toString());
    });

    // Process exit handling
    this.daemonProcess.on('exit', (code) => {
      this.connected = false;
      this.closeHandlers.forEach((h) => h());
      this.rejectAllPending(
        new DaemonCommunicationError(`Daemon exited with code ${code}`)
      );

      if (code !== 0) {
        this.errorHandlers.forEach((h) =>
          h(new Error(`Daemon crashed with code ${code}`))
        );
      }
    });
  }

  private async waitForReady(timeoutMs: number = 5000): Promise<void> {
    // Send ping to verify daemon is responsive
    try {
      await this.sendRequest('ping', undefined, timeoutMs);
    } catch (err) {
      throw new DaemonStartupError('Daemon failed to respond to ping', {
        cause: err as Error,
      });
    }
  }

  private handleMessage(message: JsonRpcResponse): void {
    const pending = this.pendingRequests.get(message.id);
    if (!pending) {
      console.warn('Received response for unknown request ID:', message.id);
      return;
    }

    this.pendingRequests.delete(message.id);

    if (pending.timeout) {
      clearTimeout(pending.timeout);
    }

    if (message.error) {
      pending.reject(new Error(`JSON-RPC error: ${message.error.message}`));
    } else {
      pending.resolve(message.result);
    }
  }

  async sendRequest<T = unknown>(
    method: string,
    params?: unknown,
    timeoutMs: number = 30000
  ): Promise<T> {
    if (!this.connected || !this.daemonProcess?.stdin) {
      throw new DaemonCommunicationError('Not connected');
    }

    const id = this.nextId++;
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id,
    };

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(
          new DaemonCommunicationError(
            `Request ${method} timed out after ${timeoutMs}ms`
          )
        );
      }, timeoutMs);

      this.pendingRequests.set(id, {
        resolve,
        reject,
        method,
        timeout,
      });

      // Write JSON-RPC request to stdin
      const json = JSON.stringify(request) + '\n';
      this.daemonProcess!.stdin!.write(json);
    });
  }

  async close(): Promise<void> {
    if (this.daemonProcess) {
      this.rejectAllPending(
        new DaemonCommunicationError('Connection closed by client')
      );

      return new Promise((resolve) => {
        this.daemonProcess!.once('exit', () => {
          this.connected = false;
          resolve();
        });
        this.daemonProcess!.kill('SIGTERM');
      });
    }
  }

  isConnected(): boolean {
    return this.connected;
  }

  on(event: 'error' | 'close', handler: (err?: Error) => void): void {
    if (event === 'error') {
      this.errorHandlers.push(handler);
    } else if (event === 'close') {
      this.closeHandlers.push(handler);
    }
  }

  private rejectAllPending(error: Error): void {
    for (const [id, pending] of this.pendingRequests) {
      if (pending.timeout) {
        clearTimeout(pending.timeout);
      }
      pending.reject(error);
    }
    this.pendingRequests.clear();
  }
}
```

### Connection Factory

```typescript
// packages/daemon-client/src/connection-factory.ts

import { Connection, ConnectionMode, ConnectionConfig } from './connection';
import { SocketConnection } from './socket';
import { StdioConnection } from './stdio';
import { connectOrSpawn, getDefaultConfig } from './discovery';
import * as fs from 'fs';

export async function createConnection(
  config: Partial<ConnectionConfig> = {}
): Promise<Connection> {
  const mode = config.mode ?? detectConnectionMode();

  switch (mode) {
    case ConnectionMode.Socket:
      return await createSocketConnection(config);

    case ConnectionMode.Stdio:
      return await createStdioConnection(config);

    case ConnectionMode.Auto:
      // Try socket first, fallback to stdio
      try {
        return await createSocketConnection(config);
      } catch (err) {
        console.warn('Socket connection failed, falling back to stdio', err);
        return await createStdioConnection(config);
      }

    default:
      throw new Error(`Unknown connection mode: ${mode}`);
  }
}

async function createSocketConnection(
  config: Partial<ConnectionConfig>
): Promise<Connection> {
  const daemonConfig = {
    ...getDefaultConfig(),
    binaryPath: config.binaryPath ?? 'crewchief-maproom',
    socketPath: config.socketPath ?? getDefaultConfig().socketPath,
    startupTimeout: config.startupTimeout ?? 10000,
  };

  return await connectOrSpawn(daemonConfig);
}

async function createStdioConnection(
  config: Partial<ConnectionConfig>
): Promise<Connection> {
  const binaryPath = config.binaryPath ?? 'crewchief-maproom';
  const conn = new StdioConnection(binaryPath);
  await conn.connect();
  return conn;
}

function detectConnectionMode(): ConnectionMode {
  // Check environment variable override
  const envMode = process.env.MAPROOM_CONNECTION_MODE?.toLowerCase();
  if (envMode === 'socket') return ConnectionMode.Socket;
  if (envMode === 'stdio') return ConnectionMode.Stdio;
  if (envMode === 'auto') return ConnectionMode.Auto;

  // Platform-based detection
  if (process.platform === 'win32') {
    // Windows: always use stdio (no Unix sockets)
    return ConnectionMode.Stdio;
  }

  // Unix: default to auto (try socket, fallback stdio)
  return ConnectionMode.Auto;
}
```

### Updated DaemonClient

Refactor DaemonClient to use Connection interface:

```typescript
// packages/daemon-client/src/client.ts

import { Connection, ConnectionMode, ConnectionConfig } from './connection';
import { createConnection } from './connection-factory';

export class DaemonClient {
  private connection: Connection | null = null;

  constructor(private config: Partial<ConnectionConfig> = {}) {}

  async connect(): Promise<void> {
    this.connection = await createConnection(this.config);
  }

  async search(params: SearchParams): Promise<SearchResult> {
    if (!this.connection) {
      throw new Error('Not connected. Call connect() first.');
    }

    return await this.connection.sendRequest<SearchResult>('search', params);
  }

  async index(params: IndexParams): Promise<IndexResult> {
    if (!this.connection) {
      throw new Error('Not connected. Call connect() first.');
    }

    return await this.connection.sendRequest<IndexResult>('index', params);
  }

  // ... other RPC methods ...

  async close(): Promise<void> {
    if (this.connection) {
      await this.connection.close();
      this.connection = null;
    }
  }

  isConnected(): boolean {
    return this.connection?.isConnected() ?? false;
  }

  onError(handler: (err: Error) => void): void {
    this.connection?.on('error', handler);
  }

  onClose(handler: () => void): void {
    this.connection?.on('close', handler);
  }
}
```

## Implementation Notes

### Backward Compatibility

Critical: Existing code using DaemonClient should work without changes.

**Before (existing code):**
```typescript
const client = new DaemonClient();
await client.connect();
const results = await client.search({ query: 'test' });
```

**After (still works):**
```typescript
const client = new DaemonClient(); // Auto-detects connection mode
await client.connect();
const results = await client.search({ query: 'test' });
```

**New explicit mode:**
```typescript
const client = new DaemonClient({ mode: ConnectionMode.Socket });
await client.connect();
```

### Environment Variable Override

Users can force specific mode:

```bash
# Force stdio mode
MAPROOM_CONNECTION_MODE=stdio code .

# Force socket mode
MAPROOM_CONNECTION_MODE=socket code .

# Auto-detect (default)
MAPROOM_CONNECTION_MODE=auto code .
# or omit entirely
```

### Platform Detection Logic

```
Windows → stdio (no Unix sockets)
Unix + auto → try socket, fallback stdio
Unix + socket → socket only (error if fails)
Unix + stdio → stdio only
```

This ensures Windows users get working stdio mode automatically.

### Dual-Mode Test Suite

All existing tests should pass with both modes:

```typescript
describe.each([
  ['stdio', ConnectionMode.Stdio],
  ['socket', ConnectionMode.Socket],
])('DaemonClient with %s mode', (modeName, mode) => {
  it('connects and sends requests', async () => {
    const client = new DaemonClient({ mode });
    await client.connect();
    expect(client.isConnected()).toBe(true);

    const result = await client.search({ query: 'test' });
    expect(result).toBeDefined();

    await client.close();
  });

  it('handles errors correctly', async () => {
    const client = new DaemonClient({ mode });
    await client.connect();

    await expect(client.search({ query: '' })).rejects.toThrow();

    await client.close();
  });
});
```

### Migration Path

1. **Phase 1 (MVP)**: Socket mode opt-in, stdio default
2. **Phase 2**: Auto-detect (socket preferred, stdio fallback)
3. **Phase 3**: Socket default after 6 months stability

## Dependencies

- MULTICN-2005 (Socket Connection Class)
- MULTICN-2006 (Connect-or-Spawn Logic)

## Risk Assessment

- **Risk**: Breaking changes to existing DaemonClient API
  - **Mitigation**: Maintain identical API surface. Only internal implementation changes.

- **Risk**: Auto-detection chooses wrong mode
  - **Mitigation**: Conservative defaults (stdio on Windows). Environment variable escape hatch.

- **Risk**: Stdio mode regressions when refactoring
  - **Mitigation**: Comprehensive test suite runs against both modes. No behavior changes.

## Files/Packages Affected

- `packages/daemon-client/src/connection.ts` (MODIFY - add ConnectionMode enum and config)
- `packages/daemon-client/src/stdio.ts` (NEW - refactored from client.ts)
- `packages/daemon-client/src/connection-factory.ts` (NEW - mode detection and creation)
- `packages/daemon-client/src/client.ts` (MODIFY - use Connection interface)
- `packages/daemon-client/src/__tests__/client.test.ts` (MODIFY - dual-mode tests)
- `packages/daemon-client/README.md` (UPDATE - document connection modes)
