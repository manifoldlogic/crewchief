import { RunManager } from './runManager'
import { messageBus } from '../bus/index'
import { AgentMessage } from '../bus/message.types'

export function startOrchestratorEventBridge(): void {
  const rm = new RunManager()
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
