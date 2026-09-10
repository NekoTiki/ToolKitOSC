import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

const DEFAULT_INTIFACE_URL = 'ws://localhost:12345'
const URL_STORAGE_KEY = 'intiface_url'
const ENABLED_STORAGE_KEY = 'intiface_enabled'
const CLIENT_NAME = 'VRC OSC Toolkit'
const RECONNECT_DELAY = 3000

// Buttplug protocol (raw JSON messages exchanged with an Intiface Engine over WebSocket) - only
// the subset needed to enumerate vibration-capable actuators and send ScalarCmd. No client
// library is installed anywhere in this repo (same call as OpenShock - see useOpenShock.ts):
// hand-rolled request/response shapes, no SDK. Spec: https://buttplug-spec.docs.buttplug.io
// Note: no `Index` field here - unlike the Scalars entries in an actual ScalarCmd command, a
// device's attribute descriptors don't carry one. An actuator's index is implicit: its position
// in this array (see toDevice() below, which is why that has to capture it before filtering).
interface ButtplugDeviceMessageAttributes {
  FeatureDescriptor?: string
  ActuatorType?: string
  StepCount?: number
}

interface ButtplugDeviceInfo {
  DeviceName: string
  DeviceIndex: number
  DeviceMessages: {
    ScalarCmd?: ButtplugDeviceMessageAttributes[]
    [key: string]: unknown
  }
}

type ButtplugServerMessage =
  | { ServerInfo: { Id: number; ServerName: string; MessageVersion: number; MaxPingTime: number } }
  | { DeviceList: { Id: number; Devices: ButtplugDeviceInfo[] } }
  | { DeviceAdded: ButtplugDeviceInfo & { Id: number } }
  | { DeviceRemoved: { Id: number; DeviceIndex: number } }
  | { Ok: { Id: number } }
  | { Error: { Id: number; ErrorMessage: string; ErrorCode: number } }

export interface IntifaceActuator {
  index: number
  description: string
}

export interface IntifaceDevice {
  index: number
  name: string
  actuators: IntifaceActuator[]
}

// 'disabled': the user hasn't turned the integration on in Settings. This is the default and,
// unlike OpenShock (gated on a token nobody has until they paste one), it's the only thing that
// stops every install from silently retrying a WebSocket connection to localhost forever.
// 'connecting': a connection attempt is in flight, including auto-reconnect retries.
// 'connected': the connection is open and the server handshake (RequestServerInfo) succeeded.
// 'error': the last attempt failed; a retry is still scheduled while `enabled` stays true.
// Fail-closed like OpenShockStatus: only 'connected' counts as available.
export type IntifaceStatus = 'disabled' | 'connecting' | 'connected' | 'error'

// Module-scope singletons - every useIntiface() caller shares the same connection/device list, so
// a change made in Settings (or a toy being plugged in) is instantly reflected everywhere
// (ControlModal's type list and toy picker, useControls' `unavailable` flag), same pattern as
// useOpenShock.ts's token/status.
const url = ref<string>(localStorage.getItem(URL_STORAGE_KEY) || '')
const enabled = ref<boolean>(localStorage.getItem(ENABLED_STORAGE_KEY) === 'true')
const status = ref<IntifaceStatus>('disabled')
const devices = ref<Map<number, IntifaceDevice>>(new Map())

let socket: WebSocket | null = null
let messageId = 1
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

const nextId = (): number => messageId++

// Buttplug lists every actuator a device has (vibrate, oscillate, rotate, ...) under ScalarCmd -
// only the vibration-capable ones are relevant here, per the task's "controls the intensity of
// all the vibrators". The index has to be captured from each entry's position in the *unfiltered*
// array (a ScalarCmd targeting this device addresses actuators by that position, regardless of
// type) - filtering first would renumber e.g. a device with [Vibrate, Oscillate, Vibrate] down to
// [0, 1], silently aiming commands at the wrong motor.
const toDevice = (info: ButtplugDeviceInfo): IntifaceDevice => ({
  index: info.DeviceIndex,
  name: info.DeviceName,
  actuators: (info.DeviceMessages.ScalarCmd ?? [])
    .map((actuator, index) => ({ actuator, index }))
    .filter(({ actuator }) => (actuator.ActuatorType ?? 'Vibrate') === 'Vibrate')
    .map(({ actuator, index }) => ({
      index,
      description: actuator.FeatureDescriptor || `Vibrator ${index + 1}`
    }))
})

const send = (messages: unknown[]): void => {
  if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify(messages))
}

const clearReconnectTimer = (): void => {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
}

const handleServerMessage = (raw: string): void => {
  let messages: ButtplugServerMessage[]

  try {
    messages = JSON.parse(raw)
  } catch {
    return
  }

  messages.forEach((message) => {
    if ('ServerInfo' in message) {
      status.value = 'connected'
      send([{ RequestDeviceList: { Id: nextId() } }])
    } else if ('DeviceList' in message) {
      const map = new Map<number, IntifaceDevice>()

      message.DeviceList.Devices.forEach((info) => {
        const device = toDevice(info)

        if (device.actuators.length) map.set(device.index, device)
      })

      devices.value = map
    } else if ('DeviceAdded' in message) {
      const device = toDevice(message.DeviceAdded)

      if (device.actuators.length) {
        const next = new Map(devices.value)

        next.set(device.index, device)
        devices.value = next
      }
    } else if ('DeviceRemoved' in message) {
      const next = new Map(devices.value)

      next.delete(message.DeviceRemoved.DeviceIndex)
      devices.value = next
    } else if ('Error' in message) {
      console.error('Intiface error:', message.Error.ErrorMessage)
    }
  })
}

const scheduleReconnect = (): void => {
  clearReconnectTimer()

  reconnectTimer = setTimeout(connect, RECONNECT_DELAY)
}

const connect = (): void => {
  if (!enabled.value) return
  if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
    return
  }

  clearReconnectTimer()
  status.value = 'connecting'

  const ws = new WebSocket(url.value || DEFAULT_INTIFACE_URL)

  socket = ws

  ws.onopen = () => {
    send([{ RequestServerInfo: { Id: nextId(), ClientName: CLIENT_NAME, MessageVersion: 3 } }])
  }
  ws.onmessage = (ev) => handleServerMessage(ev.data)
  ws.onerror = () => {
    status.value = 'error'
  }
  ws.onclose = () => {
    devices.value = new Map()
    socket = null

    if (enabled.value) {
      status.value = 'error'
      scheduleReconnect()
    }
  }
}

const disconnect = (): void => {
  clearReconnectTimer()
  devices.value = new Map()
  status.value = 'disabled'

  socket?.close()
  socket = null
}

export function useIntiface(): {
  url: Ref<string>
  defaultUrl: string
  isCustomUrl: ComputedRef<boolean>
  setUrl: (value?: string) => void
  enabled: Ref<boolean>
  setEnabled: (value: boolean) => void
  status: Ref<IntifaceStatus>
  isAvailable: ComputedRef<boolean>
  devices: Ref<Map<number, IntifaceDevice>>
  setVibratorIntensity: (
    vibrators: { deviceIndex: number; actuatorIndex: number }[],
    value: number
  ) => void
} {
  const isCustomUrl = computed(() => !!url.value)

  const setUrl = (value?: string): void => {
    const trimmed = value?.trim()

    if (trimmed && trimmed !== DEFAULT_INTIFACE_URL) {
      url.value = trimmed
      localStorage.setItem(URL_STORAGE_KEY, trimmed)
    } else {
      url.value = ''
      localStorage.removeItem(URL_STORAGE_KEY)
    }

    // Reconnect against the new URL immediately, same as OpenShock re-validating on setToken().
    if (enabled.value) {
      disconnect()
      connect()
    }
  }

  const setEnabled = (value: boolean): void => {
    enabled.value = value
    localStorage.setItem(ENABLED_STORAGE_KEY, String(value))

    if (value) connect()
    else disconnect()
  }

  const isAvailable = computed(() => status.value === 'connected')

  // Batches per device (a single ScalarCmd targets one DeviceIndex but can carry several
  // Scalars), so a control spanning multiple toys sends one message per toy, not per actuator.
  const setVibratorIntensity = (
    vibrators: { deviceIndex: number; actuatorIndex: number }[],
    value: number
  ): void => {
    if (status.value !== 'connected') return

    const byDevice = new Map<number, number[]>()

    vibrators.forEach((vibrator) => {
      if (!byDevice.has(vibrator.deviceIndex)) byDevice.set(vibrator.deviceIndex, [])

      byDevice.get(vibrator.deviceIndex)!.push(vibrator.actuatorIndex)
    })

    byDevice.forEach((actuatorIndexes, deviceIndex) => {
      send([
        {
          ScalarCmd: {
            Id: nextId(),
            DeviceIndex: deviceIndex,
            Scalars: actuatorIndexes.map((index) => ({
              Index: index,
              Scalar: value,
              ActuatorType: 'Vibrate'
            }))
          }
        }
      ])
    })
  }

  return {
    url,
    defaultUrl: DEFAULT_INTIFACE_URL,
    isCustomUrl,
    setUrl,
    enabled,
    setEnabled,
    status,
    isAvailable,
    devices,
    setVibratorIntensity
  }
}

// Reconnect automatically at module load if the user previously enabled Intiface (connect() is a
// no-op otherwise), so `isAvailable` reflects reality before Settings is ever opened - same idea
// as useOpenShock's auto-validate-on-load.
connect()
