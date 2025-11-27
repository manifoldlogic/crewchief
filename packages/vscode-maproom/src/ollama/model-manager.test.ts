/**
 * Tests for Ollama model manager
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import http from 'node:http'
import {
  ensureOllamaModel,
  checkModelAvailability,
  DEFAULT_EMBEDDING_MODEL,
  OLLAMA_INSTALL_URL,
} from './model-manager'
import { OllamaNotRunningError, ModelPullError, ModelCheckError } from './errors'

// Mock vscode module
vi.mock('vscode', () => ({
  window: {
    withProgress: vi.fn(async (_options, task) => {
      // Execute the task with a mock progress object
      return task({
        report: vi.fn(),
      })
    }),
    showErrorMessage: vi.fn(),
  },
  env: {
    openExternal: vi.fn(),
  },
  Uri: {
    parse: vi.fn((url: string) => ({ toString: () => url })),
  },
  ProgressLocation: {
    Notification: 15,
  },
}))

// Run tests sequentially to avoid port conflicts
describe.sequential('Model Manager', () => {
  let server: http.Server | null = null

  afterEach(async () => {
    if (server) {
      await new Promise<void>((resolve) => {
        server!.close(() => resolve())
      })
      server = null
      // Small delay to ensure port is released
      await new Promise((resolve) => setTimeout(resolve, 100))
    }
    vi.clearAllMocks()
  })

  /**
   * Helper to create a mock HTTP server
   */
  async function createMockServer(
    handler: (req: http.IncomingMessage, res: http.ServerResponse) => void
  ): Promise<http.Server> {
    const srv = http.createServer(handler)
    await new Promise<void>((resolve, reject) => {
      srv.once('error', reject)
      srv.listen(11434, '127.0.0.1', () => {
        srv.removeListener('error', reject)
        resolve()
      })
    })
    server = srv
    return srv
  }

  describe('ensureOllamaModel', () => {
    it('should throw OllamaNotRunningError when Ollama is not running', async () => {
      // No server running
      await expect(ensureOllamaModel()).rejects.toThrow(OllamaNotRunningError)
    })

    it('should return immediately if model already exists', async () => {
      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({
            models: [{ name: 'nomic-embed-text:latest' }],
          }))
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      // Should not throw and not call pull
      await expect(
        ensureOllamaModel('nomic-embed-text', { skipNotification: true })
      ).resolves.toBeUndefined()
    })

    it('should pull model if it does not exist', async () => {
      const progressUpdates: string[] = []

      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({ models: [] })) // No models
        } else if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(200, { 'Content-Type': 'application/x-ndjson' })
          res.write(JSON.stringify({ status: 'pulling manifest' }) + '\n')
          res.write(JSON.stringify({ status: 'downloading', completed: 50, total: 100 }) + '\n')
          res.write(JSON.stringify({ status: 'success' }) + '\n')
          res.end()
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      await ensureOllamaModel('nomic-embed-text', {
        skipNotification: true,
        onProgress: (msg) => progressUpdates.push(msg),
      })

      expect(progressUpdates.length).toBeGreaterThan(0)
      expect(progressUpdates).toContain('pulling manifest')
    })

    it('should throw ModelCheckError if hasModel fails', async () => {
      // The isRunning check uses /api/tags, so we need to make the first call succeed
      // but the second call (hasModel) fail - this is tricky with HTTP
      // Instead, we'll make the response return invalid JSON which will cause a parse error
      let callCount = 0
      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          callCount++
          if (callCount === 1) {
            // First call (isRunning) - return valid response
            res.writeHead(200, { 'Content-Type': 'application/json' })
            res.end(JSON.stringify({ models: [] }))
          } else {
            // Second call (hasModel) - return server error
            res.writeHead(500)
            res.end('Internal Server Error')
          }
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      await expect(
        ensureOllamaModel('nomic-embed-text', { skipNotification: true })
      ).rejects.toThrow(ModelCheckError)
    })

    it('should throw ModelPullError if pull fails', async () => {
      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({ models: [] })) // No models
        } else if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(500)
          res.end('Internal Server Error')
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      await expect(
        ensureOllamaModel('nomic-embed-text', { skipNotification: true })
      ).rejects.toThrow(ModelPullError)
    })

    it('should use default model name when none provided', async () => {
      let requestedUrl = ''

      await createMockServer((req, res) => {
        requestedUrl = req.url || ''
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({
            models: [{ name: `${DEFAULT_EMBEDDING_MODEL}:latest` }],
          }))
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      await ensureOllamaModel(undefined, { skipNotification: true })

      // The model check should succeed because we return the default model
      expect(requestedUrl).toBe('/api/tags')
    })

    it('should report progress with percentage during pull', async () => {
      const progressUpdates: string[] = []

      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({ models: [] }))
        } else if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(200, { 'Content-Type': 'application/x-ndjson' })
          res.write(JSON.stringify({
            status: 'downloading',
            completed: 250000000,
            total: 500000000,
          }) + '\n')
          res.end()
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      await ensureOllamaModel('test-model', {
        skipNotification: true,
        onProgress: (msg) => progressUpdates.push(msg),
      })

      // Should contain percentage
      expect(progressUpdates.some((msg) => msg.includes('50%'))).toBe(true)
    })
  })

  describe('checkModelAvailability', () => {
    it('should return exists: false when Ollama is not running', async () => {
      // No server running
      const result = await checkModelAvailability()

      expect(result.exists).toBe(false)
      expect(result.error).toBe('Ollama is not running')
    })

    it('should return exists: true when model exists', async () => {
      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({
            models: [{ name: 'nomic-embed-text:latest' }],
          }))
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      const result = await checkModelAvailability('nomic-embed-text')

      expect(result.exists).toBe(true)
      expect(result.error).toBeUndefined()
    })

    it('should return exists: false when model does not exist', async () => {
      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({
            models: [{ name: 'other-model:latest' }],
          }))
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      const result = await checkModelAvailability('nomic-embed-text')

      expect(result.exists).toBe(false)
      expect(result.error).toBeUndefined()
    })

    it('should return error on API failure', async () => {
      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(500)
          res.end('Internal Server Error')
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      const result = await checkModelAvailability()

      expect(result.exists).toBe(false)
      expect(result.error).toBeDefined()
    })

    it('should use default model name', async () => {
      await createMockServer((req, res) => {
        if (req.url === '/api/tags') {
          res.writeHead(200, { 'Content-Type': 'application/json' })
          res.end(JSON.stringify({
            models: [{ name: `${DEFAULT_EMBEDDING_MODEL}:latest` }],
          }))
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      const result = await checkModelAvailability()

      expect(result.exists).toBe(true)
    })
  })

  describe('error classes', () => {
    it('OllamaNotRunningError should have correct properties', () => {
      const error = new OllamaNotRunningError()

      expect(error.name).toBe('OllamaNotRunningError')
      expect(error.message).toBe('Ollama is not running')
      expect(error).toBeInstanceOf(Error)
    })

    it('ModelPullError should have correct properties', () => {
      const cause = new Error('Network error')
      const error = new ModelPullError('test-model', cause)

      expect(error.name).toBe('ModelPullError')
      expect(error.message).toContain('test-model')
      expect(error.modelName).toBe('test-model')
      expect(error.cause).toBe(cause)
      expect(error).toBeInstanceOf(Error)
    })

    it('ModelCheckError should have correct properties', () => {
      const cause = new Error('API error')
      const error = new ModelCheckError('test-model', cause)

      expect(error.name).toBe('ModelCheckError')
      expect(error.message).toContain('test-model')
      expect(error.modelName).toBe('test-model')
      expect(error.cause).toBe(cause)
      expect(error).toBeInstanceOf(Error)
    })
  })

  describe('constants', () => {
    it('DEFAULT_EMBEDDING_MODEL should be nomic-embed-text', () => {
      expect(DEFAULT_EMBEDDING_MODEL).toBe('nomic-embed-text')
    })

    it('OLLAMA_INSTALL_URL should be https://ollama.ai', () => {
      expect(OLLAMA_INSTALL_URL).toBe('https://ollama.ai')
    })
  })
})
