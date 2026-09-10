import { ref } from 'vue'

// Same pattern as useIntifaceControl.ts: the currently-playing pattern id (or 'off') for an
// 'intiface-pattern' control, kept as a module-scope singleton so every viewer's chip picker (and
// the host's own) shows the same choice, synced via 'intiface-pattern-value-update'.
export type IntifacePatternControlValue = string

const controlValue = ref<Map<string, IntifacePatternControlValue>>(new Map())

const lastUpdate = ref<{ controlId: string; value: IntifacePatternControlValue } | null>(null)

export function useIntifacePatternControl(): {
  controlValue: typeof controlValue
  lastUpdate: typeof lastUpdate
  setValue: (controlId: string, value: IntifacePatternControlValue) => void
} {
  const setValue = (controlId: string, value: IntifacePatternControlValue): void => {
    const next = new Map(controlValue.value)

    next.set(controlId, value)
    controlValue.value = next

    lastUpdate.value = { controlId, value }
  }

  return { controlValue, lastUpdate, setValue }
}
