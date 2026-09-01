/// <reference types="vite/client" />

// OSC/avatar wire types now live in @vrc-osc-toolkit/shared-ui/types (shared with the server's
// Nitro relay) — re-exported here so existing `from '../env.d'` imports keep working.
export type { AvatarDetails, AvatarParameter, OSCArg, OscCommand, OSCMessage } from '@vrc-osc-toolkit/shared-ui'

declare global {
  interface ImportMetaEnv {
    readonly VITE_SERVER_URL?: string
    readonly VITE_SERVER_WS_URL?: string
  }
}
