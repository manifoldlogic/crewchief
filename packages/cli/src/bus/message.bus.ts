import { EventEmitter } from 'node:events'
import { AgentMessage } from './message.types'

export class MessageBus {
  private emitter = new EventEmitter()

  send(message: AgentMessage): void {
    this.emitter.emit('message', message)
  }

  onMessage(handler: (msg: AgentMessage) => void): void {
    this.emitter.on('message', handler)
  }

  waitForResponse(predicate: (msg: AgentMessage) => boolean, timeoutMs = 30_000): Promise<AgentMessage> {
    return new Promise((resolve, reject) => {
      const onMsg = (msg: AgentMessage) => {
        if (predicate(msg)) {
          cleanup()
          resolve(msg)
        }
      }
      const cleanup = () => {
        clearTimeout(timer)
        this.emitter.off('message', onMsg)
      }
      const timer = setTimeout(() => {
        cleanup()
        reject(new Error('Timeout waiting for response'))
      }, timeoutMs)
      this.emitter.on('message', onMsg)
    })
  }
}
