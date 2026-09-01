import { ref } from 'vue'

import type { OpenShockControlValue } from '../types/openShock'

export type { OpenShockControlValue }

const controlValue = ref<Map<string, OpenShockControlValue>>(new Map())

const lastUpdate = ref<{ controlId: string; value: OpenShockControlValue } | null>(null)

export function useOpenShockControl(): {
  controlValue: typeof controlValue
  lastUpdate: typeof lastUpdate
  setValue: (controlId: string, value: OpenShockControlValue) => void
} {
  const setValue = (controlId: string, value: OpenShockControlValue): void => {
    const next = new Map(controlValue.value)

    next.set(controlId, value)
    controlValue.value = next

    lastUpdate.value = { controlId, value }
  }

  return { controlValue, lastUpdate, setValue }
}
