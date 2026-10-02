import type { ControlTypes } from '@vrc-osc-toolkit/shared-ui'
import { CONTROL_TYPE_LABELS } from '@vrc-osc-toolkit/shared-ui'

// Human labels for control types in the dashboard charts. Per-type colors are gone: the charts
// rank types as single-color bars (see components/admin/RankedBars.vue), so a type's name - not
// its color - identifies it.
export function useControlTypeChartColors(): { labelFor: (type: string) => string } {
  function labelFor(type: string): string {
    return CONTROL_TYPE_LABELS[type as ControlTypes] ?? type
  }

  return { labelFor }
}
