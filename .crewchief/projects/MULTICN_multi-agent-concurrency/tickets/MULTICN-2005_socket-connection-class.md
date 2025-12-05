# Ticket: MULTICN-2005: Socket Connection Class

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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

Create SocketConnection class implementing the Connection interface with length-prefixed message reading, buffer management for partial reads, and request/response multiplexing via request IDs.

## Background

The TypeScript daemon client needs a socket-based transport to communicate with the Unix socket server (MULTICN-2003). This SocketConnection class mirrors the Rust JsonRpcCodec's length-delimited framing and provides the same Connection interface as the existing stdio-based transport.

This enables dual-mode support (socket/stdio) with minimal client code changes.

Reference: [architecture.md](../planning/architecture.md) - Socket Connection Implementation section

## Acceptance Criteria

- [x] SocketConnection implements Connection interface (sendRequest, close, isConnected, on)
- [x] 4-byte big-endian length prefix reading/writing matches Rust codec
- [x] Buffer management handles partial reads correctly
- [x] Request/response multiplexing via request ID
- [x] Test: Round-trip encode/decode preserves message
- [x] Test: Partial reads reassemble correctly
- [x] Test: Concurrent requests get correct responses
- [x] Test: Connection error handling and cleanup

## Technical Requirements

Create `packages/daemon-client/src/socket.ts` implementing socket-based connection.

### Connection Interface

First, define the Connection interface in `packages/daemon-client/src/connection.ts`:

```typescript
export interface Connection {
  sendRequest<T = unknown>(method: string, params?: unknown): Promise<T>;
  close(): Promise<void>;
  isConnected(): boolean;
  on(event: 'error' | 'close', handler: (err?: Error) => void): void;
}
```

### SocketConnection Implementation

```typescript
import * as net from 'net';
import { Connection } from './connection';
import { JsonRpcRequest, JsonRpcResponse, RequestId } from './rpc';
import { SocketConnectionError, SocketTimeoutError } from './errors';

interface PendingRequest {
  resolve: (value: any) => void;
  reject: (error: Error) => void;
  method: string;
  timeout?: NodeJS.Timeout;
}

export class SocketConnection implements Connection {
  private socket: net.Socket | null = null;
  private pendingRequests = new Map<RequestId, PendingRequest>();
  private buffer = Buffer.alloc(0);
  private nextId = 1;
  private connected = false;
  private errorHandlers: Array<(err?: Error) => void> = [];
  private closeHandlers: Array<(err?: Error) => void> = [];

  constructor(private socketPath: string) {}

  async connect(timeoutMs: number = 10000): Promise<void> {
    return new Promise((resolve, reject) => {
      this.socket = net.createConnection(this.socketPath);

      const timeout = setTimeout(() => {
        this.socket?.destroy();
        reject(new SocketTimeoutError(this.socketPath, timeoutMs));
      }, timeoutMs);

      this.socket.on('connect', () => {
        clearTimeout(timeout);
        this.connected = true;
        this.setupHandlers();
        resolve();
      });

      this.socket.on('error', (err) => {
        clearTimeout(timeout);
        reject(new SocketConnectionError(
          `Failed to connect to ${this.socketPath}`,
          { cause: err }
        ));
      });
    });
  }

  private setupHandlers(): void {
    if (!this.socket) return;

    this.socket.on('data', (data: Buffer) => {
      this.handleData(data);
    });

    this.socket.on('error', (err) => {
      this.connected = false;
      this.errorHandlers.forEach(h => h(err));
      this.rejectAllPending(new SocketConnectionError('Socket error', { cause: err }));
    });

    this.socket.on('close', () => {
      this.connected = false;
      this.closeHandlers.forEach(h => h());
      this.rejectAllPending(new SocketConnectionError('Socket closed unexpectedly'));
    });
  }

  private handleData(data: Buffer): void {
    // Append to buffer
    this.buffer = Buffer.concat([this.buffer, data]);

    // Process complete messages
    while (this.buffer.length >= 4) {
      // Read length prefix (4 bytes, big-endian)
      const messageLength = this.buffer.readUInt32BE(0);

      // Check if we have the full message
      if (this.buffer.length < 4 + messageLength) {
        // Need more data
        break;
      }

      // Extract message
      const messageBytes = this.buffer.slice(4, 4 + messageLength);
      this.buffer = this.buffer.slice(4 + messageLength);

      // Parse JSON
      try {
        const json = messageBytes.toString('utf8');
        const message = JSON.parse(json) as JsonRpcResponse;
        this.handleMessage(message);
      } catch (err) {
        console.error('Failed to parse JSON-RPC message:', err);
      }
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
    if (!this.connected || !this.socket) {
      throw new SocketConnectionError('Not connected');
    }

    const id = this.nextId++;
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id,
    };

    return new Promise((resolve, reject) => {
      // Set up timeout
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new SocketTimeoutError(
          `Request ${method} timed out after ${timeoutMs}ms`,
          timeoutMs
        ));
      }, timeoutMs);

      // Store pending request
      this.pendingRequests.set(id, {
        resolve,
        reject,
        method,
        timeout,
      });

      // Encode and send
      try {
        const json = JSON.stringify(request);
        const messageBytes = Buffer.from(json, 'utf8');

        // Write length prefix (4 bytes, big-endian)
        const lengthPrefix = Buffer.alloc(4);
        lengthPrefix.writeUInt32BE(messageBytes.length, 0);

        // Write to socket
        this.socket!.write(lengthPrefix);
        this.socket!.write(messageBytes);
      } catch (err) {
        clearTimeout(timeout);
        this.pendingRequests.delete(id);
        reject(new SocketConnectionError('Failed to send request', { cause: err as Error }));
      }
    });
  }

  async close(): Promise<void> {
    if (this.socket) {
      // Reject all pending requests
      this.rejectAllPending(new SocketConnectionError('Connection closed by client'));

      return new Promise((resolve) => {
        this.socket!.once('close', () => {
          this.connected = false;
          resolve();
        });
        this.socket!.end();
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

### Error Types

Update `packages/daemon-client/src/errors.ts`:

```typescript
export class SocketConnectionError extends DaemonCommunicationError {
  constructor(message: string, options?: { cause?: Error }) {
    super(message, options);
    this.code = 'SOCKET_CONNECTION_ERROR';
  }
}

export class SocketTimeoutError extends DaemonCommunicationError {
  constructor(message: string, public timeoutMs: number) {
    super(message);
    this.code = 'SOCKET_TIMEOUT';
  }
}
```

## Implementation Notes

### Buffer Management Strategy

The key challenge is handling partial reads correctly:

1. **Accumulate data**: Append incoming data to buffer
2. **Check for complete message**: Need at least 4 bytes for length
3. **Extract when ready**: Once we have length + payload, extract and parse
4. **Preserve remainder**: Keep unprocessed bytes in buffer for next iteration

This is the same strategy used by `LengthDelimitedCodec` in Rust.

### Request/Response Multiplexing

Using a `Map<RequestId, PendingRequest>`:
- **Send**: Store Promise resolve/reject with request ID
- **Receive**: Look up pending request by ID and resolve/reject
- **Timeout**: Clean up pending request after timeout

This allows multiple in-flight requests without blocking.

### Compatibility with Rust Codec

The wire format must match exactly:

**Rust encoder:**
```rust
lengthPrefix.writeUInt32BE(payload.length, 0)
socket.write(lengthPrefix + payload)
```

**TypeScript decoder:**
```typescript
const length = buffer.readUInt32BE(0);
const payload = buffer.slice(4, 4 + length);
```

Both use big-endian 32-bit integers for the length prefix.

### Unit Tests

```typescript
// packages/daemon-client/src/__tests__/socket.test.ts

import { SocketConnection } from '../socket';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as net from 'net';

describe('SocketConnection', () => {
  let server: net.Server;
  let socketPath: string;

  beforeEach(async () => {
    socketPath = `/tmp/test-socket-${Date.now()}.sock`;
    server = net.createServer();
    await new Promise<void>((resolve) => {
      server.listen(socketPath, resolve);
    });
  });

  afterEach(async () => {
    if (server) {
      await new Promise<void>((resolve) => {
        server.close(() => resolve());
      });
    }
  });

  it('connects to socket', async () => {
    const conn = new SocketConnection(socketPath);
    await conn.connect();
    expect(conn.isConnected()).toBe(true);
    await conn.close();
  });

  it('handles partial reads correctly', async () => {
    server.on('connection', (socket) => {
      // Send response in two chunks to simulate partial read
      const response = {
        jsonrpc: '2.0',
        result: { status: 'ok' },
        id: 1,
      };
      const json = JSON.stringify(response);
      const messageBytes = Buffer.from(json, 'utf8');
      const lengthPrefix = Buffer.alloc(4);
      lengthPrefix.writeUInt32BE(messageBytes.length, 0);

      const fullMessage = Buffer.concat([lengthPrefix, messageBytes]);

      // Send first half
      socket.write(fullMessage.slice(0, fullMessage.length / 2));

      // Send second half after delay
      setTimeout(() => {
        socket.write(fullMessage.slice(fullMessage.length / 2));
      }, 10);
    });

    const conn = new SocketConnection(socketPath);
    await conn.connect();

    const result = await conn.sendRequest('test');
    expect(result).toEqual({ status: 'ok' });

    await conn.close();
  });

  it('multiplexes concurrent requests', async () => {
    server.on('connection', (socket) => {
      // Echo back request ID in response
      socket.on('data', (data) => {
        // Parse request to get ID
        const length = data.readUInt32BE(0);
        const json = data.slice(4, 4 + length).toString('utf8');
        const request = JSON.parse(json);

        const response = {
          jsonrpc: '2.0',
          result: { requestId: request.id },
          id: request.id,
        };

        const responseJson = JSON.stringify(response);
        const responseBytes = Buffer.from(responseJson, 'utf8');
        const lengthPrefix = Buffer.alloc(4);
        lengthPrefix.writeUInt32BE(responseBytes.length, 0);

        socket.write(lengthPrefix);
        socket.write(responseBytes);
      });
    });

    const conn = new SocketConnection(socketPath);
    await conn.connect();

    // Send 3 requests concurrently
    const [result1, result2, result3] = await Promise.all([
      conn.sendRequest('test1'),
      conn.sendRequest('test2'),
      conn.sendRequest('test3'),
    ]);

    // Each should get its own response
    expect(result1).toEqual({ requestId: 1 });
    expect(result2).toEqual({ requestId: 2 });
    expect(result3).toEqual({ requestId: 3 });

    await conn.close();
  });

  it('rejects pending requests on disconnect', async () => {
    const conn = new SocketConnection(socketPath);
    await conn.connect();

    const requestPromise = conn.sendRequest('test');

    // Close socket from server side
    server.close();

    await expect(requestPromise).rejects.toThrow(SocketConnectionError);
  });

  it('times out long-running requests', async () => {
    server.on('connection', () => {
      // Don't send response (simulate hang)
    });

    const conn = new SocketConnection(socketPath);
    await conn.connect();

    await expect(
      conn.sendRequest('slow', undefined, 100) // 100ms timeout
    ).rejects.toThrow(SocketTimeoutError);

    await conn.close();
  });
});
```

## Dependencies

- MULTICN-2001 (JSON-RPC Codec) - must match wire format exactly
- Node.js net module (built-in)

## Risk Assessment

- **Risk**: Buffer management bugs cause message corruption
  - **Mitigation**: Comprehensive unit tests with partial reads. Same pattern as Rust LengthDelimitedCodec.

- **Risk**: Memory leak from unclosed sockets or pending requests
  - **Mitigation**: Explicit cleanup in close() and error handlers. RAII-style resource management.

- **Risk**: Request/response ID collision after integer overflow
  - **Mitigation**: Use 32-bit IDs. Overflow unlikely in practice (2^32 requests). Could use UUID if needed.

## Files/Packages Affected

- `packages/daemon-client/src/connection.ts` (NEW - interface definition)
- `packages/daemon-client/src/socket.ts` (NEW - SocketConnection class)
- `packages/daemon-client/src/errors.ts` (MODIFY - add socket errors)
- `packages/daemon-client/src/__tests__/socket.test.ts` (NEW - unit tests)
