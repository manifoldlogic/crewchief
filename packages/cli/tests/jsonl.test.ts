import { describe, it, expect } from 'vitest'
import { encodeJsonl, decodeJsonl } from '../src/bus/jsonl'

describe('JSONL', () => {
  it('encodes and decodes envelopes', () => {
    const env = { id: '1', type: 'status', payload: { a: 1 }, ts: new Date().toISOString() }
    const line = encodeJsonl(env)
    const parsed = decodeJsonl(line)
    expect(parsed?.type).toBe('status')
    expect((parsed?.payload as any).a).toBe(1)
  })
})
