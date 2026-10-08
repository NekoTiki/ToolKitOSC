import { useChatbox } from '@renderer/composables/useChatbox'
import { cancelPendingShocks } from '@renderer/composables/useControls'
import { useIntiface } from '@renderer/composables/useIntiface'
import { useIntifacePatterns } from '@renderer/composables/useIntifacePatterns'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { api } from '@renderer/lib/tauri-bridge'
import { isRegistered, register, unregister } from '@tauri-apps/plugin-global-shortcut'
import { useIntifaceControl } from '@toolkitosc/shared-ui'
import type { Ref } from 'vue'
import { ref, watch } from 'vue'

const PAUSED_STORAGE_KEY = 'stopEverything_paused'
const HOTKEY_STORAGE_KEY = 'stopEverything_hotkey'

// In Tauri's accelerator format. Not Ctrl+Pause: Windows turns Pause into Break while Ctrl is held,
// so that hotkey never fires.
export const DEFAULT_STOP_HOTKEY = 'CommandOrControl+Shift+F12'

// Turning this Bool on (e.g. from the expression menu) stops everything. Turning it off does
// nothing, so a slip in the menu can't give control back - resume from the app or the hotkey.
export const PANIC_PARAMETER = 'TKOSC/Panic'
export const PANIC_ADDRESS = `/avatar/parameters/${PANIC_PARAMETER}`

// 'CommandOrControl+Shift+KeyP' -> 'Ctrl+Shift+P', for labels.
export const formatHotkey = (value: string): string =>
  value
    .split('+')
    .map((part) => (part === 'CommandOrControl' ? 'Ctrl' : part.replace(/^(Key|Digit)/, '')))
    .join('+')

// Stays paused across restarts: a crash or a relaunch must never quietly hand control back.
const paused = ref(localStorage.getItem(PAUSED_STORAGE_KEY) === 'true')
// Empty when the hotkey is turned off.
const hotkey = ref(localStorage.getItem(HOTKEY_STORAGE_KEY) ?? DEFAULT_STOP_HOTKEY)
// Why the hotkey couldn't be registered (usually another app already uses it).
const hotkeyError = ref<string | null>(null)

let registeredHotkey: string | null = null

export function useStopEverything(): {
  paused: Ref<boolean>
  hotkey: Ref<string>
  hotkeyError: Ref<string | null>
  // Stops every toy, pattern and shocker, then rejects viewer commands until resume().
  stop: () => void
  resume: () => void
  toggle: () => void
  // Saves the hotkey ('' turns it off) and registers it.
  setHotkey: (value: string) => Promise<void>
  // Registers the saved hotkey. Called once at startup (see useStopEverythingTriggers).
  registerHotkey: () => Promise<void>
} {
  const { stopAllDevices } = useIntiface()
  const { stopAllPatterns } = useIntifacePatterns()
  const { controlValue: toyValues, setValue: setToyValue } = useIntifaceControl()
  const { command: openShockCommand, isAvailable: openShockAvailable, shockerNames } = useOpenShock()
  const toast = useToast()

  const setPaused = (value: boolean): void => {
    paused.value = value
    localStorage.setItem(PAUSED_STORAGE_KEY, String(value))
  }

  const stop = (): void => {
    cancelPendingShocks()
    stopAllPatterns()
    // Every toy slider back to 0, so the app and viewers don't show toys running after a resume.
    toyValues.value.forEach((value, controlId) => {
      if (value > 0) setToyValue(controlId, 0)
    })
    // Also catches toys no control of this avatar drives.
    stopAllDevices()
    // Every output of every ESP32 board.
    void api.boardsStopAll()

    // Every shocker on the account, not only the ones a control uses.
    if (openShockAvailable.value && shockerNames.value.size) {
      const shockers = Array.from(shockerNames.value.keys()).map((id) => ({ id, type: 'Stop' as const, intensity: 0, duration: 300 }))

      openShockCommand(shockers).catch(() => toast.add({ title: 'Could not reach OpenShock to stop the shockers', color: 'error' }))
    }

    if (paused.value) return

    setPaused(true)
    toast.add({
      title: 'Everything stopped',
      description: "Viewers can't use your controls until you resume.",
      icon: 'i-lucide-octagon-x',
      color: 'error'
    })
  }

  const resume = (): void => {
    if (!paused.value) return

    setPaused(false)
    toast.add({ title: 'Controls resumed', description: 'Viewers can use your controls again.', icon: 'i-lucide-play', color: 'success' })
  }

  const toggle = (): void => {
    if (paused.value) resume()
    else stop()
  }

  const registerHotkey = async (): Promise<void> => {
    const value = hotkey.value

    if (registeredHotkey) {
      await unregister(registeredHotkey).catch(() => undefined)
      registeredHotkey = null
    }

    hotkeyError.value = null

    if (!value) return

    try {
      // Another app (or a second window of this one) may hold it already.
      if (await isRegistered(value)) await unregister(value)

      await register(value, (event) => {
        if (event.state === 'Pressed') toggle()
      })
      registeredHotkey = value
    } catch (error) {
      console.error('Failed to register the Stop everything hotkey:', error)
      hotkeyError.value = "Couldn't register this hotkey. Another app may already use it, so pick a different one."
    }
  }

  const setHotkey = async (value: string): Promise<void> => {
    hotkey.value = value
    localStorage.setItem(HOTKEY_STORAGE_KEY, value)

    await registerHotkey()
  }

  return { paused, hotkey, hotkeyError, stop, resume, toggle, setHotkey, registerHotkey }
}

// The triggers that work while the app isn't in front (the hotkey and the avatar parameter), and the
// chatbox notice. Called once, from App.vue.
export function useStopEverythingTriggers(): void {
  const { paused, stop, registerHotkey } = useStopEverything()
  const { announceNotice } = useChatbox()

  void registerHotkey()

  // So people in the instance know why nothing reacts.
  watch(paused, (value) => announceNotice(value ? 'Viewer controls paused' : 'Viewer controls are back on'))

  // Live changes only, not the values read when an avatar loads: a parameter saved as on must not
  // pause you every time you switch into that avatar. Works while already paused too: it stops
  // anything you turned on yourself since.
  api.onOscMessage((msg) => {
    if (msg.address !== PANIC_ADDRESS) return

    const [value] = msg.args

    if (value === true || value === 1) stop()
  })
}
