import type { ShareSettingsMessage } from '@toolkitosc/shared-ui'
import { computed, ref } from 'vue'

const STORAGE_KEY = 'showViewersToViewers'

// Read once at load, so it's right before the first share-settings message goes out. On unless
// the streamer turned it off.
const showViewers = ref(localStorage.getItem(STORAGE_KEY) !== 'false')

// What the server enforces on the share page (see useWebsocketHost.ts, which sends it).
const shareSettings = computed<ShareSettingsMessage>(() => ({ showViewers: showViewers.value }))

export function useShareSettings(): {
  showViewers: typeof showViewers
  shareSettings: typeof shareSettings
  setShowViewers: (value: boolean) => void
} {
  const setShowViewers = (value: boolean): void => {
    showViewers.value = value
    localStorage.setItem(STORAGE_KEY, String(value))
  }

  return { showViewers, shareSettings, setShowViewers }
}
