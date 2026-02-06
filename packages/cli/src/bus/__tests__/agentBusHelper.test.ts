import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { createBusWriter, reportCompletion, reportError, reportFileChanges, reportStatus } from '../agentBusHelper'
import { decodeJsonl, JsonlEnvelope } from '../jsonl'
import type { AgentMessage, ErrorPayload, FileChangePayload, StatusPayload, CompletionPayload } from '../message.types'

let tmpDir: string
let originalBusPath: string | undefined

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agentBusHelper-test-'))
  originalBusPath = process.env.CREWCHIEF_BUS_PATH
})

afterEach(() => {
  if (originalBusPath === undefined) {
    delete process.env.CREWCHIEF_BUS_PATH
  } else {
    process.env.CREWCHIEF_BUS_PATH = originalBusPath
  }
  fs.rmSync(tmpDir, { recursive: true, force: true })
})

function readBusMessages(busPath: string): JsonlEnvelope<AgentMessage>[] {
  const content = fs.readFileSync(busPath, 'utf8')
  return content
    .trim()
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => decodeJsonl(line) as JsonlEnvelope<AgentMessage>)
}

describe('createBusWriter', () => {
  it('returns undefined when no env var', () => {
    delete process.env.CREWCHIEF_BUS_PATH
    const writer = createBusWriter()
    expect(writer).toBeUndefined()
  })

  it('returns writer when env var set', () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    process.env.CREWCHIEF_BUS_PATH = busPath
    const writer = createBusWriter()
    expect(writer).toBeDefined()
    expect(writer?.path).toBe(busPath)
  })

  it('returns undefined when env var is empty string', () => {
    process.env.CREWCHIEF_BUS_PATH = ''
    const writer = createBusWriter()
    expect(writer).toBeUndefined()
  })
})

describe('reportStatus', () => {
  it('writes correct message structure', () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    process.env.CREWCHIEF_BUS_PATH = busPath
    reportStatus('analyzing code', 'test-agent')
    const messages = readBusMessages(busPath)
    expect(messages).toHaveLength(1)
    const envelope = messages[0]
    expect(envelope.type).toBe('status')
    expect(envelope.payload.type).toBe('status')
    expect(envelope.payload.from).toBe('test-agent')
    expect(envelope.payload.to).toBe('orchestrator')
    const payload = envelope.payload.payload as StatusPayload
    expect(payload.activity).toBe('analyzing code')
  })

  it('is no-op when env var is not set', () => {
    delete process.env.CREWCHIEF_BUS_PATH
    expect(() => reportStatus('testing', 'agent')).not.toThrow()
  })
})

describe('reportCompletion', () => {
  it('writes correct message structure', () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    process.env.CREWCHIEF_BUS_PATH = busPath
    reportCompletion('Task completed successfully', 'test-agent')
    const messages = readBusMessages(busPath)
    expect(messages).toHaveLength(1)
    expect(messages[0].type).toBe('completion')
    const payload = messages[0].payload.payload as CompletionPayload
    expect(payload.summary).toBe('Task completed successfully')
  })
})

describe('reportError', () => {
  it('writes correct message structure', () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    process.env.CREWCHIEF_BUS_PATH = busPath
    reportError('Something went wrong', 'test-agent', true)
    const messages = readBusMessages(busPath)
    expect(messages).toHaveLength(1)
    expect(messages[0].type).toBe('error')
    const payload = messages[0].payload.payload as ErrorPayload
    expect(payload.message).toBe('Something went wrong')
    expect(payload.recoverable).toBe(true)
  })

  it('populates code and stack from Error object', () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    process.env.CREWCHIEF_BUS_PATH = busPath
    const error = new TypeError('Invalid argument')
    reportError('Operation failed', 'test-agent', false, error)
    const messages = readBusMessages(busPath)
    const payload = messages[0].payload.payload as ErrorPayload
    expect(payload.code).toBe('TypeError')
    expect(payload.stack).toContain('TypeError: Invalid argument')
  })

  it('leaves code and stack undefined without Error object', () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    process.env.CREWCHIEF_BUS_PATH = busPath
    reportError('Manual error message', 'test-agent', false)
    const messages = readBusMessages(busPath)
    const payload = messages[0].payload.payload as ErrorPayload
    expect(payload.code).toBeUndefined()
    expect(payload.stack).toBeUndefined()
  })
})

describe('reportFileChanges', () => {
  it('writes correct message structure with file list', () => {
    const busPath = path.join(tmpDir, 'bus.jsonl')
    process.env.CREWCHIEF_BUS_PATH = busPath
    const files: Array<{ path: string; status: 'added' | 'modified' | 'deleted' }> = [
      { path: 'src/index.ts', status: 'modified' },
      { path: 'src/new-file.ts', status: 'added' },
    ]
    reportFileChanges(files, 'test-agent')
    const messages = readBusMessages(busPath)
    expect(messages).toHaveLength(1)
    expect(messages[0].type).toBe('file-change')
    const payload = messages[0].payload.payload as FileChangePayload
    expect(payload.files).toHaveLength(2)
  })
})

describe('graceful degradation', () => {
  it('all functions are no-ops when env var is not set', () => {
    delete process.env.CREWCHIEF_BUS_PATH
    expect(() => reportStatus('testing', 'agent')).not.toThrow()
    expect(() => reportCompletion('done', 'agent')).not.toThrow()
    expect(() => reportError('error', 'agent', false)).not.toThrow()
    expect(() => reportFileChanges([{ path: 'test.ts', status: 'added' }], 'agent')).not.toThrow()
  })
})
