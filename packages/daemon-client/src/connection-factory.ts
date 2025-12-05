/**
 * Connection factory with mode detection
 *
 * Handles automatic mode selection based on platform and environment variables:
 * - Windows: Always uses stdio (no Unix sockets)
 * - Unix: Defaults to auto (socket with stdio fallback)
 * - Override: MAPROOM_CONNECTION_MODE env var (socket/stdio/auto)
 */

import { Connection, ConnectionMode, ConnectionConfig } from './connection.js'
import { SocketConnection } from './socket.js'
import { StdioConnection } from './stdio.js'
import { connectOrSpawn, getDefaultConfig } from './discovery.js'

/**
 * Create a connection based on configuration
 *
 * Supports three modes:
 * - socket: Connect to daemon via Unix socket (fast path, multi-client)
 * - stdio: Spawn daemon and communicate via stdin/stdout (portable)
 * - auto: Try socket first, fallback to stdio if socket fails
 *
 * @param config - Partial connection configuration (uses defaults for missing values)
 * @returns Promise resolving to a connected Connection
 * @throws {Error} If connection fails in all attempted modes
 */
export async function createConnection(
  config: Partial<ConnectionConfig> = {}
): Promise<Connection> {
  const mode = config.mode ?? detectConnectionMode()
  const binaryPath = config.binaryPath ?? 'crewchief-maproom'

  switch (mode) {
    case ConnectionMode.Socket:
      return await createSocketConnection(config)

    case ConnectionMode.Stdio:
      return await createStdioConnection(config)

    case ConnectionMode.Auto:
      // Try socket first, fallback to stdio
      try {
        console.log('Auto mode: trying socket connection first')
        return await createSocketConnection(config)
      } catch (err) {
        console.warn(
          `Socket connection failed, falling back to stdio: ${err instanceof Error ? err.message : String(err)}`
        )
        return await createStdioConnection(config)
      }

    default:
      throw new Error(`Unknown connection mode: ${mode}`)
  }
}

/**
 * Create a socket-based connection
 *
 * Uses the connect-or-spawn pattern:
 * 1. Try connecting to existing daemon
 * 2. If no daemon, spawn one and wait for socket
 * 3. Connect to the newly spawned daemon
 *
 * @param config - Connection configuration
 * @returns Promise resolving to a connected SocketConnection
 * @throws {DaemonStartupError} if daemon fails to start
 * @throws {SocketConnectionError} if connection fails
 */
async function createSocketConnection(
  config: Partial<ConnectionConfig>
): Promise<Connection> {
  const defaultConfig = getDefaultConfig()

  const discoveryConfig = {
    binaryPath: config.binaryPath ?? defaultConfig.binaryPath,
    socketPath: config.socketPath ?? defaultConfig.socketPath,
    lockPath: defaultConfig.lockPath,
    startupTimeout: config.startupTimeout ?? defaultConfig.startupTimeout,
  }

  return await connectOrSpawn(discoveryConfig)
}

/**
 * Create a stdio-based connection
 *
 * Spawns a daemon process and communicates via stdin/stdout.
 * This is the fallback mode for platforms without Unix sockets.
 *
 * @param config - Connection configuration
 * @returns Promise resolving to a connected StdioConnection
 * @throws {DaemonStartupError} if daemon fails to start
 */
async function createStdioConnection(
  config: Partial<ConnectionConfig>
): Promise<Connection> {
  const binaryPath = config.binaryPath ?? 'crewchief-maproom'
  const conn = new StdioConnection(binaryPath)
  await conn.connect()
  return conn
}

/**
 * Detect the appropriate connection mode based on platform and environment
 *
 * Detection logic:
 * 1. Check MAPROOM_CONNECTION_MODE env var (socket/stdio/auto)
 * 2. Platform detection:
 *    - Windows → stdio (no Unix sockets)
 *    - Unix → auto (socket preferred, stdio fallback)
 *
 * @returns Detected ConnectionMode
 */
export function detectConnectionMode(): ConnectionMode {
  // Check environment variable override
  const envMode = process.env.MAPROOM_CONNECTION_MODE?.toLowerCase()
  if (envMode === 'socket') return ConnectionMode.Socket
  if (envMode === 'stdio') return ConnectionMode.Stdio
  if (envMode === 'auto') return ConnectionMode.Auto

  // Platform-based detection
  if (process.platform === 'win32') {
    // Windows: always use stdio (no Unix sockets)
    console.log('Platform detection: Windows → stdio mode')
    return ConnectionMode.Stdio
  }

  // Unix: default to auto (try socket, fallback stdio)
  console.log('Platform detection: Unix → auto mode (socket preferred)')
  return ConnectionMode.Auto
}
