import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

const DEFAULT_INTIFACE_URL = 'ws://localhost:12345'
const URL_STORAGE_KEY = 'intiface_url'
const ENABLED_STORAGE_KEY = 'intiface_enabled'
const CLIENT_NAME = 'VRC OSC Toolkit'
const RECONNECT_DELAY = 3000

// Buttplug protocol (raw JSON messages exchanged with an Intiface Engine over WebSocket) - only
// the subset needed to enumerate ScalarCmd-capable actuators and send ScalarCmd. No client
// library is installed anywhere in this repo (same call as OpenShock - see useOpenShock.ts):
// hand-rolled request/response shapes, no SDK. Spec: https://buttplug-spec.docs.buttplug.io
//
// Buttplug v3 unified vibration/rotation/oscillation/etc. into a single ScalarCmd message - the
// only thing that varies per actuator is its `ActuatorType` (known values include "Vibrate",
// "Rotate", "Oscillate", "Constrict", "Inflate", "Position"). Kept as a plain string rather than a
// union here since new actuator types can show up without this app needing a code change to at
// least send them through - only the label falls back to the raw string it doesn't recognize.
// Devices whose rotation/linear motion is only exposed via the older, separate RotateCmd/LinearCmd
// messages (not ScalarCmd) aren't covered - those need per-message direction/duration handling
// this composable doesn't implement.
// Note: no `Index` field here - unlike the Scalars entries in an actual ScalarCmd command, a
// device's attribute descriptors don't carry one. An actuator's index is implicit: its position
// in this array (see toDevice() below, which is why that has to capture it before filtering).
interface ButtplugDeviceMessageAttributes {
  FeatureDescriptor?: string
  ActuatorType?: string
  StepCount?: number
}

// Same "no Index field, position is the index" rule as ButtplugDeviceMessageAttributes above -
// SensorReadCmd's own attribute descriptors don't carry one either.
interface ButtplugSensorAttributes {
  FeatureDescriptor?: string
  SensorType?: string
  SensorRange?: number[][]
}

interface ButtplugDeviceInfo {
  DeviceName: string
  DeviceIndex: number
  DeviceMessages: {
    ScalarCmd?: ButtplugDeviceMessageAttributes[]
    SensorReadCmd?: ButtplugSensorAttributes[]
    [key: string]: unknown
  }
}

type ButtplugServerMessage =
  | { ServerInfo: { Id: number; ServerName: string; MessageVersion: number; MaxPingTime: number } }
  | { DeviceList: { Id: number; Devices: ButtplugDeviceInfo[] } }
  | { DeviceAdded: ButtplugDeviceInfo & { Id: number } }
  | { DeviceRemoved: { Id: number; DeviceIndex: number } }
  | {
      SensorReading: {
        Id: number
        DeviceIndex: number
        SensorIndex: number
        SensorType: string
        Data: number[]
      }
    }
  | { Ok: { Id: number } }
  | { Error: { Id: number; ErrorMessage: string; ErrorCode: number } }

export interface IntifaceActuator {
  index: number
  description: string
  actuatorType: string
}

export interface IntifaceDevice {
  index: number
  name: string
  actuators: IntifaceActuator[]
  // Position of this device's "Battery" entry in its own SensorReadCmd attribute list (see
  // toDevice() below), or null if it doesn't report one at all - most BLE toys don't. Used to ask
  // for a reading (see requestBatteryReadings()); consumers that just want to display the level
  // can ignore this and read `battery` alone.
  batterySensorIndex: number | null
  // null until a reading actually comes back (or forever, if batterySensorIndex is null) - not
  // distinguished from "still loading" here, since in practice a reading arrives moments after
  // requestBatteryReadings() fires and the difference isn't worth surfacing to the UI.
  battery: number | null
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

const BATTERY_POLL_INTERVAL = 30000

let socket: WebSocket | null = null
let messageId = 1
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let batteryPollTimer: ReturnType<typeof setInterval> | null = null

const nextId = (): number => messageId++

// Every ScalarCmd-capable actuator a device has - vibrate, rotate, oscillate, and whatever else a
// given toy exposes - not just vibration-capable ones, so the toy picker can offer all of them.
// The index is each entry's position in this array (a ScalarCmd targeting this device addresses
// actuators by that position, regardless of type).
const toDevice = (info: ButtplugDeviceInfo): IntifaceDevice => {
  const batterySensorIndex = (info.DeviceMessages.SensorReadCmd ?? []).findIndex(
    (sensor) => sensor.SensorType === 'Battery'
  )

  return {
    index: info.DeviceIndex,
    name: info.DeviceName,
    actuators: (info.DeviceMessages.ScalarCmd ?? []).map((actuator, index) => {
      const actuatorType = actuator.ActuatorType ?? 'Vibrate'

      return {
        index,
        actuatorType,
        description: actuator.FeatureDescriptor || actuatorType
      }
    }),
    batterySensorIndex: batterySensorIndex === -1 ? null : batterySensorIndex,
    battery: null
  }
}

const send = (messages: unknown[]): void => {
  if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify(messages))
}

// Fired right after the device list changes and on a timer while connected (see connect()/
// disconnect() below) - a toy's reported charge isn't pushed by the server on its own, so this is
// the only way the popover's battery reading stays anywhere close to current.
const requestBatteryReadings = (): void => {
  devices.value.forEach((device) => {
    if (device.batterySensorIndex === null) return

    send([
      {
        SensorReadCmd: {
          Id: nextId(),
          DeviceIndex: device.index,
          SensorIndex: device.batterySensorIndex,
          SensorType: 'Battery'
        }
      }
    ])
  })
}

const clearReconnectTimer = (): void => {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
}

const stopBatteryPolling = (): void => {
  if (batteryPollTimer) {
    clearInterval(batteryPollTimer)
    batteryPollTimer = null
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
      stopBatteryPolling()
      batteryPollTimer = setInterval(requestBatteryReadings, BATTERY_POLL_INTERVAL)
    } else if ('DeviceList' in message) {
      const map = new Map<number, IntifaceDevice>()

      message.DeviceList.Devices.forEach((info) => {
        const device = toDevice(info)

        if (device.actuators.length) map.set(device.index, device)
      })

      devices.value = map
      requestBatteryReadings()
    } else if ('DeviceAdded' in message) {
      const device = toDevice(message.DeviceAdded)

      if (device.actuators.length) {
        const next = new Map(devices.value)

        next.set(device.index, device)
        devices.value = next
        requestBatteryReadings()
      }
    } else if ('DeviceRemoved' in message) {
      const next = new Map(devices.value)

      next.delete(message.DeviceRemoved.DeviceIndex)
      devices.value = next
    } else if ('SensorReading' in message) {
      const { DeviceIndex, SensorType, Data } = message.SensorReading

      if (SensorType !== 'Battery') return

      const device = devices.value.get(DeviceIndex)

      if (!device || Data[0] === undefined) return

      const next = new Map(devices.value)

      next.set(DeviceIndex, { ...device, battery: Data[0] })
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
    stopBatteryPolling()

    if (enabled.value) {
      status.value = 'error'
      scheduleReconnect()
    }
  }
}

const disconnect = (): void => {
  clearReconnectTimer()
  stopBatteryPolling()
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
  setActuatorIntensity: (
    actuators: { deviceIndex: number; actuatorIndex: number; actuatorType: string }[],
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
  // Each actuator carries its own ActuatorType (vibrate/rotate/oscillate/...), captured at
  // selection time on the control itself (see IntifaceVibratorRef) - it has to match what the
  // device actually reports for that index, or the Buttplug server rejects the whole command.
  const setActuatorIntensity = (
    actuators: { deviceIndex: number; actuatorIndex: number; actuatorType: string }[],
    value: number
  ): void => {
    if (status.value !== 'connected') return

    const byDevice = new Map<number, { index: number; actuatorType: string }[]>()

    actuators.forEach((actuator) => {
      if (!byDevice.has(actuator.deviceIndex)) byDevice.set(actuator.deviceIndex, [])

      byDevice.get(actuator.deviceIndex)!.push({
        index: actuator.actuatorIndex,
        actuatorType: actuator.actuatorType
      })
    })

    byDevice.forEach((deviceActuators, deviceIndex) => {
      send([
        {
          ScalarCmd: {
            Id: nextId(),
            DeviceIndex: deviceIndex,
            Scalars: deviceActuators.map(({ index, actuatorType }) => ({
              Index: index,
              Scalar: value,
              ActuatorType: actuatorType
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
    setActuatorIntensity
  }
}

// Reconnect automatically at module load if the user previously enabled Intiface (connect() is a
// no-op otherwise), so `isAvailable` reflects reality before Settings is ever opened - same idea
// as useOpenShock's auto-validate-on-load.
connect()
