import { ref } from 'vue'

import type { OwoMuscle } from '../types/controls'

// What the wearer's OWO Skin plays right now, for every OWO tile: 0-100 per muscle that plays, and
// the sides avatar touch drives (viewers' taps there wait). The host fills it from the app and
// sends it on as 'owo-value-update'; viewers fill it from that.
export interface OwoControlState {
  playing: Partial<Record<OwoMuscle, number>>
  touched: ('front' | 'back')[]
}

// Module-scope singleton, like useBhapticsControl.ts: one suit, whatever tile shows it.
const state = ref<OwoControlState>({ playing: {}, touched: [] })

export function useOwoControl(): {
  state: typeof state
  setState: (value: OwoControlState) => void
} {
  const setState = (value: OwoControlState): void => {
    if (JSON.stringify(value) === JSON.stringify(state.value)) return

    state.value = value
  }

  return { state, setState }
}
