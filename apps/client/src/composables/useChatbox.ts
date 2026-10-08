import { api } from '@renderer/lib/tauri-bridge'
import type { CommandValue, ControlType, ControlTypes } from '@toolkitosc/shared-ui'
import { describeCommand } from '@toolkitosc/shared-ui'
import type { Ref } from 'vue'
import { ref } from 'vue'

const ENABLED_STORAGE_KEY = 'chatbox_enabled'
const KINDS_STORAGE_KEY = 'chatbox_kinds'

const CHATBOX_ADDRESS = '/chatbox/input'
// VRChat's chatbox limit.
const MAX_LENGTH = 144
// VRChat drops chatbox messages sent faster than about one every 1.5s; actions arriving in between
// are merged into the next message.
const MIN_INTERVAL_MS = 3000

export type ChatboxKind = 'toggles' | 'options' | 'sliders' | 'toys' | 'patterns' | 'shocks' | 'presets'

export const CHATBOX_KINDS: { value: ChatboxKind; label: string; types: ControlTypes[] }[] = [
  { value: 'toggles', label: 'Toggles', types: ['boolean', 'boolean-group'] },
  { value: 'options', label: 'Options', types: ['enum', 'step-enum', 'boolean-enum'] },
  { value: 'sliders', label: 'Sliders', types: ['slider'] },
  { value: 'toys', label: 'Toys', types: ['intiface-toy'] },
  { value: 'patterns', label: 'Toy patterns', types: ['intiface-pattern'] },
  { value: 'shocks', label: 'Shocks and vibrations', types: ['open-shock-shocker'] },
  { value: 'presets', label: 'Presets', types: ['preset'] }
]

const readKinds = (): ChatboxKind[] => {
  try {
    const stored = JSON.parse(localStorage.getItem(KINDS_STORAGE_KEY) ?? 'null')

    if (Array.isArray(stored)) return stored.filter((kind) => CHATBOX_KINDS.some((k) => k.value === kind))
  } catch {
    // Falls through to the default.
  }

  return CHATBOX_KINDS.map((kind) => kind.value)
}

// Off until the streamer turns it on: it's visible to everyone in the instance.
const enabled = ref(localStorage.getItem(ENABLED_STORAGE_KEY) === 'true')
const kinds = ref<ChatboxKind[]>(readKinds())

export interface ChatboxAction {
  // Who did it, or null to leave the name out (the host hides viewers from each other).
  viewerKey: string
  viewerName: string | null
  controlId: string
  controlName: string
  control: ControlType | undefined
  type: ControlTypes
  value: CommandValue | undefined
}

// Waiting for the next slot, per viewer then per control: a dragged slider keeps only its latest
// value, and one viewer's actions read as one sentence.
const pending = new Map<string, { name: string | null; phrases: Map<string, string> }>()
// A pause or resume notice, sent on its own ahead of any actions.
let pendingNotice: string | null = null
let lastSentAt = 0
let flushTimer: ReturnType<typeof setTimeout> | null = null

// "set to Blush" -> "set Face to Blush", "turned on" -> "turned on Ears".
const phrase = (action: ChatboxAction): string => {
  const verb = describeCommand(action.control, action.type, action.value)
  const name = action.controlName

  if (verb.startsWith('set to ')) return `set ${name} to ${verb.slice('set to '.length)}`
  if (verb === 'applied it') return `applied ${name}`
  if (verb === 'used it') return `used ${name}`

  if (verb.startsWith('sent a ')) {
    const [what, details] = verb.split(' · ')

    return details ? `${what} with ${name} (${details})` : `${what} with ${name}`
  }

  return `${verb} ${name}`
}

const truncate = (text: string): string => (text.length > MAX_LENGTH ? `${text.slice(0, MAX_LENGTH - 1)}…` : text)

// As many viewers' sentences as fit, then "+N more" for the rest.
const buildMessage = (): string => {
  const parts = Array.from(pending.values()).map(
    ({ name, phrases }) => `${name ?? 'Someone'} ${Array.from(phrases.values()).join(', ')}`
  )

  let message = truncate(parts[0] ?? '')

  for (let i = 1; i < parts.length; i++) {
    const rest = parts.length - i - 1
    const next = `${message} · ${parts[i]}`
    const suffix = rest ? ` · +${rest} more` : ''

    if ((next + suffix).length > MAX_LENGTH) {
      message += ` · +${parts.length - i} more`
      break
    }

    message = next
  }

  return truncate(message)
}

const send = (text: string): void => {
  // Send right away (not into the keyboard), without the notification sound.
  api.sendOscMessage({ address: CHATBOX_ADDRESS, args: [text, true, false] })
  lastSentAt = Date.now()
}

const flush = (): void => {
  flushTimer = null

  if (pendingNotice) {
    send(pendingNotice)
    pendingNotice = null
  } else if (pending.size) {
    send(buildMessage())
    pending.clear()
  }

  if (pending.size || pendingNotice) schedule()
}

const schedule = (): void => {
  if (flushTimer) return

  flushTimer = setTimeout(flush, Math.max(0, lastSentAt + MIN_INTERVAL_MS - Date.now()))
}

export function useChatbox(): {
  enabled: Ref<boolean>
  kinds: Ref<ChatboxKind[]>
  setEnabled: (value: boolean) => void
  setKind: (kind: ChatboxKind, on: boolean) => void
  // Queues a viewer's action, if the chatbox is on and its kind is picked.
  announceAction: (action: ChatboxAction) => void
  // Pause and resume notices replace any actions still waiting.
  announceNotice: (text: string) => void
  sendTest: () => void
} {
  const setEnabled = (value: boolean): void => {
    enabled.value = value
    localStorage.setItem(ENABLED_STORAGE_KEY, String(value))

    if (!value) {
      pending.clear()
      pendingNotice = null
    }
  }

  const setKind = (kind: ChatboxKind, on: boolean): void => {
    kinds.value = on ? [...new Set([...kinds.value, kind])] : kinds.value.filter((k) => k !== kind)
    localStorage.setItem(KINDS_STORAGE_KEY, JSON.stringify(kinds.value))
  }

  const announceAction = (action: ChatboxAction): void => {
    if (!enabled.value) return

    const kind = CHATBOX_KINDS.find((k) => k.types.includes(action.type))

    if (!kind || !kinds.value.includes(kind.value)) return

    // Hidden names share one entry, so their actions merge into one "Someone …".
    const key = action.viewerName === null ? '' : action.viewerKey
    const entry = pending.get(key) ?? { name: action.viewerName, phrases: new Map<string, string>() }

    entry.phrases.set(action.controlId, phrase(action))
    pending.set(key, entry)
    schedule()
  }

  const announceNotice = (text: string): void => {
    if (!enabled.value) return

    pending.clear()
    pendingNotice = truncate(text)
    schedule()
  }

  const sendTest = (): void => {
    pendingNotice = 'ToolKitOSC: viewer actions will show up here'
    schedule()
  }

  return { enabled, kinds, setEnabled, setKind, announceAction, announceNotice, sendTest }
}
