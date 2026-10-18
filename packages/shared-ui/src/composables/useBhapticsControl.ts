import { ref } from 'vue'

import type { BhapticsPosition } from '../types/controls'

// What the wearer's bHaptics gear plays right now, for every bHaptics tile: 0-100 per motor for
// each position that plays, and the positions avatar touch drives (viewers' taps there wait). The
// host fills it from the app and sends it on as 'bhaptics-value-update'; viewers fill it from that.
export interface BhapticsControlState {
  playing: Partial<Record<BhapticsPosition, number[]>>
  touched: BhapticsPosition[]
}

// Module-scope singleton, like useIntifaceControl.ts: one body, whatever tile shows it.
const state = ref<BhapticsControlState>({ playing: {}, touched: [] })

export function useBhapticsControl(): {
  state: typeof state
  setState: (value: BhapticsControlState) => void
} {
  const setState = (value: BhapticsControlState): void => {
    if (JSON.stringify(value) === JSON.stringify(state.value)) return

    state.value = value
  }

  return { state, setState }
}
