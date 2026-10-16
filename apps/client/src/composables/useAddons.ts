import type { Addon, AddonsState } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

export type { Addon, AddonState } from '@renderer/lib/tauri-bridge'

// Add-ons run in Rust (src-tauri/src/addons): OpenShock, Intiface toys and ESP32 boards each stay
// off until you turn them on here. Everything they bring to the app (settings sections, control
// types, automation steps, rail pages) is only there while they're on.

// The Settings section of each add-on (`/settings/<section>`), shown only while it's on.
export const ADDON_SECTION: Record<string, string> = { openshock: 'openshock', intiface: 'intiface', boards: 'esp32' }

// Module-scope singletons - every caller shares Rust's state, so turning one on or off is
// reflected everywhere at once.
const state = ref<AddonsState | null>(null)

api.onAddonsUpdate((value) => (state.value = value))
void api.addonsList().then((value) => (state.value ??= value))

const addons = computed(() => state.value?.addons ?? [])
const byId = computed(() => new Map(addons.value.map((addon) => [addon.id, addon] as const)))
// Until Rust answers, nothing counts as on: an add-on's pieces appear a moment late rather than
// flash in and out.
const enabledIds = computed(() => new Set(addons.value.filter((addon) => addon.enabled).map((addon) => addon.id)))

const addonForControlType = (type: string): Addon | undefined => addons.value.find((addon) => addon.controlTypes.includes(type))
const addonForAutomationKind = (kind: string): Addon | undefined => addons.value.find((addon) => addon.automationKinds.includes(kind))
const addonForPage = (page: string): Addon | undefined => addons.value.find((addon) => addon.pages.includes(page))

export function useAddons(): {
  state: Ref<AddonsState | null>
  addons: ComputedRef<Addon[]>
  loaded: ComputedRef<boolean>
  getAddon: (id: string) => Addon | undefined
  isOn: (id: string) => boolean
  // The add-on that owns a control type, automation step kind or page, if any.
  addonForControlType: (type: string) => Addon | undefined
  addonForAutomationKind: (kind: string) => Addon | undefined
  addonForPage: (page: string) => Addon | undefined
  // The add-on that owns this control type or automation step kind, when it's off.
  offAddonForControlType: (type: string) => Addon | undefined
  offAddonForAutomationKind: (kind: string) => Addon | undefined
  setEnabled: (id: string, enabled: boolean) => Promise<void>
  finishSetup: (dismissKept: boolean) => Promise<void>
} {
  return {
    state,
    addons,
    loaded: computed(() => state.value !== null),
    getAddon: (id) => byId.value.get(id),
    isOn: (id) => enabledIds.value.has(id),
    addonForControlType,
    addonForAutomationKind,
    addonForPage,
    offAddonForControlType: (type) => {
      const addon = addonForControlType(type)

      return addon && !addon.enabled ? addon : undefined
    },
    offAddonForAutomationKind: (kind) => {
      const addon = addonForAutomationKind(kind)

      return addon && !addon.enabled ? addon : undefined
    },
    setEnabled: api.addonsSetEnabled,
    finishSetup: api.addonsFinishSetup
  }
}
