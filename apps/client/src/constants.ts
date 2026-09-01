// Replaces the old preload-injected `window.serverUrl`/`window.serverWsUrl` globals (which came
// from src/main/constants.ts + contextBridge). Tauri has no preload script, so these are now
// ordinary Vite build-time env vars instead — set via `.env`, `--mode`, or (see package.json's
// `dev:prod` script) inline on the command line.
export const SERVER_URL = import.meta.env.VITE_SERVER_URL || 'http://localhost:3000'

// Build-time fallback only. The effective WS URL is `useWebsocketSettings().serverWsUrl`, which
// layers a user-set override (Settings > Connection, persisted in localStorage) on top of this.
export const DEFAULT_SERVER_WS_URL = import.meta.env.VITE_SERVER_WS_URL || 'ws://localhost:3000'
