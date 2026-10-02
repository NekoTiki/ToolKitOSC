// Thin re-export: the actual read-only OSC-state singleton now lives in @toolkitosc/shared-ui
// (shared with the desktop client's control-rendering components). useWebsocketClient.ts populates
// `args` directly from `args-initial`/`args-update` WS messages.
export { args, useOscMessages } from '@toolkitosc/shared-ui'
