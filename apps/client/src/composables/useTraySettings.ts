import { api } from '@renderer/lib/tauri-bridge'
import type { Ref } from 'vue'
import { ref } from 'vue'

const STORAGE_KEY = 'minimizeToTray_enabled'

// Module-scope singleton, same pattern as the other Settings toggles.
const enabled = ref<boolean>(localStorage.getItem(STORAGE_KEY) === 'true')

export function useTraySettings(): {
  enabled: Ref<boolean>
  setEnabled: (value: boolean) => void
} {
  const setEnabled = (value: boolean): void => {
    enabled.value = value
    localStorage.setItem(STORAGE_KEY, String(value))
    api.setMinimizeToTray(value)
  }

  return { enabled, setEnabled }
}

// The window's native close handler lives in Rust (lib.rs), out of reach of this localStorage
// value - push it over at module load so a value from a previous session takes effect immediately,
// not just after the user next touches the toggle.
api.setMinimizeToTray(enabled.value)
