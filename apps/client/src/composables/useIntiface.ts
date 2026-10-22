import type { IntifaceDevice, IntifaceState, IntifaceStatus, ToyActuatorRef, ToyTouched } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef } from 'vue'
import { computed, ref } from 'vue'

export type { IntifaceActuator, IntifaceDevice, IntifaceStatus } from '@renderer/lib/tauri-bridge'

// Where the webview kept these before Rust took over (see src-tauri/src/intiface). Handed over
// once, then removed.
const LEGACY_URL_KEY = 'intiface_url'
const LEGACY_ENABLED_KEY = 'intiface_enabled'
const LEGACY_AUTO_LAUNCH_KEY = 'intiface_autoLaunch'

// The connection to Intiface runs in Rust (src-tauri/src/intiface), so toys keep their timing
// while this window is hidden and keep going through a reload. Status, as Rust reports it:
// 'disabled': the Intiface toys add-on is off (Settings > Add-ons), the default.
// 'connecting': a connection attempt is in flight, including retries.
// 'connected': the handshake succeeded. Fail-closed: only this counts as available.
// 'error': nothing is listening at the URL; a retry is still scheduled.
// 'refused': something is listening but wouldn't accept us (e.g. another app is connected).
// 'busy': nothing is listening, but Intiface Central is running here: busy with another client,
// or its server is stopped.
// 'launching': auto-launch opened Intiface Central and is giving it time to start its server.

// Module-scope singletons - every useIntiface() caller shares Rust's state, so a change made in
// Settings (or a toy being plugged in) is instantly reflected everywhere.
const state = ref<IntifaceState | null>(null)

api.onIntifaceState((value) => (state.value = value))
void api.intifaceState().then((value) => (state.value ??= value))

// The actuators avatar touch drives right now ("device:actuator"). Viewers wait on them, so their
// toy and pattern tiles read Paused.
const touched = ref<Set<string>>(new Set())
const touchKey = (deviceIndex: number, actuatorIndex: number): string => `${deviceIndex}:${actuatorIndex}`
const setTouched = (list: ToyTouched[]): void => {
  touched.value = new Set(list.map((actuator) => touchKey(actuator.deviceIndex, actuator.actuatorIndex)))
}

api.onToysState(({ touched }) => setTouched(touched))
void api.toysTouched().then(setTouched)

export const isTouched = (actuators: ToyActuatorRef[]): boolean =>
  actuators.some((actuator) => touched.value.has(touchKey(actuator.deviceIndex, actuator.actuatorIndex)))

const legacyUrl = localStorage.getItem(LEGACY_URL_KEY)
const legacyEnabled = localStorage.getItem(LEGACY_ENABLED_KEY)
const legacyAutoLaunch = localStorage.getItem(LEGACY_AUTO_LAUNCH_KEY)

if (legacyUrl !== null || legacyEnabled !== null || legacyAutoLaunch !== null) {
  void api
    .intifaceMigrate(legacyUrl, legacyEnabled === null ? null : legacyEnabled === 'true', legacyAutoLaunch === null ? null : legacyAutoLaunch === 'true')
    .then(() => {
      localStorage.removeItem(LEGACY_URL_KEY)
      localStorage.removeItem(LEGACY_ENABLED_KEY)
      localStorage.removeItem(LEGACY_AUTO_LAUNCH_KEY)
    })
    .catch((error) => console.error('Could not move the Intiface settings to the app:', error))
}

const url = computed(() => state.value?.url ?? '')
const enabled = computed(() => state.value?.enabled ?? false)
const status = computed<IntifaceStatus>(() => state.value?.status ?? 'disabled')
const devices = computed(() => new Map((state.value?.devices ?? []).map((device) => [device.index, device] as const)))
const knownDevices = computed(() => state.value?.knownDevices ?? [])
const installed = computed(() => state.value?.installed ?? false)
const autoLaunch = computed(() => state.value?.autoLaunch ?? false)
const launchAttempts = computed(() => state.value?.launchAttempts ?? 0)
const refusedReason = computed(() => state.value?.refusedReason ?? '')
const isAvailable = computed(() => status.value === 'connected')

const DEFAULT_INTIFACE_URL = 'ws://localhost:12345'
const MAX_LAUNCH_ATTEMPTS = 5

export function useIntiface(): {
  url: ComputedRef<string>
  defaultUrl: string
  isCustomUrl: ComputedRef<boolean>
  setUrl: (value?: string) => void
  enabled: ComputedRef<boolean>
  status: ComputedRef<IntifaceStatus>
  isAvailable: ComputedRef<boolean>
  devices: ComputedRef<Map<number, IntifaceDevice>>
  knownDevices: ComputedRef<IntifaceDevice[]>
  installed: ComputedRef<boolean>
  launch: () => Promise<void>
  autoLaunch: ComputedRef<boolean>
  setAutoLaunch: (value: boolean) => void
  launchAttempts: ComputedRef<number>
  maxLaunchAttempts: number
  refusedReason: ComputedRef<string>
  setActuatorIntensity: (actuators: ToyActuatorRef[], value: number) => void
  // Stops every connected toy, not only the ones a control drives.
  stopAllDevices: () => void
} {
  // Opens the local install; the connection loop in Rust picks it up once its server starts.
  const launch = async (): Promise<void> => {
    try {
      await api.intifaceLaunch()
    } catch (error) {
      console.error('Failed to launch Intiface Central:', error)
    }
  }

  return {
    url,
    defaultUrl: state.value?.defaultUrl ?? DEFAULT_INTIFACE_URL,
    isCustomUrl: computed(() => !!url.value),
    // Reconnects against the new URL right away.
    setUrl: (value) => void api.intifaceSetUrl(value?.trim() || null),
    enabled,
    status,
    isAvailable,
    devices,
    knownDevices,
    installed,
    launch,
    autoLaunch,
    // Turning it on gives a fresh 5 tries.
    setAutoLaunch: (value) => void api.intifaceSetAutoLaunch(value),
    launchAttempts,
    maxLaunchAttempts: state.value?.maxLaunchAttempts ?? MAX_LAUNCH_ATTEMPTS,
    refusedReason,
    // Only the plain fields go to Rust: a control's actuator refs also carry names.
    setActuatorIntensity: (actuators, value) =>
      void api.toysSet(
        actuators.map(({ deviceIndex, actuatorIndex, actuatorType }) => ({ deviceIndex, actuatorIndex, actuatorType })),
        value
      ),
    stopAllDevices: () => void api.toysStopAll()
  }
}
