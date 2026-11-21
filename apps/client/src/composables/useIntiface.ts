import type { IntifaceDevice, IntifaceServer, IntifaceState, ToyActuatorRef, ToyTouched } from '@renderer/lib/tauri-bridge'
import { api, THIS_PC } from '@renderer/lib/tauri-bridge'
import type { ComputedRef } from 'vue'
import { computed, ref } from 'vue'

export type { IntifaceActuator, IntifaceDevice, IntifaceServer, IntifaceStatus } from '@renderer/lib/tauri-bridge'
export { THIS_PC } from '@renderer/lib/tauri-bridge'

// Where the webview kept these before Rust took over (see src-tauri/src/intiface). Handed over
// once, then removed.
const LEGACY_URL_KEY = 'intiface_url'
const LEGACY_ENABLED_KEY = 'intiface_enabled'
const LEGACY_AUTO_LAUNCH_KEY = 'intiface_autoLaunch'

// The connections to Intiface run in Rust (src-tauri/src/intiface), one per server you added, so
// toys keep their timing while this window is hidden and keep going through a reload. Each
// server's status, as Rust reports it:
// 'disabled': the Intiface toys add-on is off (Settings > Add-ons), the default.
// 'connecting': a connection attempt is in flight, including retries.
// 'connected': the handshake succeeded. Fail-closed: only this counts as available.
// 'error': nothing is listening at the address; a retry is still scheduled.
// 'refused': something is listening but wouldn't accept us (e.g. another app is connected).
// 'busy': nothing is listening, but Intiface Central is running here: busy with another client,
// or its server is stopped.
// 'launching': auto-launch opened Intiface Central and is giving it time to start its server.

// Module-scope singletons - every useIntiface() caller shares Rust's state, so a change made in
// Settings (or a toy being plugged in) is instantly reflected everywhere.
const state = ref<IntifaceState | null>(null)

api.onIntifaceState((value) => (state.value = value))
void api.intifaceState().then((value) => (state.value ??= value))

// A toy's key: indexes are only unique within one server. Refs saved before there were several
// servers have none: this PC.
export const toyKey = (server: string | undefined, deviceIndex: number): string => `${server ?? THIS_PC}:${deviceIndex}`

// The actuators avatar touch drives right now. Viewers wait on them, so their toy and pattern
// tiles read Paused.
const touched = ref<Set<string>>(new Set())
const touchKey = (server: string | undefined, deviceIndex: number, actuatorIndex: number): string => `${toyKey(server, deviceIndex)}:${actuatorIndex}`
const setTouched = (list: ToyTouched[]): void => {
  touched.value = new Set(list.map((actuator) => touchKey(actuator.server, actuator.deviceIndex, actuator.actuatorIndex)))
}

api.onToysState(({ touched }) => setTouched(touched))
void api.toysTouched().then(setTouched)

export const isTouched = (actuators: ToyActuatorRef[]): boolean =>
  actuators.some((actuator) => touched.value.has(touchKey(actuator.server, actuator.deviceIndex, actuator.actuatorIndex)))

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

const DEFAULT_PORT = 12345
const MAX_LAUNCH_ATTEMPTS = 5

// How a server is called everywhere: your name for it, else its address (the port only when it
// isn't Intiface's default).
export const serverName = (server: Pick<IntifaceServer, 'label' | 'host' | 'port'>): string =>
  server.label || (server.port === (state.value?.defaultPort ?? DEFAULT_PORT) ? server.host : `${server.host}:${server.port}`)

const enabled = computed(() => state.value?.enabled ?? false)
const servers = computed(() => state.value?.servers ?? [])
const serversById = computed(() => new Map(servers.value.map((server) => [server.id, server] as const)))
const devices = computed(
  () => new Map(servers.value.flatMap((server) => (server.status === 'connected' ? server.devices : [])).map((device) => [toyKey(device.server, device.index), device] as const))
)
const knownDevices = computed(() => state.value?.knownDevices ?? [])
const installed = computed(() => state.value?.installed ?? false)
const autoLaunch = computed(() => state.value?.autoLaunch ?? false)
const isAvailable = computed(() => servers.value.some((server) => server.status === 'connected'))
// Toys only say which server they're on once there's more than one to tell apart.
const showServers = computed(() => servers.value.length > 1 || new Set(knownDevices.value.map((device) => device.server)).size > 1)

// The server a toy is on, by name, or '' while there's only one.
const toyServerName = (server: string | undefined): string => {
  if (!showServers.value) return ''
  const found = serversById.value.get(server ?? THIS_PC)

  return found ? serverName(found) : 'Removed server'
}

export function useIntiface(): {
  enabled: ComputedRef<boolean>
  servers: ComputedRef<IntifaceServer[]>
  defaultPort: number
  addServer: (label: string, host: string, port: number) => Promise<string>
  updateServer: (id: string, label: string, host: string, port: number) => Promise<void>
  removeServer: (id: string) => Promise<void>
  // At least one server is connected.
  isAvailable: ComputedRef<boolean>
  // Every toy on a connected server now, by toyKey().
  devices: ComputedRef<Map<string, IntifaceDevice>>
  knownDevices: ComputedRef<IntifaceDevice[]>
  // Whether toys should say which server they're on (more than one server).
  showServers: ComputedRef<boolean>
  toyServerName: (server: string | undefined) => string
  installed: ComputedRef<boolean>
  launch: () => Promise<void>
  autoLaunch: ComputedRef<boolean>
  setAutoLaunch: (value: boolean) => void
  maxLaunchAttempts: number
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
    enabled,
    servers,
    defaultPort: state.value?.defaultPort ?? DEFAULT_PORT,
    addServer: (label, host, port) => api.intifaceAddServer(label, host, port),
    updateServer: (id, label, host, port) => api.intifaceUpdateServer(id, label, host, port),
    removeServer: (id) => api.intifaceRemoveServer(id),
    isAvailable,
    devices,
    knownDevices,
    showServers,
    toyServerName,
    installed,
    launch,
    autoLaunch,
    // Turning it on gives a fresh 5 tries.
    setAutoLaunch: (value) => void api.intifaceSetAutoLaunch(value),
    maxLaunchAttempts: state.value?.maxLaunchAttempts ?? MAX_LAUNCH_ATTEMPTS,
    // Only the plain fields go to Rust: a control's actuator refs also carry names.
    setActuatorIntensity: (actuators, value) =>
      void api.toysSet(
        actuators.map(({ server, deviceIndex, actuatorIndex, actuatorType }) => ({ server, deviceIndex, actuatorIndex, actuatorType })),
        value
      ),
    stopAllDevices: () => void api.toysStopAll()
  }
}
