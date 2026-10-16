import type { OpenShockFire, OpenShockHub, OpenShockState } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

export type { OpenShockHub, OpenShockShocker } from '@renderer/lib/tauri-bridge'

// Where the webview kept these before Rust took over (see src-tauri/src/openshock). Handed over
// once, then removed.
const LEGACY_TOKEN_KEY = 'openShock_token'
const LEGACY_ENABLED_KEY = 'openShock_enabled'

// 'disabled': the user has switched OpenShock off in Settings - independent of whether a token is
// saved, same idea as Intiface's own enable toggle (see useIntiface.ts). Defaults to enabled.
// 'unconfigured': enabled, but no token saved yet (or it was just cleared).
// 'checking': a validation request against the OpenShock API is in flight.
// 'valid'/'invalid': the last validation request succeeded/failed.
// Fail-closed: any state other than 'valid' means OpenShock controls are treated as unavailable,
// and Rust refuses to send anything either.
export type OpenShockStatus = 'disabled' | 'unconfigured' | 'checking' | 'valid' | 'invalid'

// Module-scope singletons - every useOpenShock() caller shares Rust's state, so a change made in
// Settings is instantly reflected everywhere (ControlModal's type list, useControls' `unavailable`
// flag, the host's command enforcement). The token itself never comes back from Rust.
const state = ref<OpenShockState | null>(null)

api.onOpenShockState((value) => (state.value = value))
void api.openShockState().then((value) => (state.value ??= value))

const legacyToken = localStorage.getItem(LEGACY_TOKEN_KEY)
const legacyEnabled = localStorage.getItem(LEGACY_ENABLED_KEY)

if (legacyToken !== null || legacyEnabled !== null) {
  void api
    .openShockMigrate(legacyToken, legacyEnabled === null ? null : legacyEnabled !== 'false')
    .then(() => {
      localStorage.removeItem(LEGACY_TOKEN_KEY)
      localStorage.removeItem(LEGACY_ENABLED_KEY)
    })
    .catch((error) => console.error('Could not move the OpenShock settings to the app:', error))
}

const enabled = computed(() => state.value?.enabled ?? true)
const hasToken = computed(() => state.value?.hasToken ?? false)
const status = computed<OpenShockStatus>(() => (enabled.value ? (state.value?.status ?? 'unconfigured') : 'disabled'))
const isAvailable = computed(() => status.value === 'valid')

// id -> name, refreshed with every token check and shocker list change. Lets useControls'
// handleCommand resolve a shocker's name synchronously and write it straight into the command log
// at the moment the command fires, rather than the log viewer resolving ids on read.
const shockerNames = computed(
  () => new Map((state.value?.hubs ?? []).flatMap((hub) => hub.shockers.map((shocker) => [shocker.id, shocker.name] as const)))
)

// Shocker id -> the hub (device) it's paired to.
const shockerHub = computed(
  () => new Map((state.value?.hubs ?? []).flatMap((hub) => hub.shockers.map((shocker) => [shocker.id, hub.id] as const)))
)

// Whether every hub these shockers are paired to is offline. Only known while Rust is connected to
// OpenShock's live hub; until then nothing counts as offline.
const hubsOffline = (shockerIds: string[]): boolean => {
  if (!state.value?.live) return false

  const online = new Set(state.value.online)
  const hubs = shockerIds.map((id) => shockerHub.value.get(id)).filter((id): id is string => !!id)

  return hubs.length > 0 && hubs.every((id) => !online.has(id))
}

export function useOpenShock(): {
  getShockers: () => Promise<OpenShockHub[]>
  setToken: (token: string) => Promise<void>
  hasToken: ComputedRef<boolean>
  enabled: ComputedRef<boolean>
  setEnabled: (value: boolean) => void
  status: ComputedRef<OpenShockStatus>
  isAvailable: ComputedRef<boolean>
  testConnection: () => void
  shockerNames: ComputedRef<Map<string, string>>
  hubsOffline: (shockerIds: string[]) => boolean
  fire: (fire: OpenShockFire) => Promise<void>
  stopAll: () => Promise<void>
  state: Ref<OpenShockState | null>
} {
  return {
    getShockers: api.openShockShockers,
    // An empty token removes it. Rust checks it against OpenShock right away.
    setToken: api.openShockSetToken,
    hasToken,
    enabled,
    // OpenShock is an add-on: the registry turns it on and off (see src-tauri/src/addons).
    setEnabled: (value) => void api.addonsSetEnabled('openshock', value),
    status,
    isAvailable,
    testConnection: () => void api.openShockRecheck(),
    shockerNames,
    hubsOffline,
    fire: api.openShockFire,
    stopAll: api.openShockStopAll,
    state
  }
}
