// OSC to and from VRChat, the worn avatar and its presets - src-tauri/src/osc/ and presets.rs.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { AvatarDetails, OscCommand, OSCMessage, Preset, PresetStore } from '@toolkitosc/shared-ui'

export const oscApi = {
  ready: (): void => {
    void invoke('ready')
  },
  onOscMessage: (callback: (msg: OSCMessage) => void): void => {
    void listen<OSCMessage>('vrc-osc-message', (event) => callback(event.payload))
  },
  onOscMessageBulk: (callback: (msg: OSCMessage[]) => void): void => {
    void listen<OSCMessage[]>('vrc-osc-message-bulk', (event) => callback(event.payload))
  },
  // Current values read from VRChat after an avatar loads (see the backend's
  // pull_on_avatar_load): merged into what's known, not a replacement like the bulk event.
  onOscMessageMerge: (callback: (msg: OSCMessage[]) => void): void => {
    void listen<OSCMessage[]>('vrc-osc-message-merge', (event) => callback(event.payload))
  },
  // Fired every time VRChat answers the backend's periodic avatar check (see poll_avatar_changes),
  // whether or not anything changed.
  onOscAlive: (callback: () => void): void => {
    void listen('vrc-osc-alive', () => callback())
  },
  sendOscMessage: (msg: OscCommand): void => {
    void invoke('send_osc_message', { msg })
  },
  onAvatarDetails: (callback: (details: AvatarDetails) => void): void => {
    void listen<AvatarDetails>('vrc-avatar-details', (event) => callback(event.payload))
  },
  loadPresets: (avatarId: string): Promise<PresetStore> => invoke('load_presets', { avatarId }),
  savePresets: (avatarId: string, store: PresetStore): Promise<void> =>
    invoke('save_presets', { avatarId, store }),
  // The avatar's values from its first load this session, kept unsaved (see the backend's
  // keep_original_state). Null until VRChat's values are in.
  originalState: (avatarId: string): Promise<Preset | null> => invoke('original_state', { avatarId }),
  onOriginalState: (callback: (preset: Preset) => void): void => {
    void listen<Preset>('original-state', (event) => callback(event.payload))
  },
  forcePullParameters: (missingOnly: boolean): Promise<number> =>
    invoke('force_pull_parameters', { missingOnly })
}
