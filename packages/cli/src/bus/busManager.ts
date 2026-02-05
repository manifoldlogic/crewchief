import { CrossProcessBusReader } from './crossProcessBusReader'
import { AgentMessage } from './message.types'

export class BusManager {
  private readers = new Map<string, CrossProcessBusReader>()
  private onMessage: (message: AgentMessage) => void
  private intervalMs: number

  constructor(options?: { onMessage?: (message: AgentMessage) => void; intervalMs?: number }) {
    this.onMessage = options?.onMessage ?? (() => {})
    this.intervalMs = options?.intervalMs ?? 500
  }

  startFollowing(runId: string, busFilePath: string): void {
    if (this.readers.has(runId)) return // Already following

    const reader = new CrossProcessBusReader({
      filePath: busFilePath,
      intervalMs: this.intervalMs,
      onMessage: this.onMessage,
    })
    reader.start()
    this.readers.set(runId, reader)
  }

  stopFollowing(runId: string): void {
    const reader = this.readers.get(runId)
    if (reader) {
      reader.stop()
      this.readers.delete(runId)
    }
  }

  stopAll(): void {
    for (const [_runId, reader] of this.readers) {
      reader.stop()
    }
    this.readers.clear()
  }

  isFollowing(runId: string): boolean {
    return this.readers.has(runId)
  }

  get activeRunIds(): string[] {
    return Array.from(this.readers.keys())
  }
}
