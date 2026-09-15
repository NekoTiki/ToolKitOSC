import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

const ENABLED_STORAGE_KEY = 'vrcxAutoLaunch_enabled'

export type VrcxAutoLaunchStatus = 'disabled' | 'enabled' | 'error'

// Module-scope singletons - same reasoning as useSteamVrLaunch/useIntiface.
const enabled = ref<boolean>(localStorage.getItem(ENABLED_STORAGE_KEY) === 'true')
const status = ref<VrcxAutoLaunchStatus>('disabled')
const error = ref<string>('')

// Whether VRCX's Auto-Launch Folder was found on this machine (checked once at module load,
// below) - lets Settings hide/disable the toggle when VRCX isn't installed at all.
const available = ref(false)

export function useVrcxLaunch(): {
  enabled: Ref<boolean>
  setEnabled: (value: boolean) => Promise<void>
  status: Ref<VrcxAutoLaunchStatus>
  error: Ref<string>
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
    available: computed(() => available.value)
  }
}

api
  .vrcxAvailable()
  .then((value) => (available.value = value))
  .catch(() => (available.value = false))

// Unlike SteamVR's registration (which lives in a running SteamVR's own memory/config and needs
// re-establishing every session), the shortcut this drops in VRCX's Auto-Launch Folder is plain
// state on disk - it either exists or it doesn't, no retry-on-launch needed to keep it in sync.
