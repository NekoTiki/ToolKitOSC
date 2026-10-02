import type { OSCArg } from '@renderer/env'
import { api } from '@renderer/lib/tauri-bridge'
import type { ShallowRef } from 'vue'
import { ref, shallowRef } from 'vue'

// Every OSC parameter value received from VRChat, newest last, for the Parameters > Change log
// page. Recording starts at app launch (see main.ts), so the log already has history the first
// time the page is opened.
//
// Face tracking alone sends dozens of values a second, so the buffer is a plain array outside
// Vue's reactivity, and the page is told about new entries at most a few times a second through
// `version`, instead of on every message.
export interface ParameterLogEntry {
  id: number
  at: number
  address: string
  value: OSCArg | undefined
  previous: OSCArg | undefined
}

export const PARAMETER_LOG_LIMIT = 10_000
// Trimmed in chunks rather than one shift() per message, which would copy the whole array each time.
const TRIM_CHUNK = 1_000
const FLUSH_MS = 250

const buffer: ParameterLogEntry[] = []
const lastValues = new Map<string, OSCArg | undefined>()
const version = shallowRef(0)
const paused = ref(false)

let nextId = 0
let flushTimer: ReturnType<typeof setTimeout> | undefined
let started = false

const scheduleFlush = (): void => {
  flushTimer ??= setTimeout(() => {
    flushTimer = undefined
    version.value++
  }, FLUSH_MS)
}

export function startParameterLog(): void {
  if (started) return
  started = true

  // Only live changes - the bulk dump VRChat sends on avatar load isn't a change.
  api.onOscMessage((msg) => {
    const value = msg.args[0]
    const previous = lastValues.get(msg.address)

    lastValues.set(msg.address, value)

    if (paused.value) return

    buffer.push({ id: nextId++, at: Date.now(), address: msg.address, value, previous })

    if (buffer.length > PARAMETER_LOG_LIMIT + TRIM_CHUNK) buffer.splice(0, buffer.length - PARAMETER_LOG_LIMIT)

    scheduleFlush()
  })
}

export function useParameterLog(): {
  entries: () => readonly ParameterLogEntry[]
  version: ShallowRef<number>
  paused: typeof paused
  clear: () => void
} {
  const clear = (): void => {
    buffer.length = 0
    version.value++
  }

  return { entries: () => buffer, version, paused, clear }
}
