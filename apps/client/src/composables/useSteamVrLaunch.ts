import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

const ENABLED_STORAGE_KEY = 'steamVrAutoLaunch_enabled'

// 'disabled': the user hasn't turned this on (or turned it back off) in Settings.
// 'checking': a registration attempt (steamvr_set_auto_launch) is in flight.
// 'enabled': SteamVR confirmed the auto-launch flag is set - same pattern as useIntiface's status.
// 'error': the last attempt failed - most commonly because SteamVR wasn't running (registering
// requires a live SteamVR to talk to, there's no way around that), surfaced via `error` below.
export type SteamVrAutoLaunchStatus = 'disabled' | 'checking' | 'enabled' | 'error'

// Module-scope singletons - same reasoning as useIntiface/useOpenShock: every caller (Settings,
// anything else that cares) shares one source of truth.
const enabled = ref<boolean>(localStorage.getItem(ENABLED_STORAGE_KEY) === 'true')
const status = ref<SteamVrAutoLaunchStatus>('disabled')
const error = ref<string>('')

// Whether SteamVR was found on this machine at all (checked once at module load, below) - lets
// Settings hide/disable the toggle entirely rather than offering something that can only ever fail.
const available = ref(false)

export function useSteamVrLaunch(): {
  enabled: Ref<boolean>
  setEnabled: (value: boolean) => Promise<void>
  status: Ref<SteamVrAutoLaunchStatus>
  error: Ref<string>
  available: ComputedRef<boolean>
} {
  const setEnabled = async (value: boolean): Promise<void> => {
    enabled.value = value
    localStorage.setItem(ENABLED_STORAGE_KEY, String(value))
    status.value = 'checking'
    error.value = ''

    try {
      await api.steamVrSetAutoLaunch(value)
      status.value = value ? 'enabled' : 'disabled'
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
  .steamVrAvailable()
  .then((value) => (available.value = value))
  .catch(() => (available.value = false))

// Registering with SteamVR requires SteamVR to already be running (see steamvr.rs), so a previous
// attempt may well have failed for no reason other than bad timing. Retrying once at every app
// start - not just when the user touches the toggle - is what actually makes "launch with
// SteamVR" self-healing across sessions where SteamVR happens to already be up.
if (enabled.value) {
  status.value = 'checking'

  api
    .steamVrSetAutoLaunch(true)
    .then(() => (status.value = 'enabled'))
    .catch((err) => {
      status.value = 'error'
      error.value = String(err)
    })
}
