import { useClientType } from '@renderer/composables/useClientType'
import { viewersSetup } from '@renderer/composables/useSetup'
import type { ShareSettingsMessage } from '@toolkitosc/shared-ui'
import { computed, ref } from 'vue'

const STORAGE_KEY = 'showViewersToViewers'
const TOP_STORAGE_KEY = 'showTopViewers'

// Read once at load, so it's right before the first share-settings message goes out. On unless
// the streamer turned it off.
const showViewers = ref(localStorage.getItem(STORAGE_KEY) !== 'false')
// Off unless the streamer turns it on.
const showTopViewers = ref(localStorage.getItem(TOP_STORAGE_KEY) === 'true')

// What the server enforces on the share page (see useWebsocketHost.ts, which sends it).
const { clientType } = useClientType()

const shareSettings = computed<ShareSettingsMessage>(() => ({
  showViewers: showViewers.value,
  // "Only people you let in": the server keeps everyone else waiting on the share page.
  approval: clientType.value === 'approved',
  approved: viewersSetup.value.approved ?? [],
  showTopViewers: showTopViewers.value
}))

export function useShareSettings(): {
  showViewers: typeof showViewers
  shareSettings: typeof shareSettings
  setShowViewers: (value: boolean) => void
  showTopViewers: typeof showTopViewers
  setShowTopViewers: (value: boolean) => void
} {
  const setShowViewers = (value: boolean): void => {
    showViewers.value = value
    localStorage.setItem(STORAGE_KEY, String(value))
  }

  const setShowTopViewers = (value: boolean): void => {
    showTopViewers.value = value
    localStorage.setItem(TOP_STORAGE_KEY, String(value))
  }

  return { showViewers, shareSettings, setShowViewers, showTopViewers, setShowTopViewers }
}
