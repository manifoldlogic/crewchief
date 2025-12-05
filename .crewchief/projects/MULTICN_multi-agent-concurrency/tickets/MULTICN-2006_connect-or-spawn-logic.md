# Ticket: MULTICN-2006: Connect-or-Spawn Logic

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
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Implement daemon discovery and auto-start logic using the double-check pattern with `proper-lockfile` for race-free coordination. Try connecting to existing socket first (fast path), then spawn daemon detached if needed with 10-second wait for socket availability.

## Background

Multiple agents starting simultaneously must coordinate to ensure only one daemon spawns. The double-check pattern (check → lock → check again → spawn) prevents race conditions that could result in multiple daemons. Using the battle-tested `proper-lockfile` library provides cross-platform file locking.

This is a critical component for production reliability - must handle concurrent startup correctly.

Reference: [architecture.md](../planning/architecture.md) - Connect-or-Spawn State Machine section

## Acceptance Criteria

- [x] Uses `proper-lockfile` library (not custom lock implementation)
- [x] Lock file is `/tmp/maproom-{uid}.lock` (distinct from socket path)
- [x] Implements double-check pattern: try connect before and after acquiring lock
- [x] Spawns daemon with `detached: true`, `stdio: 'ignore'`, and calls `daemon.unref()`
- [x] Waits up to 10 seconds for socket to become available after spawn
- [x] Test: 5 concurrent clients calling connectOrSpawn() only create 1 daemon
- [x] Test: PID verification shows only 1 maproom daemon process

## Technical Requirements

Create `packages/daemon-client/src/discovery.ts` implementing connect-or-spawn logic.

### ConnectOrSpawn Implementation

```typescript
import { lock, unlock, LockOptions } from 'proper-lockfile';
import * as fs from 'fs';
import * as net from 'net';
import { spawn, ChildProcess } from 'child_process';
import { SocketConnection } from './socket';
import { DaemonStartupError, DaemonLockError } from './errors';

export interface DaemonConfig {
  binaryPath: string;
  socketPath: string;
  lockPath: string;
  startupTimeout: number; // milliseconds
}

export function getDefaultConfig(): DaemonConfig {
  const uid = process.getuid?.() ?? 0;
  return {
    binaryPath: 'crewchief-maproom', // Assume in PATH
    socketPath: `/tmp/maproom-${uid}.sock`,
    lockPath: `/tmp/maproom-${uid}.lock`,
    startupTimeout: 10000, // 10 seconds
  };
}

/**
 * Connect to existing daemon or spawn new one if needed.
 * Uses double-check pattern with file locking to prevent race conditions.
 *
 * @throws DaemonStartupError if daemon fails to start
 * @throws DaemonLockError if lock acquisition fails
 */
export async function connectOrSpawn(
  config: DaemonConfig = getDefaultConfig()
): Promise<SocketConnection> {
  // 1. Fast path: Try connecting to existing daemon
  try {
    const conn = new SocketConnection(config.socketPath);
    await conn.connect(1000); // Fast timeout for existing daemon
    console.log('Connected to existing daemon');
    return conn;
  } catch (err) {
    console.log('No existing daemon found, will attempt spawn');
  }

  // 2. Acquire lock to coordinate concurrent spawn attempts
  const lockRelease = await acquireLock(config.lockPath);

  try {
    // 3. Double-check: Another process may have spawned daemon while we waited for lock
    try {
      const conn = new SocketConnection(config.socketPath);
      await conn.connect(1000);
      console.log('Another process spawned daemon while waiting for lock');
      return conn;
    } catch {
      console.log('Verified no daemon exists, will spawn');
    }

    // 4. Spawn daemon process
    console.log('Spawning new daemon', { socketPath: config.socketPath });
    spawnDaemon(config);

    // 5. Wait for socket to become available
    await waitForSocket(config.socketPath, {
      timeout: config.startupTimeout,
      pollInterval: 100,
    });

    // 6. Connect to newly spawned daemon
    const conn = new SocketConnection(config.socketPath);
    await conn.connect(2000);
    console.log('Successfully spawned and connected to daemon');
    return conn;
  } finally {
    // Always release lock
    await lockRelease();
  }
}

/**
 * Acquire exclusive lock on daemon spawn coordination file.
 */
async function acquireLock(lockPath: string): Promise<() => Promise<void>> {
  // Ensure lock file exists (proper-lockfile requires it)
  if (!fs.existsSync(lockPath)) {
    fs.writeFileSync(lockPath, '', { mode: 0o600 });
  }

  const lockOptions: LockOptions = {
    retries: {
      retries: 10,
      minTimeout: 100,
      maxTimeout: 1000,
    },
    stale: 30000, // Lock expires after 30s (prevents deadlock if process crashes)
  };

  try {
    const release = await lock(lockPath, lockOptions);
    console.log('Acquired spawn coordination lock');
    return release;
  } catch (err) {
    throw new DaemonLockError(
      `Failed to acquire lock: ${lockPath}`,
      { cause: err as Error }
    );
  }
}

/**
 * Spawn daemon process in detached mode.
 * Process runs independently of parent and doesn't block.
 */
function spawnDaemon(config: DaemonConfig): void {
  const daemon = spawn(
    config.binaryPath,
    ['serve', '--socket', config.socketPath],
    {
      detached: true, // Run independently of parent
      stdio: 'ignore', // Don't inherit stdio (prevents blocking)
      env: {
        ...process.env,
        RUST_LOG: process.env.RUST_LOG ?? 'info',
      },
    }
  );

  // Allow parent to exit without waiting for daemon
  daemon.unref();

  console.log('Daemon process spawned', { pid: daemon.pid });
}

/**
 * Wait for socket file to appear and be connectable.
 */
async function waitForSocket(
  socketPath: string,
  options: { timeout: number; pollInterval: number }
): Promise<void> {
  const start = Date.now();

  while (Date.now() - start < options.timeout) {
    // Check if socket file exists
    if (fs.existsSync(socketPath)) {
      // Try connecting to verify socket is ready
      try {
        const testSocket = net.createConnection(socketPath);
        await new Promise<void>((resolve, reject) => {
          testSocket.on('connect', () => {
            testSocket.destroy();
            resolve();
          });
          testSocket.on('error', reject);
          setTimeout(() => reject(new Error('timeout')), 500);
        });

        console.log('Socket is ready', { socketPath });
        return; // Success
      } catch {
        // Socket file exists but not ready yet
        console.log('Socket file exists but not ready, retrying...');
      }
    }

    // Wait before next check
    await new Promise((resolve) => setTimeout(resolve, options.pollInterval));
  }

  throw new DaemonStartupError(
    `Socket not ready after ${options.timeout}ms: ${socketPath}`
  );
}
```

### Error Types

Update `packages/daemon-client/src/errors.ts`:

```typescript
export class DaemonLockError extends DaemonStartupError {
  constructor(message: string, options?: { cause?: Error }) {
    super(message, options);
    this.code = 'DAEMON_LOCK_ERROR';
  }
}
```

## Implementation Notes

### Why proper-lockfile?

Using the `proper-lockfile` library instead of custom implementation:
- **Cross-platform**: Works on Windows (lockfiles) and Unix (flock)
- **Stale lock handling**: Automatically expires locks from crashed processes
- **Retry logic**: Built-in exponential backoff for lock acquisition
- **Battle-tested**: Used by npm and other production tools

Alternative considered: Custom flock implementation - rejected due to cross-platform complexity.

### Lock File vs Socket File

**Separate files for lock and socket:**
- Lock: `/tmp/maproom-{uid}.lock` - coordination only
- Socket: `/tmp/maproom-{uid}.sock` - actual communication

Why separate?
- Socket file is owned by daemon process
- Lock file is shared across all clients
- Prevents confusion about lifecycle ownership

### Double-Check Pattern

The state machine prevents race conditions:

```
Time  | Client A              | Client B
------+----------------------+----------------------
T0    | Try connect (fail)    |
T1    | Acquire lock          |
T2    |                       | Try connect (fail)
T3    |                       | Wait for lock...
T4    | Check again (fail)    |
T5    | Spawn daemon          |
T6    | Wait for socket       |
T7    | Connect (success)     |
T8    | Release lock          |
T9    |                       | Acquire lock
T10   |                       | Check again (SUCCESS!)
T11   |                       | Release lock
```

Client B benefits from Client A's spawn without duplicating work.

### Detached Process Requirements

Three things needed for proper daemon detachment:

1. **`detached: true`**: Process runs in own session
2. **`stdio: 'ignore'`**: Don't inherit parent's stdio
3. **`daemon.unref()`**: Allow parent to exit without waiting

All three are critical - missing any causes issues.

### Integration Tests

```typescript
// packages/daemon-client/src/__tests__/discovery.test.ts

import { connectOrSpawn, getDefaultConfig } from '../discovery';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

describe('connectOrSpawn', () => {
  let config: DaemonConfig;

  beforeEach(() => {
    config = {
      ...getDefaultConfig(),
      socketPath: `/tmp/test-maproom-${Date.now()}.sock`,
      lockPath: `/tmp/test-maproom-${Date.now()}.lock`,
    };

    // Clean up any existing files
    [config.socketPath, config.lockPath].forEach((path) => {
      if (fs.existsSync(path)) {
        fs.unlinkSync(path);
      }
    });
  });

  afterEach(async () => {
    // Kill any test daemons
    try {
      await execAsync('pkill -f "crewchief-maproom.*serve.*test-maproom"');
    } catch {
      // Ignore if no processes found
    }

    // Clean up files
    [config.socketPath, config.lockPath].forEach((path) => {
      if (fs.existsSync(path)) {
        fs.unlinkSync(path);
      }
    });
  });

  it('spawns daemon on first call', async () => {
    const conn = await connectOrSpawn(config);
    expect(conn.isConnected()).toBe(true);

    // Verify daemon process exists
    const { stdout } = await execAsync('ps aux | grep crewchief-maproom | grep -v grep');
    expect(stdout).toContain('serve');

    await conn.close();
  });

  it('reuses existing daemon on second call', async () => {
    // First call spawns daemon
    const conn1 = await connectOrSpawn(config);

    // Second call should reuse
    const conn2 = await connectOrSpawn(config);

    expect(conn1.isConnected()).toBe(true);
    expect(conn2.isConnected()).toBe(true);

    // Verify only one daemon process
    const { stdout } = await execAsync('ps aux | grep crewchief-maproom | grep -v grep');
    const lines = stdout.trim().split('\n');
    expect(lines.length).toBe(1);

    await conn1.close();
    await conn2.close();
  });

  it('handles concurrent spawn attempts (race condition)', async () => {
    // Spawn 5 clients simultaneously
    const connections = await Promise.all([
      connectOrSpawn(config),
      connectOrSpawn(config),
      connectOrSpawn(config),
      connectOrSpawn(config),
      connectOrSpawn(config),
    ]);

    // All should connect successfully
    expect(connections.every((c) => c.isConnected())).toBe(true);

    // Verify only one daemon spawned
    const { stdout } = await execAsync('ps aux | grep crewchief-maproom | grep -v grep');
    const lines = stdout.trim().split('\n');
    expect(lines.length).toBe(1);

    // Clean up
    await Promise.all(connections.map((c) => c.close()));
  }, 30000); // Longer timeout for this test

  it('throws error if daemon fails to start', async () => {
    const badConfig = {
      ...config,
      binaryPath: '/nonexistent/binary',
    };

    await expect(connectOrSpawn(badConfig)).rejects.toThrow(DaemonStartupError);
  });

  it('times out if daemon never creates socket', async () => {
    const badConfig = {
      ...config,
      binaryPath: 'sleep', // Won't create socket
      startupTimeout: 1000, // Short timeout for test
    };

    await expect(connectOrSpawn(badConfig)).rejects.toThrow(DaemonStartupError);
  });
});
```

## Dependencies

- MULTICN-2005 (Socket Connection Class)
- proper-lockfile package (add to package.json)

```json
{
  "dependencies": {
    "proper-lockfile": "^4.1.2"
  }
}
```

## Risk Assessment

- **Risk**: Lock file coordination fails on NFS filesystems
  - **Mitigation**: Use /tmp (local filesystem). Document NFS limitations.

- **Risk**: Daemon spawn fails silently
  - **Mitigation**: Wait for socket with timeout. Error if socket never appears.

- **Risk**: Orphaned daemon processes after crashes
  - **Mitigation**: PID file tracking (MULTICN-2003) and idle timeout (MULTICN-2004).

- **Risk**: Lock file becomes stale and blocks startup
  - **Mitigation**: proper-lockfile has built-in stale lock detection (30s timeout).

## Files/Packages Affected

- `packages/daemon-client/src/discovery.ts` (NEW)
- `packages/daemon-client/src/errors.ts` (MODIFY - add DaemonLockError)
- `packages/daemon-client/package.json` (ADD proper-lockfile dependency)
- `packages/daemon-client/src/__tests__/discovery.test.ts` (NEW - integration tests)
