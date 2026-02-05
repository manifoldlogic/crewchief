import { JsonlEnvelope } from './jsonl'
import { LogFollower } from './logFollower'
import { AgentMessage } from './message.types'

export interface CrossProcessBusReaderOptions {
  filePath: string
  intervalMs?: number
  onMessage?: (message: AgentMessage) => void
}

export class CrossProcessBusReader {
  private follower: LogFollower
  private onMessage: (message: AgentMessage) => void

  constructor(options: CrossProcessBusReaderOptions) {
    this.follower = new LogFollower(options.filePath, options.intervalMs)
    this.onMessage = options.onMessage ?? (() => {})
  }

  start(): void {
    this.follower.start((envelope: JsonlEnvelope) => {
      const message = envelope.payload as AgentMessage
      if (message && typeof message.type === 'string') {
        this.onMessage(message)
      }
    })
  }

  stop(): void {
    this.follower.stop()
  }
}
