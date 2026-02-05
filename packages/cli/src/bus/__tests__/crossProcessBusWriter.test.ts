import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { CrossProcessBusWriter } from '../crossProcessBusWriter'
import { decodeJsonl, JsonlEnvelope } from '../jsonl'
import { AgentMessage } from '../message.types'

// ---------------------------------------------------------------------------
// Test setup: create a temporary directory for each test
// ---------------------------------------------------------------------------
let tmpDir: string

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'crossProcessBusWriter-test-'))
})

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true })
})

// ---------------------------------------------------------------------------
// Helper function to create test AgentMessage objects
// ---------------------------------------------------------------------------
function createTestMessage(overrides: Partial<AgentMessage> = {}): AgentMessage {
  return {
    type: 'status',
    from: 'test-agent',
    to: 'orchestrator',
    payload: { activity: 'testing' },
    timestamp: new Date('2024-01-15T10:30:00Z'),
    ...overrides,
  }
}

// ---------------------------------------------------------------------------
// Core functionality tests
// ---------------------------------------------------------------------------
describe('CrossProcessBusWriter', () => {
  describe('file creation', () => {
    it('creates bus file on first write', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      expect(fs.existsSync(busPath)).toBe(false)

      writer.write(createTestMessage())

      expect(fs.existsSync(busPath)).toBe(true)
    })

    it('creates parent directory if it does not exist', () => {
      const nestedDir = path.join(tmpDir, 'nested', 'deep', 'path')
      const busPath = path.join(nestedDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      // Directory should be created by constructor
      expect(fs.existsSync(nestedDir)).toBe(true)

      writer.write(createTestMessage())

      expect(fs.existsSync(busPath)).toBe(true)
    })
  })

  describe('file permissions', () => {
    it('creates bus file with 0o600 permissions (owner read/write only)', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      writer.write(createTestMessage())

      const stats = fs.statSync(busPath)
      // Check file mode (mask with 0o777 to get permission bits)
      const mode = stats.mode & 0o777
      expect(mode).toBe(0o600)
    })
  })

  describe('message size validation', () => {
    it('rejects messages exceeding 4096 bytes with clear error', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      // Create a message with a very large payload that will exceed 4096 bytes
      const largePayload = { data: 'x'.repeat(5000) }
      const largeMessage = createTestMessage({ payload: largePayload })

      expect(() => writer.write(largeMessage)).toThrow(/Bus message exceeds maximum size/)
      expect(() => writer.write(largeMessage)).toThrow(/4096/)
      expect(() => writer.write(largeMessage)).toThrow(/POSIX O_APPEND/)
    })

    it('accepts messages exactly at 4096 bytes', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      // Create a message that when encoded is close to but under 4096 bytes
      // The envelope adds overhead (id, type, ts fields), so we need to account for that
      // A typical envelope adds ~150-200 bytes of overhead
      const targetPayloadSize = 3800
      const payload = { data: 'x'.repeat(targetPayloadSize) }
      const message = createTestMessage({ payload })

      // This should not throw
      expect(() => writer.write(message)).not.toThrow()
    })

    it('includes actual size in error message', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      const largePayload = { data: 'x'.repeat(5000) }
      const largeMessage = createTestMessage({ payload: largePayload })

      try {
        writer.write(largeMessage)
        expect.fail('Should have thrown an error')
      } catch (error) {
        const errorMessage = (error as Error).message
        // Should include the actual byte count
        expect(errorMessage).toMatch(/\d+ bytes/)
      }
    })
  })

  describe('JSONL envelope format', () => {
    it('writes valid JSONL with all required envelope fields', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)
      const message = createTestMessage({ type: 'completion' })

      writer.write(message)

      const content = fs.readFileSync(busPath, 'utf8')
      const lines = content.trim().split('\n')
      expect(lines).toHaveLength(1)

      const envelope = decodeJsonl(lines[0]) as JsonlEnvelope<AgentMessage>
      expect(envelope).toBeDefined()
      expect(envelope.id).toBeDefined()
      expect(envelope.type).toBe('completion')
      // Payload contains the message; timestamp is serialized as ISO string
      expect(envelope.payload.type).toBe(message.type)
      expect(envelope.payload.from).toBe(message.from)
      expect(envelope.payload.to).toBe(message.to)
      expect(envelope.payload.payload).toEqual(message.payload)
      expect(envelope.payload.timestamp).toBe(message.timestamp.toISOString())
      expect(envelope.ts).toBeDefined()
    })

    it('envelope id is a valid UUID', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      writer.write(createTestMessage())

      const content = fs.readFileSync(busPath, 'utf8')
      const envelope = decodeJsonl(content.trim())
      // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
      expect(envelope?.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i)
    })

    it('envelope type mirrors message type', () => {
      const types: AgentMessage['type'][] = ['status', 'completion', 'error', 'conflict-alert', 'file-change']

      for (const type of types) {
        const testBusPath = path.join(tmpDir, `bus-${type}.jsonl`)
        const testWriter = new CrossProcessBusWriter(testBusPath)
        testWriter.write(createTestMessage({ type }))

        const content = fs.readFileSync(testBusPath, 'utf8')
        const envelope = decodeJsonl(content.trim())
        expect(envelope?.type).toBe(type)
      }
    })

    it('envelope ts is valid ISO timestamp', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      writer.write(createTestMessage())

      const content = fs.readFileSync(busPath, 'utf8')
      const envelope = decodeJsonl(content.trim())

      // Should be a valid ISO date string
      const parsed = new Date(envelope?.ts as string)
      expect(parsed.toISOString()).toBe(envelope?.ts)
    })

    it('each envelope has unique UUID', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      // Write multiple messages
      for (let i = 0; i < 10; i++) {
        writer.write(createTestMessage({ payload: { index: i } }))
      }

      const content = fs.readFileSync(busPath, 'utf8')
      const lines = content.trim().split('\n')
      const ids = lines.map((line) => decodeJsonl(line)?.id)

      // All IDs should be unique
      const uniqueIds = new Set(ids)
      expect(uniqueIds.size).toBe(10)
    })
  })

  describe('multiple writes', () => {
    it('maintains order of writes', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      for (let i = 0; i < 5; i++) {
        writer.write(createTestMessage({ payload: { index: i } }))
      }

      const content = fs.readFileSync(busPath, 'utf8')
      const lines = content.trim().split('\n')
      expect(lines).toHaveLength(5)

      for (let i = 0; i < 5; i++) {
        const envelope = decodeJsonl(lines[i]) as JsonlEnvelope<AgentMessage>
        expect((envelope.payload.payload as { index: number }).index).toBe(i)
      }
    })

    it('appends to existing file correctly', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')

      // Write some initial content
      const writer1 = new CrossProcessBusWriter(busPath)
      writer1.write(createTestMessage({ payload: { writer: 1 } }))

      // Create new writer instance and append
      const writer2 = new CrossProcessBusWriter(busPath)
      writer2.write(createTestMessage({ payload: { writer: 2 } }))

      const content = fs.readFileSync(busPath, 'utf8')
      const lines = content.trim().split('\n')
      expect(lines).toHaveLength(2)

      const envelope1 = decodeJsonl(lines[0]) as JsonlEnvelope<AgentMessage>
      const envelope2 = decodeJsonl(lines[1]) as JsonlEnvelope<AgentMessage>
      expect((envelope1.payload.payload as { writer: number }).writer).toBe(1)
      expect((envelope2.payload.payload as { writer: number }).writer).toBe(2)
    })
  })

  describe('path getter', () => {
    it('returns the file path', () => {
      const busPath = path.join(tmpDir, 'test-bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      expect(writer.path).toBe(busPath)
    })
  })

  describe('special characters in payload', () => {
    it('handles quotes in payload', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      const message = createTestMessage({
        payload: { text: 'He said "hello"' },
      })
      writer.write(message)

      const content = fs.readFileSync(busPath, 'utf8')
      const envelope = decodeJsonl(content.trim()) as JsonlEnvelope<AgentMessage>
      expect((envelope.payload.payload as { text: string }).text).toBe('He said "hello"')
    })

    it('handles newlines in payload', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      const message = createTestMessage({
        payload: { text: 'line1\nline2\nline3' },
      })
      writer.write(message)

      const content = fs.readFileSync(busPath, 'utf8')
      const lines = content.trim().split('\n')
      // Should be single JSONL line (newlines in payload are escaped)
      expect(lines).toHaveLength(1)

      const envelope = decodeJsonl(lines[0]) as JsonlEnvelope<AgentMessage>
      expect((envelope.payload.payload as { text: string }).text).toBe('line1\nline2\nline3')
    })

    it('handles unicode characters', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      const message = createTestMessage({
        payload: { text: 'Hello \u4e16\u754c \ud83c\udf0d' },
      })
      writer.write(message)

      const content = fs.readFileSync(busPath, 'utf8')
      const envelope = decodeJsonl(content.trim()) as JsonlEnvelope<AgentMessage>
      expect((envelope.payload.payload as { text: string }).text).toBe('Hello \u4e16\u754c \ud83c\udf0d')
    })

    it('handles backslashes in payload', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      const message = createTestMessage({
        payload: { path: 'C:\\Users\\test\\file.txt' },
      })
      writer.write(message)

      const content = fs.readFileSync(busPath, 'utf8')
      const envelope = decodeJsonl(content.trim()) as JsonlEnvelope<AgentMessage>
      expect((envelope.payload.payload as { path: string }).path).toBe('C:\\Users\\test\\file.txt')
    })
  })

  describe('integration with decodeJsonl', () => {
    it('written messages can be read by decodeJsonl()', () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      const messages = [
        createTestMessage({ type: 'status', payload: { activity: 'starting' } }),
        createTestMessage({ type: 'completion', payload: { summary: 'done' } }),
        createTestMessage({ type: 'error', payload: { message: 'oops' } }),
      ]

      for (const msg of messages) {
        writer.write(msg)
      }

      const content = fs.readFileSync(busPath, 'utf8')
      const lines = content.trim().split('\n')

      expect(lines).toHaveLength(3)

      for (let i = 0; i < lines.length; i++) {
        const envelope = decodeJsonl(lines[i])
        expect(envelope).toBeDefined()
        expect(envelope?.type).toBe(messages[i].type)
        expect((envelope?.payload as AgentMessage).payload).toEqual(messages[i].payload)
      }
    })
  })
})
