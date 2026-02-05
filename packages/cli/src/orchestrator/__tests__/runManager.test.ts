import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { RunManager } from '../runManager'

// ---------------------------------------------------------------------------
// Test setup: create a temporary directory for each test
// ---------------------------------------------------------------------------
let tmpDir: string
let runManager: RunManager

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'runManager-test-'))
  runManager = new RunManager(tmpDir)
})

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true })
})

// ---------------------------------------------------------------------------
// getRunBusPath
// ---------------------------------------------------------------------------
describe('getRunBusPath', () => {
  describe('happy path', () => {
    it('returns correct path format: <baseDir>/<runId>/bus.jsonl', () => {
      const runId = '550e8400-e29b-41d4-a716-446655440000'
      const result = runManager.getRunBusPath(runId)
      expect(result).toBe(path.join(tmpDir, runId, 'bus.jsonl'))
    })

    it('path is consistent with getRunDir() output', () => {
      const runId = 'a1b2c3d4-e5f6-7890-abcd-ef1234567890'
      const busPath = runManager.getRunBusPath(runId)
      const runDir = runManager.getRunDir(runId)
      expect(busPath).toBe(path.join(runDir, 'bus.jsonl'))
    })

    it('multiple calls with same runId return identical paths', () => {
      const runId = '00000000-0000-0000-0000-000000000001'
      const path1 = runManager.getRunBusPath(runId)
      const path2 = runManager.getRunBusPath(runId)
      const path3 = runManager.getRunBusPath(runId)
      expect(path1).toBe(path2)
      expect(path2).toBe(path3)
    })

    it('returns absolute path', () => {
      const runId = '11111111-2222-3333-4444-555555555555'
      const result = runManager.getRunBusPath(runId)
      expect(path.isAbsolute(result)).toBe(true)
    })

    it('different runIds return different paths', () => {
      const runId1 = '22222222-3333-4444-5555-666666666666'
      const runId2 = '33333333-4444-5555-6666-777777777777'
      const path1 = runManager.getRunBusPath(runId1)
      const path2 = runManager.getRunBusPath(runId2)
      expect(path1).not.toBe(path2)
    })
  })

  describe('edge cases', () => {
    it('run ID that does not correspond to persisted run still returns a path', () => {
      const nonExistentRunId = 'ffffffff-ffff-ffff-ffff-ffffffffffff'
      const result = runManager.getRunBusPath(nonExistentRunId)
      expect(result).toBe(path.join(tmpDir, nonExistentRunId, 'bus.jsonl'))
    })

    it('empty string runId returns path (caller validation)', () => {
      const result = runManager.getRunBusPath('')
      // Empty runId still constructs a path - validation is caller's responsibility
      expect(result).toBe(path.join(tmpDir, '', 'bus.jsonl'))
    })

    it('runId with special characters handled by getRunDir (no additional escaping)', () => {
      // This tests that getRunBusPath delegates to getRunDir without modification
      const specialRunId = 'test-run-with-dashes'
      const busPath = runManager.getRunBusPath(specialRunId)
      const runDir = runManager.getRunDir(specialRunId)
      expect(busPath).toBe(path.join(runDir, 'bus.jsonl'))
    })
  })

  describe('negative tests', () => {
    it('does not create bus.jsonl file', () => {
      const runId = '44444444-5555-6666-7777-888888888888'
      const busPath = runManager.getRunBusPath(runId)
      expect(fs.existsSync(busPath)).toBe(false)
    })

    it('does not create run directory', () => {
      const runId = '55555555-6666-7777-8888-999999999999'
      const runDir = runManager.getRunDir(runId)
      // Ensure directory doesn't exist before calling getRunBusPath
      expect(fs.existsSync(runDir)).toBe(false)
      // Call getRunBusPath
      runManager.getRunBusPath(runId)
      // Directory should still not exist
      expect(fs.existsSync(runDir)).toBe(false)
    })

    it('method is pure path computation (no filesystem operations)', () => {
      const runId = '66666666-7777-8888-9999-aaaaaaaaaaaa'
      // Calling getRunBusPath multiple times should not create any files or directories
      for (let i = 0; i < 10; i++) {
        runManager.getRunBusPath(runId)
      }
      const runDir = runManager.getRunDir(runId)
      const busPath = runManager.getRunBusPath(runId)
      expect(fs.existsSync(runDir)).toBe(false)
      expect(fs.existsSync(busPath)).toBe(false)
    })
  })

  describe('integration with createRun', () => {
    it('getRunBusPath works with run created via createRun', () => {
      const run = runManager.createRun('test-agent', 'test task', 'pane-1', '/path/to/worktree')
      const busPath = runManager.getRunBusPath(run.id)
      const runDir = runManager.getRunDir(run.id)
      // createRun creates the run directory
      expect(fs.existsSync(runDir)).toBe(true)
      // but getRunBusPath does not create the bus file
      expect(fs.existsSync(busPath)).toBe(false)
      // path format is correct
      expect(busPath).toBe(path.join(runDir, 'bus.jsonl'))
    })
  })
})
