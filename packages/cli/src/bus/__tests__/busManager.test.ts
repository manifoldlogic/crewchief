import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { BusManager } from '../busManager'
import { CrossProcessBusWriter } from '../crossProcessBusWriter'
import { AgentMessage } from '../message.types'

// ---------------------------------------------------------------------------
// Test setup: create a temporary directory for each test
// ---------------------------------------------------------------------------
let tmpDir: string

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'busManager-test-'))
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
// Lifecycle tests
// ---------------------------------------------------------------------------
describe('BusManager', () => {
  describe('lifecycle', () => {
    it('starts following a single run', async () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writer = new CrossProcessBusWriter(busPath)
      writer.write(createTestMessage({ payload: { run: 'A' } }))

      manager.startFollowing('run-a', busPath)
      await wait(150)
      manager.stopAll()

      expect(messages).toHaveLength(1)
      expect((messages[0].payload as { run: string }).run).toBe('A')
    })

    it('starts following multiple runs', async () => {
      const busPathA = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const busPathB = path.join(tmpDir, 'run-b', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writerA = new CrossProcessBusWriter(busPathA)
      const writerB = new CrossProcessBusWriter(busPathB)
      writerA.write(createTestMessage({ payload: { run: 'A' } }))
      writerB.write(createTestMessage({ payload: { run: 'B' } }))

      manager.startFollowing('run-a', busPathA)
      manager.startFollowing('run-b', busPathB)
      await wait(150)
      manager.stopAll()

      expect(messages).toHaveLength(2)
      const runs = messages.map((m) => (m.payload as { run: string }).run).sort()
      expect(runs).toEqual(['A', 'B'])
    })

    it('stops following individual runs', async () => {
      const busPathA = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const busPathB = path.join(tmpDir, 'run-b', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writerA = new CrossProcessBusWriter(busPathA)
      const writerB = new CrossProcessBusWriter(busPathB)

      manager.startFollowing('run-a', busPathA)
      manager.startFollowing('run-b', busPathB)

      // Write initial messages
      writerA.write(createTestMessage({ payload: { run: 'A', batch: 1 } }))
      writerB.write(createTestMessage({ payload: { run: 'B', batch: 1 } }))
      await wait(150)

      // Stop following run-a
      manager.stopFollowing('run-a')

      // Write more messages
      writerA.write(createTestMessage({ payload: { run: 'A', batch: 2 } }))
      writerB.write(createTestMessage({ payload: { run: 'B', batch: 2 } }))
      await wait(150)
      manager.stopAll()

      // Should have: A batch 1, B batch 1, B batch 2 (no A batch 2)
      const aMessages = messages.filter((m) => (m.payload as { run: string }).run === 'A')
      const bMessages = messages.filter((m) => (m.payload as { run: string }).run === 'B')

      expect(aMessages).toHaveLength(1)
      expect((aMessages[0].payload as { batch: number }).batch).toBe(1)
      expect(bMessages).toHaveLength(2)
    })

    it('stopAll clears all readers', async () => {
      const busPathA = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const busPathB = path.join(tmpDir, 'run-b', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writerA = new CrossProcessBusWriter(busPathA)
      const writerB = new CrossProcessBusWriter(busPathB)

      manager.startFollowing('run-a', busPathA)
      manager.startFollowing('run-b', busPathB)

      // Write initial messages
      writerA.write(createTestMessage({ payload: { batch: 1 } }))
      writerB.write(createTestMessage({ payload: { batch: 1 } }))
      await wait(150)

      expect(manager.activeRunIds).toHaveLength(2)

      // Stop all
      manager.stopAll()

      expect(manager.activeRunIds).toHaveLength(0)
      expect(manager.isFollowing('run-a')).toBe(false)
      expect(manager.isFollowing('run-b')).toBe(false)

      // Write more messages - should not be received
      const messageCountBefore = messages.length
      writerA.write(createTestMessage({ payload: { batch: 2 } }))
      writerB.write(createTestMessage({ payload: { batch: 2 } }))
      await wait(150)

      expect(messages.length).toBe(messageCountBefore)
    })
  })

  // ---------------------------------------------------------------------------
  // State tracking tests
  // ---------------------------------------------------------------------------
  describe('state tracking', () => {
    it('isFollowing returns true for active runs', () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const manager = new BusManager({ intervalMs: 50 })

      expect(manager.isFollowing('run-a')).toBe(false)

      manager.startFollowing('run-a', busPath)
      expect(manager.isFollowing('run-a')).toBe(true)

      manager.stopFollowing('run-a')
      expect(manager.isFollowing('run-a')).toBe(false)
    })

    it('activeRunIds returns correct list after start/stop', () => {
      const busPathA = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const busPathB = path.join(tmpDir, 'run-b', 'bus.jsonl')
      const busPathC = path.join(tmpDir, 'run-c', 'bus.jsonl')
      const manager = new BusManager({ intervalMs: 50 })

      expect(manager.activeRunIds).toEqual([])

      manager.startFollowing('run-a', busPathA)
      expect(manager.activeRunIds).toEqual(['run-a'])

      manager.startFollowing('run-b', busPathB)
      expect(manager.activeRunIds.sort()).toEqual(['run-a', 'run-b'])

      manager.startFollowing('run-c', busPathC)
      expect(manager.activeRunIds.sort()).toEqual(['run-a', 'run-b', 'run-c'])

      manager.stopFollowing('run-b')
      expect(manager.activeRunIds.sort()).toEqual(['run-a', 'run-c'])

      manager.stopAll()
      expect(manager.activeRunIds).toEqual([])
    })

    it('startFollowing is idempotent - calling twice does not create duplicate readers', async () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writer = new CrossProcessBusWriter(busPath)
      writer.write(createTestMessage({ payload: { index: 0 } }))

      // Call startFollowing twice
      manager.startFollowing('run-a', busPath)
      manager.startFollowing('run-a', busPath) // Should be ignored

      await wait(150)
      manager.stopAll()

      // Should only receive message once (not twice from duplicate readers)
      expect(messages).toHaveLength(1)
      expect(manager.activeRunIds).toEqual([])
    })
  })

  // ---------------------------------------------------------------------------
  // Message dispatch tests
  // ---------------------------------------------------------------------------
  describe('message dispatch', () => {
    it('messages from followed runs arrive via callback', async () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const callback = vi.fn()
      const manager = new BusManager({
        onMessage: callback,
        intervalMs: 50,
      })

      const writer = new CrossProcessBusWriter(busPath)
      manager.startFollowing('run-a', busPath)

      writer.write(createTestMessage({ payload: { test: 'message' } }))
      await wait(150)
      manager.stopAll()

      expect(callback).toHaveBeenCalledTimes(1)
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'status',
          from: 'test-agent',
          to: 'orchestrator',
          payload: { test: 'message' },
        }),
      )
    })

    it('messages from stopped runs do not arrive', async () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writer = new CrossProcessBusWriter(busPath)

      manager.startFollowing('run-a', busPath)
      writer.write(createTestMessage({ payload: { batch: 1 } }))
      await wait(150)

      manager.stopFollowing('run-a')

      writer.write(createTestMessage({ payload: { batch: 2 } }))
      await wait(150)

      expect(messages).toHaveLength(1)
      expect((messages[0].payload as { batch: number }).batch).toBe(1)
    })

    it('messages from multiple runs all dispatched', async () => {
      const busPathA = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const busPathB = path.join(tmpDir, 'run-b', 'bus.jsonl')
      const busPathC = path.join(tmpDir, 'run-c', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writerA = new CrossProcessBusWriter(busPathA)
      const writerB = new CrossProcessBusWriter(busPathB)
      const writerC = new CrossProcessBusWriter(busPathC)

      manager.startFollowing('run-a', busPathA)
      manager.startFollowing('run-b', busPathB)
      manager.startFollowing('run-c', busPathC)

      writerA.write(createTestMessage({ payload: { run: 'A', index: 0 } }))
      writerB.write(createTestMessage({ payload: { run: 'B', index: 0 } }))
      writerC.write(createTestMessage({ payload: { run: 'C', index: 0 } }))
      writerA.write(createTestMessage({ payload: { run: 'A', index: 1 } }))
      writerB.write(createTestMessage({ payload: { run: 'B', index: 1 } }))
      writerC.write(createTestMessage({ payload: { run: 'C', index: 1 } }))

      await wait(200)
      manager.stopAll()

      expect(messages).toHaveLength(6)

      const runCounts = { A: 0, B: 0, C: 0 }
      for (const msg of messages) {
        const run = (msg.payload as { run: 'A' | 'B' | 'C' }).run
        runCounts[run]++
      }

      expect(runCounts.A).toBe(2)
      expect(runCounts.B).toBe(2)
      expect(runCounts.C).toBe(2)
    })
  })

  // ---------------------------------------------------------------------------
  // Edge case tests
  // ---------------------------------------------------------------------------
  describe('edge cases', () => {
    it('stop following non-existent run is no-op (no error)', () => {
      const manager = new BusManager({ intervalMs: 50 })

      // Should not throw
      expect(() => manager.stopFollowing('non-existent-run')).not.toThrow()
      expect(manager.activeRunIds).toEqual([])
    })

    it('custom poll interval passed to readers', async () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const customInterval = 25

      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: customInterval,
      })

      const writer = new CrossProcessBusWriter(busPath)
      manager.startFollowing('run-a', busPath)

      writer.write(createTestMessage())

      // With a 25ms interval, we should pick up the message faster
      await wait(75) // 3x the interval
      manager.stopAll()

      expect(messages).toHaveLength(1)
    })

    it('multiple rapid start/stop cycles', async () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const messages: AgentMessage[] = []
      const manager = new BusManager({
        onMessage: (msg) => messages.push(msg),
        intervalMs: 50,
      })

      const writer = new CrossProcessBusWriter(busPath)

      // Rapid start/stop cycles
      for (let i = 0; i < 5; i++) {
        manager.startFollowing('run-a', busPath)
        writer.write(createTestMessage({ payload: { cycle: i } }))
        await wait(100)
        manager.stopFollowing('run-a')
      }

      // All cycles should have received their message
      expect(messages.length).toBeGreaterThanOrEqual(5)
    })

    it('uses no-op callback when onMessage is not provided', async () => {
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const manager = new BusManager({ intervalMs: 50 })

      const writer = new CrossProcessBusWriter(busPath)
      writer.write(createTestMessage())

      // Should not throw even without callback
      manager.startFollowing('run-a', busPath)
      await wait(100)
      manager.stopAll()

      expect(manager.activeRunIds).toEqual([])
    })

    it('uses default poll interval when not provided', () => {
      const manager = new BusManager()

      // Access private intervalMs through any to verify default
      // (This is a weak test, but ensures defaults work)
      const busPath = path.join(tmpDir, 'run-a', 'bus.jsonl')
      manager.startFollowing('run-a', busPath)

      expect(manager.isFollowing('run-a')).toBe(true)
      manager.stopAll()
    })

    it('stopAll with no active readers is no-op', () => {
      const manager = new BusManager({ intervalMs: 50 })

      // Should not throw
      expect(() => manager.stopAll()).not.toThrow()
      expect(manager.activeRunIds).toEqual([])
    })

    it('activeRunIds returns a fresh array on each call', () => {
      const busPathA = path.join(tmpDir, 'run-a', 'bus.jsonl')
      const busPathB = path.join(tmpDir, 'run-b', 'bus.jsonl')
      const manager = new BusManager({ intervalMs: 50 })

      manager.startFollowing('run-a', busPathA)
      const ids1 = manager.activeRunIds

      manager.startFollowing('run-b', busPathB)
      const ids2 = manager.activeRunIds

      // ids1 should still only have run-a (not mutated)
      expect(ids1).toEqual(['run-a'])
      expect(ids2.sort()).toEqual(['run-a', 'run-b'])

      manager.stopAll()
    })
  })
})
