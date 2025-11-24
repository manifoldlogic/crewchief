/**
 * Docker Compose service manager for VSCode extension
 *
 * Manages the lifecycle of Docker Compose services (PostgreSQL, Ollama, MCP server)
 * with robust health checking, error handling, and graceful shutdown.
 *
 * Key features:
 * - Idempotent service startup
 * - Exponential backoff health checks
 * - Docker daemon detection
 * - Clean shutdown with proper signal handling
 * - Comprehensive error reporting to VSCode output channel
 */

import { spawn, type ChildProcess } from 'node:child_process'
import type { OutputChannel } from 'vscode'
import { createConnection } from 'node:net'
import path from 'node:path'

/**
 * Error thrown when Docker operations fail
 */
export class DockerError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly exitCode?: number,
    public readonly stderr?: string
  ) {
    super(message)
    this.name = 'DockerError'
  }
}

/**
 * Health check configuration
 */
interface HealthCheckConfig {
  /** Maximum number of health check attempts */
  maxAttempts: number
  /** Initial delay in milliseconds */
  initialDelay: number
  /** Maximum delay in milliseconds */
  maxDelay: number
  /** Total timeout in milliseconds */
  totalTimeout: number
}

/**
 * Docker Compose manager for Maproom services
 *
 * Handles startup, health checking, and shutdown of:
 * - PostgreSQL database (maproom-postgres)
 * - Ollama embedding service (maproom-ollama)
 * - MCP server (maproom-mcp)
 */
export class DockerManager {
  private readonly outputChannel: OutputChannel
  private readonly composeFilePath: string
  private readonly workingDirectory: string
  private readonly healthCheckConfig: HealthCheckConfig

  /**
   * Create a new Docker manager
   *
   * @param outputChannel - VSCode output channel for logging
   * @param extensionRoot - Root directory of the VSCode extension (defaults to auto-detected)
   * @param composeFilePath - Path to docker-compose.yml (defaults to config/docker-compose.yml)
   */
  constructor(outputChannel: OutputChannel, extensionRoot?: string, composeFilePath?: string) {
    this.outputChannel = outputChannel

    // Auto-detect extension root if not provided
    if (!extensionRoot) {
      // In CommonJS, __dirname is available globally
      // From src/docker/manager.ts, we go up two levels to reach extension root
      extensionRoot = path.resolve(__dirname, '..', '..')
    }

    this.workingDirectory = extensionRoot
    this.composeFilePath = composeFilePath || path.join(extensionRoot, 'config', 'docker-compose.yml')

    // Health check configuration with exponential backoff
    this.healthCheckConfig = {
      maxAttempts: 10,
      initialDelay: 1000, // 1s
      maxDelay: 16000, // 16s
      totalTimeout: 30000, // 30s total
    }

    this.log('Docker manager initialized')
    this.log(`Extension root: ${extensionRoot}`)
    this.log(`Compose file: ${this.composeFilePath}`)
  }

  /**
   * Log a message to the output channel
   *
   * @param message - Message to log
   */
  private log(message: string): void {
    const timestamp = new Date().toISOString()
    this.outputChannel.appendLine(`[${timestamp}] ${message}`)
  }

  /**
   * Log an error to the output channel
   *
   * @param message - Error message
   * @param error - Optional error object
   */
  private logError(message: string, error?: Error): void {
    this.log(`ERROR: ${message}`)
    if (error) {
      this.log(`  ${error.message}`)
      if (error.stack) {
        this.log(`  ${error.stack}`)
      }
    }
  }

  /**
   * Check if Docker command exists and daemon is running
   *
   * @throws DockerError if Docker is not available
   */
  private async checkDockerAvailable(): Promise<void> {
    try {
      const result = await this.spawnCommand('docker', ['info'], { timeout: 5000 })

      if (result.code !== 0) {
        // Check for common "daemon not running" error messages
        const stderrLower = result.stderr.toLowerCase()
        if (
          stderrLower.includes('daemon') ||
          stderrLower.includes('not running') ||
          stderrLower.includes('cannot connect')
        ) {
          throw new DockerError(
            'Docker daemon is not running. Please start Docker Desktop and try again.',
            'DOCKER_DAEMON_NOT_RUNNING',
            result.code,
            result.stderr
          )
        }

        throw new DockerError(
          `Docker command failed: ${result.stderr}`,
          'DOCKER_COMMAND_FAILED',
          result.code,
          result.stderr
        )
      }
    } catch (error: any) {
      if (error instanceof DockerError) {
        throw error
      }

      // Check if Docker command itself doesn't exist
      if (error.code === 'ENOENT' || error.code === 'SPAWN_ERROR') {
        throw new DockerError(
          'Docker command not found. Please install Docker Desktop and ensure it is in your PATH.',
          'DOCKER_NOT_FOUND'
        )
      }

      throw new DockerError(
        `Failed to check Docker availability: ${error.message}`,
        'DOCKER_CHECK_FAILED'
      )
    }
  }

  /**
   * Ensure Docker Compose services are running
   *
   * This method is idempotent - it will not restart services that are already running.
   *
   * @param provider - Embedding provider ('ollama', 'openai', 'google')
   * @param envVars - Environment variables to pass to docker compose (provider config, API keys)
   * @throws DockerError if services cannot be started or health checks fail
   */
  async ensureServicesRunning(
    provider: string,
    envVars: Record<string, string> = {}
  ): Promise<void> {
    this.log(`Ensuring Docker Compose services are running for provider: ${provider}...`)

    try {
      // Check Docker availability first
      await this.checkDockerAvailable()

      // Determine which services to start based on provider
      // Always start: postgres, maproom-mcp
      // Conditional: ollama (only if provider === 'ollama')
      const services = provider === 'ollama'
        ? ['postgres', 'ollama', 'maproom-mcp']
        : ['postgres', 'maproom-mcp']

      this.log(`Starting services: ${services.join(', ')}`)

      // Start services with docker compose up -d
      const result = await this.spawnCommand(
        'docker',
        ['compose', '-f', this.composeFilePath, 'up', '-d', ...services],
        {
          timeout: 60000, // 60s timeout for pulling images
          cwd: this.workingDirectory,
          env: envVars,
        }
      )

      if (result.code !== 0) {
        throw new DockerError(
          `Failed to start Docker Compose services: ${result.stderr}`,
          'COMPOSE_UP_FAILED',
          result.code,
          result.stderr
        )
      }

      this.log('Services started successfully')
      this.log(result.stdout)

      // Wait for services to be healthy
      this.log('Waiting for services to be healthy...')
      await this.waitForHealthy()

      this.log('All services are healthy and ready')
    } catch (error: any) {
      this.logError('Failed to ensure services are running', error)
      throw error
    }
  }

  /**
   * Stop Docker Compose services gracefully
   *
   * @throws DockerError if shutdown fails
   */
  async stop(): Promise<void> {
    this.log('Stopping Docker Compose services...')

    try {
      const result = await this.spawnCommand(
        'docker',
        ['compose', '-f', this.composeFilePath, 'down'],
        {
          timeout: 30000, // 30s timeout
          cwd: this.workingDirectory,
        }
      )

      if (result.code !== 0) {
        throw new DockerError(
          `Failed to stop Docker Compose services: ${result.stderr}`,
          'COMPOSE_DOWN_FAILED',
          result.code,
          result.stderr
        )
      }

      this.log('Services stopped successfully')
      this.log(result.stdout)
    } catch (error: any) {
      this.logError('Failed to stop services', error)
      throw error
    }
  }

  /**
   * Wait for all services to pass health checks
   *
   * Uses exponential backoff: 1s, 2s, 4s, 8s, 16s
   *
   * @throws DockerError if health checks fail or timeout
   */
  private async waitForHealthy(): Promise<void> {
    const startTime = Date.now()

    // Currently only checking PostgreSQL health
    // MCP server health can be added later if needed
    await this.checkPostgresHealth(startTime)
  }

  /**
   * Check PostgreSQL health with exponential backoff
   *
   * @param startTime - Start time for timeout calculation
   * @throws DockerError if health check fails or times out
   */
  private async checkPostgresHealth(startTime: number): Promise<void> {
    const { maxAttempts, initialDelay, maxDelay, totalTimeout } = this.healthCheckConfig

    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      // Check total timeout
      const elapsed = Date.now() - startTime
      if (elapsed >= totalTimeout) {
        throw new DockerError(
          `PostgreSQL health check timed out after ${totalTimeout}ms`,
          'HEALTH_CHECK_TIMEOUT'
        )
      }

      this.log(`PostgreSQL health check attempt ${attempt}/${maxAttempts}`)

      try {
        // Use docker exec to run pg_isready inside the container
        const result = await this.spawnCommand(
          'docker',
          ['exec', 'maproom-postgres', 'pg_isready', '-U', 'maproom', '-d', 'maproom'],
          { timeout: 5000 }
        )

        if (result.code === 0) {
          this.log('PostgreSQL is healthy')
          return
        }

        this.log(`PostgreSQL not ready yet (exit code ${result.code}): ${result.stdout}`)
      } catch (error: any) {
        this.log(`Health check attempt ${attempt} failed: ${error.message}`)
      }

      // Calculate exponential backoff delay: 1s, 2s, 4s, 8s, 16s, 16s, ...
      const delay = Math.min(initialDelay * Math.pow(2, attempt - 1), maxDelay)
      const remaining = totalTimeout - elapsed

      if (remaining <= 0) {
        throw new DockerError(
          `PostgreSQL health check timed out after ${totalTimeout}ms`,
          'HEALTH_CHECK_TIMEOUT'
        )
      }

      // Don't wait longer than remaining time
      const actualDelay = Math.min(delay, remaining)
      this.log(`Waiting ${actualDelay}ms before next health check...`)
      await this.sleep(actualDelay)
    }

    throw new DockerError(
      `PostgreSQL health check failed after ${maxAttempts} attempts`,
      'HEALTH_CHECK_FAILED'
    )
  }

  /**
   * Spawn a child process and capture output
   *
   * @param command - Command to execute
   * @param args - Arguments to pass to command
   * @param options - Spawn options
   * @returns Promise resolving to process result
   * @throws DockerError if process fails
   */
  private async spawnCommand(
    command: string,
    args: string[],
    options: { timeout?: number; cwd?: string; env?: Record<string, string> } = {}
  ): Promise<{ code: number; stdout: string; stderr: string }> {
    const { timeout = 0, cwd, env } = options

    return new Promise((resolve, reject) => {
      let child: ChildProcess | undefined
      let timeoutHandle: NodeJS.Timeout | undefined
      let isCleanedUp = false

      const cleanup = () => {
        if (isCleanedUp) return
        isCleanedUp = true

        if (timeoutHandle) {
          clearTimeout(timeoutHandle)
          timeoutHandle = undefined
        }

        if (child) {
          // Remove all event listeners to prevent memory leaks
          child.removeAllListeners()
          child.stdout?.removeAllListeners()
          child.stderr?.removeAllListeners()
        }
      }

      try {
        child = spawn(command, args, {
          cwd,
          env: env ? { ...process.env, ...env } : process.env,
          stdio: ['ignore', 'pipe', 'pipe'],
        })

        let stdout = ''
        let stderr = ''

        // Capture stdout
        if (child.stdout) {
          child.stdout.on('data', (chunk: Buffer) => {
            stdout += chunk.toString('utf8')
          })
        }

        // Capture stderr
        if (child.stderr) {
          child.stderr.on('data', (chunk: Buffer) => {
            stderr += chunk.toString('utf8')
          })
        }

        // Handle process close
        child.on('close', (code: number | null) => {
          cleanup()
          resolve({
            code: code ?? -1,
            stdout: stdout.trim(),
            stderr: stderr.trim(),
          })
        })

        // Handle spawn errors
        child.on('error', (error: NodeJS.ErrnoException) => {
          cleanup()
          reject(
            new DockerError(
              `Failed to spawn ${command}: ${error.message}`,
              error.code || 'SPAWN_ERROR'
            )
          )
        })

        // Set up timeout if specified
        if (timeout > 0) {
          timeoutHandle = setTimeout(() => {
            if (!child || isCleanedUp) return

            this.log(`Command timed out after ${timeout}ms, sending SIGTERM...`)

            // Send SIGTERM first
            child.kill('SIGTERM')

            // Force SIGKILL after 5 seconds if process doesn't exit
            const killTimer = setTimeout(() => {
              if (child && !child.killed) {
                this.log('Process did not respond to SIGTERM, sending SIGKILL...')
                child.kill('SIGKILL')
              }
            }, 5000)

            // Clean up kill timer when process exits
            child.once('close', () => {
              clearTimeout(killTimer)
            })

            cleanup()
            reject(
              new DockerError(`Command timed out after ${timeout}ms`, 'TIMEOUT')
            )
          }, timeout)
        }
      } catch (error: any) {
        cleanup()
        reject(
          new DockerError(
            `Failed to spawn ${command}: ${error.message}`,
            'SPAWN_ERROR'
          )
        )
      }
    })
  }

  /**
   * Get PostgreSQL connection configuration
   *
   * Returns the connection details for the managed PostgreSQL instance.
   * These values match the docker-compose.yml configuration.
   *
   * @returns PostgreSQL connection configuration
   */
  getPostgresConfig(): {
    host: string
    port: number
    user: string
    password: string
    database: string
  } {
    return {
      host: 'localhost',
      port: 5433, // External port mapping from docker-compose.yml
      user: 'maproom',
      password: 'maproom',
      database: 'maproom',
    }
  }

  /**
   * Sleep for the specified duration
   *
   * @param ms - Milliseconds to sleep
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms))
  }
}
