import { useAddons } from '@renderer/composables/useAddons'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useIntiface } from '@renderer/composables/useIntiface'
import type {
  AvatarHaptics,
  HapticsOutput,
  HapticsSettings,
  HapticsSource,
  HapticsState,
  HapticsZoneKind
} from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import _ from 'lodash'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref, toRaw, watch } from 'vue'

// Avatar haptics run in Rust (src-tauri/src/haptics): SPS contacts on the avatar you wear drive
// your toys. This keeps a copy of haptics.json for the Haptics page, saves edits back (a moment
// later, so a dragged slider isn't written on every step) and follows the live zones and levels.

// Module-scope singletons, shared by the Haptics page and its source editor.
const settings = ref<HapticsSettings | null>(null)
const state = ref<HapticsState>({ avatarId: null, zones: [], outputs: [], ogbHint: false })
// Which setup the page edits: the one for every avatar, or the worn avatar's own.
const scope = ref<'global' | 'avatar'>('global')

const SAVE_DELAY_MS = 250
let saveTimer: ReturnType<typeof setTimeout> | undefined
let saving = 0

// Rust announces every save, ours included. While one of ours is pending, an older copy coming
// back would undo what was just changed, so it's ignored.
api.onHapticsSettings((value) => {
  if (!saveTimer && !saving) settings.value = value
})
api.onHapticsState((value) => (state.value = value))

const { isOn } = useAddons()

watch(
  () => isOn('haptics'),
  (on) => {
    if (!on) return
    void api.hapticsSettings().then((value) => (settings.value = value)).catch(() => {})
    void api.hapticsState().then((value) => (state.value = value)).catch(() => {})
  },
  { immediate: true }
)

const save = (): void => {
  clearTimeout(saveTimer)
  saveTimer = setTimeout(() => {
    saveTimer = undefined
    if (!settings.value) return
    saving++
    void api
      .hapticsSave(_.cloneDeep(toRaw(settings.value)))
      .catch((error) => console.error('Could not save the haptics settings:', error))
      .finally(() => saving--)
  }, SAVE_DELAY_MS)
}

// "device-actuator", as the page's routes use it.
export const actuatorKey = (actuator: { deviceIndex: number; actuatorIndex: number }): string =>
  `${actuator.deviceIndex}-${actuator.actuatorIndex}`

export const ZONE_KIND_LABEL: Record<HapticsZoneKind, string> = { socket: 'Socket', plug: 'Plug', touch: 'Touch zone' }

export const SOURCE_ZONE_KIND: Record<HapticsSource['kind'], HapticsZoneKind> = {
  'sps-socket': 'socket',
  'sps-plug': 'plug',
  'sps-touch': 'touch'
}

// What each kind of source can react to, in the order the editor lists the switches.
export const SOURCE_CONTACTS: Record<HapticsSource['kind'], { key: string; label: string }[]> = {
  'sps-socket': [
    { key: 'otherPlugs', label: "Other people's plugs" },
    { key: 'myPlugs', label: 'Your own plugs' },
    { key: 'otherHands', label: "Other people's hands" },
    { key: 'ownHands', label: 'Your own hands' },
    { key: 'otherSockets', label: 'Rubbing against sockets' }
  ],
  'sps-plug': [
    { key: 'otherSockets', label: "Other people's sockets" },
    { key: 'mySockets', label: 'Your own sockets' },
    { key: 'otherHands', label: "Other people's hands" },
    { key: 'ownHands', label: 'Your own hands' },
    { key: 'otherPlugs', label: 'Rubbing against plugs' }
  ],
  'sps-touch': [
    { key: 'otherHands', label: "Other people's hands" },
    { key: 'ownHands', label: 'Your own hands' }
  ]
}

// OscGoesBrrr's defaults: other people on, yourself off.
export const newSource = (kind: HapticsSource['kind']): HapticsSource => {
  const base = {
    zones: { include: [], exclude: [] },
    ownHands: false,
    otherHands: true,
    shape: { followMovement: false, deadZone: 0, scale: 1 }
  }

  if (kind === 'sps-socket') return { ...base, kind, myPlugs: false, otherPlugs: true, otherSockets: true }
  if (kind === 'sps-plug') return { ...base, kind, mySockets: false, otherSockets: true, otherPlugs: true }

  return { ...base, kind }
}

// A zone on the worn avatar, from its parameter names (`OGB/<Orf|Pen|Touch>/<zone>/...` and
// `VFH/Zone/<Orf|Pen|Touch>/<zone>/...`), the same way Rust groups them.
const ZONE_KINDS: Record<string, HapticsZoneKind> = { Orf: 'socket', Pen: 'plug', Touch: 'touch' }

export const parseZone = (name: string): { kind: HapticsZoneKind; name: string } | null => {
  const parts = name.split('/')
  const [kind, zone] = parts[0] === 'OGB' ? [parts[1], parts[2]] : parts[0] === 'VFH' && parts[1] === 'Zone' ? [parts[2], parts[3]] : []

  if (!kind || !zone || !ZONE_KINDS[kind]) return null

  return { kind: ZONE_KINDS[kind], name: zone }
}

export interface HapticsToy {
  key: string
  deviceIndex: number
  actuatorIndex: number
  actuatorType: string
  deviceName: string
  actuatorDescription: string
  // Whether Intiface has it now.
  connected: boolean
  // Its device has more than one motor, so the name says which.
  oneOfMany: boolean
}

export function useAvatarHaptics(): {
  settings: Ref<HapticsSettings | null>
  state: Ref<HapticsState>
  scope: Ref<'global' | 'avatar'>
  avatarId: ComputedRef<string | null>
  avatarName: ComputedRef<string>
  avatar: ComputedRef<AvatarHaptics | undefined>
  toys: ComputedRef<HapticsToy[]>
  zones: ComputedRef<{ kind: HapticsZoneKind; name: string; level: number }[]>
  toyName: (toy: HapticsToy) => string
  globalOutput: (key: string) => HapticsOutput | undefined
  avatarOutput: (key: string) => HapticsOutput | undefined
  // What applies on the worn avatar: its own, else the global one.
  activeOutput: (key: string) => HapticsOutput | undefined
  // The output the page edits in the current scope, if it has one.
  editedOutput: (key: string) => HapticsOutput | undefined
  level: (key: string) => { level: number; sources: number[] }
  change: (mutate: (settings: HapticsSettings) => void) => void
  // The output to edit in the current scope, made when missing. On the worn avatar, it starts as
  // a copy of the global one.
  outputToEdit: (settings: HapticsSettings, toy: HapticsToy) => HapticsOutput
  removeOverride: (key: string) => void
} {
  const { avatarDetails } = useAvatarDetails()
  const { devices } = useIntiface()

  const avatarId = computed(() => avatarDetails.value?.id ?? null)
  const avatarName = computed(() => avatarDetails.value?.name ?? 'This avatar')
  const avatar = computed(() => (avatarId.value ? settings.value?.avatars[avatarId.value] : undefined))

  const findOutput = (outputs: HapticsOutput[] | undefined, key: string): HapticsOutput | undefined =>
    outputs?.find((output) => actuatorKey(output.actuator) === key)

  const globalOutput = (key: string): HapticsOutput | undefined => findOutput(settings.value?.outputs, key)
  const avatarOutput = (key: string): HapticsOutput | undefined => findOutput(avatar.value?.outputs, key)
  const activeOutput = (key: string): HapticsOutput | undefined => avatarOutput(key) ?? globalOutput(key)
  const editedOutput = (key: string): HapticsOutput | undefined => (scope.value === 'avatar' ? avatarOutput(key) : globalOutput(key))

  // Every motor Intiface has now, then the ones set up for a toy that isn't connected.
  const toys = computed<HapticsToy[]>(() => {
    const list: HapticsToy[] = []

    for (const device of devices.value.values()) {
      for (const actuator of device.actuators) {
        list.push({
          key: actuatorKey({ deviceIndex: device.index, actuatorIndex: actuator.index }),
          deviceIndex: device.index,
          actuatorIndex: actuator.index,
          actuatorType: actuator.actuatorType,
          deviceName: device.name,
          actuatorDescription: actuator.description,
          connected: true,
          oneOfMany: device.actuators.length > 1
        })
      }
    }

    const saved = [...(settings.value?.outputs ?? []), ...(avatar.value?.outputs ?? [])]

    for (const { actuator } of saved) {
      const key = actuatorKey(actuator)

      if (list.some((toy) => toy.key === key)) continue

      list.push({ key, ...actuator, connected: false, oneOfMany: false })
    }

    return list
  })

  const toyName = (toy: HapticsToy): string =>
    toy.oneOfMany ? `${toy.deviceName} · ${toy.actuatorDescription || `motor ${toy.actuatorIndex + 1}`}` : toy.deviceName

  // The worn avatar's zones: every one its parameters declare, and any that sent a value.
  const zones = computed(() => {
    const byKey = new Map<string, { kind: HapticsZoneKind; name: string; level: number }>()

    for (const parameter of avatarDetails.value?.parameters ?? []) {
      const zone = parseZone(parameter.name)

      if (zone) byKey.set(`${zone.kind}/${zone.name}`, { ...zone, level: 0 })
    }

    for (const zone of state.value.zones) {
      const key = `${zone.kind}/${zone.name}`

      byKey.set(key, { kind: zone.kind, name: zone.name, level: Math.max(zone.level, byKey.get(key)?.level ?? 0) })
    }

    return [...byKey.values()].sort((a, b) => a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name))
  })

  const level = (key: string): { level: number; sources: number[] } => {
    const found = state.value.outputs.find((output) => actuatorKey(output) === key)

    return { level: found?.level ?? 0, sources: found?.sources ?? [] }
  }

  const change = (mutate: (settings: HapticsSettings) => void): void => {
    if (!settings.value) return

    mutate(settings.value)
    save()
  }

  const outputToEdit = (target: HapticsSettings, toy: HapticsToy): HapticsOutput => {
    const actuator = _.pick(toy, ['deviceIndex', 'actuatorIndex', 'actuatorType', 'deviceName', 'actuatorDescription'])
    const fresh = (): HapticsOutput => ({ actuator, sources: [], maxStrength: 1 })

    if (scope.value === 'avatar' && avatarId.value) {
      const own = (target.avatars[avatarId.value] ??= { name: avatarName.value, enabled: true, outputs: [] })
      own.name = avatarName.value
      let output = findOutput(own.outputs, toy.key)

      if (!output) {
        const global = findOutput(target.outputs, toy.key)
        output = global ? { ..._.cloneDeep(toRaw(global)), actuator } : fresh()
        own.outputs.push(output)
      }

      return output
    }

    let output = findOutput(target.outputs, toy.key)

    if (!output) {
      output = fresh()
      target.outputs.push(output)
    }

    return output
  }

  const removeOverride = (key: string): void =>
    change((target) => {
      const own = avatarId.value ? target.avatars[avatarId.value] : undefined

      if (own) own.outputs = own.outputs.filter((output) => actuatorKey(output.actuator) !== key)
    })

  return {
    settings,
    state,
    scope,
    avatarId,
    avatarName,
    avatar,
    toys,
    zones,
    toyName,
    globalOutput,
    avatarOutput,
    activeOutput,
    editedOutput,
    level,
    change,
    outputToEdit,
    removeOverride
  }
}
