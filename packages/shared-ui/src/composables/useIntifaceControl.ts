import { ref } from 'vue'

// The raw slider value (0-1) last sent to an 'intiface-vibrator' control's actuators. Unlike
// OpenShockControlValue there's nothing to animate here - it's just the live position - so this
// is a plain number alias, not a struct.
export type IntifaceControlValue = number

// Module-scope singletons, same pattern as useOpenShockControl.ts: every caller (the host, and
// every viewer that receives an 'intiface-value-update') shares the same map, so a control's
// slider always reflects the value actually last sent, not just what this instance dragged it to.
const controlValue = ref<Map<string, IntifaceControlValue>>(new Map())

const lastUpdate = ref<{ controlId: string; value: IntifaceControlValue } | null>(null)

export function useIntifaceControl(): {
  controlValue: typeof controlValue
  lastUpdate: typeof lastUpdate
  setValue: (controlId: string, value: IntifaceControlValue) => void
} {
  const setValue = (controlId: string, value: IntifaceControlValue): void => {
    const next = new Map(controlValue.value)

    next.set(controlId, value)
    controlValue.value = next

    lastUpdate.value = { controlId, value }
  }

  return { controlValue, lastUpdate, setValue }
}
