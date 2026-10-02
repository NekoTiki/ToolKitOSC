import type { IntegrationAvailability } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

const ENABLED_STORAGE_KEY = 'vrcxAutoLaunch_enabled'

export type VrcxAutoLaunchStatus = 'disabled' | 'enabled' | 'error'

// Module-scope singletons - same reasoning as useSteamVrLaunch/useIntiface.
const enabled = ref<boolean>(localStorage.getItem(ENABLED_STORAGE_KEY) === 'true')
const status = ref<VrcxAutoLaunchStatus>('disabled')
const error = ref<string>('')

// Whether VRCX's Auto-Launch Folder can be used here (checked once at module load, below) - lets
// Settings hide the toggle where VRCX doesn't support it, and disable it when VRCX isn't installed.
// 'unsupported' until the check answers.
const availability = ref<IntegrationAvailability>('unsupported')

export function useVrcxLaunch(): {
  enabled: Ref<boolean>
  setEnabled: (value: boolean) => Promise<void>
  status: Ref<VrcxAutoLaunchStatus>
  error: Ref<string>
  availability: Ref<IntegrationAvailability>
  available: ComputedRef<boolean>
} {
  const setEnabled = async (value: boolean): Promise<void> => {
    enabled.value = value
    localStorage.setItem(ENABLED_STORAGE_KEY, String(value))

    try {
      await api.vrcxSetAutoLaunch(value)
      status.value = value ? 'enabled' : 'disabled'
      error.value = ''
    } catch (err) {
      status.value = 'error'
      error.value = String(err)
    }
  }

  return {
    enabled,
    setEnabled,
    status,
    error,
    availability,
    available: computed(() => availability.value === 'available')
  }
}

api
  .vrcxAvailable()
  .then((value) => (availability.value = value))
  .catch(() => (availability.value = 'unsupported'))

// Unlike SteamVR's registration (which lives in a running SteamVR's own memory/config and needs
// re-establishing every session), the shortcut this drops in VRCX's Auto-Launch Folder is plain
// state on disk - it either exists or it doesn't, no retry-on-launch needed to keep it in sync.
// It can still drift from the last-known `enabled` flag though (e.g. the shortcut was deleted
// externally), so check the actual file once at load and reconcile both `enabled` and `status`
// to match reality instead of trusting localStorage blindly.
api
  .vrcxGetAutoLaunch()
  .then((value) => {
    enabled.value = value
    localStorage.setItem(ENABLED_STORAGE_KEY, String(value))
    status.value = value ? 'enabled' : 'disabled'
  })
  .catch(() => {})
