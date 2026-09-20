// Replaces the old preload-injected `window.serverWsUrl` global (which came from
// src/main/constants.ts + contextBridge). Tauri has no preload script, so this is now an ordinary
// Vite build-time env var instead — set via `.env`, `--mode`, or (see package.json's `dev:prod`
// script) inline on the command line.
//
// Build-time fallback only. The effective WS URL is `useWebsocketSettings().serverWsUrl`, which
// layers a user-set override (Settings > Connection, persisted in localStorage) on top of this -
// and every plain HTTP(S) request derives its origin from that same effective URL too (see
// `serverHttpUrl` in useWebsocketSettings.ts), rather than a separate VITE_SERVER_URL build-time
// constant that could silently disagree with it.
export const DEFAULT_SERVER_WS_URL = import.meta.env.VITE_SERVER_WS_URL || 'ws://localhost:3000'
