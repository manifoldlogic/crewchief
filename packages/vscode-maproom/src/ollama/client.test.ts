/**
 * Tests for OllamaClient
 *
 * Note: These tests create mock HTTP servers on port 11434.
 * Tests run sequentially to avoid port conflicts.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import http from 'node:http'
import {
  OllamaClient,
  InvalidModelNameError,
  OllamaApiError,
} from './client'

// Run tests sequentially to avoid port conflicts with setupWizard tests
describe.sequential('OllamaClient', () => {
  let client: OllamaClient
  let server: http.Server | null = null

  beforeEach(() => {
    client = new OllamaClient()
  })

  afterEach(async () => {
    if (server) {
      await new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error('Server close timeout'))
        }, 5000)

        server!.close(() => {
          clearTimeout(timeout)
          resolve()
        })
      })
      server = null
      // Small delay to ensure port is released
      await new Promise((resolve) => setTimeout(resolve, 100))
    }
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

  describe('isRunning', () => {
    it('should return true when Ollama is running', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ models: [] }))
      })

      const result = await client.isRunning()
      expect(result).toBe(true)
    })

    it('should return false when Ollama is not running', async () => {
      // No server running
      const result = await client.isRunning()
      expect(result).toBe(false)
    })

    it('should return false on server error response', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(500)
        res.end('Internal Server Error')
      })

      const result = await client.isRunning()
      expect(result).toBe(false)
    })

    it('should complete within timeout period', async () => {
      const startTime = Date.now()
      await client.isRunning()
      const duration = Date.now() - startTime

      // Should complete within 3 seconds (2s timeout + overhead)
      expect(duration).toBeLessThan(3000)
    }, 5000)
  })

  describe('hasModel', () => {
    it('should return true when model exists', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({
          models: [
            { name: 'mxbai-embed-large:latest' },
            { name: 'llama2:latest' },
          ],
        }))
      })

      const result = await client.hasModel('mxbai-embed-large')
      expect(result).toBe(true)
    })

    it('should return true when model exists with :latest suffix', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({
          models: [{ name: 'mxbai-embed-large:latest' }],
        }))
      })

      const result = await client.hasModel('mxbai-embed-large:latest')
      expect(result).toBe(true)
    })

    it('should return true for model without :latest when server has :latest', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({
          models: [{ name: 'mxbai-embed-large:latest' }],
        }))
      })

      // Request without :latest should match model with :latest
      const result = await client.hasModel('mxbai-embed-large')
      expect(result).toBe(true)
    })

    it('should return false when model does not exist', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({
          models: [{ name: 'llama2:latest' }],
        }))
      })

      const result = await client.hasModel('mxbai-embed-large')
      expect(result).toBe(false)
    })

    it('should return false when models array is empty', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ models: [] }))
      })

      const result = await client.hasModel('mxbai-embed-large')
      expect(result).toBe(false)
    })

    it('should return false when models is undefined', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({}))
      })

      const result = await client.hasModel('mxbai-embed-large')
      expect(result).toBe(false)
    })

    it('should throw OllamaApiError on server error', async () => {
      await createMockServer((_req, res) => {
        res.writeHead(500)
        res.end('Internal Server Error')
      })

      await expect(client.hasModel('test')).rejects.toThrow(OllamaApiError)
    })

    it('should throw OllamaApiError when Ollama is not running', async () => {
      // No server running
      await expect(client.hasModel('test')).rejects.toThrow(OllamaApiError)
    })
  })

  describe('pullModel', () => {
    it('should pull model and stream progress', async () => {
      await createMockServer((req, res) => {
        if (req.method === 'POST' && req.url === '/api/pull') {
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

      const progressUpdates: string[] = []
      await client.pullModel('mxbai-embed-large', (progress) => {
        progressUpdates.push(progress.status)
      })

      expect(progressUpdates).toContain('pulling manifest')
      expect(progressUpdates).toContain('downloading')
      expect(progressUpdates).toContain('success')
    })

    it('should handle pull without progress callback', async () => {
      await createMockServer((req, res) => {
        if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(200, { 'Content-Type': 'application/x-ndjson' })
          res.write(JSON.stringify({ status: 'success' }) + '\n')
          res.end()
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      // Should not throw
      await expect(client.pullModel('mxbai-embed-large')).resolves.toBeUndefined()
    })

    it('should throw InvalidModelNameError for invalid model names', async () => {
      // Model names with invalid characters
      await expect(client.pullModel('invalid name')).rejects.toThrow(InvalidModelNameError)
      await expect(client.pullModel('../../etc/passwd')).rejects.toThrow(InvalidModelNameError)
      await expect(client.pullModel('')).rejects.toThrow(InvalidModelNameError)
      await expect(client.pullModel('-invalid')).rejects.toThrow(InvalidModelNameError)
      await expect(client.pullModel('model;rm -rf /')).rejects.toThrow(InvalidModelNameError)
    })

    it('should accept valid model names', async () => {
      await createMockServer((req, res) => {
        if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(200, { 'Content-Type': 'application/x-ndjson' })
          res.write(JSON.stringify({ status: 'success' }) + '\n')
          res.end()
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      // Valid model names
      await expect(client.pullModel('mxbai-embed-large')).resolves.toBeUndefined()
      await expect(client.pullModel('llama2:latest')).resolves.toBeUndefined()
      await expect(client.pullModel('codellama:7b-code')).resolves.toBeUndefined()
      await expect(client.pullModel('model_name.v1')).resolves.toBeUndefined()
    })

    it('should throw OllamaApiError on server error', async () => {
      await createMockServer((req, res) => {
        if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(500)
          res.end('Internal Server Error')
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      await expect(client.pullModel('test-model')).rejects.toThrow(OllamaApiError)
    })

    it('should throw OllamaApiError on network error', async () => {
      // No server running - should throw network error wrapped as OllamaApiError
      await expect(client.pullModel('test-model')).rejects.toThrow(OllamaApiError)
    })

    it('should handle malformed JSON in progress stream gracefully', async () => {
      await createMockServer((req, res) => {
        if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(200, { 'Content-Type': 'application/x-ndjson' })
          res.write(JSON.stringify({ status: 'pulling manifest' }) + '\n')
          res.write('not valid json\n')
          res.write(JSON.stringify({ status: 'success' }) + '\n')
          res.end()
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      const progressUpdates: string[] = []
      await client.pullModel('mxbai-embed-large', (progress) => {
        progressUpdates.push(progress.status)
      })

      // Should have received valid updates despite malformed line
      expect(progressUpdates).toContain('pulling manifest')
      expect(progressUpdates).toContain('success')
    })

    it('should report progress with download percentages', async () => {
      await createMockServer((req, res) => {
        if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(200, { 'Content-Type': 'application/x-ndjson' })
          res.write(JSON.stringify({
            status: 'downloading',
            digest: 'sha256:abc123',
            completed: 500000000,
            total: 1000000000,
          }) + '\n')
          res.end()
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      let capturedProgress: { completed?: number; total?: number } = {}
      await client.pullModel('mxbai-embed-large', (progress) => {
        capturedProgress = progress
      })

      expect(capturedProgress.completed).toBe(500000000)
      expect(capturedProgress.total).toBe(1000000000)
    })

    it('should handle empty response body gracefully', async () => {
      await createMockServer((req, res) => {
        if (req.method === 'POST' && req.url === '/api/pull') {
          res.writeHead(200, { 'Content-Type': 'application/x-ndjson' })
          res.end() // Empty body
        } else {
          res.writeHead(404)
          res.end()
        }
      })

      const progressUpdates: string[] = []
      await client.pullModel('mxbai-embed-large', (progress) => {
        progressUpdates.push(progress.status)
      })

      // Should complete without errors
      expect(progressUpdates).toEqual([])
    })
  })

  describe('isValidModelName', () => {
    it('should validate correct model names', () => {
      expect(OllamaClient.isValidModelName('mxbai-embed-large')).toBe(true)
      expect(OllamaClient.isValidModelName('llama2')).toBe(true)
      expect(OllamaClient.isValidModelName('llama2:latest')).toBe(true)
      expect(OllamaClient.isValidModelName('llama2:7b')).toBe(true)
      expect(OllamaClient.isValidModelName('codellama:7b-code')).toBe(true)
      expect(OllamaClient.isValidModelName('model_name')).toBe(true)
      expect(OllamaClient.isValidModelName('model.name')).toBe(true)
      expect(OllamaClient.isValidModelName('model-name-v1')).toBe(true)
      expect(OllamaClient.isValidModelName('0model')).toBe(true)
    })

    it('should reject invalid model names', () => {
      expect(OllamaClient.isValidModelName('')).toBe(false)
      expect(OllamaClient.isValidModelName('invalid name')).toBe(false)
      expect(OllamaClient.isValidModelName('-invalid')).toBe(false)
      expect(OllamaClient.isValidModelName('_invalid')).toBe(false)
      expect(OllamaClient.isValidModelName('.invalid')).toBe(false)
      expect(OllamaClient.isValidModelName('../../etc/passwd')).toBe(false)
      expect(OllamaClient.isValidModelName('model;rm')).toBe(false)
      expect(OllamaClient.isValidModelName('model|cat')).toBe(false)
      expect(OllamaClient.isValidModelName('model&command')).toBe(false)
      expect(OllamaClient.isValidModelName('model$var')).toBe(false)
    })
  })

  describe('error classes', () => {
    it('InvalidModelNameError should have correct name and message', () => {
      const error = new InvalidModelNameError('bad-model!')
      expect(error.name).toBe('InvalidModelNameError')
      expect(error.message).toContain('bad-model!')
      expect(error).toBeInstanceOf(Error)
    })

    it('OllamaApiError should have correct name, message and statusCode', () => {
      const error = new OllamaApiError('Request failed', 500)
      expect(error.name).toBe('OllamaApiError')
      expect(error.message).toBe('Request failed')
      expect(error.statusCode).toBe(500)
      expect(error).toBeInstanceOf(Error)
    })

    it('OllamaApiError should work without statusCode', () => {
      const error = new OllamaApiError('Network error')
      expect(error.message).toBe('Network error')
      expect(error.statusCode).toBeUndefined()
    })
  })
})
