import { useControls } from '@renderer/composables/useControls'
import { api } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { Ref } from 'vue'
import { computed, ref } from 'vue'

// "Listen" in the automation editor: while it's on, collects the avatar parameters that change,
// so you can touch a contact or grab a PhysBone and see which parameters it sets - often several
// at once. VRChat's avatar files don't say which parameters are contacts, so this is the reliable
// way to find them.
//
// Noisy parameters are left out, the same ones the pickers hide (see addressFilter.ts), and so
// are VRChat's own built-ins, which change all the time as you move. Everything else that
// changes is listed, parameters your controls drive included: those name the control.

export interface HeardParameter {
  address: string
  value: unknown
  at: number
  count: number
  // The control that drives it, when one does: it may have changed because of the app.
  control?: string
}

const LISTEN_MS = 20_000

// https://creators.vrchat.com/avatars/animator-parameters/#parameters
const VRCHAT_BUILT_INS = new Set([
  'IsLocal',
  'PreviewMode',
  'Viseme',
  'Voice',
  'GestureLeft',
  'GestureRight',
  'GestureLeftWeight',
  'GestureRightWeight',
  'AngularY',
  'VelocityX',
  'VelocityY',
  'VelocityZ',
  'VelocityMagnitude',
  'Upright',
  'Grounded',
  'Seated',
  'AFK',
  'TrackingType',
  'VRMode',
  'MuteSelf',
  'InStation',
  'Earmuffs',
  'IsOnFriendsList',
  'AvatarVersion',
  'IsAnimatorEnabled',
  'ScaleModified',
  'ScaleFactor',
  'ScaleFactorInverse',
  'EyeHeightAsMeters',
  'EyeHeightAsPercent'
])

const listening = ref(false)
const endsAt = ref(0)
const heard = ref<Map<string, HeardParameter>>(new Map())
let timer: ReturnType<typeof setTimeout> | undefined
// Filled by the first caller, read by the module-level listener.
let controlNames: () => Map<string, string> = () => new Map()

// Once, at module scope: a listener per Listen press would never be removed.
api.onOscMessage((msg) => {
  if (!listening.value || !msg.address.startsWith('/avatar/parameters/')) return
  const name = msg.address.replace('/avatar/parameters/', '')
  // The app's own parameters (TKOSC/Panic...) aren't something you touch.
  if (isNoisyAddress(name) || VRCHAT_BUILT_INS.has(name) || name.startsWith('TKOSC/')) return

  const next = new Map(heard.value)
  const before = next.get(msg.address)
  next.set(msg.address, {
    address: msg.address,
    value: msg.args[0],
    at: Date.now(),
    count: (before?.count ?? 0) + 1,
    control: controlNames().get(msg.address)
  })
  heard.value = next
})

const stop = (): void => {
  clearTimeout(timer)
  listening.value = false
}

export function useParameterListener(): {
  listening: Ref<boolean>
  endsAt: Ref<number>
  // Newest first. Kept after listening stops, until the next Listen.
  heard: Ref<HeardParameter[]>
  start: () => void
  stop: () => void
} {
  const { controls } = useControls()
  controlNames = () => {
    const names = new Map<string, string>()
    for (const control of controls.value.flatMap((group) => group.controls)) {
      const addresses = (() => {
        switch (control.type) {
          case 'boolean':
          case 'enum':
          case 'step-enum':
          case 'slider':
            return [control.inputAddress]
          case 'boolean-group':
            return control.inputs.map((input) => input.inputAddress)
          case 'boolean-enum':
            return control.inputs.flatMap((input) => [...input.inputAddress.true, ...input.inputAddress.false])
          default:
            return []
        }
      })()
      addresses.forEach((address) => names.set(address, control.name))
    }
    return names
  }

  const start = (): void => {
    heard.value = new Map()
    listening.value = true
    endsAt.value = Date.now() + LISTEN_MS
    clearTimeout(timer)
    timer = setTimeout(stop, LISTEN_MS)
  }

  return {
    listening,
    endsAt,
    heard: computed(() => [...heard.value.values()].sort((a, b) => b.at - a.at)),
    start,
    stop
  }
}
