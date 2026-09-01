import { ref } from 'vue'

import type { OSCArg } from '../types/osc'

// Read-only OSC-state singleton. Both apps' control-rendering components read through `get()`
// regardless of who populates `args` — the desktop client's populate-side composable mutates it
// from window.api/Tauri events, the server's viewer populates it from `args-initial`/`args-update`
// WS messages. Each app's own useOscMessages.ts wraps this with its populate-side logic.
export const args = ref<Record<string, OSCArg[]>>({})

export function useOscMessages(): {
  args: typeof args
  get: <T>(address: string, defaultValue: T) => T
} {
  function get<T>(address: string, defaultValue: T): T {
    return (args.value?.[address]?.[0] as T) ?? defaultValue
  }

  return { args, get }
}
