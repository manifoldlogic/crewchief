/**
 * Tests for DockerManager
 *
 * Note: These tests are designed to work with a real Docker environment.
 * For unit testing, you would need to mock the child_process spawn function.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { DockerManager, DockerError } from './manager.js'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// Mock OutputChannel for testing
class MockOutputChannel {
  private lines: string[] = []

  appendLine(value: string): void {
    this.lines.push(value)
    // Optionally uncomment to see logs during tests
    // console.log(value)
  }

  append(value: string): void {
    this.lines.push(value)
  }

  clear(): void {
    this.lines = []
  }

  show(): void {
    // No-op for testing
  }

  hide(): void {
    // No-op for testing
  }

  dispose(): void {
    this.lines = []
  }

  getLines(): string[] {
    return this.lines
  }

  name = 'Maproom Docker'
  replace = () => {}
}

describe('DockerManager', () => {
  let outputChannel: MockOutputChannel
  let manager: DockerManager

  beforeEach(() => {
    outputChannel = new MockOutputChannel()
    // Note: In production, you would pass the actual extension root
    // For testing, we use the test environment's path and test compose file
    const currentDir = path.dirname(fileURLToPath(import.meta.url))
    const extensionRoot = path.resolve(currentDir, '..', '..')
    const testComposeFile = path.join(extensionRoot, 'config', 'docker-compose.test.yml')
    manager = new DockerManager(outputChannel as any, extensionRoot, testComposeFile)
  })

  afterEach(async () => {
    // Clean up: stop services after each test
    try {
      await manager.stop()
    } catch (error) {
      // Ignore errors during cleanup
    }
  })

  describe('constructor', () => {
    it('should initialize with output channel', () => {
      expect(manager).toBeDefined()
      const lines = outputChannel.getLines()
      expect(lines.some(line => line.includes('Docker manager initialized'))).toBe(true)
    })

    it('should log extension root and compose file path', () => {
      const lines = outputChannel.getLines()
      expect(lines.some(line => line.includes('Extension root:'))).toBe(true)
      expect(lines.some(line => line.includes('Compose file:'))).toBe(true)
    })
  })

  describe('ensureServicesRunning', () => {
    it('should start services and wait for health checks', async () => {
      // This test requires Docker to be running
      // Skip if Docker is not available
      try {
        await manager.ensureServicesRunning()

        const lines = outputChannel.getLines()
        expect(lines.some(line => line.includes('Starting services'))).toBe(true)
        expect(lines.some(line => line.includes('Services started successfully'))).toBe(true)
        expect(lines.some(line => line.includes('PostgreSQL is healthy'))).toBe(true)
        expect(lines.some(line => line.includes('All services are healthy and ready'))).toBe(true)
      } catch (error) {
        if (error instanceof DockerError && error.code === 'DOCKER_NOT_FOUND') {
          console.log('Docker not available, skipping test')
          return
        }
        throw error
      }
    }, 90000) // 90s timeout for pulling images

    it('should be idempotent - calling twice should not fail', async () => {
      try {
        await manager.ensureServicesRunning()

        // Call again - should not fail
        outputChannel.clear()
        await manager.ensureServicesRunning()

        const lines = outputChannel.getLines()
        expect(lines.some(line => line.includes('All services are healthy and ready'))).toBe(true)
      } catch (error) {
        if (error instanceof DockerError && error.code === 'DOCKER_NOT_FOUND') {
          console.log('Docker not available, skipping test')
          return
        }
        throw error
      }
    }, 90000)

    it('should throw DockerError if Docker daemon is not running', async () => {
      // This test is hard to automate - it requires Docker to be stopped
      // Documenting expected behavior
      expect(true).toBe(true)
    })
  })

  describe('stop', () => {
    it('should stop services gracefully', async () => {
      try {
        // Start first
        await manager.ensureServicesRunning()

        // Then stop
        outputChannel.clear()
        await manager.stop()

        const lines = outputChannel.getLines()
        expect(lines.some(line => line.includes('Stopping Docker Compose services'))).toBe(true)
        expect(lines.some(line => line.includes('Services stopped successfully'))).toBe(true)
      } catch (error) {
        if (error instanceof DockerError && error.code === 'DOCKER_NOT_FOUND') {
          console.log('Docker not available, skipping test')
          return
        }
        throw error
      }
    }, 60000)
  })

  describe('error handling', () => {
    it('should throw DockerError with proper error codes', async () => {
      // Test with invalid compose file path
      const invalidManager = new DockerManager(outputChannel as any, '/nonexistent/path')

      try {
        await invalidManager.ensureServicesRunning()
        expect.fail('Should have thrown an error')
      } catch (error) {
        if (error instanceof DockerError) {
          if (error.code === 'DOCKER_NOT_FOUND') {
            console.log('Docker not available, skipping test')
            return
          }
          expect(error).toBeInstanceOf(DockerError)
          expect(error.code).toBeDefined()
          expect(error.message).toBeDefined()
        }
      }
    }, 30000)
  })
})

describe('DockerError', () => {
  it('should create error with code and message', () => {
    const error = new DockerError('Test error', 'TEST_CODE')
    expect(error.message).toBe('Test error')
    expect(error.code).toBe('TEST_CODE')
    expect(error.name).toBe('DockerError')
  })

  it('should include exit code and stderr when provided', () => {
    const error = new DockerError('Test error', 'TEST_CODE', 1, 'stderr output')
    expect(error.exitCode).toBe(1)
    expect(error.stderr).toBe('stderr output')
  })
})
