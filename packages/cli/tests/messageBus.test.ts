import { describe, it, expect } from 'vitest'
import { MessageBus } from '../src/bus/message.bus'

describe('MessageBus', () => {
  it('delivers messages to subscribers', async () => {
    const bus = new MessageBus()
    const received: any[] = []
    bus.onMessage((m) => received.push(m))
    bus.send({ type: 'status', from: 'a', to: 'orchestrator', payload: { ok: true }, timestamp: new Date() })
    expect(received.length).toBe(1)
    expect(received[0].payload.ok).toBe(true)
  })

  it('waits for response matching predicate', async () => {
    const bus = new MessageBus()
    const p = bus.waitForResponse((m) => m.type === 'result', 1000)
    bus.send({ type: 'status', from: 'a', to: 'orchestrator', payload: {}, timestamp: new Date() })
    setTimeout(
      () => bus.send({ type: 'result', from: 'a', to: 'orchestrator', payload: { v: 1 }, timestamp: new Date() }),
      10,
    )
    const res = await p
    expect((res.payload as any).v).toBe(1)
  })
})
