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

export function useWebsocketSettings(): {
  serverWsUrl: ComputedRef<string>
  defaultServerWsUrl: string
  isCustom: ComputedRef<boolean>
  setServerWsUrl: (url?: string) => void
} {
  const isCustom = computed(() => !!override.value)

  const setServerWsUrl = (url?: string): void => {
    const trimmed = url?.trim()

    if (trimmed && trimmed !== DEFAULT_SERVER_WS_URL) {
      override.value = trimmed
      localStorage.setItem(STORAGE_KEY, trimmed)
    } else {
      override.value = undefined
      localStorage.removeItem(STORAGE_KEY)
    }
  }

  return { serverWsUrl, defaultServerWsUrl: DEFAULT_SERVER_WS_URL, isCustom, setServerWsUrl }
}
