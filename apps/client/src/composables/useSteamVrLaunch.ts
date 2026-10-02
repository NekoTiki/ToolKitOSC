import type { IntegrationAvailability } from '@renderer/lib/tauri-bridge'
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

// Whether SteamVR can be used here (checked once at module load, below) - lets Settings hide the
// toggle on an OS SteamVR doesn't run on, and disable it when SteamVR just isn't installed, rather
// than offering something that can only ever fail. 'unsupported' until the check answers.
const availability = ref<IntegrationAvailability>('unsupported')

export function useSteamVrLaunch(): {
  enabled: Ref<boolean>
  setEnabled: (value: boolean) => Promise<void>
  status: Ref<SteamVrAutoLaunchStatus>
  error: Ref<string>
  availability: Ref<IntegrationAvailability>
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
    availability,
    available: computed(() => availability.value === 'available')
  }
}

api
  .steamVrAvailable()
  .then((value) => (availability.value = value))
  .catch(() => (availability.value = 'unsupported'))

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
