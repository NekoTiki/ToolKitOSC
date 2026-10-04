import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

const DEFAULT_INTIFACE_URL = 'ws://localhost:12345'
const URL_STORAGE_KEY = 'intiface_url'
const ENABLED_STORAGE_KEY = 'intiface_enabled'
const AUTO_LAUNCH_STORAGE_KEY = 'intiface_autoLaunch'
const CLIENT_NAME = 'ToolKitOSC'
const RECONNECT_DELAY = 3000

// Auto-launch: how many times Intiface Central gets opened before giving up, and how long each
// launch gets to start its server before the next one - without that gap the 3s reconnect loop
// would open it 5 times in 15s.
const MAX_LAUNCH_ATTEMPTS = 5
const LAUNCH_RETRY_DELAY = 15000
const LOCAL_HOSTS = ['localhost', '127.0.0.1', '::1']

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
// 'error': the last attempt failed and nothing is listening at the URL (Intiface Central isn't
// running, or its server isn't started); a retry is still scheduled while `enabled` stays true.
// 'refused': something is listening at the URL but wouldn't accept us (e.g. another app is
// already connected to Intiface Central) - launching it again wouldn't help, so auto-launch skips it.
// 'busy': nothing is listening, but Intiface Central is running on this machine. It stops
// listening while a client is connected, so either another app holds the connection or its server
// is stopped. Launching another copy wouldn't help either, so auto-launch skips it.
// 'launching': auto-launch opened Intiface Central and is giving it time to start its server.
// Fail-closed like OpenShockStatus: only 'connected' counts as available.
export type IntifaceStatus = 'disabled' | 'connecting' | 'connected' | 'error' | 'refused' | 'busy' | 'launching'

// Module-scope singletons - every useIntiface() caller shares the same connection/device list, so
// a change made in Settings (or a toy being plugged in) is instantly reflected everywhere
// (ControlModal's type list and toy picker, useControls' `unavailable` flag), same pattern as
// useOpenShock.ts's token/status.
const url = ref<string>(localStorage.getItem(URL_STORAGE_KEY) || '')
const enabled = ref<boolean>(localStorage.getItem(ENABLED_STORAGE_KEY) === 'true')
const status = ref<IntifaceStatus>('disabled')
const devices = ref<Map<number, IntifaceDevice>>(new Map())

// Whether a local Intiface Central install was found on this machine - checked once at module
// load (see the bottom of this file) so the "connecting"/"error" status popover can offer to
// launch it instead of just reporting the failure.
const installed = ref(false)

const autoLaunch = ref<boolean>(localStorage.getItem(AUTO_LAUNCH_STORAGE_KEY) === 'true')
// Launches since the last successful connection - reset once it connects.
const launchAttempts = ref(0)
// Intiface's own error message when it refused the handshake, if it sent one.
const refusedReason = ref('')

const BATTERY_POLL_INTERVAL = 30000

let socket: WebSocket | null = null
let messageId = 1
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let batteryPollTimer: ReturnType<typeof setInterval> | null = null
let lastLaunchAt = 0
// Per connection attempt: whether the handshake got as far as ServerInfo, and the error Intiface
// replied with before that, if any.
let handshakeDone = false
let handshakeError = ''

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
      handshakeDone = true
      status.value = 'connected'
      resetLaunchAttempts()
      // Bluetooth toys only show up once the server scans for them - start it here so nobody has
      // to press "Start Scanning" in Intiface Central. Found toys arrive as DeviceAdded below; an
      // Error reply (no adapter, already scanning) is only logged now that the handshake is done.
      send([{ RequestDeviceList: { Id: nextId() } }, { StartScanning: { Id: nextId() } }])
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

      // Rejected during the handshake: close, and let onclose work out why (see diagnoseFailure).
      if (!handshakeDone) {
        handshakeError = message.Error.ErrorMessage
        socket?.close()
      }
    }
  })
}

const resetLaunchAttempts = (): void => {
  launchAttempts.value = 0
  lastLaunchAt = 0
  refusedReason.value = ''
}

// Host and port the WebSocket URL points at, for intifacePortOpen. `new URL` keeps the brackets
// around an IPv6 hostname, which a socket address doesn't want.
const parseTarget = (): { host: string; port: number } | null => {
  try {
    const parsed = new URL(url.value || DEFAULT_INTIFACE_URL)

    return {
      host: parsed.hostname.replace(/^\[(.*)\]$/, '$1'),
      port: Number(parsed.port) || (parsed.protocol === 'wss:' ? 443 : 80)
    }
  } catch {
    return null
  }
}

// Opens Intiface Central if auto-launch applies. Only called once it's known not to be running.
// Returns whether a launch was made, so the status can say so.
const maybeAutoLaunch = async (): Promise<boolean> => {
  await installedCheck

  if (!autoLaunch.value || !installed.value || launchAttempts.value >= MAX_LAUNCH_ATTEMPTS) return false

  lastLaunchAt = Date.now()
  launchAttempts.value++

  try {
    await api.startIntifaceCentral()
  } catch (error) {
    console.error('Failed to launch Intiface Central:', error)
  }

  return true
}

// A connection attempt failed before the handshake finished. The WebSocket API can't say why - a
// closed port and a server that hangs up on us look the same - so ask the backend:
// - something is listening: Intiface is there but refused us ('refused').
// - a launch is still within its LAUNCH_RETRY_DELAY: give it time ('launching').
// - nothing listening, but Intiface Central is running: busy with another client or its server is
//   stopped ('busy'). Checked by process, since it stops listening while a client is connected.
// - not running: launch it if auto-launch is on ('launching'), else 'error'.
// Only a local URL gets the process check and auto-launch: for an Intiface on another machine,
// what runs here says nothing.
const diagnoseFailure = async (): Promise<void> => {
  const reason = handshakeError
  const target = parseTarget()
  const local = !!target && LOCAL_HOSTS.includes(target.host)
  const portOpen = target ? await api.intifacePortOpen(target.host, target.port).catch(() => false) : false
  const launchPending = !!lastLaunchAt && Date.now() - lastLaunchAt < LAUNCH_RETRY_DELAY
  // Unknown counts as running: never risk opening a second copy.
  const running = !portOpen && !launchPending && local ? await api.intifaceCentralRunning().catch(() => true) : false

  // Turned off, or a new attempt (URL change) started while this was checking.
  if (!enabled.value || socket) return

  refusedReason.value = portOpen ? reason : ''

  if (portOpen) status.value = 'refused'
  else if (launchPending) status.value = 'launching'
  else if (running) status.value = 'busy'
  else status.value = local && (await maybeAutoLaunch()) ? 'launching' : 'error'

  if (enabled.value && !socket) scheduleReconnect()
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
  // Keep showing 'launching'/'refused' across retries instead of flickering to 'connecting' every 3s.
  if (status.value !== 'launching' && status.value !== 'refused' && status.value !== 'busy') status.value = 'connecting'
  handshakeDone = false
  handshakeError = ''

  const ws = new WebSocket(url.value || DEFAULT_INTIFACE_URL)

  socket = ws

  ws.onopen = () => {
    send([{ RequestServerInfo: { Id: nextId(), ClientName: CLIENT_NAME, MessageVersion: 3 } }])
  }
  // Everything below ignores a socket that's since been replaced (setUrl reconnects right away,
  // and the old socket's close event lands after the new one already exists).
  ws.onmessage = (ev) => {
    if (socket === ws) handleServerMessage(ev.data)
  }
  // Always followed by onclose, which sets the status.
  ws.onerror = () => {}
  ws.onclose = () => {
    if (socket !== ws) return

    devices.value = new Map()
    socket = null
    stopBatteryPolling()

    if (!enabled.value) return

    // Dropped after being connected: retry as before, and diagnose that attempt if it fails.
    if (handshakeDone) {
      status.value = 'connecting'
      scheduleReconnect()
    } else {
      void diagnoseFailure()
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
  installed: Ref<boolean>
  launch: () => Promise<void>
  autoLaunch: Ref<boolean>
  setAutoLaunch: (value: boolean) => void
  launchAttempts: Ref<number>
  maxLaunchAttempts: number
  refusedReason: Ref<string>
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
      resetLaunchAttempts()
      connect()
    }
  }

  const setEnabled = (value: boolean): void => {
    enabled.value = value
    localStorage.setItem(ENABLED_STORAGE_KEY, String(value))
    resetLaunchAttempts()

    if (value) connect()
    else disconnect()
  }

  // Turning it on gives a fresh 5 tries, and retries right away if Intiface isn't running now
  // rather than waiting for the next scheduled attempt.
  const setAutoLaunch = (value: boolean): void => {
    autoLaunch.value = value
    localStorage.setItem(AUTO_LAUNCH_STORAGE_KEY, String(value))

    if (!value) return

    resetLaunchAttempts()

    if (enabled.value && status.value === 'error' && !socket) connect()
  }

  const isAvailable = computed(() => status.value === 'connected')

  // Launches the local install found by the module-load check below, then lets the normal
  // connect()/scheduleReconnect() retry loop (already running whenever `enabled`) pick up the
  // connection once the user starts the server inside Intiface Central - this only opens the app.
  const launch = async (): Promise<void> => {
    try {
      await api.startIntifaceCentral()
    } catch (error) {
      console.error('Failed to launch Intiface Central:', error)
    }
  }

  // Batches per device (a single ScalarCmd targets one DeviceIndex but can carry several
  // Scalars), so a control spanning multiple toys sends one message per toy, not per actuator.
  // Each actuator carries its own ActuatorType (vibrate/rotate/oscillate/...), captured at
  // selection time on the control itself (see IntifaceActuatorRef) - it has to match what the
  // device actually reports for that index, or the Buttplug server rejects the whole command.
  const setActuatorIntensity = (
    actuators: { deviceIndex: number; actuatorIndex: number; actuatorType: string }[],
    value: number
  ): void => {
    if (status.value !== 'connected') return

    const byDevice = new Map<number, { index: number; actuatorType: string }[]>()

    actuators.forEach((actuator) => {
      // A control spanning several toys stays usable while any one is connected (see useControls),
      // but a toy that's gone would reject every command with DeviceNotAvailable - skip it.
      if (!devices.value.has(actuator.deviceIndex)) return
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
    installed,
    launch,
    autoLaunch,
    setAutoLaunch,
    launchAttempts,
    maxLaunchAttempts: MAX_LAUNCH_ATTEMPTS,
    refusedReason,
    setActuatorIntensity
  }
}

// Reconnect automatically at module load if the user previously enabled Intiface (connect() is a
// no-op otherwise), so `isAvailable` reflects reality before Settings is ever opened - same idea
// as useOpenShock's auto-validate-on-load.
connect()

// Checked once at module load, same reasoning as connect() above - a filesystem check, not a
// network one, so no retry loop is needed. Kept as a promise so auto-launch can wait for the
// answer: the first connection attempt above can fail before it arrives.
const installedCheck: Promise<void> = api
  .intifaceCentralAvailable()
  .then((value) => {
    installed.value = value
  })
  .catch(() => {
    installed.value = false
  })
