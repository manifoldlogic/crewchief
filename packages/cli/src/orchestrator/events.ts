import { RunManager } from './runManager'
import { createFileChangeHandler } from '../bus/fileChangeHandler'
import { messageBus } from '../bus/index'
import { AgentMessage } from '../bus/message.types'

export function startOrchestratorEventBridge(): void {
  const rm = new RunManager()

  // Register file-change handler for Connection D (bus events → maproom upsert).
  // Uses cwd as the worktree path; the handler filters for file-change messages.
  const fileChangeHandler = createFileChangeHandler(process.cwd())
  messageBus.onMessage(fileChangeHandler)

  messageBus.onMessage((msg: AgentMessage) => {
    if (msg.to !== 'orchestrator') return
    try {
      const payload = msg.payload as any
      const runId = payload?.payload?.runId ?? undefined // optional future field
      if (runId) {
        rm.appendLog(runId, 'orchestrator.log', JSON.stringify(msg))
      }
    } catch {
      // ignore
    }
  })
}
