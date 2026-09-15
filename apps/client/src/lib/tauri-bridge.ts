// Replaces the old Electron preload's `window.api` (contextBridge) surface. Deliberately keeps
// the same method names/shapes as that API so the composables built on top of it (useOscMessages,
// useAvatarDetails) barely had to change when this app moved from Electron to Tauri — see
// apps/client's entry in the monorepo plan for the reasoning.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { AvatarDetails, OscCommand, OSCMessage } from '@vrc-osc-toolkit/shared-ui'

export const api = {
  ready: (): void => {
    void invoke('ready')
  },
  openUrl: (url: string): void => {
    void invoke('open_url', { url })
  },
  intifaceCentralAvailable: (): Promise<boolean> => invoke('intiface_central_available'),
  startIntifaceCentral: (): Promise<void> => invoke('start_intiface_central'),
  onOscMessage: (callback: (msg: OSCMessage) => void): void => {
    void listen<OSCMessage>('vrc-osc-message', (event) => callback(event.payload))
  },
  onOscMessageBulk: (callback: (msg: OSCMessage[]) => void): void => {
    void listen<OSCMessage[]>('vrc-osc-message-bulk', (event) => callback(event.payload))
  },
  sendOscMessage: (msg: OscCommand): void => {
    void invoke('send_osc_message', { msg })
  },
  onAvatarDetails: (callback: (details: AvatarDetails) => void): void => {
    void listen<AvatarDetails>('vrc-avatar-details', (event) => callback(event.payload))
  }
}

export type TauriApiType = typeof api
