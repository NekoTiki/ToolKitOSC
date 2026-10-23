import { useChatbox } from '@renderer/composables/useChatbox'
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

// Kept by Rust (src-tauri/src/setup/stop.rs), across restarts: a crash or a relaunch must never
// quietly hand control back. The last value seen is kept here too, so the first frame shows it
// and starting the app doesn't look like a change.
const cached = localStorage.getItem(PAUSED_STORAGE_KEY)
const paused = ref(cached === 'true')

const follow = (value: boolean): void => {
  paused.value = value
  localStorage.setItem(PAUSED_STORAGE_KEY, String(value))
}

api.onPausedChanged(follow)

// Before Rust kept it, this was the only copy: Rust takes it over the first time (a no-op after).
void (cached === null ? Promise.resolve() : api.stopMigrate(cached === 'true'))
  .catch((error) => console.error('Could not move Stop everything to the app:', error))
  .finally(() => void api.stopPaused().then(follow))
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
  const toast = useToast()

  const stop = (): void => {
    // Every add-on that's on stops: OpenShock drops shocks still waiting for their animation and
    // sends a Stop to every shocker on the account, Intiface stops every pattern and toy (not only
    // the ones a control drives), and every output of every ESP32 board goes off. Then Rust pauses.
    paused.value = true
    void api.addonsStopEverything().then((failures) =>
      failures.forEach((failure) =>
        toast.add({
          title: failure.id === 'openshock' ? 'Could not reach OpenShock to stop the shockers' : `Could not stop ${failure.id}`,
          description: failure.message,
          color: 'error'
        })
      )
    )
  }

  const resume = (): void => {
    paused.value = false
    void api.stopResume()
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

// The hotkey, and what follows Stop everything turning on or off from anywhere (the app, the panel,
// an automation, an assistant, TKOSC/Panic, which Rust handles). Called once, from App.vue.
export function useStopEverythingTriggers(): void {
  const { paused, registerHotkey } = useStopEverything()
  const { announceNotice } = useChatbox()
  const { controlValue: toyValues, setValue: setToyValue } = useIntifaceControl()
  const toast = useToast()

  void registerHotkey()

  watch(paused, (value) => {
    // So people in the instance know why nothing reacts.
    announceNotice(value ? 'Viewer controls paused' : 'Viewer controls are back on')

    if (value) {
      // Every toy slider back to 0, so the app and viewers don't show toys running after a resume.
      toyValues.value.forEach((level, controlId) => {
        if (level > 0) setToyValue(controlId, 0)
      })
      toast.add({ title: 'Everything stopped', description: "Viewers can't use your controls until you resume.", icon: 'i-lucide-octagon-x', color: 'error' })
    } else {
      toast.add({ title: 'Controls resumed', description: 'Viewers can use your controls again.', icon: 'i-lucide-play', color: 'success' })
    }
  })
}
