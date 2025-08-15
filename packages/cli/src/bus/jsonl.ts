export interface JsonlEnvelope<T = unknown> {
  id: string
  type: string
  payload: T
  ts: string
}

export function encodeJsonl(envelope: JsonlEnvelope): string {
  return JSON.stringify(envelope) + '\n'
}

export function decodeJsonl(line: string): JsonlEnvelope | undefined {
  try {
    return JSON.parse(line) as JsonlEnvelope
  } catch {
    return undefined
  }
}
