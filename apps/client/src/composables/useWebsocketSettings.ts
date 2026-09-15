import { DEFAULT_SERVER_WS_URL } from '@renderer/constants'
import type { ComputedRef } from 'vue'
import { computed, ref } from 'vue'

const STORAGE_KEY = 'wsServerUrlOverride'

// The client is a Tauri (client-rendered only, no SSR) webview, so — unlike the SSR-shared
// composables in shared-ui — it's safe to read localStorage synchronously at module init rather
// than waiting for onMounted. That matters here: useWebsocketHost/useWebsocketAuth read
// `serverWsUrl` while building their connection URL during their own setup(), before any
// onMounted callback would have run.
const override = ref<string | undefined>(localStorage.getItem(STORAGE_KEY) || undefined)

// Effective WS base URL: the user's override if they've set one, otherwise the build-time
// default. Passed to `useWebSocket` as a getter elsewhere, so changing it live triggers a
// reconnect (see vueuse's `watch(urlRef, open)`).
export const serverWsUrl = computed<string>(() => override.value || DEFAULT_SERVER_WS_URL)

// The server's plain HTTP(S) origin, derived from `serverWsUrl` rather than the build-time
// SERVER_URL constant - so a share link always points at whichever server the user is actually
// connected to right now, custom override included. `ws`/`wss` share the same host:port as their
// `http`/`https` counterpart on this server, so a straight protocol swap is enough.
export const serverHttpUrl = computed<string>(() => serverWsUrl.value.replace(/^ws/, 'http'))

export const isValidServerWsUrl = (url: string): boolean => {
  try {
    const protocol = new URL(url).protocol

    return protocol === 'ws:' || protocol === 'wss:'
  } catch {
    return false
  }
}

export function useWebsocketSettings(): {
  serverWsUrl: ComputedRef<string>
  defaultServerWsUrl: string
  isCustom: ComputedRef<boolean>
  setServerWsUrl: (url?: string) => void
} {
  const isCustom = computed(() => !!override.value)

  const setServerWsUrl = (url?: string): void => {
    // `undefined` is the explicit "reset to default" call (the reset button). Anything else must
    // be a valid, non-empty ws(s):// URL to take effect - a blank or malformed value (e.g. the
    // user is mid-edit and has momentarily cleared the field) is ignored rather than falling back
    // to the default and forcing a reconnect.
    if (url === undefined) {
      override.value = undefined
      localStorage.removeItem(STORAGE_KEY)
      return
    }

    const trimmed = url.trim()

    if (!trimmed || !isValidServerWsUrl(trimmed)) return

    if (trimmed === DEFAULT_SERVER_WS_URL) {
      override.value = undefined
      localStorage.removeItem(STORAGE_KEY)
    } else {
      override.value = trimmed
      localStorage.setItem(STORAGE_KEY, trimmed)
    }
  }

  return { serverWsUrl, defaultServerWsUrl: DEFAULT_SERVER_WS_URL, isCustom, setServerWsUrl }
}
