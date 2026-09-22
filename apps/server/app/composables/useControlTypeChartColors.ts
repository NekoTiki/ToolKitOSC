import type { ControlTypes } from '@vrc-osc-toolkit/shared-ui'
import { CONTROL_TYPE_LABELS } from '@vrc-osc-toolkit/shared-ui'

// A fixed tailwind-palette entry per control type, for chart legends/bars - not CONTROL_TYPE_COLORS
// (Nuxt UI semantic names like 'primary'/'neutral', used for badges elsewhere in this app), since
// those only resolve to an actual color via Nuxt UI's own runtime theming, not the plain
// --color-{name}-{shade} vars tailwindColor() reads. Picked to stay visually distinct from each
// other, same approach as stats.vue's own hardcoded provider palette.
const CONTROL_TYPE_PALETTE: Record<ControlTypes, string> = {
  boolean: 'indigo-500',
  'boolean-group': 'blue-500',
  'boolean-enum': 'cyan-500',
  enum: 'teal-500',
  slider: 'green-500',
  'step-enum': 'lime-500',
  'open-shock-shocker': 'rose-500',
  'intiface-toy': 'fuchsia-500',
  'intiface-pattern': 'purple-500',
  preset: 'orange-500'
}

export function useControlTypeChartColors() {
  function colorFor(type: string): string {
    return tailwindColor(CONTROL_TYPE_PALETTE[type as ControlTypes] ?? 'neutral-500')
  }

  function labelFor(type: string): string {
    return CONTROL_TYPE_LABELS[type as ControlTypes] ?? type
  }

  return { colorFor, labelFor }
}
