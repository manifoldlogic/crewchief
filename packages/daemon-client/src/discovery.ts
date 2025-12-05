/**
 * Daemon discovery and auto-start logic using double-check pattern
 *
 * Handles coordination between multiple concurrent clients to ensure only
 * one daemon process is spawned. Uses proper-lockfile for cross-platform
 * file locking with built-in stale lock handling.
 */

import { lock, unlock, LockOptions } from 'proper-lockfile'
import * as fs from 'node:fs'
import * as net from 'node:net'
import { spawn } from 'node:child_process'
import { SocketConnection } from './socket.js'
import { DaemonStartupError, DaemonLockError } from './errors.js'

/**
 * Configuration for daemon discovery and spawning
 */
export interface DiscoveryConfig {
  /** Path to crewchief-maproom binary */
  binaryPath: string
  /** Unix socket path for communication */
  socketPath: string
  /** Lock file path for spawn coordination */
  lockPath: string
  /** Maximum time to wait for daemon startup (ms) */
  startupTimeout: number
}

/**
 * Get default daemon configuration based on user ID
 *
 * Uses /tmp directory with user-specific naming to avoid conflicts.
 */
export function getDefaultConfig(): DiscoveryConfig {
  const uid = process.getuid?.() ?? 0
  return {
    binaryPath: 'crewchief-maproom', // Assume in PATH
    socketPath: `/tmp/maproom-${uid}.sock`,
    lockPath: `/tmp/maproom-${uid}.lock`,
    startupTimeout: 10000, // 10 seconds
  }
}

/**
 * Connect to existing daemon or spawn new one if needed.
 *
 * Uses double-check pattern with file locking to prevent race conditions:
 * 1. Try connecting (fast path)
 * 2. Acquire lock
 * 3. Try connecting again (another process may have spawned while waiting)
 * 4. Spawn daemon if still needed
 * 5. Wait for socket to become ready
 * 6. Release lock
 *
 * This ensures that even with 5+ concurrent clients, only one daemon spawns.
 *
 * @param config - Daemon configuration (defaults to user-specific config)
 * @returns Promise resolving to connected SocketConnection
 * @throws {DaemonStartupError} if daemon fails to start within timeout
 * @throws {DaemonLockError} if lock acquisition fails
 */
export async function connectOrSpawn(
  config: DiscoveryConfig = getDefaultConfig()
): Promise<SocketConnection> {
  // 1. Fast path: Try connecting to existing daemon
  try {
    const conn = new SocketConnection(config.socketPath)
    await conn.connect(1000) // Fast timeout for existing daemon
    console.log('Connected to existing daemon')
    return conn
  } catch (err) {
    console.log('No existing daemon found, will attempt spawn')
  }

  // 2. Acquire lock to coordinate concurrent spawn attempts
  const lockRelease = await acquireLock(config.lockPath)

  try {
    // 3. Double-check: Another process may have spawned daemon while we waited for lock
    try {
      const conn = new SocketConnection(config.socketPath)
      await conn.connect(1000)
      console.log('Another process spawned daemon while waiting for lock')
      return conn
    } catch {
      console.log('Verified no daemon exists, will spawn')
    }

    // 4. Spawn daemon process
    console.log('Spawning new daemon', { socketPath: config.socketPath })
    spawnDaemon(config)

    // 5. Wait for socket to become available
    await waitForSocket(config.socketPath, {
      timeout: config.startupTimeout,
      pollInterval: 100,
    })

    // 6. Connect to newly spawned daemon
    const conn = new SocketConnection(config.socketPath)
    await conn.connect(2000)
    console.log('Successfully spawned and connected to daemon')
    return conn
  } finally {
    // Always release lock
    await lockRelease()
  }
}

/**
 * Acquire exclusive lock on daemon spawn coordination file.
 *
 * Uses proper-lockfile library which provides:
 * - Cross-platform locking (flock on Unix, lockfiles on Windows)
 * - Stale lock detection (30s timeout)
 * - Retry logic with exponential backoff
 *
 * @param lockPath - Path to lock file
 * @returns Function to release the lock
 * @throws {DaemonLockError} if lock acquisition fails
 */
async function acquireLock(lockPath: string): Promise<() => Promise<void>> {
  // Ensure lock file exists (proper-lockfile requires it)
  if (!fs.existsSync(lockPath)) {
    fs.writeFileSync(lockPath, '', { mode: 0o600 })
  }

  const lockOptions: LockOptions = {
    retries: {
      retries: 10,
      minTimeout: 100,
      maxTimeout: 1000,
    },
    stale: 30000, // Lock expires after 30s (prevents deadlock if process crashes)
  }

  try {
    const release = await lock(lockPath, lockOptions)
    console.log('Acquired spawn coordination lock')
    return release
  } catch (err) {
    throw new DaemonLockError(`Failed to acquire lock: ${lockPath}`, {
      cause: err as Error,
    })
  }
}

/**
 * Spawn daemon process in detached mode.
 *
 * Three critical requirements for proper detachment:
 * 1. detached: true - Process runs in own session
 * 2. stdio: 'ignore' - Don't inherit parent's stdio (prevents blocking)
 * 3. daemon.unref() - Allow parent to exit without waiting (if available)
 *
 * Note: unref() may not be available in all environments (e.g., test mocks),
 * so we check for its existence before calling.
 *
 * @param config - Daemon configuration
 */
function spawnDaemon(config: DiscoveryConfig): void {
  const daemon = spawn(
    config.binaryPath,
    ['serve', '--socket', '--socket-path', config.socketPath],
    {
      detached: true, // Run independently of parent
      stdio: 'ignore', // Don't inherit stdio (prevents blocking)
      env: {
        ...process.env,
        RUST_LOG: process.env.RUST_LOG ?? 'info',
      },
    }
  )

  // Suppress error events on daemon process (errors will be caught by socket wait timeout)
  // This prevents unhandled error exceptions from spawn failures
  daemon.on('error', (err) => {
    console.log('Daemon spawn error (will be caught by socket wait):', err.message)
  })

  // Allow parent to exit without waiting for daemon
  // Check if unref exists (may not be available in all environments, e.g., test mocks)
  if (typeof daemon.unref === 'function') {
    daemon.unref()
  }

  console.log('Daemon process spawned', { pid: daemon.pid })
}

/**
 * Wait for socket file to appear and be connectable.
 *
 * Polls for socket file existence and tests connection readiness.
 * Simply checking file existence is insufficient - the daemon must
 * be listening and ready to accept connections.
 *
 * @param socketPath - Path to Unix socket
 * @param options - Polling configuration
 * @throws {DaemonStartupError} if socket doesn't become ready within timeout
 */
async function waitForSocket(
  socketPath: string,
  options: { timeout: number; pollInterval: number }
): Promise<void> {
  const start = Date.now()

  while (Date.now() - start < options.timeout) {
    // Check if socket file exists
    if (fs.existsSync(socketPath)) {
      // Try connecting to verify socket is ready
      try {
        const testSocket = net.createConnection(socketPath)
        await new Promise<void>((resolve, reject) => {
          testSocket.on('connect', () => {
            testSocket.destroy()
            resolve()
          })
          testSocket.on('error', reject)
          setTimeout(() => reject(new Error('timeout')), 500)
        })

        console.log('Socket is ready', { socketPath })
        return // Success
      } catch {
        // Socket file exists but not ready yet
        console.log('Socket file exists but not ready, retrying...')
      }
    }

    // Wait before next check
    await new Promise((resolve) => setTimeout(resolve, options.pollInterval))
  }

  throw new DaemonStartupError(
    `Socket not ready after ${options.timeout}ms: ${socketPath}`
  )
}
