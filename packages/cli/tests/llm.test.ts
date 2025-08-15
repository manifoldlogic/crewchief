import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { generateText } from '../src/utils/llm'

const ORIGINAL_ENV = { ...process.env }
let originalFetch: any

function setEnv(next: Record<string, string | undefined>) {
  for (const k of Object.keys(process.env)) delete (process.env as any)[k]
  Object.assign(process.env, ORIGINAL_ENV, next)
}

function mockFetchResolved(payload: any, ok = true, status = 200) {
  const res = {
    ok,
    status,
    json: async () => payload,
    text: async () => JSON.stringify(payload),
  } as any
  ;(globalThis as any).fetch = vi.fn().mockResolvedValue(res)
  return (globalThis as any).fetch as ReturnType<typeof vi.fn>
}

function mockFetchRejectedText(text: string, status = 400) {
  const res = {
    ok: false,
    status,
    text: async () => text,
  } as any
  ;(globalThis as any).fetch = vi.fn().mockResolvedValue(res)
  return (globalThis as any).fetch as ReturnType<typeof vi.fn>
}

beforeEach(() => {
  originalFetch = (globalThis as any).fetch
})

afterEach(() => {
  setEnv({})
  vi.restoreAllMocks()
  if (originalFetch) (globalThis as any).fetch = originalFetch
  else delete (globalThis as any).fetch
})

describe('generateText', () => {
  it('errors when provider is OpenAI but key missing', async () => {
    setEnv({ OPENAI_API_KEY: undefined, ANTHROPIC_API_KEY: undefined, LLM_PROVIDER: undefined })
    mockFetchResolved({})
    await expect(generateText('hi')).rejects.toThrow('OPENAI_API_KEY is not set')
  })

  it('calls OpenAI when OPENAI_API_KEY is set', async () => {
    setEnv({ OPENAI_API_KEY: 'sk-test', ANTHROPIC_API_KEY: undefined, LLM_PROVIDER: undefined })
    const content = 'hello from openai'
    const fetchMock = mockFetchResolved({ choices: [{ message: { content } }] })
    const result = await generateText('hi')
    expect(result).toBe(content)
    expect(fetchMock).toHaveBeenCalledTimes(1)
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toContain('https://api.openai.com/v1/chat/completions')
    expect((init.headers as any).Authorization).toContain('Bearer sk-test')
    const body = JSON.parse(init.body as string)
    expect(body.model).toBe(process.env.OPENAI_MODEL || 'gpt-4o-mini')
    expect(Array.isArray(body.messages)).toBe(true)
  })

  it('calls Anthropic when ANTHROPIC_API_KEY is set', async () => {
    setEnv({ OPENAI_API_KEY: undefined, ANTHROPIC_API_KEY: 'ak-test', LLM_PROVIDER: undefined })
    const text = 'hello from anthropic'
    const fetchMock = mockFetchResolved({ content: [{ text }] })
    const result = await generateText('hi')
    expect(result).toBe(text)
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toContain('https://api.anthropic.com/v1/messages')
    expect((init.headers as any)['x-api-key']).toBe('ak-test')
  })

  it('respects LLM_PROVIDER override to anthropic', async () => {
    setEnv({ OPENAI_API_KEY: 'sk-test', ANTHROPIC_API_KEY: 'ak-test', LLM_PROVIDER: 'anthropic' })
    const text = 'anthropic wins override'
    const fetchMock = mockFetchResolved({ content: [{ text }] })
    const result = await generateText('hi')
    expect(result).toBe(text)
    const [url] = fetchMock.mock.calls[0]
    expect(url).toContain('anthropic.com')
  })

  it('surfaces API error text for OpenAI', async () => {
    setEnv({ OPENAI_API_KEY: 'sk-test', ANTHROPIC_API_KEY: undefined })
    mockFetchRejectedText('rate limit')
    await expect(generateText('hi')).rejects.toThrow(/OpenAI API error: 400/i)
  })
})
