import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { CrossProcessBusReader } from '../crossProcessBusReader'
import { CrossProcessBusWriter } from '../crossProcessBusWriter'
import { decodeJsonl } from '../jsonl'
import { AgentMessage } from '../message.types'

// ---------------------------------------------------------------------------
// Test setup: create a temporary directory for each test
// ---------------------------------------------------------------------------
let tmpDir: string

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'crossProcessBus-test-'))
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
// Helper function to wait for a specified duration
// ---------------------------------------------------------------------------
function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

// ---------------------------------------------------------------------------
// CrossProcessBusReader tests
// ---------------------------------------------------------------------------
describe('CrossProcessBusReader', () => {
  describe('start/stop lifecycle', () => {
    it('starts and stops without error', async () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const reader = new CrossProcessBusReader({
        filePath: busPath,
        intervalMs: 50,
      })

      reader.start()
      await wait(100)
      reader.stop()
    })

    it('can be started and stopped multiple times', async () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const reader = new CrossProcessBusReader({
        filePath: busPath,
        intervalMs: 50,
      })

      reader.start()
      await wait(50)
      reader.stop()

      reader.start()
      await wait(50)
      reader.stop()
    })
  })

  describe('callback invocation', () => {
    it('invokes onMessage callback for each valid message', async () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)

      const messages: AgentMessage[] = []
      const reader = new CrossProcessBusReader({
        filePath: busPath,
        intervalMs: 50,
        onMessage: (msg) => messages.push(msg),
      })

      writer.write(createTestMessage({ payload: { index: 0 } }))
      writer.write(createTestMessage({ payload: { index: 1 } }))

      reader.start()
      await wait(150)
      reader.stop()

      expect(messages).toHaveLength(2)
      expect((messages[0].payload as { index: number }).index).toBe(0)
      expect((messages[1].payload as { index: number }).index).toBe(1)
    })

    it('uses no-op callback when onMessage is not provided', async () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')
      const writer = new CrossProcessBusWriter(busPath)
      writer.write(createTestMessage())

      const reader = new CrossProcessBusReader({
        filePath: busPath,
        intervalMs: 50,
      })

      // Should not throw
      reader.start()
      await wait(100)
      reader.stop()
    })
  })

  describe('message validation', () => {
    it('skips messages without type field', async () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')

      // Write a valid message
      const writer = new CrossProcessBusWriter(busPath)
      writer.write(createTestMessage({ payload: { valid: true } }))

      // Manually write an envelope with a payload missing the type field
      const invalidEnvelope = JSON.stringify({
        id: 'test-id',
        type: 'status',
        payload: { notAValidMessage: true }, // Missing 'type' field in payload
        ts: new Date().toISOString(),
      })
      fs.appendFileSync(busPath, invalidEnvelope + '\n')

      // Write another valid message
      writer.write(createTestMessage({ payload: { valid: 'second' } }))

      const messages: AgentMessage[] = []
      const reader = new CrossProcessBusReader({
        filePath: busPath,
        intervalMs: 50,
        onMessage: (msg) => messages.push(msg),
      })

      reader.start()
      await wait(150)
      reader.stop()

      // Should only have the two valid messages
      expect(messages).toHaveLength(2)
      expect((messages[0].payload as { valid: boolean }).valid).toBe(true)
      expect((messages[1].payload as { valid: string }).valid).toBe('second')
    })

    it('skips messages where type is not a string', async () => {
      const busPath = path.join(tmpDir, 'bus.jsonl')

      // Write a valid message first
      const writer = new CrossProcessBusWriter(busPath)
      writer.write(createTestMessage())

      // Manually write an envelope with non-string type in payload
      const invalidEnvelope = JSON.stringify({
        id: 'test-id',
        type: 'status',
        payload: { type: 123, from: 'test', to: 'test' }, // type is number, not string
        ts: new Date().toISOString(),
      })
      fs.appendFileSync(busPath, invalidEnvelope + '\n')

      const messages: AgentMessage[] = []
      const reader = new CrossProcessBusReader({
        filePath: busPath,
        intervalMs: 50,
        onMessage: (msg) => messages.push(msg),
      })

      reader.start()
      await wait(150)
      reader.stop()

      expect(messages).toHaveLength(1)
    })
  })
})

// ---------------------------------------------------------------------------
// Roundtrip integration tests
// ---------------------------------------------------------------------------
describe('CrossProcessBus roundtrip', () => {
  it('writer writes message, reader reads within one poll interval', async () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    const writer = new CrossProcessBusWriter(busPath)

    const messages: AgentMessage[] = []
    const reader = new CrossProcessBusReader({
      filePath: busPath,
      intervalMs: 50,
      onMessage: (msg) => messages.push(msg),
    })

    reader.start()

    // Write after reader started
    writer.write(createTestMessage({ payload: { test: 'roundtrip' } }))

    // Wait for at least one poll interval plus some buffer
    await wait(150)
    reader.stop()

    expect(messages).toHaveLength(1)
    expect((messages[0].payload as { test: string }).test).toBe('roundtrip')
  })

  it('multiple messages maintain order', async () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    const writer = new CrossProcessBusWriter(busPath)

    const messages: AgentMessage[] = []
    const reader = new CrossProcessBusReader({
      filePath: busPath,
      intervalMs: 50,
      onMessage: (msg) => messages.push(msg),
    })

    reader.start()

    for (let i = 0; i < 10; i++) {
      writer.write(createTestMessage({ payload: { index: i } }))
    }

    await wait(200)
    reader.stop()

    expect(messages).toHaveLength(10)
    for (let i = 0; i < 10; i++) {
      expect((messages[i].payload as { index: number }).index).toBe(i)
    }
  })

  it('malformed JSON line does not break subsequent reads', async () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    const writer = new CrossProcessBusWriter(busPath)

    // Write valid message
    writer.write(createTestMessage({ payload: { before: true } }))

    // Write malformed JSON directly to file
    fs.appendFileSync(busPath, 'not valid json at all\n')
    fs.appendFileSync(busPath, '{ incomplete json\n')

    // Write another valid message
    writer.write(createTestMessage({ payload: { after: true } }))

    const messages: AgentMessage[] = []
    const reader = new CrossProcessBusReader({
      filePath: busPath,
      intervalMs: 50,
      onMessage: (msg) => messages.push(msg),
    })

    reader.start()
    await wait(150)
    reader.stop()

    // Should receive both valid messages, skipping malformed lines
    expect(messages).toHaveLength(2)
    expect((messages[0].payload as { before: boolean }).before).toBe(true)
    expect((messages[1].payload as { after: boolean }).after).toBe(true)
  })

  it('reader handles file-not-yet-existing gracefully', async () => {
    const busPath = path.join(tmpDir, 'does-not-exist-yet.jsonl')

    const messages: AgentMessage[] = []
    const reader = new CrossProcessBusReader({
      filePath: busPath,
      intervalMs: 50,
      onMessage: (msg) => messages.push(msg),
    })

    // Start reader before file exists
    reader.start()
    await wait(100)

    // Now create the file and write to it
    const writer = new CrossProcessBusWriter(busPath)
    writer.write(createTestMessage({ payload: { created: 'later' } }))

    await wait(150)
    reader.stop()

    expect(messages).toHaveLength(1)
    expect((messages[0].payload as { created: string }).created).toBe('later')
  })

  it('reader offset tracking: stop/restart does not re-read old messages', async () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    const writer = new CrossProcessBusWriter(busPath)

    // Write initial messages
    writer.write(createTestMessage({ payload: { batch: 1, index: 0 } }))
    writer.write(createTestMessage({ payload: { batch: 1, index: 1 } }))

    const messages: AgentMessage[] = []
    const reader = new CrossProcessBusReader({
      filePath: busPath,
      intervalMs: 50,
      onMessage: (msg) => messages.push(msg),
    })

    // First read session
    reader.start()
    await wait(150)
    reader.stop()

    expect(messages).toHaveLength(2)

    // Write more messages while stopped
    writer.write(createTestMessage({ payload: { batch: 2, index: 0 } }))

    // Restart reader
    reader.start()
    await wait(150)
    reader.stop()

    // Should have read the new message (3 total)
    expect(messages).toHaveLength(3)
    expect((messages[2].payload as { batch: number }).batch).toBe(2)
  })

  it('preserves all message fields through roundtrip', async () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    const writer = new CrossProcessBusWriter(busPath)

    const originalMessage = createTestMessage({
      type: 'completion',
      from: 'worker-1',
      to: 'orchestrator',
      payload: { summary: 'Task completed', exitCode: 0 },
      runId: 'run-123',
      worktreeContext: {
        branch: 'feature/test',
        modifiedFiles: ['file1.ts', 'file2.ts'],
        lastCommit: 'abc123',
      },
    })

    writer.write(originalMessage)

    const messages: AgentMessage[] = []
    const reader = new CrossProcessBusReader({
      filePath: busPath,
      intervalMs: 50,
      onMessage: (msg) => messages.push(msg),
    })

    reader.start()
    await wait(150)
    reader.stop()

    expect(messages).toHaveLength(1)
    const received = messages[0]

    expect(received.type).toBe('completion')
    expect(received.from).toBe('worker-1')
    expect(received.to).toBe('orchestrator')
    expect(received.payload).toEqual({ summary: 'Task completed', exitCode: 0 })
    expect(received.runId).toBe('run-123')
    expect(received.worktreeContext).toEqual({
      branch: 'feature/test',
      modifiedFiles: ['file1.ts', 'file2.ts'],
      lastCommit: 'abc123',
    })
  })
})

// ---------------------------------------------------------------------------
// CRITICAL: Concurrent multi-writer safety test
// ---------------------------------------------------------------------------
describe('CrossProcessBus concurrent multi-writer safety', () => {
  it('10 async tasks each writing 10 messages = 100 total valid JSONL lines', async () => {
    const busPath = path.join(tmpDir, 'concurrent-bus.jsonl')

    const writerCount = 10
    const messagesPerWriter = 10
    const totalExpected = writerCount * messagesPerWriter

    // Create 10 async tasks, each creating its own writer and writing 10 messages
    const tasks = Array.from({ length: writerCount }, async (_, writerIndex) => {
      const writer = new CrossProcessBusWriter(busPath)
      for (let msgIndex = 0; msgIndex < messagesPerWriter; msgIndex++) {
        writer.write(
          createTestMessage({
            from: `writer-${writerIndex}`,
            payload: { writerIndex, msgIndex },
          }),
        )
      }
    })

    // Execute all tasks concurrently
    await Promise.all(tasks)

    // Read the file and verify all 100 messages are valid JSONL
    const content = fs.readFileSync(busPath, 'utf8')
    const lines = content.split('\n').filter((line) => line.trim().length > 0)

    // Verify we have exactly 100 lines
    expect(lines).toHaveLength(totalExpected)

    // Verify all lines are valid JSON
    const parseErrors: string[] = []
    const validEnvelopes: unknown[] = []

    for (let i = 0; i < lines.length; i++) {
      const envelope = decodeJsonl(lines[i])
      if (!envelope) {
        parseErrors.push(`Line ${i + 1}: ${lines[i].substring(0, 100)}...`)
      } else {
        validEnvelopes.push(envelope)
      }
    }

    // Report any parse errors for debugging
    if (parseErrors.length > 0) {
      console.error('Parse errors found:', parseErrors)
    }

    expect(parseErrors).toHaveLength(0)
    expect(validEnvelopes).toHaveLength(totalExpected)
  })

  it('no truncated lines (every line ends with complete JSON + newline)', async () => {
    const busPath = path.join(tmpDir, 'truncation-test.jsonl')

    const writerCount = 10
    const messagesPerWriter = 10

    // Execute concurrent writes
    const tasks = Array.from({ length: writerCount }, async (_, writerIndex) => {
      const writer = new CrossProcessBusWriter(busPath)
      for (let msgIndex = 0; msgIndex < messagesPerWriter; msgIndex++) {
        writer.write(
          createTestMessage({
            from: `writer-${writerIndex}`,
            payload: { writerIndex, msgIndex, data: 'x'.repeat(100) },
          }),
        )
      }
    })

    await Promise.all(tasks)

    const content = fs.readFileSync(busPath, 'utf8')

    // File should end with a newline
    expect(content.endsWith('\n')).toBe(true)

    // Each non-empty line should be valid JSON
    const lines = content.split('\n').filter((line) => line.trim().length > 0)

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i]

      // Line should not be truncated (should start with { and end with })
      expect(line.startsWith('{')).toBe(true)
      expect(line.endsWith('}')).toBe(true)

      // Line should be parseable
      expect(() => JSON.parse(line)).not.toThrow()
    }
  })

  it('no interleaved JSON (each line is single complete envelope)', async () => {
    const busPath = path.join(tmpDir, 'interleave-test.jsonl')

    const writerCount = 10
    const messagesPerWriter = 10
    const totalExpected = writerCount * messagesPerWriter

    // Execute concurrent writes
    const tasks = Array.from({ length: writerCount }, async (_, writerIndex) => {
      const writer = new CrossProcessBusWriter(busPath)
      for (let msgIndex = 0; msgIndex < messagesPerWriter; msgIndex++) {
        writer.write(
          createTestMessage({
            from: `writer-${writerIndex}`,
            payload: { writerIndex, msgIndex },
          }),
        )
      }
    })

    await Promise.all(tasks)

    const content = fs.readFileSync(busPath, 'utf8')
    const lines = content.split('\n').filter((line) => line.trim().length > 0)

    // Track which writer-message combinations we've seen
    const seen = new Set<string>()
    const duplicates: string[] = []

    for (const line of lines) {
      const parsed = JSON.parse(line)

      // Verify it has all expected envelope fields
      expect(parsed).toHaveProperty('id')
      expect(parsed).toHaveProperty('type')
      expect(parsed).toHaveProperty('payload')
      expect(parsed).toHaveProperty('ts')

      // Verify the payload has expected structure
      const payload = parsed.payload
      expect(payload).toHaveProperty('from')
      expect(payload).toHaveProperty('payload')

      // Track unique messages
      const { writerIndex, msgIndex } = payload.payload as { writerIndex: number; msgIndex: number }
      const key = `${writerIndex}-${msgIndex}`

      if (seen.has(key)) {
        duplicates.push(key)
      }
      seen.add(key)
    }

    // No duplicates should exist
    expect(duplicates).toHaveLength(0)

    // All expected messages should be present
    expect(seen.size).toBe(totalExpected)
  })

  it('concurrent reads and writes work correctly', async () => {
    const busPath = path.join(tmpDir, 'read-write-concurrent.jsonl')

    const writerCount = 5
    const messagesPerWriter = 10
    const totalExpected = writerCount * messagesPerWriter

    const receivedMessages: AgentMessage[] = []
    const reader = new CrossProcessBusReader({
      filePath: busPath,
      intervalMs: 20,
      onMessage: (msg) => receivedMessages.push(msg),
    })

    // Start reader
    reader.start()

    // Execute concurrent writes while reader is running
    const tasks = Array.from({ length: writerCount }, async (_, writerIndex) => {
      const writer = new CrossProcessBusWriter(busPath)
      for (let msgIndex = 0; msgIndex < messagesPerWriter; msgIndex++) {
        writer.write(
          createTestMessage({
            from: `writer-${writerIndex}`,
            payload: { writerIndex, msgIndex },
          }),
        )
        // Small delay between writes to spread them out
        await wait(5)
      }
    })

    await Promise.all(tasks)

    // Wait for reader to catch up
    await wait(200)
    reader.stop()

    // Should have received all messages
    expect(receivedMessages).toHaveLength(totalExpected)

    // Verify no duplicates in received messages
    const seen = new Set<string>()
    for (const msg of receivedMessages) {
      const { writerIndex, msgIndex } = msg.payload as { writerIndex: number; msgIndex: number }
      const key = `${writerIndex}-${msgIndex}`
      expect(seen.has(key)).toBe(false)
      seen.add(key)
    }
  })

  it('stress test with rapid concurrent writes', async () => {
    const busPath = path.join(tmpDir, 'stress-test.jsonl')

    const writerCount = 20
    const messagesPerWriter = 50
    const totalExpected = writerCount * messagesPerWriter

    // Execute many concurrent writes as fast as possible
    const tasks = Array.from({ length: writerCount }, async (_, writerIndex) => {
      const writer = new CrossProcessBusWriter(busPath)
      for (let msgIndex = 0; msgIndex < messagesPerWriter; msgIndex++) {
        writer.write(
          createTestMessage({
            from: `writer-${writerIndex}`,
            payload: { writerIndex, msgIndex },
          }),
        )
      }
    })

    await Promise.all(tasks)

    // Verify all messages are valid
    const content = fs.readFileSync(busPath, 'utf8')
    const lines = content.split('\n').filter((line) => line.trim().length > 0)

    expect(lines).toHaveLength(totalExpected)

    let validCount = 0
    for (const line of lines) {
      const envelope = decodeJsonl(line)
      if (envelope && envelope.payload && typeof (envelope.payload as AgentMessage).type === 'string') {
        validCount++
      }
    }

    expect(validCount).toBe(totalExpected)
  })
})
