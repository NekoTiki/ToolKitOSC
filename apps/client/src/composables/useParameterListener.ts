import { useControls } from '@renderer/composables/useControls'
import { api } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { Ref } from 'vue'
import { computed, ref } from 'vue'

// "Listen" in the automation editor: while it's on, collects the avatar parameters that change,
// so you can touch a contact or grab a PhysBone and see which parameter it is. VRChat's avatar
// files don't say which parameters are contacts, so this is the reliable way to find them.
// Parameters your controls drive are left out: they change because of the app, not you.

export interface HeardParameter {
  address: string
  value: unknown
  at: number
  count: number
}

const LISTEN_MS = 20_000

const listening = ref(false)
const endsAt = ref(0)
const heard = ref<Map<string, HeardParameter>>(new Map())
let timer: ReturnType<typeof setTimeout> | undefined
// Filled by the first caller, read by the module-level listener.
let ownAddresses: () => Set<string> = () => new Set()

// Once, at module scope: a listener per Listen press would never be removed.
api.onOscMessage((msg) => {
  if (!listening.value || !msg.address.startsWith('/avatar/parameters/')) return
  if (isNoisyAddress(msg.address.replace('/avatar/parameters/', '')) || ownAddresses().has(msg.address)) return

  const next = new Map(heard.value)
  const before = next.get(msg.address)
  next.set(msg.address, { address: msg.address, value: msg.args[0], at: Date.now(), count: (before?.count ?? 0) + 1 })
  heard.value = next
})

const stop = (): void => {
  clearTimeout(timer)
  listening.value = false
}

export function useParameterListener(): {
  listening: Ref<boolean>
  endsAt: Ref<number>
  // Newest first.
  heard: Ref<HeardParameter[]>
  start: () => void
  stop: () => void
} {
  const { controls } = useControls()
  ownAddresses = () =>
    new Set(
      controls.value.flatMap((group) =>
        group.controls.flatMap((control) => {
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
        })
      )
    )

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
