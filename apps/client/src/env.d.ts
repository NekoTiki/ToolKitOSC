/// <reference types="vite/client" />

// OSC/avatar wire types now live in @toolkitosc/shared-ui/types (shared with the server's
// Nitro relay) — re-exported here so existing `from '../env.d'` imports keep working.
export type { AvatarDetails, AvatarParameter, OSCArg, OscCommand, OSCMessage } from '@toolkitosc/shared-ui'

declare global {
  interface ImportMetaEnv {
    readonly VITE_SERVER_WS_URL?: string
  }
}
