import { ref } from 'vue'

import type { VoiceFxControlState } from '../types/protocol'

// Which Voice effects controls are on or cooling down, by control id. The host fills it from the
// app and sends it on as 'voicefx-value-update'; viewers fill it from that.
// Module-scope singleton, like useOwoControl.ts: one voice, whatever tile shows it.
const controls = ref<Record<string, VoiceFxControlState>>({})

export function useVoiceFxControl(): {
  controls: typeof controls
  setControls: (value: Record<string, VoiceFxControlState>) => void
} {
  const setControls = (value: Record<string, VoiceFxControlState>): void => {
    if (JSON.stringify(value) === JSON.stringify(controls.value)) return

    controls.value = value
  }

  return { controls, setControls }
}
