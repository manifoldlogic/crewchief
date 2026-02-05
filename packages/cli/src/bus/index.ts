import { BusManager } from './busManager'
import { MessageBus } from './message.bus'

export const messageBus = new MessageBus()
export const busManager = new BusManager({
  onMessage: (msg) => messageBus.send(msg),
})
