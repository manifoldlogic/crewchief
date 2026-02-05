import { randomUUID } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { encodeJsonl, JsonlEnvelope } from './jsonl'
import { AgentMessage } from './message.types'
import { ensureDirSync } from '../utils/fs'

export class CrossProcessBusWriter {
  private filePath: string

  constructor(filePath: string) {
    this.filePath = filePath
    ensureDirSync(path.dirname(filePath))
  }

  write(message: AgentMessage): void {
    const envelope: JsonlEnvelope<AgentMessage> = {
      id: randomUUID(),
      type: message.type,
      payload: message,
      ts: new Date().toISOString(),
    }
    const encoded = encodeJsonl(envelope)

    // Enforce POSIX atomicity guarantee: reject messages > 4096 bytes
    if (encoded.length > 4096) {
      throw new Error(
        'Bus message exceeds maximum size (' +
          encoded.length +
          ' bytes, limit: 4096). ' +
          'This ensures atomic writes per POSIX O_APPEND guarantees.',
      )
    }

    // Create file with restrictive permissions on first write
    const fileExists = fs.existsSync(this.filePath)
    if (!fileExists) {
      fs.writeFileSync(this.filePath, '', { mode: 0o600 })
    }

    fs.appendFileSync(this.filePath, encoded)
  }

  get path(): string {
    return this.filePath
  }
}
