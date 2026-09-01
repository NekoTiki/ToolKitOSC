// DOM-mutating theme core, shared by both apps' theme composables. The desktop client is the
// theme *authority* (its own useTheme.ts wraps this with selectedPrimary/selectedSecondary +
// localStorage persistence and broadcasts choices over the host WS connection); the server's
// viewer (useClientTheme.ts) just applies/clears whatever the host broadcasts.
//
// Constants/types live in ../types/theme.ts, not here — see that file for why.
import type { ColorFamily } from '../types/theme'
import { SHADES } from '../types/theme'

export type { ColorFamily, ColorFamilyWithoutNeutral } from '../types/theme'
export { COLOR_FAMILIES, NEUTRAL_COLOR_FAMILIES, SHADES } from '../types/theme'

export function applyThemeColor(target: 'primary' | 'secondary', color: ColorFamily): void {
  const root = document.documentElement

  root.style.setProperty(`--ui-color-${target}`, `var(--color-${color}-500)`)

  SHADES.forEach((shade) => {
    const targetVar = `--ui-color-${target}-${shade}`
    const sourceVar = `--color-${color}-${shade}`

    root.style.setProperty(targetVar, `var(${sourceVar})`)
  })
}

export function clearThemeColor(target: 'primary' | 'secondary'): void {
  const root = document.documentElement

  root.style.removeProperty(`--ui-color-${target}`)
  SHADES.forEach((shade) => root.style.removeProperty(`--ui-color-${target}-${shade}`))
}
