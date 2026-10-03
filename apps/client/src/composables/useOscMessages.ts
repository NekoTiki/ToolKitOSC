import { api } from '@renderer/lib/tauri-bridge'
import { args, useOscMessages as useSharedOscMessages } from '@toolkitosc/shared-ui'

import type { OscCommand, OSCMessage } from '../env.d'

// Populate-side wrapper: the shared package's `args` ref is read-only from its own perspective
// (it only exposes `get`) — this file is what actually feeds it, from the Tauri bridge's OSC
// events, and adds the client-only `update()` that sends a command back out to VRChat.
api.onOscMessage((msg: OSCMessage) => {
  args.value[msg.address] = msg.args
})

api.onOscMessageBulk((msg: OSCMessage[]) => {
  args.value = {}
  msg.forEach((m) => (args.value[m.address] = m.args))
})

api.onOscMessageMerge((msg: OSCMessage[]) => {
  msg.forEach((m) => (args.value[m.address] = m.args))
})

export function useOscMessages(onArgChange?: (msg: OSCMessage) => void): {
  args: typeof args
  get: <T>(address: string, defaultValue: T) => T
  update: (command: OscCommand) => void
} {
  const { get } = useSharedOscMessages()

  // Values loaded when an avatar loads are real changes to what's known, so they reach
  // onArgChange too (e.g. useWebsocketHost forwards them to viewers).
  if (onArgChange) {
    api.onOscMessage(onArgChange)
    api.onOscMessageMerge((msg) => msg.forEach(onArgChange))
  }

  const update = (command: OscCommand): void => {
    console.log(
      `%c[OSC]%c[${command.address}] %c${JSON.stringify(command.args)}`,
      'color: #42b983; font-weight: bold;',
      'color: #ba43ba; font-weight: bold;',
      'color: #ffffff;'
    )
    api.sendOscMessage(JSON.parse(JSON.stringify(command)))
  }

  return { args, get, update }
}
