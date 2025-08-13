import fs from 'node:fs'
import { decodeJsonl, JsonlEnvelope } from './jsonl'

export class LogFollower {
  private filePath: string
  private intervalMs: number
  private timer?: NodeJS.Timeout
  private offset = 0
  private buffer = ''

  constructor(filePath: string, intervalMs = 500) {
    this.filePath = filePath
    this.intervalMs = intervalMs
  }

  start(onEnvelope: (env: JsonlEnvelope) => void): void {
    const tick = () => {
      try {
        if (!fs.existsSync(this.filePath)) return
        const stat = fs.statSync(this.filePath)
        if (stat.size > this.offset) {
          const stream = fs.createReadStream(this.filePath, {
            start: this.offset,
            end: stat.size - 1,
            encoding: 'utf8',
          })
          stream.on('data', (chunk: string) => {
            this.buffer += chunk
            let idx: number
            while ((idx = this.buffer.indexOf('\n')) >= 0) {
              const line = this.buffer.slice(0, idx)
              this.buffer = this.buffer.slice(idx + 1)
              const env = decodeJsonl(line)
              if (env) onEnvelope(env)
            }
          })
          stream.on('close', () => {
            this.offset = stat.size
          })
        }
      } catch {
        // ignore transient errors
      }
    }
    this.timer = setInterval(tick, this.intervalMs)
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer)
  }
}
